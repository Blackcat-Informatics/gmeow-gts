# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""COSE signing & encryption for GTS (§9.2/§9.3, issue #272).

A focused, dependency-light RFC 9052 subset built on ``cryptography`` + ``cbor2``:

* **COSE_Sign1 (detached), EdDSA/Ed25519** — signs a frame's ``"id"`` (the payload
  is detached, since the id is already in the frame). Verification recomputes the
  id and checks the signature against a key resolved by ``kid`` (§9.2).
* **COSE_Encrypt0, AES-256-GCM** — seals a payload to a recipient keyed by ``kid``;
  decryption needs that key, else the frame degrades to a ``missing-key`` opaque
  node (§9.3, §8.3).

Key discovery / trust anchoring is **deployment policy** (a :class:`KeyProvider`):
``sigstat == "valid"`` means cryptographically valid under a *resolved* key, not
that the key is trusted (§9.2).
"""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Protocol

import cbor2
from cryptography.exceptions import InvalidSignature, InvalidTag
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

from gts.codec import CodecUnavailableError
from gts.openpgp import load_secret_key as _load_gpg_secret_key
from gts.openpgp import public_key_fingerprint as _gpg_fingerprint
from gts.wire import loads as _loads

# COSE header labels / algorithm ids (RFC 9052 §3.1, IANA COSE registries).
_ALG, _KID, _IV = 1, 4, 5
_ALG_EDDSA, _ALG_A256GCM = -8, 3
_TAG_SIGN1, _TAG_ENCRYPT0 = 18, 16
_ED25519_SIGNATURE_BYTES = 64

SigStatus = str  # "none" | "valid" | "invalid" | "unverified"


@dataclass(frozen=True)
class Signer:
    """A signing identity: a ``kid`` and its Ed25519 private key."""

    kid: str
    key: Ed25519PrivateKey  # gitleaks:allow

    @staticmethod
    def generate(kid: str) -> Signer:
        """Generate a fresh Ed25519 signer (test/dev helper)."""
        return Signer(kid, Ed25519PrivateKey.generate())

    @staticmethod
    def from_gpg_secret_key(armored: str, kid: str | None = None) -> Signer:
        """Load an Ed25519 signer from an armored OpenPGP secret key.

        If ``kid`` is omitted, the OpenPGP fingerprint is used.
        """
        private = _load_gpg_secret_key(armored)
        resolved = kid if kid is not None else _gpg_fingerprint(armored)
        return Signer(resolved, private)

    @property
    def public_raw(self) -> bytes:
        """The 32-byte raw Ed25519 public key (for registering a verifier)."""
        return self.key.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw
        )


class KeyProvider(Protocol):
    """Resolves verification / content keys by ``kid`` (deployment policy)."""

    def verification_key(self, kid: str) -> Ed25519PublicKey | None:
        """Return the Ed25519 public key for ``kid`` (``None`` if unknown)."""
        ...

    def content_key(self, kid: str) -> bytes | None:
        """Return the symmetric content key for ``kid`` (``None`` if unknown)."""
        ...


@dataclass
class InMemoryKeys:
    """A simple in-memory :class:`KeyProvider` (tests, single-process tools)."""

    verifiers: dict[str, Ed25519PublicKey]
    content: dict[str, bytes]

    def __init__(
        self,
        verifiers: dict[str, Ed25519PublicKey] | None = None,
        content: dict[str, bytes] | None = None,
    ) -> None:
        """Build a provider from optional verifier and content-key maps."""
        self.verifiers = verifiers if verifiers is not None else {}
        self.content = content if content is not None else {}

    def verification_key(self, kid: str) -> Ed25519PublicKey | None:
        """Return the registered verification key for ``kid``, if any."""
        return self.verifiers.get(kid)

    def content_key(self, kid: str) -> bytes | None:
        """Return the registered content key for ``kid``, if any."""
        return self.content.get(kid)

    def trust(self, signer: Signer) -> None:
        """Register a signer's public key for verification."""
        self.verifiers[signer.kid] = signer.key.public_key()


# -- COSE_Sign1 (detached payload = the frame id) -----------------------------


def sign_id(frame_id: bytes, signer: Signer) -> bytes:
    """Return a detached ``COSE_Sign1`` over ``frame_id`` (§9.2)."""
    protected = cbor2.dumps({_ALG: _ALG_EDDSA}, canonical=True)
    sig_structure = cbor2.dumps(
        ["Signature1", protected, b"", frame_id], canonical=True
    )
    signature = signer.key.sign(sig_structure)
    cose = cbor2.CBORTag(
        _TAG_SIGN1, [protected, {_KID: signer.kid.encode()}, None, signature]
    )
    return cbor2.dumps(cose, canonical=True)


_MAX_HEADER_DEPTH = 8


def _scan_head(buf: bytes, pos: int) -> tuple[int, int, int, int] | None:
    """Read one item head: ``(major, info, argument, next position)``.

    ``None`` for an indefinite length, a reserved additional info, a truncated
    head, or an argument that is not in shortest form.
    """
    if pos >= len(buf):
        return None
    major, info = buf[pos] >> 5, buf[pos] & 0x1F
    if info < 24:
        return major, info, info, pos + 1
    if info > 27:
        return None
    width = 1 << (info - 24)
    raw = buf[pos + 1 : pos + 1 + width]
    if len(raw) != width:
        return None
    arg = int.from_bytes(raw, "big")
    if arg < (24, 1 << 8, 1 << 16, 1 << 32)[info - 24]:
        return None
    return major, info, arg, pos + 1 + width


def _scan_header_item(buf: bytes, pos: int, depth: int, is_key: bool) -> int | None:
    """Check one item of a protected header; return the position after it.

    ``depth`` is the level of this item if it is a container.
    """
    head = _scan_head(buf, pos)
    if head is None:
        return None
    major, info, arg, nxt = head
    if is_key and major not in (0, 1, 3):
        return None
    if major in (0, 1):
        return nxt if arg < 1 << 63 else None  # integers lie in -2^63 .. 2^63-1
    if major in (2, 3):
        end = nxt + arg
        if end > len(buf):
            return None
        if major == 3:
            try:
                buf[nxt:end].decode("utf-8")
            except UnicodeDecodeError:
                return None
        return end
    if major in (4, 5):
        if depth > _MAX_HEADER_DEPTH:
            return None
        at = nxt
        previous: bytes | None = None
        for _ in range(arg):
            if major == 4:
                item_end = _scan_header_item(buf, at, depth + 1, False)
                if item_end is None:
                    return None
                at = item_end
                continue
            key_end = _scan_header_item(buf, at, depth + 1, True)
            if key_end is None:
                return None
            key = buf[at:key_end]
            # Strictly ascending bytewise: sorted, and no duplicate key.
            if previous is not None and previous >= key:
                return None
            previous = key
            value_end = _scan_header_item(buf, key_end, depth + 1, False)
            if value_end is None:
                return None
            at = value_end
        return at
    # null, true and false only.
    return nxt if major == 7 and info in (20, 21, 22) else None


def _declared_alg(protected: bytes) -> int | str | None:
    """Return the ``alg`` declared in a protected header, or ``None`` if malformed.

    The protected header is a byte string whose content is deterministically
    encoded CBOR (definite lengths, shortest-form integers and lengths, map keys
    in bytewise order of their encodings, no duplicate keys, no trailing bytes)
    consisting of exactly one map. Every map key, at any depth, is an integer or
    a text string. Every value, at any depth, is an integer, a byte string, a
    text string, an array, a map, null, true or false. Every integer, key or
    value, lies in the range -2^63 to 2^63-1 inclusive. Text strings are valid
    UTF-8. Nesting is at most 8 levels deep, counting the header map itself as
    level 1. The map contains the integer key 1, whose value is an integer or a
    text string. A header that does not meet all of this is malformed and the
    signature is ``invalid``.

    Enforced by scanning the raw bytes (:func:`_scan_header_item`), so no decoder
    behaviour (float, tag, bignum, bool-key merging, depth limit) is involved.
    """
    if not protected or protected[0] >> 5 != 5:
        return None
    if _scan_header_item(protected, 0, 1, False) != len(protected):
        return None
    head = _scan_head(protected, 0)
    if head is None:  # unreachable: the scan above accepted this head
        return None
    _, _, count, at = head
    for _ in range(count):
        key_end = _scan_header_item(protected, at, 2, True)
        if key_end is None:
            return None
        value_end = _scan_header_item(protected, key_end, 2, False)
        if value_end is None:
            return None
        if protected[at:key_end] == b"\x01":
            value = _scan_head(protected, key_end)
            if value is None:
                return None
            major, _, arg, nxt = value
            if major == 0:
                return arg
            if major == 1:
                return -1 - arg
            if major == 3:
                return protected[nxt:value_end].decode("utf-8")
            return None
        at = value_end
    return None


def verify_sig(
    sig: bytes, frame_id: bytes, provider: KeyProvider
) -> tuple[SigStatus, str | None]:
    """Verify a detached ``COSE_Sign1`` over ``frame_id``; return (status, kid).

    Dispatches on the ``alg`` declared in the protected header (§9.2). EdDSA
    is verified as Ed25519; any other declared algorithm is ``unverified``
    (the engine cannot check it), never ``invalid`` and never ``valid``.
    """
    try:
        msg = _loads(sig)
        body = msg.value if isinstance(msg, cbor2.CBORTag) else msg
        protected, unprotected, _payload, signature = body
        kid_raw = unprotected.get(_KID) if isinstance(unprotected, Mapping) else None
        kid = kid_raw.decode() if isinstance(kid_raw, bytes) else None
    except (cbor2.CBORDecodeError, ValueError, TypeError, KeyError):
        return ("invalid", None)
    if kid is None:
        return ("invalid", None)
    if not isinstance(protected, bytes) or not isinstance(signature, bytes):
        return ("invalid", None)  # RFC 9052 §3: both are bstr — reject malformed
    alg = _declared_alg(protected)
    if alg is None:
        return ("invalid", None)  # protected header malformed or declares no alg
    if alg != _ALG_EDDSA:
        return ("unverified", kid)  # declared algorithm not supported by this engine
    public = provider.verification_key(kid)
    if public is None:
        return ("unverified", kid)  # no key resolved — present but not checked
    if len(signature) != _ED25519_SIGNATURE_BYTES:
        return ("invalid", kid)
    sig_structure = cbor2.dumps(
        ["Signature1", protected, b"", frame_id], canonical=True
    )
    try:
        public.verify(signature, sig_structure)
    except InvalidSignature:
        return ("invalid", kid)
    return ("valid", kid)


# -- COSE_Encrypt0 (AES-256-GCM, keyed by kid) --------------------------------


def _encrypt0_with_iv(plaintext: bytes, kid: str, key: bytes, iv: bytes) -> bytes:
    """Seal ``plaintext`` with an explicit 12-byte ``iv`` (§9.3).

    Splitting the IV out makes the COSE_Encrypt0 transform deterministic, so the
    cross-engine conformance vector can freeze exact bytes; :func:`encrypt0` is
    the production entry point and always supplies a fresh random IV.
    """
    protected = cbor2.dumps({_ALG: _ALG_A256GCM}, canonical=True)
    aad = cbor2.dumps(["Encrypt0", protected, b""], canonical=True)
    ciphertext = AESGCM(key).encrypt(iv, plaintext, aad)
    cose = cbor2.CBORTag(
        _TAG_ENCRYPT0, [protected, {_IV: iv, _KID: kid.encode()}, ciphertext]
    )
    return cbor2.dumps(cose, canonical=True)


def encrypt0(plaintext: bytes, kid: str, key: bytes) -> bytes:
    """Seal ``plaintext`` as a ``COSE_Encrypt0`` to the recipient ``kid`` (§9.3)."""
    return _encrypt0_with_iv(plaintext, kid, key, os.urandom(12))


def decrypt0(blob: bytes, provider: KeyProvider) -> bytes:
    """Open a ``COSE_Encrypt0`` using a key resolved by ``kid``.

    Raises:
        CodecUnavailableError: ``missing-key`` if no key is held for the recipient,
            or if the ciphertext fails authentication.
    """
    try:
        msg = _loads(blob)
        body = msg.value if isinstance(msg, cbor2.CBORTag) else msg
        protected, unprotected, ciphertext = body
        iv = unprotected[_IV]
        kid = unprotected[_KID].decode()
    except (
        cbor2.CBORDecodeError,
        ValueError,
        TypeError,
        KeyError,
        IndexError,
        AttributeError,
    ) as exc:
        raise CodecUnavailableError("missing-key", "malformed COSE_Encrypt0") from exc
    key = provider.content_key(kid)
    if key is None:
        raise CodecUnavailableError("missing-key", f"no content key for {kid!r}")
    if not isinstance(key, bytes) or len(key) != 32:
        raise CodecUnavailableError("missing-key", f"invalid content key for {kid!r}")
    if not (
        isinstance(iv, bytes)
        and isinstance(ciphertext, bytes)
        and isinstance(protected, bytes)
    ):
        raise CodecUnavailableError("missing-key", "malformed COSE_Encrypt0 fields")
    aad = cbor2.dumps(["Encrypt0", protected, b""], canonical=True)
    try:
        return AESGCM(key).decrypt(iv, ciphertext, aad)
    except InvalidTag as exc:
        detail = "authentication failed (AES-GCM tag mismatch)"
        raise CodecUnavailableError("missing-key", detail) from exc


def recipient_kid(blob: bytes) -> str | None:
    """Best-effort recipient ``kid`` from a ``COSE_Encrypt0`` (cleartext)."""
    try:
        msg = _loads(blob)
        body = msg.value if isinstance(msg, cbor2.CBORTag) else msg
        kid = body[1][_KID]
        return kid.decode() if isinstance(kid, bytes) else None
    except (cbor2.CBORDecodeError, ValueError, TypeError, KeyError, IndexError):
        return None

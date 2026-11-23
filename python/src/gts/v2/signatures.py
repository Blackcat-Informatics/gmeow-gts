# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS v2 signature subjects, verifier inputs, and P6 verification outcomes."""

from collections.abc import Iterable
from dataclasses import dataclass
from hashlib import sha512
from typing import Literal

import cbor2
from blake3 import blake3
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.mldsa import MLDSA65PublicKey

from ._ed25519 import Ed25519Error, verify_ed25519
from .frame import Signature

type SignatureStatus = Literal["valid", "invalid", "unverified"]
type SignatureReason = (
    Ed25519Error
    | Literal[
        "unknown_signature_algorithm",
        "key_not_held",
        "key_mismatch",
        "value_length",
        "composite_classical_failed",
        "composite_post_quantum_failed",
    ]
)

_KEY_LENGTHS = {1: 32, 2: 1984}
_VALUE_LENGTHS = {1: 64, 2: 3373}
_COMPOSITE_PREFIX = b"CompositeAlgorithmSignatures2025"
_COMPOSITE_LABEL = b"COMPSIG-MLDSA65-Ed25519-SHA512"
_MLDSA_KEY_LEN = 1952
_MLDSA_VALUE_LEN = 3309


def _check_algorithm(algorithm: int) -> None:
    if type(algorithm) is not int or not 1 <= algorithm < (1 << 64):
        raise ValueError("signature algorithm must be a positive uint")


def _check_digest(value: bytes) -> None:
    if not isinstance(value, bytes) or len(value) != 32:
        raise ValueError("signature identifiers must be 32-byte digests")


def key_identifier(algorithm: int, public_key: bytes) -> bytes:
    """Hash the algorithm and the offered public key octets as a P6 key ID."""
    _check_algorithm(algorithm)
    if not isinstance(public_key, bytes):
        raise TypeError("public key must be bytes")
    return blake3(
        cbor2.dumps([algorithm, public_key]), derive_key_context="gts2 key id"
    ).digest()


def signature_message(
    algorithm: int, key_id: bytes, opener: bytes, effective_frame: bytes
) -> bytes:
    """Hash the typed subject passed to either signature algorithm."""
    _check_algorithm(algorithm)
    for digest in (key_id, opener, effective_frame):
        _check_digest(digest)
    return blake3(
        cbor2.dumps([algorithm, key_id, opener, effective_frame]),
        derive_key_context="gts2 signature subject",
    ).digest()


@dataclass(frozen=True, slots=True)
class HeldKey:
    key_id: bytes
    algorithm: int
    public_key: bytes

    def __post_init__(self) -> None:
        _check_digest(self.key_id)
        _check_algorithm(self.algorithm)
        if not isinstance(self.public_key, bytes):
            raise TypeError("public key must be bytes")


@dataclass(frozen=True, slots=True)
class SignatureOutcome:
    status: SignatureStatus
    reason: SignatureReason | None = None


def _check_composite(
    key: bytes, message: bytes, value: bytes
) -> SignatureReason | None:
    representative = (
        _COMPOSITE_PREFIX + _COMPOSITE_LABEL + b"\x00" + sha512(message).digest()
    )
    if verify_ed25519(key[_MLDSA_KEY_LEN:], representative, value[_MLDSA_VALUE_LEN:]):
        return "composite_classical_failed"
    verifier = MLDSA65PublicKey.from_public_bytes(key[:_MLDSA_KEY_LEN])
    try:
        verifier.verify(value[:_MLDSA_VALUE_LEN], representative, _COMPOSITE_LABEL)
    except InvalidSignature:
        return "composite_post_quantum_failed"
    return None


class SignatureVerifier:
    """Verify P6 signatures against caller-supplied held and trusted key sets."""

    def __init__(
        self,
        *,
        held_keys: Iterable[HeldKey] = (),
        trusted_keys: Iterable[bytes] = (),
    ) -> None:
        self._held_keys = tuple(
            sorted(
                set(held_keys),
                key=lambda k: cbor2.dumps([k.key_id, k.algorithm, k.public_key]),
            )
        )
        trusted = list(trusted_keys)
        for key_id in trusted:
            _check_digest(key_id)
        self._trusted_ids = frozenset(trusted)
        self._trusted_keys = tuple(sorted(self._trusted_ids))
        self._known_ids = {key.key_id for key in self._held_keys}
        self._consistent: dict[tuple[int, bytes], bytes] = {}
        for key in self._held_keys:
            if key_identifier(key.algorithm, key.public_key) == key.key_id:
                self._consistent.setdefault((key.algorithm, key.key_id), key.public_key)

    @property
    def held_keys(self) -> tuple[HeldKey, ...]:
        return self._held_keys

    @property
    def trusted_keys(self) -> tuple[bytes, ...]:
        return self._trusted_keys

    def trusts(self, key_id: bytes) -> bool:
        return key_id in self._trusted_ids

    def verify(
        self, signature: Signature, opener: bytes, effective_frame: bytes
    ) -> SignatureOutcome:
        """Decide the status of one decoded signature without consulting trust."""
        algorithm, key_id, value = (
            signature.algorithm,
            signature.key_id,
            signature.value,
        )
        if algorithm not in _KEY_LENGTHS:
            return SignatureOutcome("unverified", "unknown_signature_algorithm")
        if key_id not in self._known_ids:
            return SignatureOutcome("unverified", "key_not_held")
        key = self._consistent.get((algorithm, key_id))
        if key is None:
            return SignatureOutcome("invalid", "key_mismatch")
        if (
            len(key) != _KEY_LENGTHS[algorithm]
            or len(value) != _VALUE_LENGTHS[algorithm]
        ):
            return SignatureOutcome("invalid", "value_length")
        message = signature_message(algorithm, key_id, opener, effective_frame)
        if algorithm == 1:
            reason: SignatureReason | None = verify_ed25519(key, message, bytes(value))
        else:
            reason = _check_composite(key, message, bytes(value))
        return (
            SignatureOutcome("valid")
            if reason is None
            else SignatureOutcome("invalid", reason)
        )

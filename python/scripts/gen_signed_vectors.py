# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Regenerate the cross-engine signed-GTS conformance vector (§9.2).

A small GTS file signed with a deterministic Ed25519 key: every frame carries a
COSE_Sign1 over its content id. Because both the canonical CBOR encoding and
Ed25519 are deterministic, every engine must (a) reproduce these exact bytes when
signing the same content and (b) verify every frame against ``pub``. Reproducible:

    uv run python scripts/gen_signed_vectors.py
    git diff --exit-code ../vectors/signed
"""

from __future__ import annotations

import json
from collections.abc import Callable
from pathlib import Path
from unittest import mock

import cbor2
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

import gts.writer
from gts import Term, TermKind, Writer
from gts.crypto import Signer

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "signed"

CAT = "https://example.org/Cat"
LABEL = "http://www.w3.org/2000/01/rdf-schema#label"

# COSE header labels / algorithm ids (RFC 9052 §3.1).
_ALG, _KID, _TAG_SIGN1 = 1, 4, 18
_ALG_EDDSA = -8
# A private-use COSE algorithm id (RFC 9053 §2: values below -65536). No engine
# implements it, and unlike a registered id such as ES256 (-7) none will later,
# so the vectors stay valid when an engine adds a registered algorithm.
_ALG_PRIVATE_USE = -65537

_CoseBuilder = Callable[[Ed25519PrivateKey, str, bytes], bytes]


def _protected(header: dict[int, int]) -> bytes:
    return cbor2.dumps(header, canonical=True)


def _cose(
    protected: bytes,
    kid: str,
    signature: bytes,
    unprotected: dict[int, object] | None = None,
) -> bytes:
    """A detached COSE_Sign1 with a caller-chosen protected header and signature.

    ``unprotected`` defaults to ``{4: kid}``.
    """
    header = {_KID: kid.encode()} if unprotected is None else unprotected
    return cbor2.dumps(
        cbor2.CBORTag(_TAG_SIGN1, [protected, header, None, signature]),
        canonical=True,
    )


def _ed25519_cose(
    sk: Ed25519PrivateKey,
    protected: bytes,
    kid: str,
    frame_id: bytes,
    unprotected: dict[int, object] | None = None,
) -> bytes:
    """A COSE_Sign1 whose Ed25519 signature is valid over ``protected`` + the id."""
    structure = cbor2.dumps(["Signature1", protected, b"", frame_id], canonical=True)
    return _cose(protected, kid, sk.sign(structure), unprotected)


# Algorithm-dispatch cases (GTS-SPEC §9.2: readers MUST honour the declared
# algorithm). name -> (description, COSE builder, status with the kid's key
# resolved, status with no key resolved, whether the kid is reported).
_ALG_CASES: dict[str, tuple[str, _CoseBuilder, str, str, bool]] = {
    "sigalg-unsupported-64": (
        "Unsupported algorithm with a 64-byte signature that is not an Ed25519 "
        "signature of the frame: unverified, never invalid.",
        lambda sk, kid, fid: _cose(
            _protected({_ALG: _ALG_PRIVATE_USE}), kid, b"\xa5" * 64
        ),
        "unverified",
        "unverified",
        True,
    ),
    "sigalg-unsupported-long": (
        "Unsupported algorithm with a 96-byte signature: unverified, kid reported.",
        lambda sk, kid, fid: _cose(
            _protected({_ALG: _ALG_PRIVATE_USE}), kid, b"\xa5" * 96
        ),
        "unverified",
        "unverified",
        True,
    ),
    "sigalg-unsupported-over-ed25519": (
        "Unsupported algorithm declared over a signature that is a valid Ed25519 "
        "signature of the frame: unverified, never valid.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, _protected({_ALG: _ALG_PRIVATE_USE}), kid, fid
        ),
        "unverified",
        "unverified",
        True,
    ),
    "sigalg-eddsa-short": (
        "EdDSA with a 32-byte signature: invalid once the key resolves.",
        lambda sk, kid, fid: _cose(_protected({_ALG: _ALG_EDDSA}), kid, b"\xa5" * 32),
        "invalid",
        "unverified",
        True,
    ),
    "sigalg-missing-alg": (
        "Protected header without alg, over a valid Ed25519 signature: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, _protected({2: 1}), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-protected-not-map": (
        "Protected header is a bstr holding a CBOR integer, not a map: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, cbor2.dumps(1), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-protected-trailing-bytes": (
        "Protected header is a valid map followed by one extra byte: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a1012700"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-alg-float": (
        "alg is the float -8.0, not the integer -8: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a101fbc020000000000000"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-label-true": (
        "The protected header key is the boolean true, not the integer 1: no alg.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a1f527"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-label-float": (
        "The protected header key is the float 1.0, not the integer 1: no alg.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a1f93c0027"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-alg-unprotected-only": (
        "alg -8 only in the unprotected header; the protected map is empty: "
        "malformed, alg is read from the protected header.",
        lambda sk, kid, fid: _ed25519_cose(
            sk,
            bytes.fromhex("a0"),
            kid,
            fid,
            {_ALG: _ALG_EDDSA, _KID: kid.encode()},
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-unsupported-text-alg": (
        "A text-string alg naming an unsupported algorithm: unverified, kid reported.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, cbor2.dumps({_ALG: "X-GTS-unsupported"}), kid, fid
        ),
        "unverified",
        "unverified",
        True,
    ),
    "sigalg-not-four-elements": (
        "A three-element array instead of the four COSE_Sign1 elements: malformed.",
        lambda sk, kid, fid: cbor2.dumps(
            cbor2.CBORTag(
                _TAG_SIGN1,
                [
                    _protected({_ALG: _ALG_EDDSA}),
                    {_KID: kid.encode()},
                    sk.sign(
                        cbor2.dumps(
                            ["Signature1", _protected({_ALG: _ALG_EDDSA}), b"", fid],
                            canonical=True,
                        )
                    ),
                ],
            ),
            canonical=True,
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-missing-kid": (
        "EdDSA with a valid signature but no kid in the unprotected header: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, _protected({_ALG: _ALG_EDDSA}), kid, fid, {}
        ),
        "invalid",
        "invalid",
        False,
    ),
}


def _alg_file(sk: Ed25519PrivateKey, kid: str, build: _CoseBuilder) -> bytes:
    """The ``basic`` file with every frame signature replaced by ``build``'s COSE.

    ``"sig"`` is outside the frame-id preimage, so the frame ids match ``basic``.
    """
    with mock.patch.object(
        gts.writer, "sign_id", lambda fid, _signer: build(sk, kid, fid)
    ):
        w = Writer(profile="dist", signer=Signer(kid, sk))
        w.add_terms(
            [
                Term(TermKind.IRI, CAT),
                Term(TermKind.IRI, LABEL),
                Term(TermKind.LITERAL, "Cat", lang="en"),
            ]
        )
        w.add_quads([(0, 1, 2, None)])
        return bytes(w.to_bytes())


def main() -> None:
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    seed = bytes(range(32))
    sk = Ed25519PrivateKey.from_private_bytes(seed)
    pub = sk.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    kid = "test-kid"

    w = Writer(profile="dist", signer=Signer(kid, sk))
    w.add_terms(
        [
            Term(TermKind.IRI, CAT),
            Term(TermKind.IRI, LABEL),
            Term(TermKind.LITERAL, "Cat", lang="en"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    data = w.to_bytes()

    case = {
        "alg": "Ed25519",
        "seed": seed.hex(),
        "pub": pub.hex(),
        "kid": kid,
        "gts": data.hex(),
    }
    (VECTORS_DIR / "basic.json").write_text(
        json.dumps(case, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote signed vector ({len(data)} bytes) to {VECTORS_DIR}")

    for name, (
        description,
        build,
        with_key,
        without_key,
        reports_kid,
    ) in _ALG_CASES.items():
        alg_case: dict[str, object] = {
            "description": description,
            "gts": _alg_file(sk, kid, build).hex(),
            "kid": kid,
            "pub": pub.hex(),
            "seed": seed.hex(),
            "status_with_key": with_key,
            "status_without_key": without_key,
        }
        if reports_kid:
            alg_case["reported_kid"] = kid
        (VECTORS_DIR / f"{name}.json").write_text(
            json.dumps(alg_case, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print(f"wrote {len(_ALG_CASES)} algorithm-dispatch vectors to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

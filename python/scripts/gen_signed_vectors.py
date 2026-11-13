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
    "sigalg-header-indefinite-map": (
        "Non-deterministic header: A header map of indefinite "
        "length (bf .. ff): malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("bf0127ff"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-indefinite-text-alg": (
        "Non-deterministic header: An indefinite-length text-string alg: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a1017f6145ff"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-label-nonminimal-1byte": (
        "Non-deterministic header: Label 1 encoded with a one-byte "
        "argument (18 01): malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a1180127"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-label-nonminimal-8byte": (
        "Non-deterministic header: Label 1 encoded with an "
        "eight-byte argument: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a11b000000000000000127"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-alg-nonminimal-8byte": (
        "Non-deterministic header: Alg -8 encoded with an "
        "eight-byte argument: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a1013b0000000000000007"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-label-duplicate-eddsa-first": (
        "Non-deterministic header: Label 1 repeated, EdDSA first "
        "({1: -8, 1: -7}): malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a201270126"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-label-duplicate-eddsa-second": (
        "Non-deterministic header: Label 1 repeated, EdDSA second "
        "({1: -7, 1: -8}): malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a201260127"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-keys-unsorted": (
        "Non-deterministic header: Map keys out of bytewise order "
        "(label 4 before label 1): malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a20441780127"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-float16": (
        "Header rule: a half-precision float value under an unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a2012705f93c00"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-float64": (
        "Header rule: a double-precision float value under an "
        "unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a2012705fb3ff0000000000000"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-tag": (
        "Header rule: a tagged value under an unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a2012705c100"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-bignum": (
        "Header rule: a bignum under an unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a2012705c249010000000000000000"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-undefined": (
        "Header rule: the simple value undefined under an unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a2012705f7"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-simple-16": (
        "Header rule: an unassigned simple value (16) under an "
        "unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a2012705f0"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-simple-32": (
        "Header rule: an unassigned one-byte simple value (32) under "
        "an unrelated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex("a2012705f820"), kid, fid),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-nested-unsorted": (
        "Header rule: a nested map with unsorted keys: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a2012705a202000100"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-nested-duplicate": (
        "Header rule: a nested map with a repeated key: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a2012705a201000101"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-nested-indefinite-array": (
        "Header rule: a nested indefinite-length array: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a20127059f00ff"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-keys-lengthfirst-order": (
        "Header rule: keys in length-first order (-1 before 24), not "
        "bytewise: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a301272000181800"), kid, fid
        ),
        "invalid",
        "invalid",
        False,
    ),
    "sigalg-header-null-bool-values": (
        "Header rule: null, true and false under unrelated keys are "
        "acceptable: accepted.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a4012705f606f507f4"), kid, fid
        ),
        "valid",
        "unverified",
        True,
    ),
    "sigalg-keys-bytewise-order": (
        "Header rule: keys in bytewise order (24 before -1) are acceptable: accepted.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, bytes.fromhex("a301271818002000"), kid, fid
        ),
        "valid",
        "unverified",
        True,
    ),
    "sigalg-empty-protected-alg-unprotected": (
        "Empty protected byte string with alg only in the unprotected "
        "header: malformed.",
        lambda sk, kid, fid: _ed25519_cose(
            sk, b"", kid, fid, {_ALG: _ALG_EDDSA, _KID: kid.encode()}
        ),
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


def _nested(levels: int, kinds: str) -> str:
    """Hex of a value nested ``levels`` deep, from level 2 of the header.

    ``kinds`` is ``"a"`` for arrays only or ``"m"`` to alternate arrays and
    maps; the innermost container is empty.
    """
    out = ""
    for level in range(2, levels + 1):
        is_array = kinds == "a" or level % 2 == 0
        last = level == levels
        if is_array:
            out += "80" if last else "81"
        else:
            out += "a0" if last else "a100"
    return out


def _header_case(
    description: str, header_hex: str, valid: bool
) -> tuple[str, _CoseBuilder, str, str, bool]:
    return (
        description,
        lambda sk, kid, fid: _ed25519_cose(sk, bytes.fromhex(header_hex), kid, fid),
        "valid" if valid else "invalid",
        "unverified" if valid else "invalid",
        valid,
    )


# Header grammar boundary cases (identical grammar in every engine): integer
# range, key types, UTF-8 and nesting depth. Every header is otherwise
# deterministic and declares EdDSA, so only the boundary decides the status.
_I64_MAX = "1b7fffffffffffffff"
_I64_OVER = "1b8000000000000000"
_I64_MIN = "3b7fffffffffffffff"
_I64_UNDER = "3b8000000000000000"
for _name, _text, _hex, _ok in (
    ("int-max-value", "integer value 2^63-1", "a2012705" + _I64_MAX, True),
    ("int-over-max-value", "integer value 2^63", "a2012705" + _I64_OVER, False),
    ("int-min-value", "integer value -2^63", "a2012705" + _I64_MIN, True),
    ("int-under-min-value", "integer value -2^63-1", "a2012705" + _I64_UNDER, False),
    ("int-max-key", "integer key 2^63-1", "a20127" + _I64_MAX + "00", True),
    ("int-over-max-key", "integer key 2^63", "a20127" + _I64_OVER + "00", False),
    ("int-min-key", "integer key -2^63", "a20127" + _I64_MIN + "00", True),
    ("int-under-min-key", "integer key -2^63-1", "a20127" + _I64_UNDER + "00", False),
    ("key-bool", "boolean key", "a20127f500", False),
    ("key-array", "array key", "a2012780" + "00", False),
    ("key-map", "map key", "a20127a0" + "00", False),
    ("key-bstr", "byte-string key", "a20127417800", False),
    ("key-text", "text-string key", "a20127617800", True),
    (
        "text-invalid-utf8",
        "text value that is not valid UTF-8",
        "a20127" + "0561ff",
        False,
    ),
    ("depth-8", "nesting of exactly 8 levels", "a2012705" + _nested(8, "m"), True),
    ("depth-9", "nesting of 9 levels", "a2012705" + _nested(9, "m"), False),
    (
        "depth-9-arrays",
        "nesting of 9 levels, arrays only",
        "a2012705" + _nested(9, "a"),
        False,
    ),
):
    _ALG_CASES["sigalg-" + _name] = _header_case(
        "Header grammar: " + _text + (": accepted." if _ok else ": malformed."),
        _hex,
        _ok,
    )


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

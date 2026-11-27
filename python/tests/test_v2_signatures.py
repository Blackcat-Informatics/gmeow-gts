# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Verify GTS v2 signature outcomes against the P6 corpus."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2.authentication import AuthenticationFrame, authenticate_segment
from gts.v2.frame import Signature, decode_frame
from gts.v2.link import FrameLink
from gts.v2.range import RangeAccumulator
from gts.v2.signatures import (
    HeldKey,
    SignatureOutcome,
    SignatureVerifier,
    key_identifier,
    signature_message,
)

_VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P6"
_STATUSES = {1: "valid", 2: "invalid", 3: "unverified"}
_AUTHENTICITY = {1: "authenticated", 2: "partly_authenticated", 3: "unauthenticated"}
_REASONS = {
    53: "unknown_signature_algorithm",
    54: "key_not_held",
    57: "key_mismatch",
    58: "value_length",
    59: "non_canonical_scalar",
    60: "small_order_key",
    61: "small_order_nonce",
    62: "equation_failed",
    63: "composite_classical_failed",
    64: "composite_post_quantum_failed",
}


@pytest.mark.parametrize("algorithm, name", [(1, "ed25519"), (2, "composite")])
def test_signature_hash_preimages(algorithm: int, name: str) -> None:
    case = json.loads((_VECTORS / f"key-id-{name}.json").read_text())["cases"][0]
    key = case["result"]
    public_key = (
        bytes.fromhex(key["public_key"])
        if algorithm == 1
        else (_VECTORS / case["input_file"]).read_bytes()
    )
    assert key_identifier(algorithm, public_key).hex() == key["key_id"]
    subject = json.loads((_VECTORS / f"subject-{name}.json").read_text())["cases"][0][
        "result"
    ]
    args = (bytes.fromhex(subject[field]) for field in ("K", "H0", "F"))
    assert signature_message(algorithm, *args).hex() == subject["message"]


_SIGNATURE_CASES = sorted(
    {
        *(p.stem for p in _VECTORS.glob("ed25519-*.json")),
        *(
            p.stem
            for p in _VECTORS.glob("composite-*.json")
            if p.stem != "composite-message-representative"
        ),
        "proc-key-not-held",
        "proc-key-mismatch",
        "proc-mismatch-before-length",
        "proc-lengths-key-first",
        "proc-length-before-policy",
        "proc-unknown-algorithm",
        "proc-unknown-algorithm-long",
        "key-held-not-trusted",
        "key-trusted-not-held",
        "key-mismatch-algorithm",
        "key-mismatch-bytes",
        "key-mismatch-two-entries",
        "key-held-order",
        "key-length-short",
    }
)


@pytest.mark.parametrize("name", _SIGNATURE_CASES)
def test_signature_verification_order_and_equations(name: str) -> None:
    manifest = json.loads((_VECTORS / f"{name}.json").read_text())
    checked = 0
    for case in manifest["cases"]:
        data = (_VECTORS / case["input_file"]).read_bytes()
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        trusted, held = report[5]
        verifier = SignatureVerifier(
            held_keys=(HeldKey(*key) for key in reversed(held + held)),
            trusted_keys=reversed(trusted + trusted),
        )
        assert verifier.held_keys == tuple(HeldKey(*key) for key in held)
        assert verifier.trusted_keys == tuple(trusted)
        for segment in report[-1][1]:
            opener = segment[4][1]
            for _, offset, _, _, _, _, _, signatures, *_ in segment[-2]:
                if not signatures:
                    continue
                frame = decode_frame(data, offset, registry_version=segment[5][1])
                for signature, expected in zip(
                    frame.signatures, signatures, strict=True
                ):
                    status, reason, algorithm, key_id, *_ = expected
                    result = verifier.verify(signature, opener, frame.identifier)
                    assert (signature.algorithm, signature.key_id) == (
                        algorithm,
                        key_id,
                    )
                    assert result.status == _STATUSES[status]
                    assert result.reason == (
                        _REASONS[reason[1]] if reason[0] == 2 else None
                    )
                    checked += 1
    assert checked


@pytest.mark.parametrize(
    "name",
    [
        "cov-simple",
        "cov-break-before",
        "cov-break-after",
        "cov-broken-signed-frame",
        "cov-healed",
        "cov-transparent-between",
        "auth-broken-middle",
        "auth-commit-valid",
        "auth-commit-bad-count",
        "auth-commit-bad-root",
        "auth-payload-damaged",
        "auth-tail",
        "auth-untrusted",
        "auth-two-commits-two-signers",
        "auth-empty-segment-index",
    ],
)
def test_signature_coverage_and_authentication(name: str) -> None:
    for case in json.loads((_VECTORS / f"{name}.json").read_text())["cases"]:
        data = (_VECTORS / case["input_file"]).read_bytes()
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        trusted, held = report[5]
        verifier = SignatureVerifier(
            held_keys=map(lambda key: HeldKey(*key), held), trusted_keys=trusted
        )
        for segment in report[-1][1]:
            opener = head = segment[4][1]
            history = RangeAccumulator(opener)
            frames = []
            expected = []
            for position, offset, _, _, link_code, _, _, signatures, *_ in segment[-2]:
                frame = decode_frame(data, offset, registry_version=segment[5][1])
                link_state = {1: "linked", 2: "healed", 3: "broken"}[link_code]
                computed = frame.computed_identifier(head)
                effective = computed if link_state == "healed" else frame.identifier
                valid_commit = False
                if not frame.transparent:
                    if frame.frame_type == 14 and link_state != "broken":
                        count, digest = cbor2.loads(frame.content)
                        root = history.root()
                        valid_commit = (count, digest) == (root.count - 1, root.digest)
                    history.append(effective)
                    head = effective
                frames.append(
                    AuthenticationFrame(
                        position,
                        frame.frame_type,
                        FrameLink(link_state, computed, effective),
                        frame.signatures,
                        frame.transparent,
                        valid_commit=valid_commit,
                    )
                )
                if signatures:
                    expected.append(
                        (position, [tuple(result[4:]) for result in signatures])
                    )
            result = authenticate_segment(opener, frames, verifier)
            assert [
                (
                    position,
                    [
                        (s.coverage_first, s.coverage_end, s.authenticated)
                        for s in signatures
                    ],
                )
                for position, signatures in result.signatures
            ] == expected
            assert result.authenticated_frames == segment[12]
            assert result.authenticity == _AUTHENTICITY[segment[9]]


def _composite_test_key(private: bytes):
    """The ML-DSA-65 key, the Ed25519 key and the composite public key of a seed."""
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from cryptography.hazmat.primitives.asymmetric.mldsa import MLDSA65PrivateKey

    ml = MLDSA65PrivateKey.from_seed_bytes(private[:32])
    ed = Ed25519PrivateKey.from_private_bytes(private[32:])
    public = ml.public_key().public_bytes_raw() + ed.public_key().public_bytes_raw()
    return ml, ed, public


def _composite_halves(key, opener: bytes, frame: bytes) -> tuple[bytes, bytes]:
    """The ML-DSA and Ed25519 halves of a composite of the subject (P6 4.2)."""
    from hashlib import sha512

    ml, ed, public = key
    message = signature_message(2, key_identifier(2, public), opener, frame)
    representative = (
        b"CompositeAlgorithmSignatures2025"
        + b"COMPSIG-MLDSA65-Ed25519-SHA512"
        + b"\x00"
        + sha512(message).digest()
    )
    return (
        ml.sign(representative, b"COMPSIG-MLDSA65-Ed25519-SHA512"),
        ed.sign(representative),
    )


def _composite_subject() -> tuple[bytes, bytes, bytes]:
    case = json.loads((_VECTORS / "sign-composite-reproduce.json").read_text())[
        "cases"
    ][0]
    private = bytes.fromhex(case["signer"]["private_key"])
    opener = bytes.fromhex(case["subject_fields"]["header_identifier"])
    frame = bytes.fromhex(case["subject_fields"]["frame_identifier"])
    return private, opener, frame


def test_composite_halves_of_two_signatures_do_not_combine() -> None:
    """Swapping the ML-DSA half of one valid signature with the Ed25519 half of another
    gives a composite that the classical half refuses first (P6-26)."""
    private, opener, frame = _composite_subject()
    first = _composite_test_key(private)
    second = _composite_test_key(bytes([0x80] * 32 + [0xA0] * 32))
    first_id = key_identifier(2, first[2])
    second_id = key_identifier(2, second[2])
    verifier = SignatureVerifier(
        held_keys=[
            HeldKey(first_id, 2, first[2]),
            HeldKey(second_id, 2, second[2]),
        ]
    )
    first_ml, first_ed = _composite_halves(first, opener, frame)
    _, second_ed = _composite_halves(second, opener, frame)
    valid = Signature(2, first_id, memoryview(first_ml + first_ed))
    assert verifier.verify(valid, opener, frame).status == "valid"
    swapped = Signature(2, first_id, memoryview(first_ml + second_ed))
    assert verifier.verify(swapped, opener, frame) == SignatureOutcome(
        "invalid", "composite_classical_failed"
    )


def test_composite_is_not_verified_by_one_component_key() -> None:
    """The composite is refused as unverified without its own key, and as a key mismatch
    beside its identifier with the Ed25519 bytes (P6-33, key-mismatch-bytes)."""
    private, opener, frame = _composite_subject()
    key = _composite_test_key(private)
    composite_id = key_identifier(2, key[2])
    ml_half, ed_half = _composite_halves(key, opener, frame)
    signature = Signature(2, composite_id, memoryview(ml_half + ed_half))
    ed_public = key[1].public_key().public_bytes_raw()
    ed_id = key_identifier(1, ed_public)
    assert (
        SignatureVerifier(held_keys=[HeldKey(composite_id, 2, key[2])])
        .verify(signature, opener, frame)
        .status
        == "valid"
    )
    assert SignatureVerifier(held_keys=[HeldKey(ed_id, 1, ed_public)]).verify(
        signature, opener, frame
    ) == SignatureOutcome("unverified", "key_not_held")
    assert SignatureVerifier(held_keys=[HeldKey(composite_id, 2, ed_public)]).verify(
        signature, opener, frame
    ) == SignatureOutcome("invalid", "key_mismatch")

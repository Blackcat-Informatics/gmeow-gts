# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise P9 result admission, stop order, and the detached proof."""

import json
from dataclasses import replace
from pathlib import Path

import pytest

from gts.v2._conversion_output import (
    ConversionCandidate,
    ConversionCapacity,
    prepare_conversion,
)
from gts.v2._conversion_result import (
    ConversionProof,
    Converted,
    ResultRead,
    ResultSegment,
    _convert,
)
from gts.v2._decode import WireError
from gts.v2._legacy_cbor import MapValue, decode_legacy_item
from gts.v2.conversion_fold import ConversionStop
from test_v2_conversion_fold import _encode_segment

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"


def _verified(content_digest: bytes) -> ResultRead:
    segment = ResultSegment(
        canonical_claimed=True,
        structure_verified=True,
        labelling_verified=True,
        intact=True,
        content_digest=content_digest,
    )
    return ResultRead(
        segments=(segment,),
        content_digest=content_digest,
        visible_complete=True,
        capacity_refused=False,
        error_or_fatal=False,
    )


def test_conversion_reads_the_candidate_at_the_requested_capacity_before_proof() -> (
    None
):
    fixture = json.loads((_P9 / "convert__signatures.json").read_bytes())
    expected = fixture["cases"][0]["result"]["conversions"][0]
    data = bytes.fromhex(expected["source"]["hex"])
    candidate = bytes.fromhex(expected["result"]["hex"])
    proof_wire = bytes.fromhex(expected["proof"])
    proof = ConversionProof.decode(proof_wire)
    capacity = ConversionCapacity(labelling_units=7, rebuilt_bytes_per_read=1024)
    inputs = iter((data, data))
    result_reads = []

    def read_result(raw: bytes, given: ConversionCapacity) -> ResultRead:
        assert raw == candidate
        assert given == capacity
        assert list(inputs) == []
        result_reads.append(raw)
        return _verified(proof.content)

    outcome = _convert(lambda: next(inputs), read_result, capacity)
    assert outcome == Converted(candidate, proof)
    assert result_reads == [candidate]
    assert proof.encode() == proof_wire
    with pytest.raises(WireError, match="wrong_length"):
        ConversionProof.decode(proof_wire + b"\x00")


def test_admission_requires_all_independent_result_conditions() -> None:
    result = _verified(bytes(32))
    assert result.segments is not None
    (segment,) = result.segments
    for changed in (
        replace(result, segments=None),
        replace(result, segments=()),
        replace(result, segments=(segment, segment)),
        replace(result, content_digest=None),
        replace(result, content_digest=b"d" * 32),
        replace(result, visible_complete=False),
        replace(result, error_or_fatal=True),
        replace(result, segments=(replace(segment, canonical_claimed=False),)),
        replace(result, segments=(replace(segment, structure_verified=False),)),
        replace(result, segments=(replace(segment, labelling_verified=False),)),
        replace(result, segments=(replace(segment, intact=False),)),
    ):
        assert changed.admitted_digest() is None


def test_oversized_canonical_frame_is_judged_after_the_result_read() -> None:
    source = (_P9 / "convert__result-frame-above-capacity.bin").read_bytes()
    capacity = ConversionCapacity(decoded_bytes_per_frame=3000)
    admitted = False

    def read_result(raw: bytes, given: ConversionCapacity) -> ResultRead:
        nonlocal admitted
        assert raw and given == capacity
        admitted = True
        return replace(_verified(bytes(32)), capacity_refused=True)

    # The index decodes to 3773 bytes. The source fits within the limit.
    assert _convert(lambda: source, read_result, capacity) == ConversionStop(
        "source_exceeds_capacity", 0
    )
    assert admitted


def test_result_capacity_and_digest_stops_precede_source_visibility() -> None:
    prefix = [("blob", b"repeat")] * 2
    raw, positions = _encode_segment(prefix, streamable=False)
    envelope = decode_legacy_item(raw, positions[0]).value
    assert isinstance(envelope, MapValue)
    target = {"kind": "frame", "id": dict(envelope.entries)["id"]}
    data, _ = _encode_segment(
        [*prefix, ("suppress", {"targets": [target]})], streamable=False
    )
    candidate = prepare_conversion(lambda: data)
    assert isinstance(candidate, ConversionCandidate)
    correct = _verified(candidate.expected_digest)
    wrong_digest = _verified(bytes(32))
    overwhelmed = replace(wrong_digest, segments=None, capacity_refused=True)
    for observed, reason in (
        (overwhelmed, "source_exceeds_capacity"),
        (wrong_digest, "content_mismatch"),
        (correct, "unrepresentable_term"),
    ):
        assert _convert(
            lambda: data, lambda raw, capacity, result=observed: result
        ) == ConversionStop(reason, 0)

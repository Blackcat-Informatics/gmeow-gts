# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check GTS v2 blob declarations and value selection against the P4 corpus."""

import json
from collections import Counter
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2.blobs import (
    BlobRead,
    blob_set,
    decode_blob_content,
)

_P4 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P4"
_FINDINGS = {
    (13, 12): ("damaged_frame", "payload_digest"),
    (13, 14): ("damaged_frame", "blob_length"),
    (13, 15): ("damaged_frame", "blob_digest"),
    (19, 24): ("missing_dependency", "chunk"),
    (20, 89): ("exceeds_capacity", "total_rebuilt"),
    (44, None): ("unreferenced_unit", None),
    (45, None): ("conflicting_declaration", None),
}


def _read_cases(name: str):
    for case in json.loads((_P4 / f"{name}.json").read_text())["cases"]:
        data = (_P4 / case["input_file"]).read_bytes()
        expected = cbor2.loads(bytes.fromhex(case["read_result"]))
        stored, decoded, _, _, read_decoded, _, rebuilt, _, _, access = expected[3]
        payloads = PayloadBudget(
            stored_bytes_per_frame=stored,
            decoded_bytes_per_frame=decoded,
            decoded_bytes_per_read=read_decoded,
        )
        reader = BlobRead(rebuilt, whole=access == 1)
        declarations = []
        value_keys = set()
        actual_findings = Counter()
        expected_findings = Counter()
        for segment in expected[-1][1]:
            ordinal = segment[0]
            frames = []
            for record in segment[-2]:
                position, offset, _, kind, _, state, _, _, fold, *_ = record
                if kind[0] != 2 or kind[1] not in range(8, 14) or fold != [1]:
                    continue
                assert state == [1]
                frame = decode_frame(data, offset)
                decoded = payloads.check(frame).decoded
                assert decoded is not None
                frames.append((position, decode_blob_content(frame, decoded)))
                if kind == [2, 11]:
                    digest, length, _ = cbor2.loads(frame.content)
                    value_keys.add((digest, length))
                for code, reason, _, _, _, _, count in record[-1]:
                    finding = _FINDINGS.get(
                        (code, reason[-1] if reason[0] == 2 else None)
                    )
                    if finding is not None:
                        expected_findings[
                            (
                                ordinal,
                                position,
                                *finding,
                                count[1] if count[0] == 2 else None,
                            )
                        ] += 1
            folded = reader.fold_segment(ordinal, frames)
            declarations.extend(folded.declarations)
            for finding in folded.findings:
                loc = finding.location
                actual_findings[
                    (
                        loc.segment,
                        loc.position,
                        finding.code,
                        finding.reason,
                        finding.count,
                    )
                ] += 1
        blobs = blob_set(declarations)
        for loc in blobs.length_conflicts:
            actual_findings[
                (loc.segment, loc.position, "conflicting_declaration", None, None)
            ] += 1
        assert actual_findings == expected_findings
        yield reader, blobs, declarations, expected[-1][1], value_keys


@pytest.mark.parametrize(
    "name",
    [
        "blobset-entry",
        "blobset-conflict",
        "blobset-order-free",
        "unit-unreferenced",
        "rebuild-capacity",
        "rebuild-other-segment",
        "rebuild-unit-after-recipe",
        "rebuild-charge-order",
        "pack-member-digest",
    ],
)
def test_blob_set_and_unit_findings(name: str) -> None:
    cases = list(_read_cases(name))
    if name == "blobset-order-free":
        assert all(case[1].entries == cases[0][1].entries for case in cases)
    if name == "blobset-entry":
        for _, result, declarations, _, _ in cases:
            (entry,) = result.entries
            assert (entry.present, entry.length, entry.media_types) == (
                True,
                5,
                ("a/b", "c/d"),
            )
            assert entry.state == "complete"
            blob, external = declarations
            payload = blob.bytes()
            assert payload is not None and len(payload) == 5
            assert external.bytes() is None


@pytest.mark.parametrize(
    "name, verified",
    [
        ("rebuild-value-first-verified", True),
        ("rebuild-value-none-verified", False),
    ],
)
def test_value_availability_is_temporal_and_distinguishes_failed_declarations(
    name: str, verified: bool
) -> None:
    for reader, _, _, segments, values in _read_cases(name):
        ((digest, length),) = values
        first, second = (segment[0] for segment in segments)
        assert not reader.value_before(
            digest, length, segment=first, position=2
        ).declared
        choice = reader.value_before(digest, length, segment=second, position=3)
        assert choice.declared
        assert (choice.rebuilt is not None) == verified
        assert not reader.value_before(
            digest, length + 1, segment=second, position=3
        ).declared


def test_body_does_not_follow_a_mutable_source() -> None:
    case = json.loads((_P4 / "blobset-entry.json").read_text())["cases"][0]
    data = (_P4 / case["input_file"]).read_bytes()
    expected = cbor2.loads(bytes.fromhex(case["read_result"]))
    blob = expected[-1][1][0][-2][0]
    frame = decode_frame(data, blob[1])
    original = bytes(frame.stored_payload)
    source = bytearray(original)
    decoded = decode_blob_content(frame, memoryview(source))
    source[0] ^= 1
    (declaration,) = BlobRead(0).fold_segment(1, [(blob[0], decoded)]).declarations
    assert declaration.bytes() == original

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check file entry and recovery against the published P1 and P7 results."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import Header, PayloadBudget, dispatch_file, item_end
from gts.v2.resync import RESYNC_MAX, encode_resync

_VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2"
_FRAME_FINDINGS = {
    1: "empty_input",
    2: "not_this_format",
    3: "unsupported_generation",
    8: "undelimitable_input",
    9: "torn_tail",
}


@pytest.mark.parametrize(
    "fixture",
    [
        "P1/file-first-incomplete",
        "P1/file-first-undelimitable",
        "P1/header-long-form-first-item",
        "P1/resync-first-item",
        "P1/resync-first-item-fails",
        "P1/resync-first-item-torn",
        "P1/resync-first-byte-foreign",
        "P1/resync-resync-first",
        "P1/first-item-five-alone",
        "P7/read__empty",
        "P7/read__not-this-format",
        "P7/read__generation-three",
    ],
)
def test_dispatch_matches_corpus(fixture: str) -> None:
    manifest = _VECTORS / f"{fixture}.json"
    for case in json.loads(manifest.read_text())["cases"]:
        data = (manifest.parent / case["input_file"]).read_bytes()
        capacity = case["capacity"]
        budget = PayloadBudget(
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
        )
        result = dispatch_file(data, budget, items_per_read=capacity["items_per_read"])
        report = cbor2.loads(bytes.fromhex(case["read_result"]))[-1]
        segments, findings = report[1:3]
        if segments:
            first = segments[0]
            assert result.generation == 2
            assert result.opener is not None
            assert result.opener_offset == first[1]
            assert result.next_offset == item_end(data, first[1])
            assert result.scanned == (first[3][:2] == [2, 2])
        else:
            assert result.generation is None
            assert result.opener is None

        expected = next(
            (
                finding
                for finding in findings
                if finding[0] in _FRAME_FINDINGS and finding[4] in ([1], [2, 0])
            ),
            None,
        )
        if expected is None:
            assert result.finding is None
        else:
            assert result.finding is not None
            assert result.finding.reason == _FRAME_FINDINGS[expected[0]]
            offset = expected[4][1] if expected[4][0] == 2 else None
            length = expected[5][1] if expected[5][0] == 2 else None
            assert (result.finding.offset, result.finding.length) == (offset, length)


def _budget(total: int = RESYNC_MAX) -> PayloadBudget:
    return PayloadBudget(
        stored_bytes_per_frame=1 << 26,
        decoded_bytes_per_frame=1 << 26,
        decoded_bytes_per_read=total,
    )


def test_dispatch_recovery_admission_and_refusal() -> None:
    data = b"\xff" + encode_resync(Header(), offset=1)
    refused = dispatch_file(data, _budget(RESYNC_MAX - 1))
    assert refused.generation is None
    assert refused.scan_refusal is not None
    assert (refused.scan_refusal.offset, refused.scan_refusal.count) == (1, RESYNC_MAX)
    budget = _budget()
    stopped = dispatch_file(data, budget, items_per_read=1)
    assert stopped.generation is None
    assert stopped.scan_refusal is None
    assert budget.charge_decoded(RESYNC_MAX) is None
    recovered = dispatch_file(data, _budget(), items_per_read=2)
    assert recovered.generation == 2 and recovered.scanned


def test_dispatch_legacy_choice_is_explicit() -> None:
    data = cbor2.dumps({"v": 1})
    assert dispatch_file(data, _budget(), first_and_second=True).generation == 1
    second_only = dispatch_file(data, _budget())
    assert second_only.finding is not None
    assert second_only.finding.reason == "unsupported_generation"

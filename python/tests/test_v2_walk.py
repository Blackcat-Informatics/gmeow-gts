# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare Python segment boundaries and frame checks with P1 vectors."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import Header, PayloadBudget, encode_header, walk_file
from gts.v2.items import ClosingFinding

_P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"
_LINK = {"linked": 1, "healed": 2, "broken": 3}
_PAYLOAD = {"verified": 1, "damaged": 2, "unchecked": 3}


@pytest.mark.parametrize(
    "fixture",
    [
        "link-healed",
        "link-healed-effective",
        "link-healed-before-transparent",
        "payload-total-order",
        "payload-digest",
        "stored-mismatch",
        "offsets-counts-opaque",
        "locator-not-index-item",
        "lineage-previous-unreadable",
        "links-unavailable-lineage-not-compared",
        "resync-after-undelimitable",
        "resync-first-item-torn",
    ],
)
def test_walk_matches_segment_and_frame_boundaries(fixture: str) -> None:
    manifest = json.loads((_P1 / f"{fixture}.json").read_text())
    for case in manifest["cases"]:
        capacity = case["capacity"]
        if capacity["access"] != "whole":
            continue
        data = (_P1 / case["input_file"]).read_bytes()
        result = walk_file(
            data,
            PayloadBudget(
                stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
                decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
                decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
            ),
            items_per_read=capacity["items_per_read"],
        )
        expected = cbor2.loads(bytes.fromhex(case["read_result"]))[-1][1]
        assert len(result.segments) == len(expected)
        for segment, reference in zip(result.segments, expected, strict=True):
            assert (segment.offset, segment.length) == tuple(reference[1:3])
            assert segment.scanned == (reference[3][:2] == [2, 2])
            frames = reference[16]
            assert len(segment.frames) == len(frames)
            for frame, record in zip(segment.frames, frames, strict=True):
                assert (frame.item.offset, frame.item.end - frame.item.offset) == tuple(
                    record[1:3]
                )
                assert _LINK[frame.link.state] == record[4]
                if frame.payload is not None:
                    assert _PAYLOAD[frame.payload.state] == record[5][0]
                    stored = frame.payload.stored_matches
                    assert (
                        [1] if stored is None else [2, 1 if stored else 2]
                    ) == record[6]
            expected_closing = [
                ClosingFinding(
                    finding[3][1],
                    {19: "index_offsets", 23: "index_locator"}[finding[1][1]],
                )
                for finding in reference[17]
                if finding[0] == 18 and finding[1][1] in (19, 23)
            ]
            assert [
                finding
                for finding in segment.closing_findings
                if finding.reason in ("index_offsets", "index_locator")
            ] == expected_closing


def test_walk_stops_at_unread_item_boundary() -> None:
    header = encode_header(Header())
    budget = PayloadBudget(
        stored_bytes_per_frame=1024,
        decoded_bytes_per_frame=1024,
        decoded_bytes_per_read=4096,
    )
    result = walk_file(header * 2, budget, items_per_read=1)
    assert len(result.segments) == 1
    assert result.segments[0].length == len(header)
    (finding,) = result.framing_findings
    assert (finding.reason, finding.offset, finding.length, finding.count) == (
        "item_count",
        len(header),
        len(header),
        2,
    )

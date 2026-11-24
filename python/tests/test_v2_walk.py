# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare Python segment boundaries and frame checks with P1 vectors."""

import json
from pathlib import Path

import cbor2
import pytest
from zstandard import ZstdCompressor, ZstdDecompressor

from gts.v2 import (
    FrameError,
    Header,
    PayloadBudget,
    decode_header,
    encode_header,
    encode_plain_frame,
    walk_file,
)
from gts.v2.frame import _encode_frame
from gts.v2.items import ClosingFinding

_P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"
_P5 = _P1.with_name("P5")
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
        "stored-check-short",
        "frame-identifier-type",
        "frame-unknown-transparent-not-bool",
        "parity-codec-newer",
        "offsets-counts-opaque",
        "locator-not-index-item",
        "lineage-previous-unreadable",
        "links-unavailable-lineage-not-compared",
        "resync-after-undelimitable",
        "resync-after-torn-swallowed-failing",
        "resync-after-torn-swallowed-passing",
        "resync-heal-no-witness-past-region",
        "resync-other-generation-scanned",
        "resync-newer-kind-not-scanned",
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
                assert _PAYLOAD[frame.payload.state] == record[5][0]
                stored = frame.payload.stored_matches
                assert ([1] if stored is None else [2, 1 if stored else 2]) == record[6]
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


def test_rejected_envelope_still_charges_its_valid_payload() -> None:
    header = encode_header(Header())
    predecessor = decode_header(header).identifier
    assert predecessor is not None
    first = cbor2.loads(encode_plain_frame(predecessor, 8, [bytes(32)], b"abc"))
    first[-1] = 0
    second = encode_plain_frame(first[-2], 8, [bytes(32)], b"d")
    budget = PayloadBudget(
        stored_bytes_per_frame=1024,
        decoded_bytes_per_frame=16,
        decoded_bytes_per_read=3,
    )

    result = walk_file(header + cbor2.dumps(first) + second, budget)

    rejected, accepted = result.segments[0].frames
    assert isinstance(rejected.frame, FrameError)
    assert rejected.payload.state == "verified"
    assert accepted.payload.state == "unchecked"
    assert (accepted.payload.reason, accepted.payload.count) == ("total_decoded", 4)


def test_rejected_indefinite_payload_preserves_its_stored_check() -> None:
    header = encode_header(Header())
    predecessor = decode_header(header).identifier
    assert predecessor is not None
    encoded, _ = _encode_frame(
        predecessor, 8, [bytes(32)], b"abc" * 200, compressor=ZstdCompressor()
    )
    fields = cbor2.loads(encoded)
    stored = fields[8]
    split = len(stored) // 2
    encoded_fields = [cbor2.dumps(field) for field in fields]
    encoded_fields[8] = (
        b"\x5f" + cbor2.dumps(stored[:split]) + cbor2.dumps(stored[split:]) + b"\xff"
    )
    budget = PayloadBudget(
        stored_bytes_per_frame=1024,
        decoded_bytes_per_frame=1024,
        decoded_bytes_per_read=1024,
    )

    result = walk_file(header + b"\x8b" + b"".join(encoded_fields), budget)

    (frame,) = result.segments[0].frames
    assert isinstance(frame.frame, FrameError)
    assert frame.payload.state == "damaged"
    assert frame.payload.stored_matches is True


@pytest.mark.parametrize(
    "fixture",
    [
        "parity-base",
        "parity-wrong-transparent",
        "parity-damaged-each",
        "parity-damaged-shard",
        "parity-signed",
        "parity-later-version",
        "parity-above-capacity",
        "parity-above-capacity-wrong-table",
        "parity-above-capacity-and-damaged",
        "parity-frame-type-second-form",
        "parity-placement",
    ],
)
def test_walk_checks_parity_frames_and_tables(fixture: str) -> None:
    manifest = json.loads((_P5 / f"{fixture}.json").read_text())
    for case in manifest["cases"]:
        capacity = case["capacity"]
        if capacity["access"] != "whole":
            continue
        path = _P5 / case["input_file"]
        if path.is_file():
            data = path.read_bytes()
        else:
            with (
                path.with_suffix(".bin.zst").open("rb") as archive,
                ZstdDecompressor().stream_reader(archive) as decoded,
            ):
                data = decoded.read()
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
        for segment, reference in zip(result.segments, expected, strict=True):
            for frame, record in zip(segment.frames, reference[16], strict=True):
                assert frame.item.offset == record[1]
                findings = record[-1]
                damaged = any(finding[0] == 30 for finding in findings)
                newer_geometry = any(
                    finding[:3] == [5, [2, 2], [2, [2, 16]]] for finding in findings
                )
                if data[frame.item.offset + 2] != 16:
                    assert frame.parity is None
                    continue
                assert frame.parity is not None
                assert frame.parity.damaged == damaged
                if frame.payload.state == "verified":
                    assert frame.parity.newer_geometry == newer_geometry
                if fixture in (
                    "parity-base",
                    "parity-placement",
                    "parity-damaged-shard",
                ):
                    assert frame.parity.placement == "placed"


def test_short_parity_span_with_intervening_opaque_item_is_stale() -> None:
    manifest = json.loads((_P5 / "parity-base.json").read_text())
    data = (_P5 / manifest["cases"][0]["input_file"]).read_bytes()
    limits = {
        "stored_bytes_per_frame": 4096,
        "decoded_bytes_per_frame": 4096,
        "decoded_bytes_per_read": 4096,
    }
    clean = walk_file(data, PayloadBudget(**limits))
    (parity,) = (frame for frame in clean.segments[0].frames if frame.parity)
    offset = parity.item.offset
    rewritten = data[:offset] + cbor2.dumps([0]) + data[offset:]

    result = walk_file(rewritten, PayloadBudget(**limits))

    (parity,) = (frame for frame in result.segments[0].frames if frame.parity)
    assert parity.parity is not None
    assert parity.parity.placement == "stale"
    assert not parity.parity.damaged

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise recovery look-ahead and its share of the item count."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import (
    FrameError,
    Header,
    decode_header,
    encode_header,
    encode_plain_frame,
    item_end,
    read_wire,
)
from gts.v2.payload import PayloadBudget
from gts.v2.reader import _Boundary, _ItemWalk, _open_input, _Opener
from gts.v2.resync import (
    RESYNC_MAX,
    DecodedResync,
    ResyncRefusal,
    ResyncScanner,
    encode_resync,
)

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"


def test_incomplete_first_item_does_not_consume_a_slot() -> None:
    data = (
        P1 / "resync-incomplete-item-not-counted.first-item-incomplete.bin"
    ).read_bytes()
    budget = PayloadBudget(
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=RESYNC_MAX,
    )
    walk = _ItemWalk(data, items_per_read=3)
    opened = _open_input(data, walk, ResyncScanner(data, budget), first_and_second=True)
    assert opened.damaged_prefix == _Boundary(0, 199, "torn_tail")
    assert isinstance(opened.decision, _Opener)
    assert opened.decision.scanned
    assert isinstance(opened.decision.value, DecodedResync)
    assert walk.take() == _Boundary(opened.decision.value.next_offset, 298)
    assert walk.take() == _Boundary(298, 299)
    assert walk.take() is None
    assert walk.stopped_at == 299


def test_region_at_decision_point_exhausts_count_before_candidate() -> None:
    data = (P1 / "item-limit-region-candidate-untested.bin").read_bytes()
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    walk = _ItemWalk(data, items_per_read=5)
    failed = walk.take()
    assert failed is not None
    assert failed == _Boundary(0, 234)
    assert walk.recover(ResyncScanner(data, budget), 0, failed) is None
    # The candidate would refuse the decoded-byte charge if it was tested.
    assert not budget.total_refused
    assert [walk.take() for _ in range(3)] == [
        _Boundary(234, 236),
        _Boundary(236, 237),
        _Boundary(237, 271),
    ]
    assert walk.take() == _Boundary(271, len(data), "undelimitable_input")
    assert walk.take() is None
    assert walk.stopped_at is None


@pytest.mark.parametrize(
    ("record", "first_and_second", "expected"),
    [
        ({"v": 1}, True, "first_generation"),
        ({"v": 1}, False, "unsupported_generation"),
        ({"v": 2}, True, "unsupported_generation"),
        ([1, 3], True, "unsupported_generation"),
        ([5, 1], True, "not_this_format"),
    ],
)
def test_generation_dispatch_does_not_select_later_opener(
    record: object, first_and_second: bool, expected: str
) -> None:
    first = cbor2.dumps(record)
    data = first + encode_resync(Header(), offset=len(first))
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    opened = _open_input(
        data,
        _ItemWalk(data, items_per_read=8),
        ResyncScanner(data, budget),
        first_and_second=first_and_second,
    )
    assert opened.decision == expected
    assert opened.refusal is None


def test_walked_header_has_absolute_item_boundary() -> None:
    header = encode_header(Header())
    data = header + b"\xff"
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    walk = _ItemWalk(data, items_per_read=1)
    opening = _open_input(
        data, walk, ResyncScanner(data, budget), first_and_second=False
    )
    assert isinstance(opening.decision, _Opener)
    assert opening.decision.item == _Boundary(0, len(header))
    assert opening.decision.kind == "header"
    assert walk.stopped_at == len(header)


@pytest.mark.parametrize(
    "name",
    [
        "resync-after-torn-swallowed-failing",
        "resync-after-undelimitable",
        "resync-heal-at-item-limit",
        "resync-heal-witness-removed",
        "resync-healed-not-scanned",
        "resync-flood-true-offsets",
        "link-healed-before-transparent",
        "resync-parity-first",
        "link-heal-region-between",
        "lineage-head-broken-last-frame",
    ],
)
def test_read_wire_recovers_and_settles_links_as_in_corpus(name: str) -> None:
    entry = json.loads((P1 / f"{name}.json").read_text())
    for case in entry["cases"]:
        capacity = case["capacity"]
        if capacity["access"] != "whole":
            continue
        data = (P1 / case["input_file"]).read_bytes()
        result = read_wire(
            data,
            items_per_read=capacity["items_per_read"],
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
        )
        file_result = cbor2.loads(bytes.fromhex(case["read_result"]))[-1]
        for segment, report in zip(result.segments, file_result[1], strict=True):
            _, offset, length, opener, *_ = report
            assert segment.opener.item.offset == offset
            assert segment.end_offset == offset + length
            assert segment.opener.scanned == (opener[0] == 2 and opener[1] == 2)
            for frame, record in zip(segment.frames, report[-2], strict=True):
                assert frame.position == record[0]
                assert segment.items[frame.position].offset == record[1]
                assert frame.link.state == ("linked", "healed", "broken")[record[4] - 1]
                assert (
                    frame.payload.state
                    == ("verified", "damaged", "unchecked")[record[5][0] - 1]
                )
            assert list(segment.resync_refusals) == [
                ResyncRefusal(finding[4][1], finding[6][1])
                for finding in report[-1]
                if finding[0] == 20 and finding[1] == [2, 86]
            ]


def test_malformed_envelope_charges_valid_payload_before_resync() -> None:
    header = encode_header(Header())
    predecessor = decode_header(header).identifier
    assert predecessor is not None
    fields = cbor2.loads(encode_plain_frame(predecessor, 8, [bytes(32)], b"x"))
    fields[2] = False  # A chunk must be critical.
    malformed = cbor2.dumps(fields)
    prefix = header + malformed + b"\x1c"
    data = prefix + encode_resync(Header(), offset=len(prefix))
    result = read_wire(
        data,
        items_per_read=10,
        stored_bytes_per_frame=64,
        decoded_bytes_per_frame=1,
        decoded_bytes_per_read=RESYNC_MAX,
    )
    (segment,) = result.segments
    (frame,) = segment.frames
    assert isinstance(frame.envelope, FrameError)
    assert frame.envelope.reason == "forbidden_combination"
    assert frame.payload.state == "verified"
    assert segment.resync_refusals == (ResyncRefusal(len(prefix), RESYNC_MAX + 1),)


@pytest.mark.parametrize(
    "name", ["frame-compressed-empty", "payload-codec-constraint-twice"]
)
def test_earlier_envelope_error_precedes_codec_constraint(name: str) -> None:
    result = read_wire(
        (P1 / f"{name}.bin").read_bytes(),
        items_per_read=16,
        stored_bytes_per_frame=4096,
        decoded_bytes_per_frame=4096,
        decoded_bytes_per_read=4096,
    )
    damaged = [
        frame
        for segment in result.segments
        for frame in segment.frames
        if frame.payload.state == "damaged"
    ]
    (frame,) = damaged
    assert isinstance(frame.envelope, FrameError)
    assert frame.payload.reason is None


@pytest.mark.parametrize("indefinite", [False, True])
def test_stored_check_reads_bytes_with_non_deterministic_head(indefinite: bool) -> None:
    data = (P1 / "frame-overhead-compressed.bin").read_bytes()
    offset = decode_header(data).next_offset + 2
    for _ in range(7):
        offset = item_end(data, offset)
    end = item_end(data, offset)
    payload = cbor2.loads(data[offset:end])
    if indefinite:
        stored = b"\x5f" + cbor2.dumps(payload[:4]) + cbor2.dumps(payload[4:]) + b"\xff"
    else:
        stored = b"\x5b" + len(payload).to_bytes(8, "big") + payload
    result = read_wire(
        data[:offset] + stored + data[end:],
        items_per_read=16,
        stored_bytes_per_frame=4096,
        decoded_bytes_per_frame=4096,
        decoded_bytes_per_read=4096,
    )
    frame = result.segments[0].frames[0]
    assert isinstance(frame.envelope, FrameError)
    assert frame.envelope.reason == "not_deterministic"
    assert frame.link.state == "linked"
    assert frame.payload.state == "damaged"
    assert frame.payload.stored_matches is True

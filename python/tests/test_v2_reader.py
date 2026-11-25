# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise recovery look-ahead and its share of the item count."""

import json
from collections import Counter
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
    read_content,
    read_wire,
)
from gts.v2.canonical_check import CanonicalResult, CheckState
from gts.v2.content import ContentCapacity
from gts.v2.file_result import (
    FoldState,
    OpaqueFold,
    PayloadState,
    UncheckedPayload,
    fold_state,
    payload_state,
)
from gts.v2.findings import (
    FINDING_CODES,
    REASON_CODES,
    FindingRecord,
    finding_record,
    frame_findings,
    framing_findings,
    segment_findings,
    sort_findings,
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
from gts.v2.segment_result import (
    FrameResult,
    HeaderOpener,
    frame_results,
    segment_results,
)
from gts.v2.signatures import HeldKey, SignatureVerifier
from gts.v2.typed_file import CompleteVisibility, file_result
from gts.v2.writer import SegmentWriter
from test_v2_content import _PARITY_CODES, _REASONS, _VECTORS, _contents

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


def test_first_generation_input_defaults_to_the_legacy_decision() -> None:
    """Readers keep reading older generations, so the default admits them."""
    data = cbor2.dumps({"v": 1})
    opened = read_wire(
        data,
        items_per_read=8,
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=0,
    )
    assert opened.opening.decision == "first_generation"
    refused = read_wire(
        data,
        items_per_read=8,
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=0,
        first_and_second=False,
    )
    assert refused.opening.decision == "unsupported_generation"


@pytest.mark.parametrize(
    ("first", "decision"),
    [
        (cbor2.dumps(5), "not_this_format"),
        (cbor2.dumps([1, 3]), "unsupported_generation"),
    ],
    ids=["not_this_format", "generation_three"],
)
def test_refused_first_item_keeps_its_offset_and_length(
    first: bytes, decision: str
) -> None:
    """A refused generation reports the complete first item, not the whole input."""
    data = first + b"\x00"
    opened = read_wire(
        data,
        items_per_read=8,
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=0,
    )
    assert opened.opening.decision == decision
    assert opened.opening.first_item == _Boundary(0, len(first))
    # The typed file finding carries the same subject through the content read.
    read = read_content(data)
    assert read.wire.segments == ()
    assert [(f.code, f.offset, f.length) for f in read.findings] == [
        (decision, 0, len(first))
    ]


def test_later_version_parity_after_a_stop_is_opaque_as_stopped() -> None:
    """A later-version parity frame after an unsupported critical frame is `stopped`.

    It carries no finding of its own: the stop is its reason.
    """
    case = json.loads(
        (_VECTORS / "P1" / "parity-later-version-after-stop.json").read_text()
    )["cases"][0]
    c = case["capacity"]
    capacity = ContentCapacity(
        c["stored_bytes_per_frame"],
        c["decoded_bytes_per_frame"],
        c["decoded_bytes_per_read"],
        c["items_per_read"],
        c["rebuilt_bytes_per_read"],
        c["formed_bytes_per_read"],
        c["labelled_lines"],
        c["labelled_bytes_per_read"],
        c["labelling_units"],
    )
    raw = (_VECTORS / "P1" / case["input_file"]).read_bytes()
    read = read_content(raw, capacity)
    frames = {frame.position: frame for frame in read.wire.segments[0].frames}
    assert fold_state(read, 0, frames[3]) == OpaqueFold("unsupported_critical")
    assert fold_state(read, 0, frames[4]) == OpaqueFold("stopped")
    assert [finding.code for finding in read.findings] == [
        "uncommitted_tail",
        "unsupported_critical",
    ]


def test_payload_beyond_stored_capacity_is_unchecked() -> None:
    """A frame over the stored limit is unchecked for capacity, which P2 names."""
    writer = SegmentWriter()
    writer.append(1, [0, [[1, 1], [3, 1]] * 8, 2, 64], b"")
    read = read_wire(
        writer.to_bytes(),
        items_per_read=64,
        stored_bytes_per_frame=8,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=0,
    )
    frames = [frame for segment in read.segments for frame in segment.frames]
    assert [payload_state(frame.payload) for frame in frames] == [
        UncheckedPayload("exceeds_capacity")
    ]


@pytest.mark.parametrize(
    "name, expected",
    [
        # An unsupported critical frame stops the segment.
        (
            "frame-unknown-critical",
            {
                1: "not_content",
                2: OpaqueFold("unsupported_critical"),
                3: OpaqueFold("stopped"),
                4: OpaqueFold("stopped"),
            },
        ),
        (
            "frame-unknown-noncritical",
            {1: "not_content", 2: OpaqueFold("unknown_frame_type"), 3: "not_content"},
        ),
        (
            "parity-geometry-newer",
            {
                1: "not_content",
                2: "not_content",
                3: "not_content",
                4: OpaqueFold("unknown_frame_type"),
            },
        ),
    ],
)
def test_fold_state_gives_the_p2_reason_of_each_opaque_frame(
    name: str, expected: dict[int, object]
) -> None:
    """Each opaque frame carries the P2 reason its fold decision site gives."""
    raw, _, capacity = next(_contents("P1", name))
    read = read_content(raw, capacity)
    frames = read.wire.segments[0].frames
    assert {frame.position: fold_state(read, 0, frame) for frame in frames} == expected


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


_CLOSING_REASONS = {**_REASONS, 19: "index_offsets", 23: "index_locator"}
_PARITY_REASONS = {
    40: "data",
    41: "parity",
    42: "too_many_erasures",
    43: "verify_failed",
}
_PARITY_FRAME = 16
_INDEX_CLAIM_FAILED = 18


def _option(value: list) -> int | None:
    return value[1] if value[0] == 2 else None


def _parity_findings(segment: list) -> Counter:
    reports = [
        *segment[-1],
        *(
            f
            for frame in segment[-2]
            if frame[3] == [2, _PARITY_FRAME]
            for f in frame[-1]
        ),
    ]
    return Counter(
        (
            _PARITY_CODES[code],
            _option(position),
            _PARITY_REASONS.get(_option(reason)),
            _option(count),
            _option(offset),
            _option(length),
        )
        for code, reason, _, position, offset, length, count in reports
        if code in _PARITY_CODES
    )


@pytest.mark.parametrize(
    ("part", "name"),
    [
        *(
            ("P1", name)
            for name in [
                "file-first-incomplete",
                "file-first-undelimitable",
                "resync-first-item",
                "resync-first-item-fails",
                "resync-first-item-torn",
                "resync-first-byte-foreign",
                "resync-resync-first",
                "first-item-five-alone",
                "frame-identifier-type",
                "frame-unknown-transparent-not-bool",
                "parity-codec-newer",
                "links-unavailable-lineage-not-compared",
                "resync-after-torn-swallowed-passing",
                "resync-heal-no-witness-past-region",
                "resync-other-generation-scanned",
                "resync-newer-kind-not-scanned",
            ]
        ),
        *(
            ("P5", name)
            for name in [
                "parity-wrong-transparent",
                "parity-damaged-each",
                "parity-signed",
                "parity-frame-type-second-form",
                "parity-unexplained-two-stripes",
                "range-transparent",
            ]
        ),
        ("P2", "finding__parity_stale"),
        ("P7", "read__empty"),
        ("P7", "read__not-this-format"),
        ("P7", "read__generation-three"),
    ],
)
def test_read_wire_matches_corpus_read_result_for_remaining_fixtures(
    part: str, name: str
) -> None:
    """Guard the wire reader against corpus fixtures not asserted elsewhere.

    Each fixture's whole-input cases are read with `read_wire` and compared with
    the recorded `read_result` on the dispatch of the first item, segment offset,
    length and scanned flag, frame link, payload and stored state, closing
    findings, and parity findings with the unprotected tail.
    """
    folder = P1.parent / part
    entry = json.loads((folder / f"{name}.json").read_text())
    checked = 0
    for case in entry["cases"]:
        capacity = case["capacity"]
        if capacity["access"] != "whole":
            continue
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        result = read_wire(
            (folder / case["input_file"]).read_bytes(),
            items_per_read=capacity["items_per_read"],
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
            first_and_second=report[0] == 2,
        )
        file_result = report[-1]
        assert (file_result[0] == 1) == (result.opening.decision == "first_generation")
        reports = file_result[1] if file_result[0] == 2 else []
        for segment, record in zip(result.segments, reports, strict=True):
            _, offset, length, opener, *_ = record
            assert segment.opener.item.offset == offset
            assert segment.end_offset == offset + length
            assert segment.opener.scanned == (opener[0] == 2 and opener[1] == 2)
            for frame, expected in zip(segment.frames, record[-2], strict=True):
                assert frame.position == expected[0]
                assert segment.items[frame.position].offset == expected[1]
                assert (
                    frame.link.state == ("linked", "healed", "broken")[expected[4] - 1]
                )
                assert (
                    frame.payload.state
                    == ("verified", "damaged", "unchecked")[expected[5][0] - 1]
                )
                assert frame.payload.stored_matches == (
                    None if (stored := _option(expected[6])) is None else stored == 1
                )
            assert [
                (finding.position, finding.reason) for finding in segment.closing
            ] == [
                (_option(finding[3]), _CLOSING_REASONS[_option(finding[1])])
                for finding in record[-1]
                if finding[0] == _INDEX_CLAIM_FAILED
            ]
            assert Counter(
                (
                    finding.code,
                    finding.position,
                    finding.reason,
                    finding.count,
                    finding.offset,
                    finding.length,
                )
                for finding in segment.parity.findings
                if finding.code in _PARITY_CODES.values()
            ) == _parity_findings(record)
        checked += 1
    assert checked


def _corpus_input(name: str, size: int) -> tuple[bytes, ContentCapacity]:
    """Return the input and capacity of corpus case `name` with `size` bytes."""
    folder = _VECTORS / "P1"
    for case in json.loads((folder / f"{name}.json").read_text())["cases"]:
        raw = (folder / case["input_file"]).read_bytes()
        if len(raw) == size:
            c = case["capacity"]
            return raw, ContentCapacity(
                c["stored_bytes_per_frame"],
                c["decoded_bytes_per_frame"],
                c["decoded_bytes_per_read"],
                c["items_per_read"],
                c["rebuilt_bytes_per_read"],
                c["formed_bytes_per_read"],
                c["labelled_lines"],
                c["labelled_bytes_per_read"],
                c["labelling_units"],
            )
    raise AssertionError(f"no case of {name} has an input of {size} bytes")


@pytest.mark.parametrize(
    "name, size, code, length",
    [
        ("file-not-this-format", 4, "not_this_format", 4),
        ("file-not-this-format", 1, "not_this_format", 1),
        ("gen-first-generation-two", 4, "unsupported_generation", 4),
        # The first item is 11 bytes; the 12th byte is trailing input, not part of it.
        ("gen-record-three", 12, "unsupported_generation", 11),
    ],
)
def test_rejected_first_item_is_a_segmentless_file_result(
    name: str, size: int, code: str, length: int
) -> None:
    """A rejected first item gives a segmentless file with one finding of its length."""
    raw, capacity = _corpus_input(name, size)
    result = file_result(read_content(raw, capacity), raw, capacity)
    assert [(f.code, f.offset, f.length) for f in result.findings] == [
        (FINDING_CODES[code], 0, length)
    ]
    assert result.end == "no_commit"
    assert result.left_out == 0
    assert result.visible == CompleteVisibility(0)


@pytest.mark.parametrize(
    "name, size, first_and_second, code, offset, length, end, left_out",
    [
        ("file-first-incomplete", 3, True, "torn_tail", 0, 3, "torn", 1),
        ("locator-short-input", 10, True, "torn_tail", 0, 10, "torn", 1),
        (
            "file-first-undelimitable",
            1,
            True,
            "undelimitable_input",
            0,
            1,
            "undelimitable",
            0,
        ),
        # A second-generation reader refuses a first-generation first item of 4 bytes.
        (
            "gen-first-generation-second-only",
            5,
            False,
            "unsupported_generation",
            0,
            4,
            "no_commit",
            0,
        ),
        ("file-empty", 0, True, "empty_input", None, None, "no_commit", 0),
    ],
)
def test_first_item_finding_of_each_decision_is_segmentless(
    name: str,
    size: int,
    first_and_second: bool,
    code: str,
    offset: int | None,
    length: int | None,
    end: str,
    left_out: int,
) -> None:
    """Each rejected first item gives a segmentless file with its own finding."""
    raw, capacity = _corpus_input(name, size)
    read = read_content(raw, capacity, first_and_second=first_and_second)
    result = file_result(read, raw, capacity)
    assert [(f.code, f.offset, f.length) for f in result.findings] == [
        (FINDING_CODES[code], offset, length)
    ]
    assert result.end == end
    assert result.left_out == left_out


def test_item_limit_of_zero_refuses_the_input_as_not_read() -> None:
    """With no item allowed, the whole input is one refused first item (P7, 1.2)."""
    raw = b"\x00\x00\x00"
    capacity = ContentCapacity(items_per_read=0)
    read = read_content(raw, capacity)
    result = file_result(read, raw, capacity)
    assert [
        (f.code, f.reason, f.count, f.offset, f.length, f.position)
        for f in result.findings
    ] == [
        (
            FINDING_CODES["exceeds_capacity"],
            REASON_CODES["item_count"],
            1,
            0,
            3,
            None,
        )
    ]
    assert result.end == "not_read"
    assert result.left_out == 0


def test_valid_first_generation_header_is_not_a_segmentless_result() -> None:
    """A valid first-generation header keeps its legacy result, not a tag-2 result."""
    raw, capacity = _corpus_input("gen-first-generation", 4)
    read = read_content(raw, capacity)
    with pytest.raises(ValueError, match="first-generation"):
        file_result(read, raw, capacity)


def _corpus_case(
    name: str, label: str
) -> tuple[bytes, ContentCapacity, SignatureVerifier, list[list[object]]]:
    """Return the input, capacity, keys and segments of case `label` of `name`."""
    folder = _VECTORS / "P1"
    for case in json.loads((folder / f"{name}.json").read_text())["cases"]:
        if case.get("label", "") != label:
            continue
        c = case["capacity"]
        capacity = ContentCapacity(
            c["stored_bytes_per_frame"],
            c["decoded_bytes_per_frame"],
            c["decoded_bytes_per_read"],
            c["items_per_read"],
            c["rebuilt_bytes_per_read"],
            c["formed_bytes_per_read"],
            c["labelled_lines"],
            c["labelled_bytes_per_read"],
            c["labelling_units"],
        )
        record = cbor2.loads(bytes.fromhex(case["read_result"]))
        trusted, held = record[5]
        verifier = SignatureVerifier(
            trusted_keys=trusted, held_keys=[HeldKey(*key) for key in held]
        )
        return (
            (folder / case["input_file"]).read_bytes(),
            capacity,
            verifier,
            record[-1][1],
        )
    raise AssertionError(f"no case {label!r} of {name}")


def _present(option: list[object]) -> object:
    return None if option == [1] else option[1]


_UNCHECKED_CODES = {"exceeds_capacity": 2, "labelling_refused": 3, "not_claimed": 5}


def _check_codes(result: CanonicalResult) -> list[object]:
    """Return `[claimed, structure, labelling]` as P2 encodes them."""

    def state(check: CheckState) -> list[int]:
        if check.state == "unchecked":
            assert check.reason is not None
            return [2, _UNCHECKED_CODES[check.reason]]
        return [1] if check.state == "verified" else [3]

    return [result.claimed, state(result.structure), state(result.labelling)]


_PAST_CODES = {"absent": 1, "matches": 2, "differs": 3, "not_checked": 4}
_REACHED_CODES = {"walked": 1, "scanned": 2}
_INTEGRITY_CODES = {
    "intact": 1,
    "intact_unchecked": 2,
    "damaged": 3,
    "not_checked": 4,
}
_AUTHENTICITY_CODES = {
    "authenticated": 1,
    "partly_authenticated": 2,
    "unauthenticated": 3,
}


@pytest.mark.parametrize(
    "name, label",
    [
        # A header opener, and a readable resync item reached by a scan.
        ("append-after-torn-refused", "ii-after"),
        # A past that matches, one that differs and one refused by the past count.
        ("append-after-torn-refused", "i-after"),
        ("head-longer-map-or-tag", "map-identifier"),
        ("resync-past-not-checked-count", ""),
        # Marks that are not applicable to the reader.
        ("marks-code-zero", ""),
        # A resync item unreadable before its offset field, and a newer hash.
        ("opener-option-tag-first", "lineage-tag-zero-then-null"),
        ("header-hash-newer", ""),
        # An unchecked payload, an opaque item that is not a frame and a damaged region.
        ("payload-unknown-codec", ""),
        ("append-opaque-tail", "after"),
        ("append-resync-fresh-after-pad", "framing"),
        # A claimed canonical form, one that holds and one that fails.
        ("append-after-canonical", "canonical"),
        ("header-minimal", ""),
        # A signed commit, a transparent frame, and frames that are not anchored.
        ("equal-heads-different-bytes", "b3"),
        ("link-heal-closing-items", "damaged"),
    ],
)
def test_segment_results_give_each_opener_of_the_corpus(name: str, label: str) -> None:
    """Each segment states its offset, length, opener and header as the corpus does."""
    raw, capacity, verifier, expected = _corpus_case(name, label)
    read = read_content(raw, capacity, verifier=verifier)
    results = segment_results(read, raw, capacity)
    assert len(results) == len(expected)
    for got, record in zip(results, expected, strict=True):
        opener = got.opener
        if isinstance(opener, HeaderOpener):
            stated: list[object] = [1]
        else:
            stated = [
                2,
                _REACHED_CODES[opener.reached],
                opener.displaced,
                _PAST_CODES[opener.past],
                opener.marks_ignored,
            ]
        assert (
            [
                got.ordinal,
                got.offset,
                got.length,
                stated,
                got.header_identifier,
                got.registry_version,
                got.hash_algorithm,
                got.profile,
                _INTEGRITY_CODES[got.integrity],
                _AUTHENTICITY_CODES[got.authenticity],
                got.anchored_frames,
                got.committed_frames,
                got.authenticated_frames,
                list(got.transparent_frames),
                _check_codes(got.canonical),
                got.content_digest,
            ]
            == [
                record[0],
                record[1],
                record[2],
                record[3],
                _present(record[4]),  # type: ignore[arg-type]
                _present(record[5]),  # type: ignore[arg-type]
                _present(record[6]),  # type: ignore[arg-type]
                _present(record[7]),  # type: ignore[arg-type]
                *record[8:14],
                record[14][:3],  # type: ignore[index]
                _present(record[15]),  # type: ignore[arg-type]
            ]
        )


@pytest.mark.parametrize("name", ["payload-unknown-codec", "frame-storage-tag-newer"])
def test_frame_with_unknown_codec_is_opaque_as_unknown_codec(name: str) -> None:
    """A frame of an unknown payload codec is opaque as `unknown_codec`."""
    raw = (_VECTORS / "P1" / f"{name}.bin").read_bytes()
    read = read_content(raw)
    frame = read.wire.segments[0].frames[0]
    assert fold_state(read, 0, frame) == OpaqueFold("unknown_codec")


_LINK_CODES = {"linked": 1, "healed": 2, "broken": 3}
_SIGNATURE_REASON_CODES = {
    "unknown_signature_algorithm": 53,
    "key_not_held": 54,
    "key_mismatch": 57,
    "value_length": 58,
    "non_canonical_scalar": 59,
    "small_order_key": 60,
    "small_order_nonce": 61,
    "equation_failed": 62,
    "composite_classical_failed": 63,
    "composite_post_quantum_failed": 64,
}
_SIGNATURE_STATUS_CODES = {"valid": 1, "invalid": 2, "unverified": 3}


def _signature_codes(frame: FrameResult) -> list[list[object]]:
    return [
        [
            _SIGNATURE_STATUS_CODES[signature.outcome.status],
            None
            if signature.outcome.reason is None
            else _SIGNATURE_REASON_CODES[signature.outcome.reason],
            signature.algorithm,
            signature.key_id,
            signature.coverage_first,
            signature.coverage_end,
            signature.authenticated,
        ]
        for signature in frame.signatures
    ]


_STORED_CODES = {"matches": 1, "mismatch": 2}
_UNCHECKED_REASON_CODES = {"unknown_codec": 1, "exceeds_capacity": 2}
_OPAQUE_REASON_CODES = {
    "damaged": 1,
    "malformed": 2,
    "unknown_frame_type": 3,
    "unsupported_critical": 4,
    "stopped": 5,
    "chain_break": 6,
    "exceeds_capacity": 7,
    "unknown_codec": 8,
}


def _payload_code(state: PayloadState) -> list[int]:
    if state == "verified":
        return [1]
    if state == "damaged":
        return [2]
    return [3, _UNCHECKED_REASON_CODES[state.reason]]


def _fold_code(state: FoldState) -> list[int]:
    if state == "folded":
        return [1]
    if state == "not_content":
        return [2]
    return [3, _OPAQUE_REASON_CODES[state.reason]]


@pytest.mark.parametrize(
    "name, label",
    [
        ("append-after-canonical", "canonical"),
        # Damaged payloads whose codec constraint is malformed and not merely damaged,
        # alone and under a broken link.
        ("payload-output-longer", ""),
        ("chain-break-withheld-codec-constraint", ""),
        ("payload-unknown-codec", ""),
        ("parity-later-version-after-stop", ""),
        # Two signatures on a commit, one of them authenticating.
        ("equal-heads-different-bytes", "b3"),
    ],
)
def test_frame_results_give_each_frame_of_the_corpus(name: str, label: str) -> None:
    """Each frame states its place, type, link, payload, stored check and fold."""
    raw, capacity, verifier, expected = _corpus_case(name, label)
    read = read_content(raw, capacity, verifier=verifier)
    for ordinal, segment in enumerate(expected):
        frames = frame_results(read, ordinal)
        assert [
            [
                frame.position,
                frame.offset,
                frame.length,
                frame.frame_type,
                _LINK_CODES[frame.link],
                _payload_code(frame.payload),
                None if frame.stored is None else _STORED_CODES[frame.stored],
                _signature_codes(frame),
                _fold_code(frame.fold),
            ]
            for frame in frames
        ] == [
            [
                record[0],
                record[1],
                record[2],
                _present(record[3]),
                record[4],
                record[5],
                _present(record[6]),
                [[s[0], _present(s[1]), *s[2:]] for s in record[7]],
                record[8],
            ]
            for record in segment[16]
        ]


@pytest.mark.parametrize(
    "name, size",
    [
        ("file-not-this-format", 4),
        ("gen-record-three", 12),
        ("file-first-incomplete", 3),
        ("file-first-undelimitable", 1),
    ],
)
def test_file_findings_are_typed_records_of_the_corpus(name: str, size: int) -> None:
    """The findings of a file with no segment encode as the corpus findings do."""
    raw, capacity = _corpus_input(name, size)
    folder = _VECTORS / "P1"
    expected = next(
        cbor2.loads(bytes.fromhex(case["read_result"]))[-1][2]
        for case in json.loads((folder / f"{name}.json").read_text())["cases"]
        if (folder / case["input_file"]).stat().st_size == size
    )
    got = [finding_record(f) for f in read_content(raw, capacity).findings]
    assert [
        (f.code, f.reason, f.position, f.offset, f.length, f.count) for f in got
    ] == [
        (
            f[0],
            _present(f[1]),
            _present(f[3]),
            _present(f[4]),
            _present(f[5]),
            _present(f[6]),
        )
        for f in expected
    ]


@pytest.mark.parametrize(
    "name, label",
    [
        # A codec constraint is malformed, and a malformed frame has no chain break.
        ("payload-output-longer", ""),
        ("chain-break-withheld-codec-constraint", ""),
        # A broken link, with the signature of a frame whose key is not held.
        ("link-heal-closing-items", "damaged"),
        ("append-damaged-length-head", "i-after"),
    ],
)
def test_frame_findings_give_the_wire_findings_of_the_corpus(
    name: str, label: str
) -> None:
    """Each frame states the findings of its link, payload and signatures."""
    raw, capacity, verifier, expected = _corpus_case(name, label)
    read = read_content(raw, capacity, verifier=verifier)
    for ordinal, segment in enumerate(expected):
        for record in segment[16]:
            frame = next(
                f for f in read.wire.segments[ordinal].frames if f.position == record[0]
            )
            got = sorted(
                (f.code, f.reason, f.position, f.offset, f.length, f.count, f.subject)
                for f in frame_findings(read, ordinal, frame)
            )
            want = sorted(
                (
                    f[0],
                    _present(f[1]),
                    _present(f[3]),
                    _present(f[4]),
                    _present(f[5]),
                    _present(f[6]),
                    None if f[2] == [1] else tuple(f[2][1]),
                )
                for f in record[10]
            )
            assert got == want


@pytest.mark.parametrize(
    "name, label",
    [
        # A lineage that the previous segment contradicts, and one that is absent.
        ("concat-continuation", "joined"),
        ("append-resync-fresh-after-pad", "framing"),
        # An unreadable opener, and a resync mark that breaks its grammar.
        ("gen-later-header-three", ""),
        ("head-longer-map-or-tag", "tag-in-mark"),
        # A region that framing could not delimit, before a resumption.
        ("append-check-recovery-resumed", ""),
    ],
)
def test_segment_and_file_findings_are_those_of_the_corpus(
    name: str, label: str
) -> None:
    """The findings of a segment and of the framing of a file match the corpus."""
    raw, capacity, verifier, expected = _corpus_case(name, label)
    read = read_content(raw, capacity, verifier=verifier)
    folder = _VECTORS / "P1"
    case = next(
        c
        for c in json.loads((folder / f"{name}.json").read_text())["cases"]
        if c.get("label", "") == label
    )
    file_findings = cbor2.loads(bytes.fromhex(case["read_result"]))[-1][2]

    def fields(f: FindingRecord) -> tuple[object, ...]:
        return (f.code, f.reason, f.position, f.offset, f.length, f.count, f.subject)

    def stated(f: list[object]) -> tuple[object, ...]:
        return (
            f[0],
            _present(f[1]),  # type: ignore[arg-type]
            _present(f[3]),  # type: ignore[arg-type]
            _present(f[4]),  # type: ignore[arg-type]
            _present(f[5]),  # type: ignore[arg-type]
            _present(f[6]),  # type: ignore[arg-type]
            None if f[2] == [1] else tuple(f[2][1]),  # type: ignore[index, arg-type]
        )

    for ordinal, segment in enumerate(expected):
        assert sorted(fields(f) for f in segment_findings(read, ordinal)) == sorted(
            stated(f)
            for f in segment[17]  # type: ignore[arg-type]
        )
    assert sorted(fields(f) for f in framing_findings(read)) == sorted(
        stated(f) for f in file_findings if f[0] in (8, 9)
    )


_FILE_END_CODES = {
    "at_authenticated_commit": 1,
    "at_valid_commit": 2,
    "after_commit": 3,
    "no_commit": 4,
    "torn": 5,
    "undelimitable": 6,
    "not_read": 7,
}


@pytest.mark.parametrize(
    "name, label",
    [
        ("append-after-canonical", "canonical"),
        ("equal-heads-different-bytes", "b3"),
        ("link-heal-closing-items", "damaged"),
        ("append-check-recovery-resumed", ""),
        ("concat-continuation", "joined"),
        ("payload-unknown-codec", ""),
    ],
)
def test_file_result_assembles_every_segment_and_frame_of_the_corpus(
    name: str, label: str
) -> None:
    """The file result states the end, left out, digest, visibility and records."""
    raw, capacity, verifier, expected = _corpus_case(name, label)
    folder = _VECTORS / "P1"
    case = next(
        c
        for c in json.loads((folder / f"{name}.json").read_text())["cases"]
        if c.get("label", "") == label
    )
    record = cbor2.loads(bytes.fromhex(case["read_result"]))[-1]
    result = file_result(read_content(raw, capacity, verifier=verifier), raw, capacity)
    assert len(result.segments) == len(expected)
    assert [len(s.frames) for s in result.segments] == [len(s[16]) for s in expected]
    assert _FILE_END_CODES[result.end] == record[5]
    assert result.left_out == record[6]
    assert result.content_digest == _present(record[3])
    assert result.visible == CompleteVisibility(record[4][1])


def test_findings_of_a_frame_are_sorted_by_procedure_step() -> None:
    """Findings sort by the P7 step that produced them, not by code."""
    records = [
        FindingRecord(22, None, 1, None, None, None, step=15),
        FindingRecord(17, None, 1, None, None, 1, step=13),
    ]
    assert [r.code for r in sort_findings(records)] == [17, 22]

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise the P1 resync grammar and its P7 opener fields."""

import json
from pathlib import Path

import cbor2
import pytest
import zstandard

from gts.v2 import Header, PayloadBudget, decode_frame, encode_plain_frame
from gts.v2.resync import (
    RESYNC_MAX,
    RESYNC_PATTERN,
    DecodedResync,
    ResyncError,
    ResyncRefusal,
    ResyncScanner,
    decode_resync,
    encode_resync,
)

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"
_MALFORMED = {
    3: "not_deterministic",
    4: "wrong_type",
    5: "wrong_length",
    6: "out_of_range",
    7: "unregistered_value",
    8: "unordered_or_repeated",
}


@pytest.mark.parametrize(
    "name",
    [
        "resync-minimal",
        "resync-full",
        "resync-displaced-join",
        "resync-identifier-field-differs",
        "resync-after-torn-header",
        "marks-code-zero",
        "marks-cut-inside-mark",
        "marks-filler-on-scope",
        "marks-future-finding-code",
        "marks-head-not-shortest",
        "marks-none",
        "marks-observed-on-scope",
        "marks-order-at-one-byte",
        "marks-outside-grammar",
        "marks-range-after-item",
        "marks-range-empty",
        "marks-unknown-tag",
    ],
)
def test_resync_opener_matches_corpus(name: str) -> None:
    entry = json.loads((P1 / f"{name}.json").read_text())
    for case in entry["cases"]:
        data = (P1 / case["input_file"]).read_bytes()
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        if report[1] != 1:
            continue
        for segment in report[-1][1]:
            _, start, length, opener, identifier, registry, *_ = segment
            if opener[0] != 2:
                continue
            _, _, displaced, _, marks_ignored = opener
            if registry == [1]:
                reason = next(
                    _MALFORMED[finding[1][1]]
                    for finding in segment[-1]
                    if finding[0] == 7 and finding[3] == [2, 0]
                )
                with pytest.raises(ResyncError) as error:
                    decode_resync(data, start)
                assert error.value.reason == reason
                assert (error.value.stated_offset != start) == displaced
            else:
                decoded = decode_resync(data, start)
                assert decoded.header.registry_version == registry[1]
                assert decoded.displaced == displaced
                assert decoded.marks_ignored == marks_ignored
                assert decoded.identifier == (
                    identifier[1] if identifier[0] == 2 else None
                )
                assert decoded.next_offset <= start + length


@pytest.mark.parametrize("name", ["resync-minimal", "marks-torn-tail-recorded"])
def test_writer_reproduces_resync_opener(name: str) -> None:
    data = (P1 / f"{name}.bin").read_bytes()
    start = data.index(RESYNC_PATTERN)
    opener = decode_resync(data, start)
    assert (
        encode_resync(
            opener.header,
            offset=opener.offset,
            past=opener.past,
            marks=[cbor2.loads(mark.encoded) for mark in reversed(opener.marks)],
        )
        == data[start : opener.next_offset]
    )


def test_writer_requires_filler_to_reach_opener() -> None:
    with pytest.raises(ValueError, match="filler must end"):
        encode_resync(Header(), offset=210, marks=[[3, [2, 200, 9]]])


@pytest.mark.parametrize(
    ("invalid", "reason"), [(True, "wrong_type"), (False, "out_of_range")]
)
def test_oversized_text_checks_utf8_through_the_cap(invalid: bool, reason: str) -> None:
    record = cbor2.loads(encode_resync(Header(), offset=0))
    record[9] = [[4, "a" * 4100]]
    data = cbor2.dumps(record)
    body = data.index(b"aaaa")
    if invalid:
        data = data[:body] + b"\xff" + data[body + 1 :]
    else:
        # The cap splits a code point in an otherwise valid prefix.
        data = data[:4095] + b"\xc3\xa9" + data[4097:]
    with pytest.raises(ResyncError) as error:
        decode_resync(data)
    assert error.value.reason == reason
    assert error.value.offset == (body - 3 if invalid else 4096)


def test_new_registry_unknown_hash_keeps_opener_fields() -> None:
    record = cbor2.loads(encode_resync(Header(), offset=0))
    record[3:5] = [2, 9]
    record[6] = [2, [b"lineage", 1]]
    record[8] = [2, b"past"]
    record[10] = b"identifier"
    decoded = decode_resync(cbor2.dumps(record))
    assert decoded.identifier is None
    assert decoded.stored_identifier == b"identifier"
    assert decoded.past == b"past"
    assert decoded.header.lineage is not None
    assert decoded.header.lineage.previous_head == b"lineage"


@pytest.mark.parametrize(
    "name", ["resync-after-torn-header", "resync-identifier-false"]
)
def test_scan_candidate_matches_corpus(name: str) -> None:
    case = json.loads((P1 / f"{name}.json").read_text())["cases"][0]
    data = (P1 / case["input_file"]).read_bytes()
    segments = cbor2.loads(bytes.fromhex(case["read_result"]))[-1][1]
    budget = PayloadBudget(
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=RESYNC_MAX,
    )
    start = data.index(RESYNC_PATTERN)
    result = ResyncScanner(data, budget).scan(start, start + 1)
    if len(segments) == 1:
        assert result is None
    else:
        assert result == decode_resync(data, segments[1][1])
        assert isinstance(result, DecodedResync)
        assert result.identifier == segments[1][4][1]
    # Both candidates claim their true offsets and consume the full charge.
    assert budget.charge_decoded(1) == RESYNC_MAX + 1


def test_scan_budget_refusal_matches_true_offset_flood() -> None:
    case = json.loads((P1 / "resync-flood-true-offsets.json").read_text())["cases"][0]
    data = (P1 / case["input_file"]).read_bytes()
    segment = cbor2.loads(bytes.fromhex(case["read_result"]))[-1][1][0]
    _, _, _, _, offset, length, count = segment[-1][0]
    budget = PayloadBudget(
        stored_bytes_per_frame=64,
        decoded_bytes_per_frame=64,
        decoded_bytes_per_read=case["capacity"]["decoded_bytes_per_read"],
    )
    for frame_result in segment[-2]:
        assert budget.check(decode_frame(data, frame_result[1])).state == "verified"
    scanner = ResyncScanner(data, budget)
    start = data.index(RESYNC_PATTERN)
    assert length[1] == RESYNC_MAX
    assert scanner.scan(start, len(data)) == ResyncRefusal(offset[1], count[1])
    assert scanner.scan(start, len(data)) is None
    later = decode_frame(encode_plain_frame(bytes(32), 8, [bytes(32)], b"z"))
    assert budget.check(later).reason == "total_decoded"


def test_scan_skips_false_offset_flood_without_charging() -> None:
    compressed = (P1 / "resync-flood-false-offsets.bin.zst").read_bytes()
    data = zstandard.ZstdDecompressor().decompress(compressed)
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    assert ResyncScanner(data, budget).scan(209, len(data)) is None
    assert not budget.total_refused


def test_scan_charges_failed_shape_once_across_overlapping_runs() -> None:
    prefix = b"\xff"
    unknown = cbor2.loads(encode_resync(Header(), offset=len(prefix)))
    unknown[3:5] = [2, 9]
    prefix += cbor2.dumps(unknown)
    invalid = cbor2.loads(encode_resync(Header(), offset=len(prefix)))
    invalid[8] = False
    next_segment = prefix + cbor2.dumps(invalid)
    data = next_segment + encode_resync(Header(), offset=len(next_segment))
    budget = PayloadBudget(
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=2 * RESYNC_MAX,
    )
    scanner = ResyncScanner(data, budget)
    assert scanner.scan(0, len(next_segment)) is None
    assert scanner.scan(1, len(data)) == decode_resync(data, len(next_segment))
    assert not budget.total_refused
    assert budget.charge_decoded(1) == 2 * RESYNC_MAX + 1

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise the P1 resync grammar and its P7 opener fields."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import Header
from gts.v2.resync import RESYNC_PATTERN, ResyncError, decode_resync, encode_resync

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

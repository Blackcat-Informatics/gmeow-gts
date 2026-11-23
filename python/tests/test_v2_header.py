# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare P1 header processing with the recorded P7 segment results."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import Header, HeaderError, Lineage, decode_header, encode_header

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"


@pytest.mark.parametrize("name, index", [("header-minimal", 0), ("header-lineage", 1)])
def test_header_round_trip_matches_corpus(name: str, index: int) -> None:
    case = json.loads((P1 / f"{name}.json").read_text())["cases"][0]
    data = (P1 / case["input_file"]).read_bytes()
    read_result = cbor2.loads(bytes.fromhex(case["read_result"]))
    segment = read_result[-1][1][index]
    _, offset, length, _, (_, identifier), *_ = segment
    decoded = decode_header(data, offset)
    assert decoded.identifier == identifier
    assert decoded.next_offset == offset + length
    assert encode_header(decoded.header) == data[offset : offset + length]
    assert decoded.header.canonical == 1
    if index:
        assert decoded.header.lineage is not None
        assert decoded.header.lineage.ordinal == 1
        assert decoded.header.lineage.previous_head == data[offset - 33 : offset - 1]


@pytest.mark.parametrize(
    "name, item_offset, reason, byte_offset",
    [
        ("header-length-six", 0, "wrong_length", 0),
        ("header-length-eight", 0, "wrong_length", 0),
        ("header-registry-zero", 0, "out_of_range", 3),
        ("header-hash-unregistered", 0, "unregistered_value", 4),
        ("header-long-form-first-item", 0, "not_deterministic", 1),
        ("header-profile-zero", 0, "unregistered_value", 7),
        ("header-outcome-unreadable-first", 0, "unregistered_value", 7),
        ("header-two-violations-byte-order", 11, "out_of_range", 13),
    ],
)
def test_malformed_header_reports_first_violation(
    name: str, item_offset: int, reason: str, byte_offset: int
) -> None:
    with pytest.raises(HeaderError) as error:
        decode_header((P1 / f"{name}.bin").read_bytes(), item_offset)
    assert error.value.reason == reason
    assert error.value.offset == byte_offset


def test_unknown_hash_in_newer_registry_preserves_header_fields() -> None:
    data = (P1 / "header-hash-newer.bin").read_bytes()
    decoded = decode_header(data)
    assert decoded.identifier is None
    assert decoded.header.registry_version == 2
    assert decoded.header.hash_algorithm == 9
    assert decoded.next_offset == 11


@pytest.mark.parametrize(
    "header",
    [
        Header(registry_version=2),
        Header(profile=7),
        Header(canonical=0),
        Header(lineage=Lineage(b"short", 1)),
        Header(lineage=Lineage(bytes(32), 1 << 64)),
    ],
)
def test_writer_refuses_unrepresentable_headers(header: Header) -> None:
    with pytest.raises(ValueError):
        encode_header(header)

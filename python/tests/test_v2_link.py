# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare P1 link decisions over consecutive items with the published corpus."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import FrameError, decode_frame, decode_header, encode_plain_frame
from gts.v2.framing import ItemFramingError
from gts.v2.link import check_frame_link, inspect_link_item

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"


@pytest.mark.parametrize(
    "name",
    [
        "link-ok",
        "link-healed",
        "link-content-damage",
        "link-two-damaged",
        "link-last-damaged",
        "link-healed-effective",
        "link-heal-opaque-between",
        "link-heal-closing-items",
        "link-transparent-damaged",
        "link-healed-before-transparent",
        "link-malformed-frame",
        "identifier-ill-typed-identity-well-typed",
        "identifier-ill-typed-identity-ill-typed",
        "pred-transparent-not-bool",
        "frame-flags-order",
        "value-zero-frame-type",
        "frame-unknown-content-map",
    ],
)
def test_link_states_match_the_corpus(name: str) -> None:
    entry = json.loads((P1 / f"{name}.json").read_text())
    for case in entry["cases"]:
        data = (P1 / case["input_file"]).read_bytes()
        read_result = cbor2.loads(bytes.fromhex(case["read_result"]))
        for segment in read_result[-1][1]:
            _, start, length, *_ = segment
            limit = start + length
            header = decode_header(data, start)
            predecessor = header.identifier
            assert predecessor is not None
            states = dict((record[1], record[4]) for record in segment[-2])
            frame, following = inspect_link_item(data, header.next_offset)
            observed = {}
            while following <= limit:
                witness, after_witness = (
                    inspect_link_item(data, following)
                    if following < limit
                    else (None, following)
                )
                if frame is not None:
                    link = check_frame_link(frame, predecessor, witness)
                    observed[frame.offset] = {"linked": 1, "healed": 2, "broken": 3}[
                        link.state
                    ]
                    if link.effective_identifier is not None:
                        predecessor = link.effective_identifier
                if following == limit:
                    break
                frame, following = witness, after_witness
            assert observed == states


def test_non_deterministic_identifier_still_compares_by_its_bytes() -> None:
    predecessor = bytes(32)
    encoded = encode_plain_frame(predecessor, 14, [0, bytes(32)], b"")
    identifier = decode_frame(encoded).identifier
    # Replace the deterministic byte string with two chunks containing the same digest.
    alternate = (
        encoded[:-35]
        + b"\x5f\x50"
        + identifier[:16]
        + b"\x50"
        + identifier[16:]
        + b"\xff\x80"
    )
    with pytest.raises(FrameError, match="not_deterministic"):
        decode_frame(alternate)
    fields, end = inspect_link_item(alternate)
    assert end == len(alternate)
    assert fields is not None
    link = check_frame_link(fields, predecessor)
    assert (link.state, link.effective_identifier) == ("linked", identifier)


def test_item_framing_failure_reports_the_envelope_start() -> None:
    data = b"\x00\x8b\x02\x0e\xf4\xf4"
    with pytest.raises(ItemFramingError) as error:
        inspect_link_item(data, 1)
    assert (error.value.reason, error.value.offset) == ("torn_tail", 1)


def test_a_frame_past_an_opaque_item_cannot_witness_healing() -> None:
    data = (P1 / "link-heal-opaque-between.bin").read_bytes()
    header = decode_header(data)
    terms, offset = inspect_link_item(data, header.next_offset)
    damaged, offset = inspect_link_item(data, offset)
    opaque, next_frame = inspect_link_item(data, offset)
    later, _ = inspect_link_item(data, next_frame)
    assert opaque is None
    assert terms is not None and terms.stored_identifier is not None
    assert damaged is not None and later is not None
    assert check_frame_link(damaged, terms.stored_identifier, later).state == "broken"

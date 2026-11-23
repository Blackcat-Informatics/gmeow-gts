# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare closing-item placement and claims against P1 and P7 corpus results."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import encode_plain_frame
from gts.v2.items import (
    check_closing_items,
    classify_item,
    encode_locator,
    encode_offsets,
    recognize_locator,
)

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"
# P2 reason codes for the selected corpus reports.
_REASONS = {
    3: "not_deterministic",
    8: "unordered_or_repeated",
    19: "index_offsets",
    23: "index_locator",
}


@pytest.mark.parametrize(
    "name",
    [
        "offsets-holds",
        "offsets-length-differs",
        "offsets-count-short",
        "offsets-count-long",
        "offsets-counts-opaque",
        "segment-offsets-then-frame",
        "segment-two-offsets",
        "segment-two-locators",
        "segment-locator-then-frame",
        "segment-locator-before-offsets",
        "segment-offsets-locator-header",
        "locator-recognised",
        "locator-short-form",
        "locator-zero-distance",
        "locator-last-index",
        "locator-index-type-long-form",
    ],
)
def test_closing_findings_match_corpus(name: str) -> None:
    entry = json.loads((P1 / f"{name}.json").read_text())
    for case in entry["cases"]:
        data = (P1 / case["input_file"]).read_bytes()
        result = cbor2.loads(bytes.fromhex(case["read_result"]))
        for segment in result[-1][1]:
            _, offset, length, _, _, registry, *_ = segment
            end = offset + length
            items = []
            while offset < end:
                item = classify_item(data, offset, registry_version=registry[1])
                items.append(item)
                offset = item.end
            assert offset == end

            expected = [
                (position[1], _REASONS[reason[1]])
                for _, reason, _, position, *_ in segment[-1]
                if position[0] == 2
                and reason[0] == 2
                and reason[1] in _REASONS
                and items[position[1]].kind in ("offsets", "locator")
            ]
            actual = [(f.position, f.reason) for f in check_closing_items(data, items)]
            assert actual == expected


@pytest.mark.parametrize(
    "name",
    [
        "locator-beyond-start",
        "locator-zero-distance",
        "locator-short-form",
        "locator-short-input",
        "locator-wrong-kind",
        "locator-not-index-item",
        "locator-torn-end",
        "locator-index-type-long-form",
    ],
)
def test_end_of_input_without_a_recognised_index_hint(name: str) -> None:
    assert recognize_locator((P1 / f"{name}.bin").read_bytes()) is None


def test_write_fixed_width_locator_and_recognize_end_of_input() -> None:
    data = (P1 / "locator-recognised.bin").read_bytes()
    assert recognize_locator(data) == 700
    assert encode_locator(289) == data[-11:]


def test_a_recognized_locator_inside_another_item_is_only_a_hint() -> None:
    data = (P1 / "locator-lookalike-torn.bin").read_bytes()
    assert recognize_locator(data) == 102


def test_recognition_refuses_an_index_that_extends_into_the_locator() -> None:
    index = encode_plain_frame(bytes(32), 15, [0, bytes(32)], b"")
    # The locator is a complete CBOR item but would occupy the index's last field.
    cut = index[:-1]
    assert recognize_locator(cut + encode_locator(len(cut))) is None


def test_write_offsets_from_observed_lengths_including_opaque_item() -> None:
    data = (P1 / "offsets-counts-opaque.bin").read_bytes()
    lengths = []
    offset = 0
    while offset < len(data):
        item = classify_item(data, offset)
        if item.kind == "offsets":
            assert encode_offsets(lengths) == data[offset : item.end]
            break
        lengths.append(item.end - item.offset)
        offset = item.end
    else:
        pytest.fail("corpus input lacks an offsets item")


def test_classification_preserves_an_opener_before_checking_its_form() -> None:
    data = bytes.fromhex("9f 01 ff")
    item = classify_item(data)
    assert item.kind == "header" and item.reason is None


def test_classification_checks_kind_encoding_before_frame_length() -> None:
    data = bytes.fromhex("82 18 02 00")
    item = classify_item(data)
    assert item.kind == "opaque" and item.reason == "not_deterministic"

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P1 item boundary and generation tests for the Python v2 wire layer."""

from pathlib import Path

import pytest
import zstandard

from gts.v2 import GenerationField, ItemFramingError, generation_field, item_end

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"


@pytest.mark.parametrize(
    "filename, expected",
    [
        ("gen-record-two.bin", GenerationField("record", 2)),
        ("gen-record-three.bin", GenerationField("record", 3)),
        ("gen-record-one.bin", GenerationField("record", 1)),
        ("header-long-form-first-item.bin", GenerationField("record", 2)),
        ("gen-first-generation.bin", GenerationField("first_generation", 1)),
        ("gen-first-generation-tagged.bin", GenerationField("first_generation", 1)),
        ("gen-first-generation-two.bin", GenerationField("first_generation", 2)),
        ("gen-first-generation-v-twice.bin", None),
        ("gen-record-tagged.first-item.bin", None),
    ],
)
def test_generation_uses_first_items_fields(
    filename: str, expected: GenerationField | None
) -> None:
    assert generation_field((P1 / filename).read_bytes()) == expected


def test_generation_can_read_chunked_version_key() -> None:
    # Indefinite map with a key written as text chunks "" and "v".
    assert generation_field(
        bytes.fromhex("bf 7f 60 61 76 ff 01 ff")
    ) == GenerationField("first_generation", 1)


def test_generation_reads_indefinite_resync_prefix_by_value() -> None:
    assert generation_field(bytes.fromhex("9f 05 18 02 ff")) == GenerationField(
        "record", 2
    )


@pytest.mark.parametrize("kind", ["array", "map", "tag"])
def test_framing_delimits_deep_items(kind: str) -> None:
    file = P1 / f"delimited-deep.{kind}.bin"
    if kind == "map":
        # The corpus distributes the map as a compressed input.
        data = zstandard.ZstdDecompressor().decompress(
            file.with_suffix(".bin.zst").read_bytes(), max_output_size=4_000_000
        )
    else:
        data = file.read_bytes()
    offset = item_end(data)
    # The corpus places a 114-byte commit after the deeply nested item.
    end = item_end(data, offset)
    assert end == len(data) - 114
    assert item_end(data, end) == len(data)


@pytest.mark.parametrize(
    "filename, offset, reason",
    [
        ("torn-deep.bin", 11, "torn_tail"),
        ("undelimitable-deep.bin", 11, "undelimitable_input"),
        ("undelimitable-chunk-wrong-major-type-cut.bin", 114, "undelimitable_input"),
        ("undelimitable-odd-map.bin", 11, "undelimitable_input"),
        ("undelimitable-simple.bin", 11, "undelimitable_input"),
    ],
)
def test_framing_distinguishes_torn_and_malformed_items(
    filename: str, offset: int, reason: str
) -> None:
    data = (P1 / filename).read_bytes()
    with pytest.raises(ItemFramingError) as error:
        item_end(data, offset)
    assert error.value.reason == reason
    assert error.value.offset == offset

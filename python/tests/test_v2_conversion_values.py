# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the P9 source value model against conversion vectors."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2._legacy_cbor import (
    InvalidText,
    MapValue,
    TaggedValue,
    decode_legacy_item,
    encode_legacy_value,
    merge_legacy_maps,
    source_id_matches,
)
from gts.v2.blobs import BlobSet
from gts.v2.conversion_graph import ConversionGraph, InvalidCarriedText
from gts.v2.dictionary import TermPool, TypedLiteral
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.lines import LineResult
from gts.v2.record_fold import RecordFold

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"
_NO_RECORDS = RecordFold(
    frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
)


@pytest.mark.parametrize(
    ("name", "holds"),
    [
        ("float-hash-finite", True),
        ("float-hash-shortest", True),
        ("float-hash-special", True),
        ("float-hash-mixed", False),
    ],
)
def test_source_identity_uses_one_float_policy_per_item(name: str, holds: bool) -> None:
    data = (_P9 / f"convert__{name}.bin").read_bytes()
    header = decode_legacy_item(data)
    assert isinstance(header.value, TaggedValue)
    assert isinstance(header.value.value, MapValue)
    assert source_id_matches(header.value.value, header=True)
    frame = decode_legacy_item(data, header.end)
    assert frame.end == len(data)
    assert isinstance(frame.value, MapValue)
    assert source_id_matches(frame.value) is holds


def test_shallow_merge_uses_cbor_identity_for_distinct_numeric_keys() -> None:
    left = decode_legacy_item(
        bytes.fromhex("bff56161016162f9fe016163f9fe026164ff")
    ).value
    right = decode_legacy_item(bytes.fromhex("a2016165fbfff80400000000006166")).value
    assert isinstance(left, MapValue) and isinstance(right, MapValue)
    merged = merge_legacy_maps((left, right))
    assert encode_legacy_value(merged).hex() == (
        "a4016165f56161fbfff80400000000006166fbfff80800000000006164"
    )


@pytest.mark.parametrize(
    ("encoded", "lexical"),
    [
        ("f98000", "8000000000000000"),
        ("f9fe01", "FFF8040000000000"),
        ("fa7f800001", "7FF0000020000000"),
    ],
)
def test_float_images_preserve_sign_and_nan_payload(encoded: str, lexical: str) -> None:
    graph = ConversionGraph(TermPool())
    value = decode_legacy_item(bytes.fromhex(encoded)).value
    term = graph.map_value(value)
    assert graph.pool.term(term) == TypedLiteral(
        lexical, "https://w3id.org/gts/conversion#Float64"
    )
    assert graph.values == {}


def test_invalid_source_text_retains_octets_for_validation() -> None:
    data = b"\x62\xc0\x80"
    value = decode_legacy_item(data).value
    assert value == InvalidText(b"\xc0\x80")
    assert encode_legacy_value(value) == data
    with pytest.raises(ValueError, match="invalid UTF-8"):
        ConversionGraph(TermPool()).map_value(value)


def test_metadata_merge_cannot_hide_an_invalid_source_key() -> None:
    bad = decode_legacy_item(bytes.fromhex("a17f61c361a9ff01")).value
    later = decode_legacy_item(bytes.fromhex("a162c3a902")).value
    assert isinstance(bad, MapValue) and isinstance(later, MapValue)
    assert merge_legacy_maps((bad, later)) == later
    with pytest.raises(InvalidCarriedText):
        ConversionGraph(TermPool()).add_segment(0, MapValue(()), (bad, later))


def test_deep_map_keys_are_read_encoded_and_mapped_iteratively() -> None:
    depth = 2048
    data = b"\xa1" + b"\x81" * depth + b"\xc2\x41\x02" + b"\xf7"
    value = decode_legacy_item(data).value
    assert encode_legacy_value(value) == data
    graph = ConversionGraph(TermPool())
    graph.map_value(value)
    assert len(graph.rows) == 1 + 4 + 2 * depth + 3


@pytest.mark.parametrize(
    "name", ["header-extension", "meta-value-types", "meta-replace-whole"]
)
def test_mapped_metadata_matches_published_conversion_digest(name: str) -> None:
    stem = f"convert__{name}"
    data = (_P9 / f"{stem}.bin").read_bytes()
    source = decode_legacy_item(data)
    assert isinstance(source.value, TaggedValue)
    assert isinstance(source.value.value, MapValue)
    metadata = []
    offset = source.end
    while offset < len(data):
        frame = decode_legacy_item(data, offset)
        assert isinstance(frame.value, MapValue) and source_id_matches(frame.value)
        frame_fields = dict(frame.value.entries)
        payload = frame_fields["d"]
        if name == "header-extension":
            assert frame_fields["t"] == "terms"
            assert payload == (MapValue((("k", 0), ("v", "http://e/a"))),)
        else:
            assert frame_fields["t"] == "meta" and isinstance(payload, MapValue)
            metadata.append(payload)
        offset = frame.end

    graph = ConversionGraph(TermPool())
    graph.add_segment(0, source.value.value, metadata)
    folded = FoldLabellingBudget(
        labelled_lines=1 << 26,
        labelled_bytes_per_read=1 << 37,
        labelling_units=(1 << 64) - 1,
    ).label_fold(
        graph.pool,
        LineResult(frozenset(graph.rows), False, False),
        _NO_RECORDS,
        BlobSet((), ()),
        graph.values.items(),
        folded_size=len(graph.rows) + len(graph.values),
    )
    case = json.loads((_P9 / f"{stem}.json").read_text())["cases"][0]
    proof = case["result"]["conversions"][0]["proof"]
    assert folded.content_digest == cbor2.loads(bytes.fromhex(proof))[1]

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise the combined P9 source read and fold."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.stream import DIGEST
from gts.v2._legacy_cbor import (
    InvalidText,
    MapValue,
    decode_legacy_item,
    encode_legacy_value,
)
from gts.v2.conversion_fold import ConversionStop, FoldedSource, fold_source
from gts.v2.conversion_source import SourceCapacity
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.wire import blake3_256, digest_str, header_id

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"


def _expected(name: str):
    return json.loads((_P9 / f"convert__{name}.json").read_bytes())["cases"][0][
        "result"
    ]["conversions"]


@pytest.mark.parametrize(
    "name",
    [
        "signatures",
        "snapshot",
        "suppress-reason-by",
        "typed-fields-values",
        "zstd-dictionary",
    ],
)
def test_fold_proof_inputs_match_published_conversion(name: str) -> None:
    expected = _expected(name)[0]
    data = bytes.fromhex(expected["source"]["hex"])
    fold = fold_source(lambda: data)
    assert isinstance(fold, FoldedSource)
    labelled = FoldLabellingBudget(
        labelled_lines=1 << 26,
        labelled_bytes_per_read=1 << 37,
        labelling_units=(1 << 64) - 1,
    ).label_fold(
        fold.terms,
        fold.lines,
        fold.records,
        fold.blobs,
        fold.values,
        folded_size=(
            len(fold.lines.lines)
            + len(fold.values)
            + len(fold.records.bindings)
            + len(fold.records.suppression)
            + len(fold.declarations)
        ),
    )
    proof = cbor2.loads(bytes.fromhex(expected["proof"]))
    assert [
        fold.digest,
        labelled.content_digest,
        fold.segments,
        fold.signatures,
    ] == proof


@pytest.mark.parametrize(
    "name",
    [
        "meta-key-not-text",
        "pub-blob-not-map",
        "quad-target-literal-subject",
        "stop-layout-header-offset",
        "stop-lowest-rule",
        "text-not-utf8-meta",
    ],
)
def test_combined_source_failure_matches_published_offset(name: str) -> None:
    for entry in _expected(name):
        data = bytes.fromhex(entry["source"]["hex"])
        assert fold_source(lambda data=data: data) == ConversionStop(**entry["stopped"])


def test_capacity_and_item_rules_are_ranked_across_the_source() -> None:
    capacity = SourceCapacity(decoded_bytes_per_frame=199)
    for entry in _expected("order-earlier-item-wins"):
        data = bytes.fromhex(entry["source"]["hex"])
        assert fold_source(lambda data=data: data, capacity) == ConversionStop(
            **entry["stopped"]
        )


def test_source_must_match_on_both_reads_before_judging_items() -> None:
    reads = iter([b"\xff", b""])
    assert fold_source(lambda: next(reads)) == ConversionStop(
        "source_not_deterministic", 0
    )
    assert list(reads) == []


def _encode_segment(parts, *, streamable=True, broken_at=None):
    header = {"gts": "GTS1", "v": 1, "cat": {1: {"name": "gzip", "cls": "compress"}}}
    if streamable:
        header["layout"] = "streamable"
    header["id"] = header_id(header)
    output = bytearray(cbor2.dumps(header))
    prev = header["id"]
    offsets = []
    identifiers = []
    for kind, body, *extras in parts:
        offsets.append(len(output))
        if kind == "index" and isinstance(body, int):
            body = {"count": body, "head": identifiers[body - 1] if body > 0 else prev}
        if not isinstance(body, MapValue):
            body = decode_legacy_item(cbor2.dumps(body)).value
        fields = (("t", kind), ("d", body), ("prev", prev))
        if extras:
            fields += (("pub", extras[0]),)
        if len(extras) > 1:
            fields += (("x", extras[1]),)
        prev = blake3_256(encode_legacy_value(MapValue(fields)))
        if len(offsets) - 1 == broken_at:
            prev = bytes(32)
        identifiers.append(prev)
        output.extend(encode_legacy_value(MapValue((*fields, ("id", prev)))))
    return bytes(output), offsets


@pytest.mark.parametrize("announce", [True, False])
def test_streamable_layout_checks_catalogue_before_covered_blob(announce: bool) -> None:
    body = b"announced blob"
    catalog = [
        (
            "terms",
            [
                {"k": 0, "v": "urn:manifest"},
                {"k": 0, "v": DIGEST},
                {"k": 1, "v": digest_str(body)},
            ],
        ),
        ("quads", [[0, 1, 2]]),
    ]
    inline = [("blob", body)]
    parts = catalog + inline if announce else inline + catalog
    data, offsets = _encode_segment(parts + [("index", 3), ("blob", b"legal tail")])
    result = fold_source(lambda: data)
    if announce:
        assert isinstance(result, FoldedSource)
        assert len(result.declarations) == 2
    else:
        assert result == ConversionStop("source_damaged", offsets[0])


def test_last_intact_index_controls_the_streamable_claim() -> None:
    data, _ = _encode_segment([("meta", {}), ("meta", {}), ("index", 1), ("index", 3)])
    assert isinstance(fold_source(lambda: data), FoldedSource)


def test_capacity_does_not_mask_a_covered_blob_layout_failure() -> None:
    data, offsets = _encode_segment(
        [("blob", b"unannounced"), ("index", 1), ("meta", {})]
    )
    assert fold_source(
        lambda: data, SourceCapacity(items_per_read=3)
    ) == ConversionStop("source_damaged", offsets[0])


def test_malformed_index_does_not_displace_an_intact_covering_index() -> None:
    data, offsets = _encode_segment(
        [("blob", b"unannounced"), ("index", 1), ("index", -1)]
    )
    assert fold_source(lambda: data) == ConversionStop("source_damaged", offsets[0])


@pytest.mark.parametrize("blob", [True, False])
def test_limited_read_does_not_judge_an_unfinished_metadata_merge(blob: bool) -> None:
    invalid = MapValue((("extra", InvalidText(b"\xff")),))
    valid = MapValue((("extra", "kept"),))
    parts = (
        [("blob", b"same", invalid), ("blob", b"same", valid)]
        if blob
        else [("meta", invalid), ("meta", valid)]
    )
    data, offsets = _encode_segment(parts, streamable=False)
    assert isinstance(fold_source(lambda: data), FoldedSource)
    assert fold_source(
        lambda: data, SourceCapacity(items_per_read=2)
    ) == ConversionStop("source_exceeds_capacity", offsets[1])


def test_blob_failure_at_an_earlier_item_precedes_the_capacity_stop() -> None:
    pub = MapValue((("digest", b"wrong"),))
    data, offsets = _encode_segment(
        [("blob", b"held", pub), ("meta", {})], streamable=False
    )
    assert fold_source(
        lambda: data, SourceCapacity(items_per_read=2)
    ) == ConversionStop("source_damaged", offsets[0])


def test_undecodable_blob_cannot_replace_earlier_public_metadata() -> None:
    body = b"inline"
    tagged = (("digest", blake3_256(body)),)
    invalid = MapValue((*tagged, ("extra", InvalidText(b"\xff"))))
    later = MapValue((*tagged, ("extra", "replacement")))
    data, offsets = _encode_segment(
        [("blob", body, invalid), ("blob", b"corrupt gzip", later, (1,))],
        streamable=False,
    )
    assert fold_source(lambda: data) == ConversionStop(
        "unrepresentable_term", offsets[0]
    )


@pytest.mark.parametrize(
    "catalog", [{"gzip": {"name": "gzip", "cls": "compress"}}, {1: {}}]
)
def test_catalogue_requires_numbered_codec_entries(catalog) -> None:
    header = {"gts": "GTS1", "v": 1, "cat": catalog}
    header["id"] = header_id(header)
    assert fold_source(lambda: cbor2.dumps(header)) == ConversionStop(
        "source_damaged", 0
    )


@pytest.mark.parametrize(
    ("dictionaries", "reference"),
    [(42, "name"), ({}, 42), ({}, "missing")],
)
def test_catalogue_requires_an_in_band_dictionary_for_each_reference(
    dictionaries, reference
) -> None:
    header = {
        "gts": "GTS1",
        "v": 1,
        "dct": dictionaries,
        "cat": {2: {"name": "zstd", "cls": "compress", "dct": reference}},
    }
    header["id"] = header_id(header)
    assert fold_source(lambda: cbor2.dumps(header)) == ConversionStop(
        "source_damaged", 0
    )


@pytest.mark.parametrize("blob", [True, False])
def test_bad_self_hash_excludes_a_frame_from_the_metadata_merge(blob: bool) -> None:
    invalid = MapValue((("extra", InvalidText(b"\xff")),))
    later = MapValue((("extra", "replacement"),))
    parts = (
        [("blob", b"same", invalid), ("blob", b"same", later)]
        if blob
        else [("meta", invalid), ("meta", later)]
    )
    data, offsets = _encode_segment(parts, streamable=False, broken_at=1)
    assert fold_source(lambda: data) == ConversionStop(
        "unrepresentable_term", offsets[0]
    )

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the P9 projection of held and external first-generation blobs."""

import json

import cbor2
import pytest

from gts.v2._legacy_cbor import InvalidText, MapValue
from gts.v2.blobs import BlobLocation, blob_set
from gts.v2.conversion_blobs import BlobSourceError, ConversionBlobs
from gts.v2.conversion_graph import ConversionGraph
from gts.v2.dictionary import Iri, TermPool, TypedLiteral
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.lines import LineResult
from gts.v2.rebuild import unit_digest
from gts.v2.record_fold import RecordFold
from gts.wire import blake3_256
from test_v2_conversion_rdf import _P9, _segments


def _project(name: str):
    graph = ConversionGraph(TermPool())
    legacy = ConversionBlobs()
    for ordinal, (header, frames) in enumerate(_segments(name)):
        segment = graph.add_segment(ordinal, header)
        for frame_ordinal, frame in enumerate(frames):
            fields = dict(frame.entries)
            assert fields["t"] == "blob"
            assert "x" not in fields
            legacy.hold_digest(fields["id"])
            data = fields.get("d")
            legacy.add_frame(frame, data, BlobLocation(ordinal, frame_ordinal))
            graph.add_frame(segment, frame_ordinal, "blob", frame, data)
    converted = legacy.project(graph)
    records = RecordFold(
        converted.bindings, frozenset(), (), (), frozenset(), frozenset(), frozenset()
    )
    return graph, records, converted.declarations


@pytest.mark.parametrize(
    "name", ["blob-external", "blob-inline-multi-unit", "blob-merged-pub"]
)
def test_blob_projection_matches_published_digest(name: str) -> None:
    graph, records, declarations = _project(name)
    labelled = FoldLabellingBudget(
        labelled_lines=1 << 26,
        labelled_bytes_per_read=1 << 37,
        labelling_units=(1 << 64) - 1,
    ).label_fold(
        graph.pool,
        LineResult(frozenset(graph.rows), (), False),
        records,
        blob_set(declarations),
        graph.values.items(),
        folded_size=(
            len(graph.rows)
            + len(graph.values)
            + len(records.bindings)
            + len(declarations)
        ),
    )
    manifest = json.loads((_P9 / f"convert__{name}.json").read_bytes())
    proof = manifest["cases"][0]["result"]["conversions"][0]["proof"]
    assert labelled.content_digest == cbor2.loads(bytes.fromhex(proof))[1]


@pytest.mark.parametrize(
    ("name", "rule"),
    [
        ("blob-digest-text-other", 6),
        ("blob-pub-digest-mismatch", 6),
        ("blob-external-no-digest", 14),
        ("pub-blob-not-map", 14),
    ],
)
def test_invalid_blob_fixtures_stop_at_their_item_rule(name: str, rule: int) -> None:
    with pytest.raises(BlobSourceError) as stop:
        _project(name)
    assert stop.value.rule == rule


def test_snapshot_registers_a_later_digest_for_an_earlier_external_frame() -> None:
    legacy = ConversionBlobs()
    graph = ConversionGraph(TermPool())
    data = b"held by the later snapshot"
    digest = blake3_256(data)
    text = "blake3:" + digest.hex()
    pub = MapValue((("digest", text), ("mt", ""), ("rep", "original")))
    legacy.add_frame(MapValue((("pub", pub),)), None, BlobLocation(0, 0))
    legacy.add_snapshot(
        MapValue((("blobs", MapValue(((text, data),))),)), BlobLocation(1, 0)
    )
    converted = legacy.project(graph)
    (blob,) = converted.declarations
    assert blob.bytes() == data
    assert blob.name == (1, unit_digest(data))
    assert blob.media_type == ""
    assert len(converted.bindings) == 1


def test_only_exact_generated_text_can_name_a_held_digest() -> None:
    legacy = ConversionBlobs()
    graph = ConversionGraph(TermPool())
    digest = bytes.fromhex("ab" * 32)
    expected = "blake3:" + digest.hex()
    other = "blake3:" + digest.hex().upper()
    for index, value in enumerate((expected, other)):
        pub = MapValue((("digest", value),))
        legacy.add_frame(MapValue((("pub", pub),)), None, BlobLocation(0, index))
    legacy.hold_digest(digest)
    converted = legacy.project(graph)
    assert not converted.declarations and not converted.bindings
    (typed_digest,) = graph.values.values()
    assert typed_digest.literal()[0] == digest.hex().upper()
    assert any(
        graph.pool.term(row.predicate)
        == Iri("https://w3id.org/gts/conversion#digestText")
        and isinstance(term := graph.pool.term(row.object), TypedLiteral)
        and term.lexical == other
        for row in graph.rows
    )


@pytest.mark.parametrize("wrong_digest", [True, False])
def test_snapshot_checks_all_digest_keys_before_bad_entry_type(
    wrong_digest: bool,
) -> None:
    legacy = ConversionBlobs()
    digest = b"wrong" if wrong_digest else blake3_256(b"inline")
    payload = MapValue(
        (("blobs", MapValue((("malformed", None), (digest, b"inline")))),)
    )
    legacy.add_snapshot(payload, BlobLocation(0, 0))
    with pytest.raises(BlobSourceError) as stop:
        legacy.project(ConversionGraph(TermPool()))
    assert stop.value.rule == (6 if wrong_digest else 14)


def test_invalid_carried_text_precedes_nontext_representation() -> None:
    legacy = ConversionBlobs()
    pub = MapValue((("rep", 41), ("extra", (InvalidText(b"\xff"),))))
    frame = MapValue((("d", b"inline"), ("pub", pub)))
    legacy.add_frame(frame, b"inline", BlobLocation(0, 0))
    with pytest.raises(BlobSourceError) as stop:
        legacy.project(ConversionGraph(TermPool()))
    assert stop.value.rule == 9


@pytest.mark.parametrize("replace", [True, False])
def test_utf8_check_uses_the_winning_pub_entry_and_its_source_item(
    replace: bool,
) -> None:
    legacy = ConversionBlobs()
    graph = ConversionGraph(TermPool())
    data = b"inline"
    digest = "blake3:" + blake3_256(data).hex()
    pub = MapValue((("extra", InvalidText(b"\xff")),))
    origin = BlobLocation(0, 1)
    legacy.add_frame(MapValue((("d", data), ("pub", pub))), data, origin)
    fields = (
        (("digest", digest), ("extra", "kept")) if replace else (("digest", digest),)
    )
    legacy.add_frame(MapValue((("pub", MapValue(fields)),)), None, BlobLocation(1, 0))
    if replace:
        assert len(legacy.project(graph).bindings) == 1
    else:
        with pytest.raises(BlobSourceError) as stop:
            legacy.project(graph)
        assert (stop.value.rule, stop.value.location) == (9, origin)

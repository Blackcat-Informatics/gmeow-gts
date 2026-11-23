# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Verify RDF projection and item-local validation against P9."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2._legacy_cbor import (
    InvalidText,
    MapValue,
    TaggedValue,
    decode_legacy_item,
    source_id_matches,
)
from gts.v2.blobs import BlobSet
from gts.v2.conversion_graph import ConversionGraph
from gts.v2.conversion_rdf import ConversionSegment, RdfSourceError
from gts.v2.dictionary import TermPool
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.lines import LineFold, LineResult
from gts.v2.record_fold import RecordFold

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"
_RDF = ("terms", "quads", "reifies", "annot", "snapshot")
_NO_RECORDS = RecordFold(
    frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
)


def _segments(name: str):
    return _parse_segments((_P9 / f"convert__{name}.bin").read_bytes())


def _parse_segments(data: bytes):
    segments = []
    offset = 0
    while offset < len(data):
        item = decode_legacy_item(data, offset)
        value = item.value
        if isinstance(value, TaggedValue):
            assert value.number == 55799
            value = value.value
        assert isinstance(value, MapValue)
        header = "gts" in dict(value.entries) and "t" not in dict(value.entries)
        assert source_id_matches(value, header=header)
        if header:
            segments.append((value, []))
        else:
            segments[-1][1].append(value)
        offset = item.end
    return segments


def _project(name: str) -> tuple[ConversionGraph, LineResult]:
    pool = TermPool()
    graph = ConversionGraph(pool)
    lines = LineFold()
    for ordinal, (header, frames) in enumerate(_segments(name)):
        metadata = []
        for frame in frames:
            fields = dict(frame.entries)
            if fields["t"] == "meta":
                metadata.append(fields["d"])
            elif fields["t"] == "snapshot" and isinstance(fields["d"], MapValue):
                snapshot = dict(fields["d"].entries)
                if "meta" in snapshot:
                    metadata.append(snapshot["meta"])
        assert all(isinstance(value, MapValue) for value in metadata)
        segment = graph.add_segment(ordinal, header, metadata)
        rdf = ConversionSegment(pool, lines, ordinal)
        for frame_ordinal, frame in enumerate(frames):
            fields = dict(frame.entries)
            kind, payload = fields["t"], fields.get("d")
            assert "x" not in fields
            if kind in _RDF:
                notes = rdf.add_frame(
                    kind, frame_ordinal, payload, envelope=frame
                ).notes
            else:
                assert kind in ("index", "meta")
                notes = ()
            graph.add_frame(segment, frame_ordinal, kind, frame, payload, notes)
    folded = lines.result()
    return graph, LineResult(
        folded.lines | graph.rows, folded.held_rows, folded.width_refused
    )


@pytest.mark.parametrize(
    "name",
    [
        "annotation-count",
        "anonymous-blank",
        "anonymous-blank-snapshots",
        "blank-label-not-utf8",
        "datatype-default",
        "language-case",
        "language-datatype-generated",
        "index-frame",
        "meta-order-snapshot",
        "minimal",
        "reifies-multi",
        "snapshot-ids-after",
        "snapshot-label-one-scope",
        "snapshot-triple-over-blank",
        "term-ignored-keys",
        "triple-term-with-rf",
        "two-segments-blank",
    ],
)
def test_rdf_projection_matches_published_digest(name: str) -> None:
    graph, lines = _project(name)
    labelled = FoldLabellingBudget(
        labelled_lines=1 << 26,
        labelled_bytes_per_read=1 << 37,
        labelling_units=(1 << 64) - 1,
    ).label_fold(
        graph.pool,
        lines,
        _NO_RECORDS,
        BlobSet((), ()),
        graph.values.items(),
        folded_size=len(lines.lines) + len(graph.values),
    )
    manifest = json.loads((_P9 / f"convert__{name}.json").read_text())
    proof = manifest["cases"][0]["result"]["conversions"][0]["proof"]
    assert labelled.content_digest == cbor2.loads(bytes.fromhex(proof))[1]


@pytest.mark.parametrize(
    ("name", "rule"),
    [
        ("direction-without-language", 9),
        ("forward-reference", 5),
        ("legacy-triple-term", 9),
        ("literal-subject", 10),
        ("stop-lowest-rule", 9),
        ("text-null-blank", 9),
        ("triple-subject", 10),
    ],
)
def test_invalid_rdf_fixture_stops_at_its_item_rule(name: str, rule: int) -> None:
    with pytest.raises(RdfSourceError) as stop:
        _project(name)
    assert stop.value.rule == rule


def test_snapshot_reference_rule_precedes_bad_term_shape() -> None:
    rdf = ConversionSegment(TermPool(), LineFold(), 0)
    snapshot = MapValue(
        (("terms", (MapValue((("k", 0), ("v", False))),)), ("quads", ((3, 0, 0),)))
    )
    with pytest.raises(RdfSourceError) as stop:
        rdf.add_frame("snapshot", 0, snapshot)
    assert stop.value.rule == 5


def test_row_position_rule_precedes_a_wrong_id_type() -> None:
    rdf = ConversionSegment(TermPool(), LineFold(), 0)
    rdf.add_frame(
        "terms",
        0,
        (MapValue((("k", 0), ("v", "http://e/p"))), MapValue((("k", 1), ("v", "x")))),
    )
    with pytest.raises(RdfSourceError) as stop:
        rdf.add_frame("quads", 1, ((1, 0, False),))
    assert stop.value.rule == 10


@pytest.mark.parametrize("in_term_note", [True, False])
def test_invalid_carried_text_precedes_the_same_items_row_failure(
    in_term_note: bool,
) -> None:
    invalid = (InvalidText(b"\xff"),)
    literal = (("k", 1), ("v", "lexical"))
    if in_term_note:
        literal += (("rf", invalid),)
    payload = MapValue(
        (
            ("terms", (MapValue(literal), MapValue((("k", 0), ("v", "http://e/p"))))),
            ("quads", ((0, 1, False),)),
        )
    )
    envelope = MapValue(()) if in_term_note else MapValue((("pub", invalid),))
    rdf = ConversionSegment(TermPool(), LineFold(), 0)
    with pytest.raises(RdfSourceError) as stop:
        rdf.add_frame("snapshot", 0, payload, envelope=envelope)
    assert stop.value.rule == 9

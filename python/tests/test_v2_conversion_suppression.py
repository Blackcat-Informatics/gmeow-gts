# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the legacy suppression entries and their effects in the P9 fold."""

import json

import cbor2
import pytest

from gts.v2._legacy_cbor import InvalidText, MapValue
from gts.v2.blobs import BlobLocation, blob_set
from gts.v2.conversion_blobs import ConversionBlobs
from gts.v2.conversion_graph import ConversionGraph
from gts.v2.conversion_rdf import ConversionSegment
from gts.v2.conversion_suppression import ConversionSuppression, SuppressionSourceError
from gts.v2.dictionary import TermPool
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.lines import LineFold, LineResult
from gts.v2.record_fold import RecordLocation
from gts.wire import blake3_256
from test_v2_conversion_rdf import _P9, _RDF, _parse_segments

_FIXTURES = [
    "blob-digest-forms",
    "blob-digest-text-external",
    "language-case-suppressed",
    "language-case-suppressed-two",
    "suppress-blob-external",
    "suppress-frame-earlier",
    "suppress-frame-later",
    "suppress-frame-twin-segments",
    "suppress-frame-unresolved",
    "suppress-reason-by",
    "suppress-term-of-segment-node",
    "suppress-term-quad-reifier",
]


def _project(data: bytes):
    pool = TermPool()
    graph = ConversionGraph(pool)
    lines = LineFold()
    blobs = ConversionBlobs()
    suppression = ConversionSuppression(blobs)
    for ordinal, (header, frames) in enumerate(_parse_segments(data)):
        segment = graph.add_segment(ordinal, header)
        rdf = ConversionSegment(pool, lines, ordinal)
        for index, frame in enumerate(frames):
            fields = dict(frame.entries)
            kind, payload = fields["t"], fields.get("d")
            assert "x" not in fields and isinstance(fields["id"], bytes)
            location = BlobLocation(ordinal, index)
            notes = rows = ()
            if kind in _RDF:
                mapped = rdf.add_frame(kind, index, payload, envelope=frame)
                notes, rows = mapped.notes, mapped.rows
                if kind == "snapshot":
                    blobs.add_snapshot(payload, location)
            elif kind == "suppress":
                suppression.add_directive(segment, index, rdf, payload, envelope=frame)
            else:
                assert kind == "blob"
                blobs.add_frame(frame, payload, location)
            suppression.add_frame(fields["id"], RecordLocation(ordinal, index), rows)
            graph.add_frame(segment, index, kind, frame, payload, notes)
    source = lines.result()
    binary = blobs.project(graph)
    records = suppression.project(graph, source.lines, binary)
    return graph, source, records, binary.declarations


@pytest.mark.parametrize("name", _FIXTURES)
def test_suppression_matches_published_digest(name: str) -> None:
    base = _P9 / f"convert__{name}"
    graph, source, records, declarations = _project(
        base.with_suffix(".bin").read_bytes()
    )
    lines = LineResult(
        source.lines | graph.rows, source.held_rows, source.width_refused
    )
    labelled = FoldLabellingBudget(
        labelled_lines=1 << 26,
        labelled_bytes_per_read=1 << 37,
        labelling_units=(1 << 64) - 1,
    ).label_fold(
        graph.pool,
        lines,
        records,
        blob_set(declarations),
        graph.values.items(),
        folded_size=(
            len(lines.lines)
            + len(records.bindings)
            + len(records.suppression)
            + len(graph.values)
            + len(declarations)
        ),
    )
    manifest = json.loads(base.with_suffix(".json").read_bytes())
    proof = manifest["cases"][0]["result"]["conversions"][0]["proof"]
    assert labelled.content_digest == cbor2.loads(bytes.fromhex(proof))[1]


@pytest.mark.parametrize(
    ("name", "hidden"),
    [
        ("suppress-frame-twin-segments", 1),
        ("suppress-frame-unresolved", 0),
        ("suppress-term-of-segment-node", 1),
        ("language-case-suppressed", 1),
        ("language-case-suppressed-two", 2),
    ],
)
def test_published_source_lines_hidden_by_suppression(name: str, hidden: int) -> None:
    data = (_P9 / f"convert__{name}.bin").read_bytes()
    _, source, records, _ = _project(data)
    assert len(source.lines & records.hidden_lines) == hidden


@pytest.mark.parametrize(
    "name", ["suppress-unknown-kind", "suppress-target-member-types"]
)
def test_invalid_suppression_targets_fail_at_rule_fourteen(name: str) -> None:
    base = _P9 / f"convert__{name}"
    expected = json.loads(base.with_suffix(".json").read_bytes())
    for case in expected["cases"][0]["result"]["conversions"]:
        with pytest.raises(SuppressionSourceError) as stop:
            _project(bytes.fromhex(case["source"]["hex"]))
        assert stop.value.rule == 14


def test_undefined_target_precedes_unrepresentable_directive_text() -> None:
    rdf = ConversionSegment(TermPool(), LineFold(), 0)
    targets = (MapValue((("kind", "term"), ("id", 4))),)
    payload = MapValue((("reason", InvalidText(b"\xff")), ("targets", targets)))
    with pytest.raises(SuppressionSourceError) as stop:
        ConversionSuppression(ConversionBlobs()).add_directive(0, 0, rdf, payload)
    assert stop.value.rule == 5


@pytest.mark.parametrize("by", [True, False])
def test_target_digest_text_is_carried_when_a_metadata_field_is_invalid(
    by: bool,
) -> None:
    target = MapValue((("kind", "frame"), ("id", InvalidText(b"\xff"))))
    if by:
        payload = MapValue((("by", 999), ("targets", (target,))))
    else:
        malformed = MapValue((("kind", "term"), ("id", ""), ("extra", True)))
        payload = MapValue((("targets", (target, malformed)),))
    rdf = ConversionSegment(TermPool(), LineFold(), 0)
    with pytest.raises(SuppressionSourceError) as stop:
        ConversionSuppression(ConversionBlobs()).add_directive(0, 0, rdf, payload)
    assert stop.value.rule == 9


def test_same_frame_target_can_hide_rows_from_two_blank_scopes() -> None:
    pool = TermPool()
    lines = LineFold()
    graph = ConversionGraph(pool)
    blobs = ConversionBlobs()
    suppression = ConversionSuppression(blobs)
    identifier = b"f" * 32
    terms = (
        MapValue((("k", 2), ("v", "b"))),
        MapValue((("k", 0), ("v", "urn:predicate"))),
    )
    for ordinal in range(2):
        rdf = ConversionSegment(pool, lines, ordinal)
        rdf.add_frame("terms", 0, terms)
        mapped = rdf.add_frame("quads", 1, ((0, 1, 0),))
        suppression.add_frame(identifier, RecordLocation(ordinal, 1), mapped.rows)
    payload = MapValue(
        (("targets", (MapValue((("kind", "frame"), ("id", identifier))),)),)
    )
    suppression.add_directive(0, 2, rdf, payload)
    records = suppression.project(graph, lines.result().lines, blobs.project(graph))
    assert len(records.suppression) == len(records.hidden_lines) == 2


def test_target_on_external_declaration_does_not_hide_inline_bytes() -> None:
    pool = TermPool()
    graph = ConversionGraph(pool)
    blobs = ConversionBlobs()
    suppression = ConversionSuppression(blobs)
    data = b"present elsewhere in the source"
    digest = blake3_256(data)
    public = MapValue((("digest", digest), ("rep", "declared")))
    inline = MapValue((("d", data), ("pub", public)))
    external = MapValue((("pub", public),))
    blobs.add_frame(inline, data, BlobLocation(0, 0))
    blobs.add_frame(external, None, BlobLocation(0, 1))
    identifier = b"e" * 32
    suppression.add_frame(identifier, RecordLocation(0, 1))
    payload = MapValue(
        (("targets", (MapValue((("kind", "frame"), ("id", identifier))),)),)
    )
    suppression.add_directive(0, 2, ConversionSegment(pool, LineFold(), 0), payload)
    records = suppression.project(graph, (), blobs.project(graph))
    assert len(records.hidden_bindings) == 1
    assert not records.hidden_blobs

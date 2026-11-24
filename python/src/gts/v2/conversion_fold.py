# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Read and project a first-generation file as the typed source fold of P9."""

from collections.abc import Callable
from dataclasses import dataclass, field
from itertools import zip_longest
from typing import Literal, cast

from blake3 import blake3

from gts.stream import DIGEST as STREAM_DIGEST
from gts.wire import digest_str

from ._conversion_fields import contains_invalid_text, extensions
from ._legacy_cbor import InvalidText, MapValue, Value
from .blobs import BlobDeclaration, BlobLocation, BlobSet, blob_set
from .conversion_blobs import BlobSourceError, ConversionBlobs
from .conversion_graph import (
    _HEADER_FIELDS,
    ConversionGraph,
    InvalidCarriedText,
    _catalog_extensions,
)
from .conversion_rdf import ConversionSegment, RdfFrameType, RdfSourceError, TermNote
from .conversion_source import (
    _DEFAULT_SOURCE_CAPACITY,
    SourceCapacity,
    SourceItem,
    _fields,
    read_source_items,
    source_boundaries,
)
from .conversion_suppression import ConversionSuppression, SuppressionSourceError
from .dictionary import Iri, LanguageLiteral, TermPool, TypedLiteral
from .lines import LineFold, LineResult
from .record_fold import RecordFold, RecordLocation
from .rows import QuadRow, Row
from .terms import TypedValue

_RDF = {"terms", "quads", "reifies", "annot", "snapshot"}
_SOURCE_CONTEXT = "gts2 source bytes"
type ConversionReason = Literal[
    "source_damaged",
    "source_not_decodable",
    "unrepresentable_term",
    "unrepresentable_frame",
    "source_not_deterministic",
    "source_exceeds_capacity",
    "content_mismatch",
]


@dataclass(frozen=True, slots=True)
class ConversionStop:
    reason: ConversionReason
    offset: int


@dataclass(frozen=True, slots=True)
class FoldedSource:
    digest: bytes
    segments: int
    signatures: int
    terms: TermPool
    lines: LineResult
    records: RecordFold
    blobs: BlobSet
    declarations: tuple[BlobDeclaration, ...]
    values: frozenset[tuple[int, TypedValue]]
    conversion_graph: int


def _stop_reason(rule: int) -> ConversionReason:
    if rule == 0:
        return "source_exceeds_capacity"
    if rule <= 6:
        return "source_damaged"
    if rule <= 8:
        return "source_not_decodable"
    if rule <= 11:
        return "unrepresentable_term"
    return "unrepresentable_frame"


@dataclass(slots=True)
class _Layout:
    """Track the last observed footer and the first unannounced inline blob."""

    header_offset: int
    previous: Value
    announced: set[str] = field(default_factory=set)
    first_unannounced: tuple[int, int] | None = None
    footer: tuple[int, int, bool] | None = None
    covered_unannounced: int | None = None

    def observe(
        self,
        item: SourceItem,
        ordinal: int,
        identifier: Value,
        rows: tuple[Row, ...],
        terms: TermPool,
    ) -> None:
        previous = self.previous
        self.previous = identifier
        if not item.id_holds:
            return
        offset = item.boundary.offset
        for row in rows:
            if isinstance(row, QuadRow) and terms.term(row.predicate) == Iri(
                STREAM_DIGEST
            ):
                obj = terms.term(row.object)
                if isinstance(obj, Iri):
                    self.announced.add(obj.text)
                elif isinstance(obj, TypedLiteral | LanguageLiteral):
                    self.announced.add(obj.lexical)
        if (
            item.kind == "blob"
            and isinstance(item.payload, bytes)
            and self.first_unannounced is None
            and digest_str(item.payload) not in self.announced
        ):
            self.first_unannounced = ordinal + 1, offset
        if item.kind == "index" and item.rule in (None, 3):
            fields = _fields(item.payload)
            count, head = fields.get("count"), fields.get("head")
            if (
                type(count) is int
                and 0 <= count < 1 << 64
                and isinstance(head, bytes)
                and len(head) == 32
            ):
                valid = count > 0 and count == ordinal and head == previous
                self.footer = count, offset, valid
                unseen = self.first_unannounced
                if valid and unseen is not None and unseen[0] <= count:
                    self.covered_unannounced = unseen[1]

    def failure(self, *, complete: bool) -> int | None:
        if not complete:
            return self.covered_unannounced
        if self.footer is None:
            return self.header_offset
        count, offset, valid = self.footer
        unseen = self.first_unannounced
        if unseen is not None and unseen[0] <= count:
            return unseen[1]
        return None if valid else offset


@dataclass(slots=True)
class _Segment:
    ordinal: int
    node: int
    rdf: ConversionSegment
    layout: _Layout | None
    metadata: dict[str, tuple[Value, int]] = field(default_factory=dict)
    frame_count: int = 0


def _header_rule(header: MapValue) -> Literal[9, 14] | None:
    fields = _fields(header)
    catalog = fields.get("cat")
    assert isinstance(catalog, MapValue)
    text = (fields.get("prof", "generic"), fields.get("layout", ""))
    carried = (
        fields.get("meta"),
        extensions(header, _HEADER_FIELDS),
        _catalog_extensions(catalog),
    )
    if any(isinstance(value, InvalidText) for value in text) or any(
        contains_invalid_text(value) for value in carried
    ):
        return 9
    if any(not isinstance(value, str) for value in text):
        return 14
    return None


class _SourceFold:
    def __init__(self) -> None:
        self.terms = TermPool()
        self.lines = LineFold()
        self.graph = ConversionGraph(self.terms)
        self.blobs = ConversionBlobs()
        self.suppression = ConversionSuppression(self.blobs)
        self.segment: _Segment | None = None
        self.segments = 0
        self.signatures = 0
        self.stop: tuple[int, int] | None = None
        self.blob_offsets: dict[RecordLocation, int] = {}

    def note(self, offset: int, rule: int) -> None:
        candidate = offset, rule
        if self.stop is None or candidate < self.stop:
            self.stop = candidate

    def finish_segment(self, *, complete: bool) -> None:
        segment = self.segment
        if segment is None:
            return
        if segment.layout is not None:
            failed = segment.layout.failure(complete=complete)
            if failed is not None:
                self.note(failed, 3)
        if not complete:
            return
        for value, offset in segment.metadata.values():
            if contains_invalid_text(value):
                self.note(offset, 9)
        if self.stop is None:
            merged = MapValue(
                tuple((key, value) for key, (value, _) in segment.metadata.items())
            )
            self.graph.add_segment_metadata(segment.node, merged)

    def add_header(self, item: SourceItem) -> None:
        self.finish_segment(complete=True)
        envelope = item.envelope
        fields = _fields(envelope)
        offset = item.boundary.offset
        rule = 1 if item.rule == 1 else _header_rule(envelope)
        if rule is not None:
            self.note(offset, rule)
        layout = (
            _Layout(offset, fields.get("id"))
            if fields.get("layout") == "streamable" and item.rule != 1
            else None
        )
        self.segment = _Segment(
            self.segments,
            self.graph.add_segment(self.segments, envelope) if rule is None else 0,
            ConversionSegment(self.terms, self.lines, self.segments),
            layout,
        )
        self.segments += 1

    def _metadata(self, segment: _Segment, value: Value, offset: int) -> None:
        if not isinstance(value, MapValue):
            self.note(offset, 14)
            return
        for key, child in value.entries:
            if isinstance(key, str):
                segment.metadata[key] = child, offset
            else:
                self.note(offset, 9 if isinstance(key, InvalidText) else 14)

    def _hold_only_digests(self, item: SourceItem, kind: str) -> None:
        if kind == "blob":
            self.blobs.hold_digest(
                _fields(_fields(item.envelope).get("pub")).get("digest")
            )
        elif kind == "snapshot":
            entries = _fields(item.payload).get("blobs")
            if isinstance(entries, MapValue):
                for key, _ in entries.entries:
                    self.blobs.hold_digest(key)
        elif kind == "suppress":
            self.suppression.hold_digests(item.payload)

    def add_frame(self, item: SourceItem, segment: _Segment, kind: str) -> None:
        fields = _fields(item.envelope)
        self.signatures += "sig" in fields
        identifier = fields.get("id")
        ordinal = segment.frame_count
        segment.frame_count += 1
        offset = item.boundary.offset
        location = RecordLocation(segment.ordinal, ordinal)
        rows: tuple[Row, ...] = ()
        notes: tuple[TermNote, ...] = ()
        if not item.id_holds or item.rule in (7, 12):
            self._hold_only_digests(item, kind)
        else:
            if kind in ("blob", "snapshot"):
                here = BlobLocation(segment.ordinal, ordinal)
                self.blob_offsets[location] = offset
                if kind == "blob":
                    self.blobs.add_frame(item.envelope, item.payload, here)
                else:
                    self.blobs.add_snapshot(item.payload, here)
            if kind == "meta":
                self._metadata(segment, item.payload, offset)
            elif kind == "snapshot":
                snapshot = _fields(item.payload)
                if "meta" in snapshot:
                    self._metadata(segment, snapshot["meta"], offset)
            try:
                if kind in _RDF:
                    contribution = segment.rdf.add_frame(
                        cast(RdfFrameType, kind),
                        ordinal,
                        item.payload,
                        envelope=item.envelope,
                    )
                    rows, notes = contribution.rows, contribution.notes
                elif kind == "suppress":
                    self.suppression.add_directive(
                        segment.node,
                        ordinal,
                        segment.rdf,
                        item.payload,
                        envelope=item.envelope,
                    )
            except (RdfSourceError, SuppressionSourceError) as error:
                self.note(offset, error.rule)
            try:
                self.graph.add_frame(
                    segment.node, ordinal, kind, item.envelope, item.payload, notes
                )
            except InvalidCarriedText:
                self.note(offset, 9)
        if isinstance(identifier, bytes):
            self.suppression.add_frame(identifier, location, rows)
        if segment.layout is not None:
            segment.layout.observe(item, ordinal, identifier, rows, self.terms)

    def _blob_failure(self, error: BlobSourceError) -> None:
        where = RecordLocation(error.location.segment, error.location.position)
        self.note(self.blob_offsets[where], error.rule)

    def _stopped(self) -> ConversionStop:
        assert self.stop is not None
        offset, rule = self.stop
        return ConversionStop(_stop_reason(rule), offset)

    def result(self, digest: bytes, *, complete: bool) -> FoldedSource | ConversionStop:
        if not complete:
            if (error := self.blobs.item_failure) is not None:
                self._blob_failure(error)
            return self._stopped()
        try:
            binary = self.blobs.project(self.graph)
        except BlobSourceError as error:
            self._blob_failure(error)
            return self._stopped()
        if self.stop is not None:
            return self._stopped()
        raw_lines = self.lines.result()
        records = self.suppression.project(self.graph, raw_lines.lines, binary)
        lines = LineResult(
            raw_lines.lines | self.graph.rows,
            raw_lines.held_rows,
            raw_lines.width_refused,
        )
        return FoldedSource(
            digest,
            self.segments,
            self.signatures,
            self.terms,
            lines,
            records,
            blob_set(binary.declarations),
            binary.declarations,
            frozenset(self.graph.values.items()),
            self.graph.dataset,
        )


def fold_source(
    read: Callable[[], bytes], capacity: SourceCapacity = _DEFAULT_SOURCE_CAPACITY
) -> FoldedSource | ConversionStop:
    """Compare two source reads before judging their ordered item observations."""
    first = read()
    second = read()
    digest = blake3(first, derive_key_context=_SOURCE_CONTEXT).digest()
    if digest != blake3(second, derive_key_context=_SOURCE_CONTEXT).digest():
        return ConversionStop("source_not_deterministic", 0)
    if any(
        a is None or b is None or (a.offset, a.end) != (b.offset, b.end)
        for a, b in zip_longest(source_boundaries(first), source_boundaries(second))
    ):
        return ConversionStop("source_not_deterministic", 0)
    del first
    fold = _SourceFold()
    complete = True
    for item in read_source_items(second, capacity):
        offset = item.boundary.offset
        if item.exceeds_capacity:
            fold.note(offset, 0)
            complete = False
            break
        if item.rule is not None:
            fold.note(offset, item.rule)
        if item.header:
            fold.add_header(item)
        elif item.kind is not None and fold.segment is not None:
            fold.add_frame(item, fold.segment, item.kind)
    fold.finish_segment(complete=complete)
    return fold.result(digest, complete=complete)

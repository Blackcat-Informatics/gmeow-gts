# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Project legacy suppression directives into the P9 records and vocabulary."""

from collections.abc import Iterable, Iterator
from dataclasses import dataclass
from itertools import chain
from typing import Literal

from ._conversion_fields import contains_invalid_text, extensions, frame_values
from ._legacy_cbor import InvalidText, MapValue, Value
from .conversion_blobs import BlobImage, ConversionBlobs, ConvertedBlobs
from .conversion_graph import _NS, _RDF, ConversionGraph
from .conversion_rdf import ConversionSegment, RdfSourceError
from .dictionary import Triple
from .record_fold import (
    RecordFold,
    RecordLocation,
    ResolvedSuppression,
    _hidden,
    exact_line_target,
)
from .records import BlobReference, SuppressionKind
from .rows import QuadRow, Row
from .terms import TypedValue

_EMPTY_MAP = MapValue(())
_DIRECTIVE_FIELDS = {"reason", "by", "targets"}
_TARGET_FIELDS = {
    "term": "id",
    "reifier": "id",
    "quad": "q",
    "blob": "digest",
    "frame": "id",
}
type _Rule = Literal[5, 9, 10, 14]


class SuppressionSourceError(ValueError):
    """A P9 item rule triggered by a suppression directive."""

    def __init__(self, rule: _Rule) -> None:
        self.rule = rule
        super().__init__(f"source suppression directive failed P9 item rule {rule}")


@dataclass(frozen=True, slots=True)
class _Target:
    kind: str
    value: int | QuadRow | bytes | str | InvalidText
    extra: MapValue


@dataclass(frozen=True, slots=True)
class _Directive:
    segment: int
    ordinal: int
    reason: str | None
    author: int | None
    extra: MapValue
    targets: tuple[_Target, ...]

    def has_metadata(self) -> bool:
        return (
            self.reason is not None
            or self.author is not None
            or bool(self.extra.entries)
            or any(target.extra.entries for target in self.targets)
        )


def _fields(value: MapValue) -> dict[str, Value]:
    return {key: field for key, field in value.entries if isinstance(key, str)}


def _resolve_term(
    rdf: ConversionSegment, raw: Value, errors: set[_Rule], *, missing: _Rule
) -> int | None:
    if type(raw) is int:
        try:
            return rdf.term(raw)
        except RdfSourceError as error:
            assert error.rule == 5
            errors.add(missing)
    else:
        errors.add(14)
    return None


def _target(
    raw: Value,
    rdf: ConversionSegment,
    errors: set[_Rule],
) -> tuple[_Target | None, bool]:
    if not isinstance(raw, MapValue):
        errors.add(14)
        return None, False
    fields = _fields(raw)
    kind = fields.get("kind")
    if not isinstance(kind, str) or kind not in _TARGET_FIELDS:
        errors.add(14)
        return None, False
    member = _TARGET_FIELDS[kind]
    extra = extensions(raw, {"kind", member})
    if contains_invalid_text(extra):
        errors.add(9)
    value = fields.get(member)
    return _target_value(kind, value, extra, rdf, errors), bool(extra.entries)


def _target_value(
    kind: str, value: Value, extra: MapValue, rdf: ConversionSegment, errors: set[_Rule]
) -> _Target | None:
    if kind in ("term", "reifier"):
        ref = _resolve_term(rdf, value, errors, missing=5)
        return None if ref is None else _Target(kind, ref, extra)
    if kind == "quad":
        if not isinstance(value, tuple):
            errors.add(14)
            return None
        resolved = tuple(
            _resolve_term(rdf, item, errors, missing=5) for item in value[:4]
        )
        try:
            rdf._row_positions("quads", resolved)
        except RdfSourceError as error:
            assert error.rule == 10
            errors.add(10)
        if len(value) not in (3, 4):
            errors.add(14)
        terms = tuple(ref for ref in resolved if ref is not None)
        if len(terms) == len(value) and len(terms) in (3, 4):
            return _Target(kind, QuadRow(*terms), extra)
    elif (
        isinstance(value, str | InvalidText)
        or isinstance(value, bytes)
        and len(value) == 32
    ):
        return _Target(kind, value, extra)
    else:
        errors.add(14)
    return None


def _blob_targets(image: BlobImage) -> Iterator[ResolvedSuppression]:
    if image.node is not None:
        yield ResolvedSuppression(SuppressionKind.TERM, (image.node,))
    if image.reference is not None:
        yield ResolvedSuppression(SuppressionKind.BLOB, referent=image.reference)


class ConversionSuppression:
    """Collect targets while reading and resolve digest targets over the full source."""

    def __init__(self, blobs: ConversionBlobs) -> None:
        self._blobs = blobs
        self._directives: list[_Directive] = []
        self._frames: dict[bytes, list[tuple[RecordLocation, tuple[Row, ...]]]] = {}

    def add_frame(
        self,
        identifier: bytes,
        location: RecordLocation,
        rows: tuple[Row, ...] = (),
    ) -> None:
        """Remember the typed contribution of a source frame with a verified ID."""
        self._blobs.hold_digest(identifier)
        self._frames.setdefault(identifier, []).append((location, rows))

    def hold_digests(self, payload: Value) -> None:
        """Register digest positions in a decoded directive."""
        if not isinstance(payload, MapValue):
            return
        targets = _fields(payload).get("targets")
        if not isinstance(targets, tuple):
            return
        for target in targets:
            if isinstance(target, MapValue):
                fields = _fields(target)
                kind = fields.get("kind")
                if kind == "frame":
                    self._blobs.hold_digest(fields.get("id"))
                elif kind == "blob":
                    self._blobs.hold_digest(fields.get("digest"))

    def add_directive(
        self,
        segment: int,
        ordinal: int,
        rdf: ConversionSegment,
        payload: Value,
        *,
        envelope: MapValue = _EMPTY_MAP,
    ) -> None:
        """Judge every member of the item and resolve its local term IDs."""
        self.hold_digests(payload)
        errors: set[_Rule] = set()
        if any(
            contains_invalid_text(value)
            for _, value in frame_values("suppress", envelope, payload)
        ):
            errors.add(9)
        if not isinstance(payload, MapValue):
            raise SuppressionSourceError(9 if errors else 14)
        fields = _fields(payload)
        extra = extensions(payload, _DIRECTIVE_FIELDS)
        reason = fields.get("reason")
        if contains_invalid_text(extra) or isinstance(reason, InvalidText):
            errors.add(9)
        if "reason" in fields and not isinstance(reason, str | InvalidText):
            errors.add(14)
        author = (
            _resolve_term(rdf, fields["by"], errors, missing=14)
            if "by" in fields
            else None
        )
        raw_targets = fields.get("targets")
        targets = []
        has_metadata = "by" in fields or "reason" in fields or bool(extra.entries)
        if isinstance(raw_targets, tuple):
            for raw in raw_targets:
                resolved, has_extension = _target(raw, rdf, errors)
                has_metadata |= has_extension
                if resolved is not None:
                    targets.append(resolved)
        else:
            errors.add(14)
        directive = _Directive(
            segment,
            ordinal,
            reason if isinstance(reason, str) else None,
            author,
            extra,
            tuple(targets),
        )
        if has_metadata and any(
            isinstance(target.value, InvalidText) for target in targets
        ):
            errors.add(9)
        if errors:
            raise SuppressionSourceError(min(errors))
        self._directives.append(directive)

    def _entries(
        self, target: _Target, blobs: ConvertedBlobs
    ) -> Iterator[ResolvedSuppression]:
        value = target.value
        if isinstance(value, int):
            kind = (
                SuppressionKind.TERM
                if target.kind == "term"
                else SuppressionKind.REIFIER
            )
            yield ResolvedSuppression(kind, (value,))
        elif isinstance(value, QuadRow):
            entry = exact_line_target(value)
            assert entry is not None
            yield entry
        elif isinstance(value, bytes | str):
            digest = self._blobs.resolve_digest(value)
            if target.kind == "blob":
                image = blobs.images.get(digest)
                if image is not None:
                    yield from _blob_targets(image)
            elif isinstance(digest, bytes):
                for location, rows in self._frames.get(digest, ()):
                    for row in rows:
                        entry = exact_line_target(row)
                        if entry is not None:
                            yield entry
                    for image in blobs.frames.get(location, ()):
                        yield from _blob_targets(image)

    def _describe(self, directive: _Directive, graph: ConversionGraph) -> None:
        node = graph._blank()
        quad, iri = graph._quad, graph._iri
        quad(node, _RDF + "type", iri(_NS + "SuppressDirective"))
        quad(node, _NS + "inSegment", directive.segment)
        quad(node, _NS + "frameOrdinal", graph._typed(TypedValue(3, directive.ordinal)))
        if directive.reason is not None:
            quad(node, _NS + "reason", graph._literal(directive.reason))
        if directive.author is not None:
            quad(node, _NS + "by", directive.author)
        if directive.extra.entries:
            quad(node, _NS + "extension", graph.map_value(directive.extra))
        for ordinal, target in enumerate(directive.targets):
            other = graph._blank()
            quad(node, _NS + "target", other)
            quad(other, _RDF + "type", iri(_NS + "SuppressTarget"))
            quad(other, _NS + "targetOrdinal", graph._typed(TypedValue(3, ordinal)))
            quad(other, _NS + "targetKind", graph._literal(target.kind))
            self._describe_target(other, target, graph)

    def _describe_target(
        self, node: int, target: _Target, graph: ConversionGraph
    ) -> None:
        value = target.value
        if isinstance(value, int):
            graph._quad(node, _NS + "targetTerm", value)
        elif isinstance(value, QuadRow):
            triple = graph.pool._intern(
                Triple(value.subject, value.predicate, value.object)
            )
            graph._quad(node, _NS + "targetTerm", triple)
            if value.graph is not None:
                graph._quad(node, _NS + "targetGraph", value.graph)
        else:
            assert isinstance(value, bytes | str)
            digest = self._blobs.resolve_digest(value)
            if isinstance(digest, bytes):
                graph._quad(
                    node, _NS + "targetDigest", graph._typed(TypedValue(5, digest))
                )
            else:
                graph._quad(node, _NS + "digestText", graph._literal(digest))
        if target.extra.entries:
            graph._quad(node, _NS + "extension", graph.map_value(target.extra))

    def project(
        self, graph: ConversionGraph, lines: Iterable[Row], blobs: ConvertedBlobs
    ) -> RecordFold:
        """State the directives and apply the shared v2 hiding rule."""
        entries: set[ResolvedSuppression] = set()
        for directive in self._directives:
            if directive.has_metadata():
                self._describe(directive, graph)
            for target in directive.targets:
                entries.update(self._entries(target, blobs))
        present = {BlobReference(*item.name) for item in blobs.declarations}
        hidden_lines, hidden_bindings, hidden_blobs, _ = _hidden(
            entries,
            chain(lines, graph.rows),
            set(blobs.bindings),
            present,
            graph.pool,
            (),
        )
        return RecordFold(
            blobs.bindings,
            frozenset(entries),
            (),
            (),
            frozenset(hidden_lines),
            frozenset(hidden_bindings),
            frozenset(hidden_blobs),
        )

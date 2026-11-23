# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Map legacy RDF terms, rows, and snapshot dictionaries for P9 conversion."""

from collections.abc import Iterator
from dataclasses import dataclass
from typing import Literal, cast

import cbor2

from ._conversion_fields import contains_invalid_text, frame_values
from ._legacy_cbor import InvalidText, MapValue, Value
from .dictionary import (
    Blank,
    Iri,
    LanguageLiteral,
    RowResolution,
    Term,
    TermPool,
    Triple,
    TypedLiteral,
)
from .lines import LineFold
from .rows import AnnotationRow, QuadRow, ReifierBindingRow, Row
from .terms import Direction

_STRING = "http://www.w3.org/2001/XMLSchema#string"
_RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
_EMPTY_MAP = MapValue(())
_TERM_FIELDS = (
    frozenset(("k", "v")),
    frozenset(("k", "v", "l", "dt", "dir")),
    frozenset(("k", "v")),
    frozenset(("k", "tt")),
)
type RdfFrameType = Literal["terms", "quads", "reifies", "annot", "snapshot"]
type _RowKind = Literal["quads", "reifies", "annot"]
_ROW_WIDTHS: dict[_RowKind, int] = {"quads": 3, "reifies": 4, "annot": 3}
type _Rule = Literal[5, 8, 9, 10, 13]


class RdfSourceError(ValueError):
    """One of the P9 item rules that judges a source RDF field."""

    def __init__(self, rule: _Rule, detail: str) -> None:
        self.rule = rule
        super().__init__(detail)


@dataclass(frozen=True, slots=True)
class TermNote:
    index: int
    term: int
    keys: MapValue


@dataclass(frozen=True, slots=True)
class RdfContribution:
    rows: tuple[Row, ...]
    notes: tuple[TermNote, ...]


def _fields(value: Value) -> dict[str, Value]:
    if isinstance(value, MapValue):
        return {key: field for key, field in value.entries if isinstance(key, str)}
    return {}


def _term_reference_values(fields: dict[str, Value]) -> tuple[Value, ...]:
    kind = fields.get("k")
    if type(kind) is int and kind == 1 and "dt" in fields:
        return (fields["dt"],)
    triple = fields.get("tt")
    if type(kind) is int and kind == 3 and isinstance(triple, tuple):
        return triple
    return ()


def _text(value: Value) -> str:
    if not isinstance(value, str):
        raise RdfSourceError(9, "source term field must be valid UTF-8 text")
    return value


def _integer_ids(values: tuple[Value, ...]) -> tuple[int, ...]:
    ids = []
    for value in values:
        if type(value) is not int:
            raise RdfSourceError(9, "source term ID must be an integer")
        ids.append(value)
    return tuple(ids)


def _blank_label(value: Value, *, anonymous: int, snapshot: int | None) -> bytes:
    scope = [1] if snapshot is None else [2, snapshot]
    if isinstance(value, InvalidText):
        label = value.octets
    else:
        label = _text(value).encode("utf-8")
    name = [2, label] if label else [1, anonymous]
    return cbor2.dumps([scope, name])


def _rows_input(
    frame_type: RdfFrameType, payload: Value
) -> tuple[Value, dict[_RowKind, Value]]:
    if frame_type == "snapshot":
        if not isinstance(payload, MapValue):
            raise RdfSourceError(8, "snapshot payload must be a map")
        fields = _fields(payload)
        return fields.get("terms", ()), {
            kind: fields[kind] for kind in _ROW_WIDTHS if kind in fields
        }
    if frame_type == "terms":
        return payload, {}
    return (), {frame_type: payload}


def _reference_check(
    terms: Value, rows: dict[_RowKind, Value], *, already_defined: int, snapshot: bool
) -> None:
    """Judge references before the later rules over any field of the same item."""
    shift = 0 if snapshot else already_defined
    if isinstance(terms, tuple):
        for index, term in enumerate(terms):
            if any(
                type(ref) is int and not 0 <= ref < shift + index
                for ref in _term_reference_values(_fields(term))
            ):
                raise RdfSourceError(
                    5, "source term references an undefined earlier ID"
                )
    available = len(terms) if snapshot and isinstance(terms, tuple) else shift
    for kind, group in rows.items():
        if not isinstance(group, tuple):
            continue
        for row in group:
            if isinstance(row, tuple) and any(
                type(ref) is int and not 0 <= ref < available
                for ref in row[: _ROW_WIDTHS[kind] + 1]
            ):
                raise RdfSourceError(5, "source row references an undefined ID")


class ConversionSegment:
    """Resolve one segment's source IDs into a common v2 term pool and line fold."""

    def __init__(self, pool: TermPool, lines: LineFold, ordinal: int) -> None:
        self._pool = pool
        self._lines = lines
        self._ordinal = ordinal
        self._refs: list[int] = []

    def term(self, source_id: int) -> int:
        """Resolve an ID defined by an already accepted source frame."""
        if not 0 <= source_id < len(self._refs):
            raise RdfSourceError(5, "source term ID is undefined")
        return self._refs[source_id]

    def _ref(self, ident: int, pending: list[int]) -> int:
        existing = len(self._refs)
        return self._refs[ident] if ident < existing else pending[ident - existing]

    def _literal(self, fields: dict[str, Value], base: int, pending: list[int]) -> Term:
        lexical = _text(fields.get("v"))
        datatype = None
        if "dt" in fields:
            (reference,) = _integer_ids((fields["dt"],))
            target = self._pool.term(self._ref(base + reference, pending))
            if not isinstance(target, Iri):
                raise RdfSourceError(9, "literal datatype must name an IRI")
            datatype = target.text
        if "l" not in fields:
            if "dir" in fields:
                raise RdfSourceError(9, "literal direction requires language")
            return TypedLiteral(lexical, _STRING if datatype is None else datatype)

        language = _text(fields["l"])
        direction: Direction | None = None
        if "dir" in fields:
            value = fields["dir"]
            if value != "ltr" and value != "rtl":
                raise RdfSourceError(9, "source direction must be ltr or rtl")
            direction = cast(Direction, value)
        expected = _RDF + ("langString" if direction is None else "dirLangString")
        if datatype is not None and datatype != expected:
            raise RdfSourceError(
                9, "language literal datatype differs from its variant"
            )
        return LanguageLiteral(lexical, language, direction)

    def _map_term(
        self,
        fields: dict[str, Value],
        kind: int,
        pending: list[int],
        snapshot: int | None,
    ) -> Term:
        base = len(self._refs) if snapshot is not None else 0
        match kind:
            case 0:
                return Iri(_text(fields.get("v")))
            case 1:
                return self._literal(fields, base, pending)
            case 2:
                label = _blank_label(
                    fields.get("v", ""),
                    anonymous=len(self._refs) + len(pending),
                    snapshot=snapshot,
                )
                return Blank(self._ordinal, label)
            case 3:
                value = fields.get("tt")
                if not isinstance(value, tuple) or len(value) != 3:
                    raise RdfSourceError(9, "triple term must hold three source IDs")
                a, b, c = _integer_ids(value)
                return Triple(
                    self._ref(base + a, pending),
                    self._ref(base + b, pending),
                    self._ref(base + c, pending),
                )
        raise AssertionError("validated term kind is outside the source registry")

    def _node(self, ident: int) -> bool:
        return isinstance(self._pool.term(ident), Iri | Blank)

    def _triple_positions(self, subject: int, predicate: int) -> bool:
        return self._node(subject) and isinstance(self._pool.term(predicate), Iri)

    def _row_positions(self, kind: _RowKind, refs: tuple[int | None, ...]) -> None:
        predicate = 2 if kind == "reifies" else 1
        width = _ROW_WIDTHS[kind]
        for index, ref in enumerate(refs):
            if ref is None:
                continue
            if index == predicate and not isinstance(self._pool.term(ref), Iri):
                raise RdfSourceError(10, "source predicate must name an IRI")
            if (index < predicate or index == width) and not self._node(ref):
                raise RdfSourceError(
                    10, "source row position must name an IRI or blank"
                )

    def _map_row(self, kind: _RowKind, refs: tuple[int, ...], width: int) -> Row:
        graph = refs[-1] if len(refs) > width else None
        s, p, o = refs[:3]
        if kind == "reifies":
            reifier, s, p, o = refs[:4]
            triple = self._pool._intern(Triple(s, p, o))
            return ReifierBindingRow(reifier, triple, graph)
        if kind == "annot":
            return AnnotationRow(1, s, p, o, graph)
        return QuadRow(s, p, o, graph)

    def _mapped_rows(
        self, groups: dict[_RowKind, Value], base: int, pending: list[int]
    ) -> Iterator[Row]:
        malformed = False
        for kind, group in groups.items():
            assert isinstance(group, tuple)
            width = _ROW_WIDTHS[kind]
            for row in group:
                if not isinstance(row, tuple):
                    malformed = True
                    continue
                resolved = tuple(
                    self._ref(base + ident, pending) if type(ident) is int else None
                    for ident in row[: width + 1]
                )
                self._row_positions(kind, resolved)
                refs = tuple(ref for ref in resolved if ref is not None)
                if len(refs) != len(row) or len(refs) not in (width, width + 1):
                    malformed = True
                    continue
                yield self._map_row(kind, refs, width)
        if malformed:
            raise RdfSourceError(13, "source row has invalid arity or ID fields")

    def add_frame(
        self,
        frame_type: RdfFrameType,
        ordinal: int,
        payload: Value,
        *,
        envelope: MapValue = _EMPTY_MAP,
    ) -> RdfContribution:
        """Project one decoded RDF payload and retain its frame contribution."""
        terms, groups = _rows_input(frame_type, payload)
        snapshot = ordinal if frame_type == "snapshot" else None
        _reference_check(
            terms,
            groups,
            already_defined=len(self._refs),
            snapshot=snapshot is not None,
        )
        if not isinstance(terms, tuple) or any(
            not isinstance(group, tuple) for group in groups.values()
        ):
            raise RdfSourceError(8, "source RDF payload must be an array")
        if frame_type == "snapshot" and not isinstance(
            _fields(payload).get("blobs", _EMPTY_MAP), MapValue
        ):
            raise RdfSourceError(8, "snapshot blobs must be a map")
        if any(
            contains_invalid_text(value)
            for _, value in frame_values(frame_type, envelope, payload)
        ):
            raise RdfSourceError(9, "source frame field contains invalid UTF-8")

        pending: list[int] = []
        notes = []
        invalid_triple = False
        for index, raw in enumerate(terms):
            fields = _fields(raw)
            kind = fields.get("k")
            if (
                not isinstance(raw, MapValue)
                or type(kind) is not int
                or not 0 <= kind <= 3
            ):
                raise RdfSourceError(9, "source term has no registered kind")
            mapped = self._map_term(fields, kind, pending, snapshot)
            if isinstance(mapped, Triple):
                invalid_triple |= not self._triple_positions(
                    mapped.subject, mapped.predicate
                )
            ref = self._pool._intern(mapped)
            pending.append(ref)
            keys = MapValue(
                tuple(
                    (key, value)
                    for key, value in raw.entries
                    if not isinstance(key, str) or key not in _TERM_FIELDS[kind]
                )
            )
            if keys.entries:
                if contains_invalid_text(keys):
                    raise RdfSourceError(
                        9, "ignored source term field contains invalid UTF-8"
                    )
                notes.append(TermNote(index, ref, keys))
        if invalid_triple:
            raise RdfSourceError(10, "source triple has an inadmissible term")
        base = len(self._refs) if snapshot is not None else 0
        mapped_rows = tuple(self._mapped_rows(groups, base, pending))
        self._lines.add(RowResolution(mapped_rows, frozenset()))
        self._refs.extend(pending)
        return RdfContribution(mapped_rows, tuple(notes))

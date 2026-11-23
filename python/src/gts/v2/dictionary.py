# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Resolve segment-local IDs to P3 term values and graph lines."""

from bisect import bisect_right
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Literal

from ._content import UINT_MAX
from .rows import (
    AnnotationRow,
    DecodedRows,
    EmptyGraphRow,
    QuadRow,
    ReifierBindingRow,
    Row,
)
from .terms import (
    BlankTerm,
    DecodedTerms,
    Direction,
    IriTerm,
    LiteralTerm,
    TimeValue,
    TripleTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
    WireTerm,
    _kind,
)


@dataclass(frozen=True, slots=True)
class Iri:
    text: str


@dataclass(frozen=True, slots=True)
class TypedLiteral:
    lexical: str
    datatype: str


@dataclass(frozen=True, slots=True)
class LanguageLiteral:
    lexical: str
    language: str
    direction: Direction | None


@dataclass(frozen=True, slots=True)
class Blank:
    segment: int
    label: bytes


@dataclass(frozen=True, slots=True)
class Triple:
    subject: int
    predicate: int
    object: int


type Term = Iri | TypedLiteral | LanguageLiteral | Blank | Triple
type TermFinding = Literal[
    "ill_formed_term", "forward_reference", "position_constraint"
]
type RowFinding = Literal["forward_reference", "opaque_term", "position_constraint"]
type _OrderKey = tuple[int | bytes, ...]
type _Values = Mapping[ValueReference, bytes | None]


class TermPool:
    """Intern semantic values without recursively hashing nested triples."""

    def __init__(self) -> None:
        self._terms: list[Term] = []
        self._ids: dict[Term, int] = {}
        self._reference_texts: dict[bytes, str | None] = {}
        self._reference_octets: dict[bytes, TypedLiteral] = {}

    def term(self, term_id: int) -> Term:
        return self._terms[term_id]

    def _intern(self, term: Term) -> int:
        found = self._ids.get(term)
        if found is not None:
            return found
        term_id = len(self._terms)
        self._terms.append(term)
        self._ids[term] = term_id
        return term_id

    def _reference_text(self, octets: bytes) -> str | None:
        if octets not in self._reference_texts:
            try:
                self._reference_texts[octets] = octets.decode("utf-8")
            except UnicodeDecodeError:
                self._reference_texts[octets] = None
        return self._reference_texts[octets]

    def _reference_value(self, octets: bytes) -> TypedLiteral:
        value = self._reference_octets.get(octets)
        if value is None:
            lexical, datatype = TypedValue(5, octets).literal()
            value = TypedLiteral(lexical, datatype)
            self._reference_octets[octets] = value
        return value


@dataclass(frozen=True, slots=True)
class TermResolution:
    findings: frozenset[TermFinding]
    missing_values: tuple[ValueReference, ...]
    order_holds: bool | None
    values: tuple[tuple[int, TypedValue], ...]


@dataclass(frozen=True, slots=True)
class RowResolution:
    rows: tuple[Row, ...]
    findings: frozenset[RowFinding]


@dataclass(frozen=True, slots=True)
class _Entry:
    wire: WireTerm
    resolved: int | None
    written_key: int
    depth: int | None
    first_source: int


def _literal_form(term: LiteralTerm) -> int:
    return (
        1
        if term.datatype is not None
        else (None, "ltr", "rtl").index(term.direction) + 2
    )


def _term_order_key(
    term: WireTerm,
    datatype_text: Callable[[int], bytes | None],
    depth: int,
) -> _OrderKey | None:
    match term:
        case IriTerm(text):
            return 0, text
        case BlankTerm(label):
            return 5, label
        case TripleTerm(subject, predicate, object_id):
            return 6, depth, subject, predicate, object_id
        case TypedValueTerm(ValueReference(digest, length)):
            return 4, digest, length
        case TypedValueTerm(TypedValue(kind, value)):
            if isinstance(value, TimeValue):
                return 3, kind, value.seconds, value.nanoseconds
            return 3, kind, value
        case LiteralTerm(lexical, datatype, language, _):
            if datatype is not None:
                form_value = datatype_text(datatype)
                if form_value is None:
                    return None
            else:
                assert language is not None
                form_value = language.encode("utf-8")
            suffix = (_literal_form(term), form_value)
            if isinstance(lexical, ValueReference):
                return 2, lexical.digest, lexical.length, *suffix
            return 1, lexical, *suffix
    raise TypeError("expected a v2 term")


def _written_term_key(
    term: WireTerm, lookup: Callable[[int], tuple[bool, int]]
) -> tuple[object, ...]:
    if isinstance(term, LiteralTerm) and term.datatype is not None:
        return (_kind(term), term.lexical, lookup(term.datatype))
    if isinstance(term, TripleTerm):
        return (4, lookup(term.subject), lookup(term.predicate), lookup(term.object))
    return (_kind(term), term)


class TermDictionary:
    """Maintain one segment's IDs and resolve them into a shared term pool."""

    def __init__(self, pool: TermPool, segment: int) -> None:
        self._pool = pool
        self._segment = segment
        self._next = 0
        self._entries: dict[int, _Entry] = {}
        self._opaque_starts: list[int] = []
        self._opaque_runs: list[tuple[int, int]] = []
        self._written: dict[tuple[object, ...], int] = {}
        self._previous_key: _OrderKey | None = None

    @property
    def next_id(self) -> int:
        return self._next

    def reserve(self, runs: Sequence[tuple[int, int]]) -> None:
        """Reserve an opaque frame's IDs from its runs without expanding counts."""
        end = self._next + sum(count for _, count in runs)
        if end > UINT_MAX + 1 or any(kind < 1 or count < 1 for kind, count in runs):
            raise ValueError("invalid term runs for ID reservation")
        for kind, count in runs:
            self._opaque_starts.append(self._next)
            self._next += count
            self._opaque_runs.append((self._next, kind))
        self._previous_key = None

    def written_alike(self, term_id: int) -> tuple[bool, int]:
        """Return an annotation equality key before value rebuilding."""
        found = self._entries.get(term_id)
        if found is None:
            return False, term_id
        return True, found.written_key

    def resolved_id(self, term_id: int) -> int | None:
        found = self._entries.get(term_id)
        return found.resolved if found is not None else None

    def value_at(
        self, term_id: int, *, hidden_before: int = 0, in_doubt: bool = False
    ) -> int | None:
        """Resolve an ID under the dictionary rules at its use."""
        entry = self._entries.get(term_id)
        if entry is None or in_doubt or entry.first_source < hidden_before:
            return None
        return entry.resolved

    def blank_id(self, label: bytes) -> int | None:
        """Find a readable blank label defined by this segment."""
        return self._pool._ids.get(Blank(self._segment, label))

    def _kind_at(self, term_id: int) -> int | None:
        entry = self._entries.get(term_id)
        if entry is not None:
            return _kind(entry.wire)
        index = bisect_right(self._opaque_starts, term_id) - 1
        if index >= 0:
            end, kind = self._opaque_runs[index]
            if term_id < end:
                return kind
        return None

    def _invalid_position(self, term_id: int, valid: tuple[int, ...]) -> bool:
        kind = self._kind_at(term_id)
        return kind is not None and kind <= 7 and kind not in valid

    def _written_key(self, term: WireTerm) -> int:
        key = _written_term_key(term, self.written_alike)
        return self._written.setdefault(key, len(self._written))

    def _order_key(self, term: WireTerm) -> tuple[_OrderKey | None, int | None]:
        depth = 0
        if isinstance(term, TripleTerm):
            for component in (term.subject, term.predicate, term.object):
                entry = self._entries.get(component)
                if entry is None:
                    kind = self._kind_at(component)
                    if kind is None or kind == 4 or kind > 7:
                        return None, None
                    continue
                if entry.depth is None:
                    return None, None
                depth = max(depth, entry.depth)
            depth += 1
        return _term_order_key(term, self._datatype_text, depth), depth

    def _datatype_text(self, term_id: int) -> bytes | None:
        entry = self._entries.get(term_id)
        if entry is None or not isinstance(entry.wire, IriTerm):
            return None
        return entry.wire.text

    def _literal(
        self, term: LiteralTerm, term_id: int, values: _Values
    ) -> tuple[Term | None, TermFinding | ValueReference | None]:
        lexical = term.lexical
        octets = values.get(lexical) if isinstance(lexical, ValueReference) else lexical
        text = None
        if octets is not None:
            if isinstance(lexical, ValueReference):
                text = self._pool._reference_text(octets)
                if text is None:
                    return None, "ill_formed_term"
            else:
                try:
                    text = octets.decode("utf-8")
                except UnicodeDecodeError:
                    return None, "ill_formed_term"

        datatype: str | None = None
        if term.datatype is not None:
            dt = term.datatype
            if dt >= term_id:
                return None, "forward_reference"
            if self._invalid_position(dt, (1,)):
                return None, "position_constraint"
            resolved = self.resolved_id(dt)
            if resolved is None:
                return None, None
            iri = self._pool.term(resolved)
            assert isinstance(iri, Iri)
            datatype = iri.text

        if text is None:
            assert isinstance(lexical, ValueReference)
            return None, lexical if lexical not in values else None
        if datatype is not None:
            return TypedLiteral(text, datatype), None
        assert term.language is not None
        return LanguageLiteral(text, term.language, term.direction), None

    def _resolve(
        self, term: WireTerm, term_id: int, values: _Values
    ) -> tuple[Term | None, TermFinding | ValueReference | None]:
        match term:
            case IriTerm(text):
                try:
                    return Iri(text.decode("utf-8")), None
                except UnicodeDecodeError:
                    return None, "ill_formed_term"
            case BlankTerm(label):
                return Blank(self._segment, label), None
            case LiteralTerm():
                return self._literal(term, term_id, values)
            case TripleTerm(subject, predicate, object_id):
                if any(
                    component >= term_id
                    for component in (subject, predicate, object_id)
                ):
                    return None, "forward_reference"
                if self._invalid_position(subject, (1, 3)) or self._invalid_position(
                    predicate, (1,)
                ):
                    return None, "position_constraint"
                s, p, o = map(self.resolved_id, (subject, predicate, object_id))
                if s is None or p is None or o is None:
                    return None, None
                return Triple(s, p, o), None
            case TypedValueTerm(value):
                if isinstance(value, ValueReference):
                    rebuilt = values.get(value)
                    if rebuilt is None:
                        return None, value if value not in values else None
                    return self._pool._reference_value(rebuilt), None
                lexical, datatype = value.literal()
                return TypedLiteral(lexical, datatype), None
        raise TypeError("expected a v2 term")

    def add(
        self,
        batch: DecodedTerms,
        *,
        values: _Values | None = None,
        in_doubt: bool = False,
    ) -> TermResolution:
        """Resolve terms using rebuilt values available before this frame.

        A present mapping entry with no bytes means candidates exist but none
        rebuilt successfully. An absent entry means no preceding declaration
        provides the pair. Mark frames in doubt after a damaged region or an
        opaque item puts a segment's dictionary in doubt.
        """
        if batch.id_base != self._next:
            raise ValueError("decoded term batch does not start at the next ID")
        sources = {} if values is None else values
        findings: set[TermFinding] = set()
        missing: dict[ValueReference, None] = {}
        last_key = self._previous_key
        last_in_frame = None
        holds = True
        kinds: dict[int, int] = {}
        typed_values = []

        for wire in batch.terms:
            term_id = self._next
            key, depth = self._order_key(wire)
            term, problem = self._resolve(wire, term_id, sources)
            if isinstance(problem, ValueReference):
                missing[problem] = None
            elif problem is not None:
                findings.add(problem)
            resolved = (
                self._pool._intern(term) if term is not None and not in_doubt else None
            )
            if isinstance(wire, TypedValueTerm) and resolved is not None:
                value = wire.value
                if isinstance(value, ValueReference):
                    octets = sources[value]
                    assert octets is not None
                    value = TypedValue(5, octets)
                typed_values.append((resolved, value))
            if key is not None:
                if last_key is not None and key <= last_key:
                    holds = False
                last_key = last_in_frame = key
            if (
                batch.canonical
                and resolved is not None
                and isinstance(term, TypedLiteral | LanguageLiteral)
            ):
                kind = _kind(wire)
                if kinds.setdefault(resolved, kind) != kind:
                    holds = False
            first_source = term_id
            if isinstance(wire, TripleTerm) and resolved is not None:
                first_source = min(
                    self._entries[part].first_source
                    for part in (wire.subject, wire.predicate, wire.object)
                )
            self._entries[term_id] = _Entry(
                wire, resolved, self._written_key(wire), depth, first_source
            )
            self._next += 1

        self._previous_key = last_in_frame if batch.canonical else None
        return TermResolution(
            frozenset(findings),
            tuple(missing),
            holds if batch.canonical else None,
            tuple(typed_values),
        )

    def _row(
        self, row: Row, hidden_before: int, in_doubt: bool
    ) -> tuple[Row | None, RowFinding | None]:
        named = ((row.graph, (1, 3)),) if row.graph is not None else ()
        slots: tuple[tuple[int, tuple[int, ...]], ...]
        match row:
            case QuadRow(subject, predicate, object_id, _):
                slots = ((subject, (1, 3)), (predicate, (1,)), (object_id, ()), *named)
            case ReifierBindingRow(reifier, triple, _):
                slots = ((reifier, (1, 3)), (triple, (4,)), *named)
            case AnnotationRow(_, reifier, predicate, object_id, _):
                slots = ((reifier, (1, 3)), (predicate, (1,)), (object_id, ()), *named)
            case _:
                slots = named

        if any(source >= self._next for source, _ in slots):
            return None, "forward_reference"
        ids = []
        for source, _ in slots:
            value = self.value_at(
                source, hidden_before=hidden_before, in_doubt=in_doubt
            )
            if value is None:
                return None, "opaque_term"
            ids.append(value)
        if any(
            valid and self._invalid_position(source, valid) for source, valid in slots
        ):
            return None, "position_constraint"

        graph = ids[-1] if named else None
        resolved_row: Row
        match row:
            case QuadRow():
                resolved_row = QuadRow(ids[0], ids[1], ids[2], graph)
            case ReifierBindingRow():
                resolved_row = ReifierBindingRow(ids[0], ids[1], graph)
            case AnnotationRow():
                resolved_row = AnnotationRow(row.count, ids[0], ids[1], ids[2], graph)
            case _:
                resolved_row = EmptyGraphRow(ids[0])
        return resolved_row, None

    def resolve_rows(
        self, batch: DecodedRows, *, hidden_before: int = 0, in_doubt: bool = False
    ) -> RowResolution:
        """Resolve rows after payload validation using the dictionary at their frame.

        Rows after a broken ordinary link can hide the earlier ID prefix.
        The caller applies the exception for frames before a valid commit.
        """
        rows = []
        findings = set()
        for source in batch.rows:
            row, problem = self._row(source, hidden_before, in_doubt)
            if row is not None:
                rows.append(row)
            if problem is not None:
                findings.add(problem)
        return RowResolution(tuple(rows), frozenset(findings))

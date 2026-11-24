# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed L1 input, line encoding, and the digest of a positioned sequence."""

from collections.abc import Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from typing import Literal as Choice

from blake3 import blake3

from .dictionary import Triple
from .rows import AnnotationRow, EmptyGraphRow, QuadRow, ReifierBindingRow, Row

_MAX_UINT = (1 << 64) - 1
_LINE_CONTEXT = "nested-refinement-labelling 1 lines"


class LabellingRefused(ValueError):
    """An L1 input refusal with its typed reason and optional blank witness."""

    def __init__(self, kind: str, reason: str, blank: bytes | None = None) -> None:
        self.kind = kind
        self.reason = reason
        self.blank = blank
        super().__init__(f"labelling refused: {kind} ({reason})")


@dataclass(frozen=True, slots=True)
class Iri:
    value: bytes


@dataclass(frozen=True, slots=True)
class Literal:
    lexical: bytes
    datatype: bytes
    language: bytes | None = None
    direction: Choice["ltr", "rtl"] | None = None


@dataclass(frozen=True, slots=True)
class Blank:
    source: bytes


type Term = Iri | Literal | Blank | Triple


def _uint(value: int) -> bytes:
    return value.to_bytes(8, "big")


def _octets(value: bytes) -> bytes:
    return _uint(len(value)) + value


class LabellingTerms:
    """Intern typed values. Triple components name previously interned terms."""

    def __init__(self) -> None:
        self._terms: list[Term] = []
        self._ids: dict[Term, int] = {}
        self._sizes: list[tuple[int, int]] = []

    def add(self, term: Term) -> int:
        """Return a shared ID for a valid typed term."""
        match term:
            case Iri(value):
                if not isinstance(value, bytes):
                    raise LabellingRefused("input", "slot")
                size = (9 + len(value), 0)
            case Literal(lexical, datatype, language, direction):
                if not (
                    isinstance(lexical, bytes)
                    and isinstance(datatype, bytes)
                    and (language is None or isinstance(language, bytes))
                    and direction in (None, "ltr", "rtl")
                ):
                    raise LabellingRefused("input", "slot")
                language_length = 0 if language is None else 8 + len(language)
                size = (
                    19
                    + len(lexical)
                    + len(datatype)
                    + language_length
                    + (direction is not None),
                    0,
                )
            case Blank(source):
                if not isinstance(source, bytes):
                    raise LabellingRefused("input", "slot")
                size = (1, 1)
            case Triple(s, p, o):
                if not all(self._has(ref) for ref in (s, p, o)):
                    raise LabellingRefused("input", "slot")
                components = tuple(self._sizes[ref] for ref in (s, p, o))
                size = (
                    min(_MAX_UINT + 1, 1 + sum(length for length, _ in components)),
                    min(_MAX_UINT + 1, sum(n for _, n in components)),
                )
            case _:
                raise LabellingRefused("input", "slot")
        found = self._ids.get(term)
        if found is not None:
            return found
        identity = len(self._terms)
        self._terms.append(term)
        self._sizes.append(size)
        self._ids[term] = identity
        return identity

    def _has(self, ref: int) -> bool:
        return type(ref) is int and 0 <= ref < len(self._terms)


@dataclass(frozen=True, slots=True)
class RecordLine:
    payload: bytes
    terms: tuple[int, ...]


type Line = Row | RecordLine


@dataclass(frozen=True, slots=True)
class _Slots:
    kind: int
    terms: tuple[int, ...]
    graph: int | None = None
    count: int | None = None
    payload: bytes | None = None

    def ordered_terms(self) -> tuple[int, ...]:
        return self.terms if self.graph is None else (*self.terms, self.graph)

    def header_length(self) -> int:
        if self.payload is not None:
            return 17 + len(self.payload)
        return (
            1 + (8 if self.count is not None else 0) + (self.kind in (0x40, 0x41, 0x42))
        )

    def free_shape(self, sizes: Sequence[tuple[int, int]]) -> tuple[int, int]:
        length = self.header_length()
        occurrences = 0
        for ref in self.ordered_terms():
            term_length, term_occurrences = sizes[ref]
            length += term_length
            occurrences += term_occurrences
        return length, occurrences


def _slots(line: Line) -> _Slots | None:
    match line:
        case QuadRow(s, p, o, graph):
            return _Slots(0x40, (s, p, o), graph)
        case ReifierBindingRow(r, triple, graph):
            return _Slots(0x41, (r, triple), graph)
        case AnnotationRow(n, r, p, o, graph):
            return _Slots(0x42, (r, p, o), graph, count=n)
        case EmptyGraphRow(graph):
            return _Slots(0x43, (graph,))
        case RecordLine(payload, terms):
            return _Slots(0x44, terms, payload=payload)
    return None


@dataclass(frozen=True, slots=True)
class PositionedSequence:
    encodings: tuple[bytes, ...]
    digest: bytes


class LabellingInput:
    """Validate a sequence of L1 lines without repairing it or choosing positions."""

    def __init__(self, terms: LabellingTerms, lines: Iterable[Line]) -> None:
        slots: list[_Slots] = []
        seen: set[tuple[object, ...]] = set()
        empty: set[int] = set()
        named: set[int] = set()
        faults: set[str] = set()
        widths: set[str] = set()
        total_length = total_occurrences = 0
        for line in lines:
            shape = _slots(line)
            if shape is None:
                faults.add("kind")
                continue
            if not self._valid_slots(terms, shape):
                faults.add("slot")
                continue
            slots.append(shape)
            if shape.count is not None:
                if shape.count < 1:
                    faults.add("count")
                if shape.count > _MAX_UINT:
                    widths.add("count")
            identity = (shape.kind, shape.terms, shape.graph, shape.payload)
            if identity in seen:
                faults.add("repeated")
            seen.add(identity)
            if shape.kind == 0x43:
                empty.add(shape.terms[0])
            elif shape.graph is not None:
                named.add(shape.graph)
            free_length, line_occurrences = shape.free_shape(terms._sizes)
            if free_length + 8 * line_occurrences > _MAX_UINT:
                widths.add("length")
            if line_occurrences:
                total_length += free_length
            total_occurrences += line_occurrences
        self._terms = tuple(terms._terms)
        self._sizes = tuple(terms._sizes)
        for fault in ("slot", "count", "repeated", "kind"):
            if fault in faults:
                raise LabellingRefused("input", fault)
        graph_conflicts = empty & named
        for variant in (Iri, Literal, Blank, Triple):
            members = [
                self._terms[ident]
                for ident in graph_conflicts
                if isinstance(self._terms[ident], variant)
            ]
            if members:
                blank = min(
                    (term.source for term in members if isinstance(term, Blank)),
                    default=None,
                )
                raise LabellingRefused("graph", variant.__name__.lower(), blank)
        if total_occurrences > _MAX_UINT:
            widths.add("occurrences")
        if total_length > _MAX_UINT:
            widths.add("length")
        for width in ("occurrences", "length", "count"):
            if width in widths:
                raise LabellingRefused("width", width)
        self._lines = tuple(slots)

    def __len__(self) -> int:
        return len(self._lines)

    def free_shape(self, index: int) -> tuple[int, int]:
        """Return the all-free encoding length and number of blank occurrences."""
        return self._lines[index].free_shape(self._sizes)

    @staticmethod
    def _valid_slots(terms: LabellingTerms, shape: _Slots) -> bool:
        if not isinstance(shape.terms, tuple) or not all(
            terms._has(ref) for ref in shape.ordered_terms()
        ):
            return False
        if shape.kind == 0x41:
            return isinstance(terms._terms[shape.terms[1]], Triple)
        if shape.kind == 0x44:
            return isinstance(shape.payload, bytes)
        if shape.kind == 0x42 and type(shape.count) is not int:
            return False
        return shape.kind == 0x43 or isinstance(terms._terms[shape.terms[1]], Iri)

    def _walk(
        self, refs: tuple[int, ...], *, blanks_only: bool = False
    ) -> Iterator[Term]:
        pending = list(reversed(refs))
        while pending:
            ref = pending.pop()
            if blanks_only and not self._sizes[ref][1]:
                continue
            term = self._terms[ref]
            yield term
            if isinstance(term, Triple):
                pending.extend((term.object, term.predicate, term.subject))

    def incidences(
        self, index: int, fixed: Mapping[bytes, int]
    ) -> dict[bytes, tuple[int, ...]]:
        """Number free occurrences of a line in slot and term order."""
        labels: dict[bytes, list[int]] = {}
        number = 0
        for term in self._walk(self._lines[index].ordered_terms(), blanks_only=True):
            if isinstance(term, Blank) and term.source not in fixed:
                labels.setdefault(term.source, []).append(number)
                number += 1
        return {source: tuple(label) for source, label in labels.items()}

    def mask(self, index: int, fixed: Mapping[bytes, int]) -> bytes:
        """Encode one line, using free or fixed leaves for its blank nodes."""
        line = self._lines[index]
        encoding = bytearray([line.kind])
        if line.count is not None:
            encoding.extend(_uint(line.count))
        if line.payload is not None:
            encoding.extend(_octets(line.payload))
            encoding.extend(_uint(len(line.terms)))
        self._encode_terms(encoding, line.terms, fixed)
        if line.kind in (0x40, 0x41, 0x42):
            if line.graph is None:
                encoding.append(0x20)
            else:
                encoding.append(0x21)
                self._encode_terms(encoding, (line.graph,), fixed)
        return bytes(encoding)

    def _encode_terms(
        self, encoding: bytearray, refs: tuple[int, ...], fixed: Mapping[bytes, int]
    ) -> None:
        for term in self._walk(refs):
            match term:
                case Iri(value):
                    encoding.append(1)
                    encoding.extend(_octets(value))
                case Literal(lexical, datatype, language, direction):
                    encoding.append(2)
                    encoding.extend(_octets(lexical))
                    encoding.extend(_octets(datatype))
                    if language is None:
                        encoding.append(0x20)
                    else:
                        encoding.append(0x21)
                        encoding.extend(_octets(language))
                    if direction is None:
                        encoding.append(0x20)
                    else:
                        encoding.extend((0x21, 0x30 if direction == "ltr" else 0x31))
                case Triple():
                    encoding.append(3)
                case Blank(source):
                    if source in fixed:
                        encoding.append(5)
                        encoding.extend(_uint(fixed[source]))
                    else:
                        encoding.append(4)

    def positioned_sequence(self, positions: Mapping[bytes, int]) -> PositionedSequence:
        """Encode and hash the sequence using an externally supplied bijection."""
        sources = {
            source
            for index in range(len(self._lines))
            for source in self.incidences(index, {})
        }
        values = positions.values()
        if (
            positions.keys() != sources
            or any(type(position) is not int for position in values)
            or sorted(values) != list(range(len(sources)))
        ):
            raise ValueError("positions must bijectively cover the input's blank nodes")
        encodings = tuple(
            sorted(self.mask(index, positions) for index in range(len(self._lines)))
        )
        digest = blake3(_uint(len(encodings)), derive_key_context=_LINE_CONTEXT)
        for encoding in encodings:
            digest.update(encoding)
        return PositionedSequence(encodings, digest.digest())

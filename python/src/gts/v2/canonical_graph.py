# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Canonical dictionary and row frames for the graph of a P3 fold."""

from collections import defaultdict
from collections.abc import Iterable, Iterator, Mapping
from dataclasses import dataclass
from itertools import batched, chain, groupby

import cbor2

from ._fold_record_lines import named_record_terms
from .dictionary import (
    Blank,
    Iri,
    LanguageLiteral,
    Term,
    TermPool,
    Triple,
    TypedLiteral,
    _OrderKey,
    _term_order_key,
)
from .labelling import _slots
from .rebuild import unit_digest
from .record_fold import RecordFold
from .rows import Row, _key, _row_parts, encode_rows, remap_row
from .terms import (
    BlankTerm,
    IriTerm,
    LiteralTerm,
    TripleTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
    WireTerm,
    _encode_terms,
    _value_record,
)

_TERMS_FRAME_BYTES = 1 << 22
_ROWS_PER_FRAME = 1 << 20
_RESTART_GROUP = 64
_LITERAL_REF_BYTES = 1 << 20
_VALUE_REF_BYTES = 1 << 19

type EncodedFrame = tuple[int, list[object], bytes]


@dataclass(frozen=True, slots=True)
class CanonicalGraph:
    term_ids: Mapping[int, int]
    terms: tuple[WireTerm, ...]
    rows: tuple[Row, ...]
    value_octets: tuple[bytes, ...]

    def frames(self) -> Iterator[EncodedFrame]:
        """Yield terms and row frames in canonical order for the segment writer."""
        first = used = 0
        for index, term in enumerate(self.terms):
            weight = 8 + _term_text_length(term)
            if index > first and used + weight > _TERMS_FRAME_BYTES:
                yield _terms_frame(self.terms[first:index], first)
                first = index
                used = 0
            used += weight
        if first < len(self.terms):
            yield _terms_frame(self.terms[first:], first)
        for _, group in groupby(self.rows, key=lambda row: _row_parts(row)[0]):
            for batch in batched(group, _ROWS_PER_FRAME, strict=False):
                yield encode_rows(batch, canonical=True)


def _terms_frame(terms: tuple[WireTerm, ...], base: int) -> EncodedFrame:
    content, payload = _encode_terms(terms, base, _RESTART_GROUP, canonical=True)
    return 1, content, payload


def _term_text_length(term: WireTerm) -> int:
    match term:
        case IriTerm(text) | BlankTerm(text) | LiteralTerm(bytes(text)):
            return len(text)
        case TypedValueTerm(TypedValue() as value):
            return len(cbor2.dumps(_value_record(value)))
    return 0


def _reachable(pool: TermPool, roots: Iterable[int]) -> dict[int, Term]:
    terms: dict[int, Term] = {}
    pending = []
    for root in roots:
        pending.append(root)
        while pending:
            ref = pending.pop()
            if ref in terms:
                continue
            term = terms[ref] = pool.term(ref)
            if isinstance(term, Triple):
                pending.extend((term.subject, term.predicate, term.object))
    return terms


def _row_terms(rows: Iterable[Row]) -> Iterator[int]:
    for row in rows:
        slots = _slots(row)
        assert slots is not None
        yield from slots.ordered_terms()


def dictionary_terms(
    pool: TermPool,
    rows: Iterable[Row],
    records: RecordFold,
    typed_values: Iterable[int],
) -> dict[int, Term]:
    """Return the terms reachable from the canonical dictionary's roots."""
    return _reachable(
        pool, chain(_row_terms(rows), named_record_terms(records), typed_values)
    )


def _reference(octets: bytes, values: dict[bytes, ValueReference]) -> ValueReference:
    if octets not in values:
        values[octets] = ValueReference(unit_digest(octets), len(octets))
    return values[octets]


def _literal(
    term: TypedLiteral | LanguageLiteral,
    value: TypedValue | None,
    iris: Mapping[str, int],
    stored: dict[bytes, ValueReference],
) -> LiteralTerm | TypedValueTerm:
    if value is not None:
        octets = value.value
        if (
            value.kind == 5
            and isinstance(octets, bytes)
            and len(octets) >= _VALUE_REF_BYTES
        ):
            return TypedValueTerm(_reference(octets, stored))
        return TypedValueTerm(value)
    lexical = term.lexical.encode("utf-8")
    text = (
        _reference(lexical, stored) if len(lexical) >= _LITERAL_REF_BYTES else lexical
    )
    if isinstance(term, TypedLiteral):
        return LiteralTerm(text, datatype=iris[term.datatype])
    return LiteralTerm(text, language=term.language, direction=term.direction)


def _triple_layers(terms: Mapping[int, Term]) -> dict[int, list[tuple[int, Triple]]]:
    layers: dict[int, list[tuple[int, Triple]]] = defaultdict(list)
    depths: dict[int, int] = {}
    # The fold interns all components before the triple that names them.
    for ref in sorted(terms):
        term = terms[ref]
        if isinstance(term, Triple):
            depth = 1 + max(
                depths.get(child, 0)
                for child in (
                    term.subject,
                    term.predicate,
                    term.object,
                )
            )
            depths[ref] = depth
            layers[depth].append((ref, term))
    return layers


def canonical_graph(
    pool: TermPool,
    rows: Iterable[Row],
    records: RecordFold,
    values: Iterable[tuple[int, TypedValue]],
    blank_positions: Mapping[int, int] | None,
) -> CanonicalGraph:
    """Build dictionary and row frames using positions or the stored blank names."""
    original_rows = tuple(rows)
    typed_values = dict(values)
    terms = dictionary_terms(pool, original_rows, records, typed_values)

    iri_names = {term.text for term in terms.values() if isinstance(term, Iri)}
    iri_names.update(
        term.datatype
        for ref, term in terms.items()
        if isinstance(term, TypedLiteral) and ref not in typed_values
    )
    iris = {text: ident for ident, text in enumerate(sorted(iri_names, key=str.encode))}
    iri_text = {ident: text.encode("utf-8") for text, ident in iris.items()}
    ordered: list[WireTerm] = [IriTerm(text) for text in iri_text.values()]
    ids = {ref: iris[term.text] for ref, term in terms.items() if isinstance(term, Iri)}
    stored: dict[bytes, ValueReference] = {}
    scalars: list[tuple[_OrderKey, int, WireTerm]] = []
    for ref, term in terms.items():
        wire: WireTerm
        if isinstance(term, Blank):
            label = (
                term.label
                if blank_positions is None
                else cbor2.dumps(blank_positions[ref])
            )
            wire = BlankTerm(label)
        elif isinstance(term, TypedLiteral | LanguageLiteral):
            wire = _literal(term, typed_values.get(ref), iris, stored)
        else:
            continue
        key = _term_order_key(wire, iri_text.get, 0)
        assert key is not None
        scalars.append((key, ref, wire))
    for _, ref, wire in sorted(scalars, key=lambda item: item[0]):
        ids[ref] = len(ordered)
        ordered.append(wire)

    layers = _triple_layers(terms)
    for depth in sorted(layers):
        triples = (
            (ref, TripleTerm(ids[term.subject], ids[term.predicate], ids[term.object]))
            for ref, term in layers[depth]
        )
        for ref, triple in sorted(
            triples,
            key=lambda item: (item[1].subject, item[1].predicate, item[1].object),
        ):
            ids[ref] = len(ordered)
            ordered.append(triple)

    sorted_rows = tuple(
        sorted(
            (remap_row(row, ids.__getitem__) for row in original_rows),
            key=lambda row: (_row_parts(row)[0], _key(row)),
        )
    )
    value_octets = tuple(sorted(stored, key=lambda data: stored[data].digest))
    return CanonicalGraph(ids, tuple(ordered), sorted_rows, value_octets)

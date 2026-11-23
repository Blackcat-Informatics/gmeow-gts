# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P3 record payloads and term slots, including unresolved pairs."""

from collections.abc import Iterable, Iterator
from dataclasses import dataclass

import cbor2

from ._fold_terms import FoldTerms
from .blobs import BlobSet
from .items import _shortest_head_size
from .labelling import RecordLine
from .record_fold import (
    GroundRef,
    GroundTerm,
    RecordFold,
    ResolvedLeaf,
    ResolvedSuppression,
)
from .records import (
    BlankPair,
    BlobReference,
    FrameReference,
    Referent,
    StateReference,
    SuppressionKind,
)
from .terms import TypedValue


@dataclass(frozen=True, slots=True)
class _Slot:
    term: ResolvedLeaf


@dataclass(frozen=True, slots=True)
class _ArrayHead:
    length: int


type _Field = int | str | bytes | _Slot | tuple[_Field, ...]
type RecordBody = tuple[_Field, ...]
type _WirePart = int | str | bytes | _ArrayHead
type _Part = _WirePart | _Slot


def _referent(value: Referent) -> RecordBody:
    match value:
        case BlobReference(algorithm, digest):
            return 1, (algorithm, digest)
        case FrameReference(identifier):
            return 2, identifier
        case StateReference(identifier):
            return 3, identifier


def _binding_body(term: ResolvedLeaf, referent: Referent) -> RecordBody:
    return _Slot(term), _referent(referent)


def _suppression_body(
    suppression: ResolvedSuppression,
) -> tuple[SuppressionKind, _Field]:
    kind = suppression.kind
    slots = tuple(_Slot(term) for term in suppression.terms)
    if kind == SuppressionKind.BLOB:
        blob = suppression.referent
        assert isinstance(blob, BlobReference)
        return kind, (blob.algorithm, blob.digest)
    if kind == SuppressionKind.BINDING:
        referent = suppression.referent
        assert referent is not None and len(slots) == 1
        return kind, _binding_body(slots[0].term, referent)
    if kind in (
        SuppressionKind.QUAD,
        SuppressionKind.REIFIER_BINDING,
        SuppressionKind.ANNOTATION,
    ):
        graph = (1,) if suppression.graph is None else (2, _Slot(suppression.graph))
        return kind, (*slots, graph)
    (target,) = slots
    return kind, target


def _graph_bodies(records: RecordFold) -> Iterator[RecordBody]:
    for binding in records.bindings:
        yield 1, _binding_body(binding.term, binding.referent)
    for suppression in records.suppression:
        kind, target = _suppression_body(suppression)
        if isinstance(target, tuple) and kind != SuppressionKind.BLOB:
            yield 2, (kind, *target)
        else:
            yield 2, (kind, target)


def record_bodies(
    records: RecordFold, blobs: BlobSet, values: Iterable[tuple[int, TypedValue]]
) -> Iterator[RecordBody]:
    yield from _graph_bodies(records)
    for blob in blobs.entries:
        yield 3, (1 if blob.present else 2, blob.name, blob.length, blob.media_types)
    for ref, value in values:
        yield 4, (value.kind, _Slot(ref))


def _parts(
    body: RecordBody, ground: tuple[GroundTerm, ...], *, wire: bool = False
) -> Iterator[_Part]:
    pending: list[_Field] = [body]
    while pending:
        field = pending.pop()
        match field:
            case tuple():
                yield _ArrayHead(len(field))
                pending.extend(reversed(field))
            case _Slot(GroundRef(index)):
                term = ground[index]
                if isinstance(term, BlankPair):
                    pair = (term.frame, term.label)
                    pending.append((2, pair) if wire else (2, *pair))
                else:
                    children = (
                        _Slot(term.subject),
                        _Slot(term.predicate),
                        _Slot(term.object),
                    )
                    pending.append((3, children) if wire else (3, *children))
            case _:
                yield field


def named_record_terms(records: RecordFold) -> Iterator[int]:
    """Yield named term slots without assigning terms to unresolved pairs."""
    for body in _graph_bodies(records):
        for part in _parts(body, records.ground_terms):
            if isinstance(part, _Slot) and isinstance(part.term, int):
                yield part.term


def record_weight(
    body: RecordBody, ground: tuple[GroundTerm, ...], terms: FoldTerms
) -> int:
    # Line tag, payload byte count, and term list count.
    weight = 17
    for part in _parts(body, ground):
        match part:
            case _Slot(int(ref)):
                weight += 2 + terms.weight(ref)
            case _ArrayHead(length):
                weight += _shortest_head_size(length)
            case int(value):
                weight += _shortest_head_size(value)
            case str() | bytes():
                length = len(part.encode() if isinstance(part, str) else part)
                weight += _shortest_head_size(length) + length
            case _:
                raise AssertionError("unresolved record slot")
    return weight


def record_line(
    body: RecordBody, ground: tuple[GroundTerm, ...], terms: FoldTerms
) -> RecordLine:
    payload = bytearray()
    refs = []
    for part in _parts(body, ground):
        match part:
            case _Slot(int(ref)):
                payload.extend(b"\x81\x01")
                refs.append(terms.lowered(ref))
            case _Slot():
                raise AssertionError("unresolved record slot")
            case _:
                payload.extend(_encode_part(part))
    return RecordLine(bytes(payload), tuple(refs))


def _array_head(length: int) -> bytes:
    argument = cbor2.dumps(length)
    return bytes((0x80 | argument[0],)) + argument[1:]


def _encode_part(part: _WirePart) -> bytes:
    return (
        _array_head(part.length) if isinstance(part, _ArrayHead) else cbor2.dumps(part)
    )

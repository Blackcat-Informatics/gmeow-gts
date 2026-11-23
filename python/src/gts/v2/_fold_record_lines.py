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
from .record_fold import GroundRef, GroundTerm, RecordFold, ResolvedLeaf
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
type _Part = int | str | bytes | _ArrayHead | _Slot


def _referent(value: Referent) -> RecordBody:
    match value:
        case BlobReference(algorithm, digest):
            return 1, (algorithm, digest)
        case FrameReference(identifier):
            return 2, identifier
        case StateReference(identifier):
            return 3, identifier


def record_bodies(
    records: RecordFold, blobs: BlobSet, values: Iterable[tuple[int, TypedValue]]
) -> Iterator[RecordBody]:
    for binding in records.bindings:
        yield 1, (_Slot(binding.term), _referent(binding.referent))
    for suppression in records.suppression:
        kind = suppression.kind
        slots = tuple(_Slot(term) for term in suppression.terms)
        if kind == SuppressionKind.BLOB:
            blob_reference = suppression.referent
            assert isinstance(blob_reference, BlobReference)
            body: RecordBody = kind, (blob_reference.algorithm, blob_reference.digest)
        elif kind == SuppressionKind.BINDING:
            referent = suppression.referent
            assert referent is not None
            body = kind, *slots, _referent(referent)
        elif kind in (
            SuppressionKind.QUAD,
            SuppressionKind.REIFIER_BINDING,
            SuppressionKind.ANNOTATION,
        ):
            graph = (1,) if suppression.graph is None else (2, _Slot(suppression.graph))
            body = kind, *slots, graph
        else:
            body = kind, *slots
        yield 2, body
    for blob in blobs.entries:
        yield 3, (1 if blob.present else 2, blob.name, blob.length, blob.media_types)
    for ref, value in values:
        yield 4, (value.kind, _Slot(ref))


def _parts(body: RecordBody, ground: tuple[GroundTerm, ...]) -> Iterator[_Part]:
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
                    pending.append((2, term.frame, term.label))
                else:
                    pending.append(
                        (
                            3,
                            _Slot(term.subject),
                            _Slot(term.predicate),
                            _Slot(term.object),
                        )
                    )
            case _:
                yield field


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
            case _ArrayHead(length):
                argument = cbor2.dumps(length)
                payload.append(0x80 | argument[0])
                payload.extend(argument[1:])
            case _Slot():
                raise AssertionError("unresolved record slot")
            case _:
                payload.extend(cbor2.dumps(part))
    return RecordLine(bytes(payload), tuple(refs))

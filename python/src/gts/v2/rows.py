# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Read and write graph row frames (P3 §§1.4, 3, 11.2)."""

from collections.abc import Callable, Hashable, Iterable, Iterator, Sequence
from dataclasses import dataclass
from itertools import chain, groupby, pairwise, repeat
from typing import Literal

import cbor2

from ._content import UINT_MAX, ContentError, content_code
from ._decode import _Decoder

_QUADS = 2
_REIFIER_BINDINGS = 3
_ANNOTATIONS = 4
_EMPTY_GRAPHS = 5
_PAYLOAD_SIZES = {2: 4, 3: 3, 4: 5, 5: 1}


@dataclass(frozen=True, slots=True)
class QuadRow:
    subject: int
    predicate: int
    object: int
    graph: int | None = None


@dataclass(frozen=True, slots=True)
class ReifierBindingRow:
    reifier: int
    triple: int
    graph: int | None = None


@dataclass(frozen=True, slots=True)
class AnnotationRow:
    count: int
    reifier: int
    predicate: int
    object: int
    graph: int | None = None


@dataclass(frozen=True, slots=True)
class EmptyGraphRow:
    graph: int


type Row = QuadRow | ReifierBindingRow | AnnotationRow | EmptyGraphRow
type _RowOrder = Literal["canonical", "unordered"]
type _TermIdentity = Callable[[int], Hashable]


@dataclass(frozen=True, slots=True)
class DecodedRows:
    frame_type: int
    rows: tuple[Row, ...]
    order: _RowOrder | None
    order_holds: bool | None


def _graph(decoder: _Decoder, registry_version: int) -> int | None:
    offset = decoder.offset
    size = decoder.array()
    if not size:
        raise ContentError("wrong_length", offset)
    tag = content_code(decoder, 2, registry_version)
    if size != tag:
        raise ContentError("wrong_length", offset)
    return decoder.uint() if tag == 2 else None


def _read_runs[T](decoder: _Decoder, read: Callable[[], T]) -> list[tuple[T, int]]:
    runs: list[tuple[T, int]] = []
    for _ in range(decoder.array()):
        decoder.record(2)
        offset = decoder.offset
        value = read()
        if runs and runs[-1][0] == value:
            raise ContentError("not_deterministic", offset)
        runs.append((value, decoder.uint(minimum=1)))
    return runs


def _deltas(decoder: _Decoder, *, minimum: int = 0) -> list[int]:
    values = []
    value = 0
    for _ in range(decoder.array()):
        offset = decoder.offset
        value += decoder.integer()
        if not minimum <= value <= UINT_MAX:
            raise ContentError("out_of_range", offset)
        values.append(value)
    return values


def _expand[T](runs: Iterable[tuple[T, int]]) -> Iterator[T]:
    return chain.from_iterable(repeat(value, count) for value, count in runs)


def _row_parts(row: Row) -> tuple[int, int | None, tuple[int, ...]]:
    match row:
        case QuadRow():
            return _QUADS, row.graph, (row.predicate, row.object, row.subject)
        case ReifierBindingRow():
            return _REIFIER_BINDINGS, row.graph, (row.reifier, row.triple)
        case AnnotationRow():
            return (
                _ANNOTATIONS,
                row.graph,
                (row.reifier, row.predicate, row.object, row.count),
            )
        case EmptyGraphRow():
            return _EMPTY_GRAPHS, None, (row.graph,)
    raise TypeError("expected a v2 graph row")


def _key(row: Row) -> tuple[int, ...]:
    frame_type, graph, columns = _row_parts(row)
    if frame_type == _EMPTY_GRAPHS:
        return columns
    return (
        -1 if graph is None else graph,
        *(columns[:-1] if frame_type == _ANNOTATIONS else columns),
    )


def _ascending(rows: Sequence[Row], earlier: Row | None) -> bool:
    if earlier is not None and _key(earlier) >= _key(rows[0]):
        return False
    return all(left < right for left, right in pairwise(map(_key, rows)))


def _identity(term_id: int) -> int:
    return term_id


def _distinct_annotations(rows: Iterable[Row], written_alike: _TermIdentity) -> bool:
    seen = set()
    for row in rows:
        if isinstance(row, AnnotationRow):
            graph = () if row.graph is None else (written_alike(row.graph),)
            key = (
                graph,
                written_alike(row.reifier),
                written_alike(row.predicate),
                written_alike(row.object),
            )
            if key in seen:
                return False
            seen.add(key)
    return True


def decode_rows(
    frame_type: int,
    content: bytes | memoryview,
    payload: bytes | memoryview,
    *,
    registry_version: int = 1,
    previous: DecodedRows | Row | None = None,
    written_alike: _TermIdentity = _identity,
) -> DecodedRows:
    """Check one row frame after its envelope, link and payload checks.

    Pass the nearest earlier decoded frame of this type or its last row when
    it declares canonical order. Map term IDs to equality keys for checking
    written-alike annotations. Undefined IDs and IDs from non-folded terms
    frames have distinct keys.
    """
    if frame_type not in _PAYLOAD_SIZES:
        raise ValueError("frame type is not a row frame")
    if registry_version < 1:
        raise ValueError("registry version must be positive")

    record = _Decoder(bytes(content), 0, ContentError)
    order: _RowOrder | None = None
    if frame_type == _EMPTY_GRAPHS:
        record.record(1)
    else:
        record.record(2)
        order = (
            "canonical"
            if content_code(record, 2, registry_version) == 1
            else "unordered"
        )
    count = record.uint(minimum=1)
    if record.offset != len(content):
        raise ContentError("wrong_length", record.offset)

    decoder = _Decoder(bytes(payload), 0, ContentError)
    decoder.record(_PAYLOAD_SIZES[frame_type])
    if frame_type == _EMPTY_GRAPHS:
        offset = decoder.offset
        graphs = _deltas(decoder)
        _end(decoder)
        if len(graphs) != count:
            raise ContentError("inconsistent_counts", offset)
        return DecodedRows(frame_type, tuple(map(EmptyGraphRow, graphs)), None, None)

    graphs_at = decoder.offset
    graph_runs = _read_runs(decoder, lambda: _graph(decoder, registry_version))
    predicates: list[tuple[int, int]] = []
    predicates_at = decoder.offset
    if frame_type == _QUADS:
        predicates = _read_runs(decoder, decoder.uint)
    columns: list[list[int]] = []
    column_offsets = []
    for index in range(4 if frame_type == _ANNOTATIONS else 2):
        column_offsets.append(decoder.offset)
        columns.append(_deltas(decoder, minimum=1 if index == 3 else 0))
    _end(decoder)

    if sum(length for _, length in graph_runs) != count:
        raise ContentError("inconsistent_counts", graphs_at)
    for column, offset in zip(columns, column_offsets, strict=True):
        if len(column) != count:
            raise ContentError("inconsistent_counts", offset)
    if frame_type == _QUADS and sum(length for _, length in predicates) != count:
        raise ContentError("inconsistent_counts", predicates_at)

    scopes = _expand(graph_runs)
    rows: tuple[Row, ...]
    if frame_type == _QUADS:
        rows = tuple(map(QuadRow, columns[1], _expand(predicates), columns[0], scopes))
    elif frame_type == _REIFIER_BINDINGS:
        rows = tuple(map(ReifierBindingRow, columns[0], columns[1], scopes))
    else:
        rows = tuple(
            map(AnnotationRow, columns[3], columns[0], columns[1], columns[2], scopes)
        )
        if not _distinct_annotations(rows, written_alike):
            raise ContentError("unordered_or_repeated", 0)
    earlier = _previous_row(frame_type, previous)
    holds = _ascending(rows, earlier) if order == "canonical" else None
    return DecodedRows(frame_type, rows, order, holds)


def _end(decoder: _Decoder) -> None:
    if decoder.offset != len(decoder.data):
        raise ContentError("wrong_length", decoder.offset)


def _previous_row(frame_type: int, previous: DecodedRows | Row | None) -> Row | None:
    if isinstance(previous, DecodedRows):
        if previous.frame_type == frame_type and previous.order == "canonical":
            return previous.rows[-1]
        return None
    return (
        previous
        if previous is not None and _row_parts(previous)[0] == frame_type
        else None
    )


def _runs[T](values: Iterable[T]) -> list[tuple[T, int]]:
    return [(value, sum(1 for _ in run)) for value, run in groupby(values)]


def _encode_deltas(values: tuple[int, ...]) -> list[int]:
    return [values[0], *(current - previous for previous, current in pairwise(values))]


def _uint(value: int) -> None:
    if type(value) is not int or not 0 <= value <= UINT_MAX:
        raise ValueError("row values must be uint values")


def encode_rows(
    rows: Sequence[Row],
    *,
    canonical: bool = False,
    previous: DecodedRows | None = None,
    written_alike: _TermIdentity = _identity,
) -> tuple[int, list[object], bytes]:
    """Return a frame type, content record and decoded payload for one row type."""
    if not rows:
        raise ValueError("a row frame requires at least one row")
    frame_type, _, _ = _row_parts(rows[0])
    graphs = []
    values = []
    for row in rows:
        kind, graph, row_values = _row_parts(row)
        if kind != frame_type:
            raise ValueError("a row frame cannot mix row types")
        if graph is not None:
            _uint(graph)
        for value in row_values:
            _uint(value)
        if kind == _ANNOTATIONS and row_values[-1] == 0:
            raise ValueError("an annotation count must be positive")
        graphs.append(graph)
        values.append(row_values)

    if frame_type == _ANNOTATIONS and not _distinct_annotations(rows, written_alike):
        raise ValueError("an annotation frame cannot repeat an annotation key")
    earlier = _previous_row(frame_type, previous)
    if (
        frame_type == _EMPTY_GRAPHS
        and previous is not None
        and previous.frame_type == frame_type
    ):
        earlier = previous.rows[-1]
    if canonical and not _ascending(rows, earlier):
        raise ValueError("canonical rows must be strictly ascending")

    columns = list(zip(*values, strict=True))
    if frame_type == _EMPTY_GRAPHS:
        return frame_type, [len(rows)], cbor2.dumps([_encode_deltas(columns[0])])
    payload: list[object] = [
        [[1] if graph is None else [2, graph], count] for graph, count in _runs(graphs)
    ]
    encoded: list[object] = [payload]
    if frame_type == _QUADS:
        encoded.append(_runs(columns.pop(0)))
    encoded.extend(map(_encode_deltas, columns))
    return frame_type, [1 if canonical else 2, len(rows)], cbor2.dumps(encoded)

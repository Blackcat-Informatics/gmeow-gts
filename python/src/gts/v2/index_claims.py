# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Build P5 index claims from the ordinary frames and their labelled fold."""

from collections import Counter, defaultdict
from dataclasses import dataclass
from itertools import groupby
from operator import attrgetter

import cbor2

from ._content import UINT_MAX
from .pack import MemberKind, decode_pack
from .rows import _PAYLOAD_SIZES, QuadRow, Row, decode_rows

_PACK = 13


@dataclass(frozen=True, slots=True)
class IndexStatistics:
    """Counts from the labelled set, excluding synthetic term nodes and records."""

    lines: int
    records: int
    blank_nodes: int

    def __post_init__(self) -> None:
        if any(
            type(count) is not int or not 0 <= count <= UINT_MAX
            for count in (self.lines, self.records, self.blank_nodes)
        ):
            raise ValueError("index statistics must be uint counts")


@dataclass(slots=True)
class _GraphRange:
    graph: int | None
    frame_type: int
    first_frame: int
    first_row: int
    count: int


class _IndexClaims:
    def __init__(self) -> None:
        self._position = 0
        self._types: list[tuple[int, int]] = []
        self._graphs: dict[int | None, list[_GraphRange]] = defaultdict(list)
        self._last_range: dict[int, _GraphRange] = {}
        self._predicates: Counter[int] = Counter()
        self._members: list[tuple[bytes, int, int]] = []

    def _add_rows(self, frame_type: int, rows: tuple[Row, ...]) -> None:
        first_row = 0
        for graph, group in groupby(rows, key=attrgetter("graph")):
            count = sum(1 for _ in group)
            last = self._last_range.get(frame_type)
            if last is not None and last.graph == graph:
                last.count += count
            else:
                entry = _GraphRange(graph, frame_type, self._position, first_row, count)
                self._graphs[graph].append(entry)
                self._last_range[frame_type] = entry
            first_row += count
        self._predicates.update(
            row.predicate for row in rows if isinstance(row, QuadRow)
        )

    def add(self, frame_type: int, content: list[object], payload: bytes) -> None:
        """Observe one encoded ordinary frame's content and decoded payload."""
        if frame_type in _PAYLOAD_SIZES:
            decoded = decode_rows(frame_type, cbor2.dumps(content), payload)
            self._add_rows(frame_type, decoded.rows)
        elif frame_type == _PACK:
            member_count, unit_length = content
            assert isinstance(member_count, int) and isinstance(unit_length, int)
            members = decode_pack(
                payload, member_count=member_count, unit_length=unit_length
            )
            self._members.extend(
                (member.digest, self._position, index)
                for index, member in enumerate(members)
                if member.kind in (MemberKind.CHUNK, MemberKind.BLOB)
            )
        if self._types and self._types[-1][0] == frame_type:
            self._types[-1] = frame_type, self._types[-1][1] + 1
        else:
            self._types.append((frame_type, 1))
        self._position += 1

    def payload(
        self, statistics: IndexStatistics, root: bytes, content_digest: bytes | None
    ) -> bytes:
        if content_digest is not None and (
            not isinstance(content_digest, bytes) or len(content_digest) != 32
        ):
            raise ValueError("index content digest must be a 32-byte digest")
        directory = [
            [
                [1] if graph is None else [2, graph],
                [
                    (entry.frame_type, entry.first_frame, entry.first_row, entry.count)
                    for entry in self._graphs[graph]
                ],
            ]
            for graph in sorted(
                self._graphs, key=lambda graph: -1 if graph is None else graph
            )
        ]
        return cbor2.dumps(
            [
                self._types,
                directory,
                [
                    statistics.lines,
                    statistics.records,
                    statistics.blank_nodes,
                    sorted(self._predicates.items()),
                ],
                root,
                [1] if content_digest is None else [2, content_digest],
                sorted(self._members),
            ]
        )

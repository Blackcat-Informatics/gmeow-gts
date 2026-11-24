# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Union resolved graph lines and sum annotations (P3 §5.1)."""

from dataclasses import dataclass

from ._content import UINT_MAX
from .dictionary import RowResolution
from .rows import AnnotationRow, EmptyGraphRow, QuadRow, ReifierBindingRow, Row


@dataclass(frozen=True, slots=True)
class LineResult:
    lines: frozenset[Row]
    held_rows: bool
    width_refused: bool


class LineFold:
    """Accumulate the lines of folded frames resolved in one term pool."""

    def __init__(self) -> None:
        self._lines: set[QuadRow | ReifierBindingRow] = set()
        self._empty: set[int] = set()
        self._named: set[int] = set()
        self._annotations: dict[tuple[int, int, int, int | None], int] = {}
        self._width_refused = False
        self._held_rows = False

    def add(self, result: RowResolution) -> None:
        """Fold resolved rows and remember any held row from their frame."""
        self._held_rows |= "opaque_term" in result.findings
        for row in result.rows:
            if isinstance(row, EmptyGraphRow):
                self._empty.add(row.graph)
                continue
            if row.graph is not None:
                self._named.add(row.graph)
            if isinstance(row, AnnotationRow):
                key = (row.reifier, row.predicate, row.object, row.graph)
                count = self._annotations.get(key, 0) + row.count
                self._width_refused |= count > UINT_MAX
                self._annotations[key] = min(count, UINT_MAX)
            else:
                self._lines.add(row)

    def result(self) -> LineResult:
        lines: set[Row] = set(self._lines)
        lines.update(EmptyGraphRow(graph) for graph in self._empty - self._named)
        lines.update(
            AnnotationRow(count, reifier, predicate, object_id, graph)
            for (
                reifier,
                predicate,
                object_id,
                graph,
            ), count in self._annotations.items()
        )
        return LineResult(frozenset(lines), self._held_rows, self._width_refused)

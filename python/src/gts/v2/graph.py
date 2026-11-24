# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Validate and fold term blocks and graph row frames in item order."""

from collections.abc import Mapping
from dataclasses import dataclass

from ._content import ContentError, UnknownContentCode
from .dictionary import (
    RowResolution,
    TermDictionary,
    TermPool,
    TermResolution,
    _written_term_key,
)
from .lines import LineFold, LineResult
from .rows import DecodedRows, Row, decode_rows
from .terms import (
    DecodedTerms,
    FormedBudget,
    FormedCapacityError,
    ValueReference,
    decode_terms,
    reserved_term_runs,
)


@dataclass(frozen=True, slots=True)
class TermReservation:
    runs: tuple[tuple[int, int], ...]


type GraphContribution = DecodedTerms | DecodedRows | TermReservation
type GraphContentError = ContentError | UnknownContentCode | FormedCapacityError


@dataclass(frozen=True, slots=True)
class GraphFrame:
    contribution: GraphContribution | None = None
    error: GraphContentError | None = None


class GraphContent:
    """Check one segment's graph payloads before rebuilding any values."""

    def __init__(self, budget: FormedBudget, *, registry_version: int = 1) -> None:
        self._budget = budget
        self._registry = registry_version
        self._next_id = 0
        self._by_id: dict[int, int] = {}
        self._written: dict[tuple[object, ...], int] = {}
        self._previous_rows: dict[int, Row] = {}

    @property
    def next_id(self) -> int:
        return self._next_id

    def _written_alike(self, term_id: int) -> tuple[bool, int]:
        key = self._by_id.get(term_id)
        return (False, term_id) if key is None else (True, key)

    def _reserve(self, content: bytes | memoryview | None) -> TermReservation:
        runs = (
            ()
            if content is None
            else reserved_term_runs(
                content, expected_base=self._next_id, registry_version=self._registry
            )
        )
        self._next_id += sum(length for _, length in runs)
        return TermReservation(runs)

    def read(
        self,
        frame_type: int,
        content: bytes | memoryview | None,
        decoded: bytes | memoryview | None,
    ) -> GraphFrame:
        """Process the nearest graph frame of its type, including opaque frames.

        Supply decoded bytes only after its envelope, link, and payload pass.
        When available, pass the raw content even for an opaque terms frame so
        its valid runs can reserve their IDs.
        """
        if not 1 <= frame_type <= 5:
            raise ValueError("frame is outside the graph row and term types")
        previous = self._previous_rows.pop(frame_type, None)
        if content is None or decoded is None:
            return GraphFrame(self._reserve(content) if frame_type == 1 else None)
        try:
            if frame_type != 1:
                rows = decode_rows(
                    frame_type,
                    content,
                    decoded,
                    registry_version=self._registry,
                    previous=previous,
                    written_alike=self._written_alike,
                )
                if rows.order == "canonical":
                    self._previous_rows[frame_type] = rows.rows[-1]
                return GraphFrame(rows)
            terms = decode_terms(
                content,
                decoded,
                expected_base=self._next_id,
                registry_version=self._registry,
                budget=self._budget,
            )
        except (ContentError, UnknownContentCode, FormedCapacityError) as error:
            return GraphFrame(
                self._reserve(content) if frame_type == 1 else None, error
            )

        for term in terms.terms:
            key = _written_term_key(term, self._written_alike)
            self._by_id[self._next_id] = self._written.setdefault(
                key, len(self._written)
            )
            self._next_id += 1
        return GraphFrame(terms)


class GraphFold:
    """Resolve validated terms and rows, applying the segment's dictionary rules."""

    def __init__(self, pool: TermPool, segment: int, lines: LineFold) -> None:
        self._dictionary = TermDictionary(pool, segment)
        self._lines = lines

    @property
    def dictionary(self) -> TermDictionary:
        """The segment's resolved term dictionary."""
        return self._dictionary

    def add(
        self,
        frame: GraphFrame,
        *,
        values: Mapping[ValueReference, bytes | None] | None = None,
        in_doubt: bool = False,
        hidden_before: int = 0,
    ) -> TermResolution | RowResolution | None:
        """Apply a frame after preparing its preceding value declarations."""
        match frame.contribution:
            case TermReservation(runs):
                self._dictionary.reserve(runs)
            case DecodedTerms() as terms:
                return self._dictionary.add(terms, values=values, in_doubt=in_doubt)
            case DecodedRows() as rows:
                result = self._dictionary.resolve_rows(
                    rows,
                    hidden_before=hidden_before,
                    in_doubt=in_doubt,
                )
                self._lines.add(result)
                return result
            case None:
                pass
        return None

    def result(self) -> LineResult:
        return self._lines.result()

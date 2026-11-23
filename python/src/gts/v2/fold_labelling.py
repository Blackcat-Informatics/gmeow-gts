# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Build and label the RDF and record set of a P3 fold."""

from collections.abc import Iterable, Iterator, Mapping
from dataclasses import dataclass
from typing import Literal

from ._fold_record_lines import record_bodies, record_line, record_weight
from ._fold_terms import FoldTerms, TermNodeCollision
from ._labelling_result import LabellingResult, label
from .blobs import BlobSet
from .dictionary import TermPool
from .graph import GraphFrame
from .labelling import LabellingInput, LabellingRefused, Line, RecordLine, _slots
from .lines import LineResult
from .record_fold import RecordFold, RecordFrame, _leaves
from .records import TripleLeaf
from .rows import (
    AnnotationRow,
    DecodedRows,
    EmptyGraphRow,
    QuadRow,
    ReifierBindingRow,
    Row,
)
from .terms import DecodedTerms, TripleTerm, TypedValue, TypedValueTerm

type RefusalReason = Literal[
    "refusal_size",
    "refusal_weight",
    "refusal_term_node",
    "refusal_width",
    "refusal_limit",
]


class FoldLabellingRefused(ValueError):
    """The reason of a P3 labelling refusal."""

    def __init__(self, reason: RefusalReason) -> None:
        self.reason = reason
        super().__init__(reason)


@dataclass(frozen=True, slots=True)
class LabelledFold:
    labelling: LabellingResult
    content_digest: bytes | None
    blank_positions: Mapping[int, int]
    term_node_positions: Mapping[int, int]


def folded_size(
    graphs: Iterable[tuple[int, Iterable[GraphFrame]]],
    record_frames: Iterable[RecordFrame],
    blob_declarations: int,
) -> int:
    """Count the folded-frame contributions before resolving terms and rows."""
    size = blob_declarations
    triples: dict[int, dict[int, int]] = {}
    for segment, frames in graphs:
        reserved = triples.setdefault(segment, {})
        for frame in frames:
            match frame.contribution:
                case DecodedRows(rows=rows):
                    size += len(rows)
                case DecodedTerms(id_base=base, terms=terms):
                    for offset, term in enumerate(terms):
                        if isinstance(term, TypedValueTerm):
                            size += 1
                        elif isinstance(term, TripleTerm):
                            reserved[base + offset] = term.object
    for reserved in triples.values():
        size += sum(object_id in reserved for object_id in reserved.values())
    for record_frame in record_frames:
        size += len(record_frame.entries)
        reserved = triples.get(record_frame.location.segment, {})
        for entry in record_frame.entries:
            for leaf in _leaves(entry):
                # Every non-root item in the flattened leaf is a child of a triple.
                size += sum(
                    isinstance(part, TripleLeaf)
                    or isinstance(part, int)
                    and part in reserved
                    for part in leaf[1:]
                )
    return size


def _lower_row(row: Row, terms: FoldTerms) -> Row:
    if isinstance(row, EmptyGraphRow):
        return EmptyGraphRow(terms.lowered(row.graph))
    term = terms.lowered
    graph = None if row.graph is None else term(row.graph)
    match row:
        case QuadRow(subject, predicate, object_id):
            return QuadRow(term(subject), term(predicate), term(object_id), graph)
        case ReifierBindingRow(reifier, triple):
            return ReifierBindingRow(term(reifier), term(triple), graph)
        case AnnotationRow(count, reifier, predicate, object_id):
            return AnnotationRow(
                count, term(reifier), term(predicate), term(object_id), graph
            )


def _fold_lines(
    lines: LineResult, records: Iterable[RecordLine], terms: FoldTerms
) -> Iterator[Line]:
    yield from (_lower_row(row, terms) for row in lines.lines)
    yield from set(records)


class FoldLabellingBudget:
    """Keep P3 size and total labelled weight limits across the folds of one read."""

    def __init__(
        self, *, labelled_lines: int, labelled_bytes_per_read: int, labelling_units: int
    ) -> None:
        self.labelled_lines = labelled_lines
        self.labelled_bytes_per_read = labelled_bytes_per_read
        self.labelling_units = labelling_units
        self.used_weight = 0
        self._weight_refused = False

    def check_size(self, folded_size: int) -> None:
        """Use the folded-frame counts before forming the union of resolved lines."""
        if folded_size > self.labelled_lines:
            raise FoldLabellingRefused("refusal_size")

    def label_fold(
        self,
        pool: TermPool,
        lines: LineResult,
        records: RecordFold,
        blobs: BlobSet,
        values: Iterable[tuple[int, TypedValue]],
        *,
        folded_size: int,
    ) -> LabelledFold:
        """Weigh one resolved fold and label all of its content in a single run."""
        self.check_size(folded_size)
        if self._weight_refused:
            raise FoldLabellingRefused("refusal_weight")

        terms = FoldTerms(pool)
        bodies = tuple(record_bodies(records, blobs, values))
        weight = 0
        for row in lines.lines:
            slots = _slots(row)
            assert slots is not None
            weight += slots.header_length() + sum(
                terms.weight(ref) for ref in slots.ordered_terms()
            )
        weight += sum(
            record_weight(body, records.ground_terms, terms) for body in bodies
        )
        weight += terms.nested_weight
        if self.used_weight + weight > self.labelled_bytes_per_read:
            self._weight_refused = True
            raise FoldLabellingRefused("refusal_weight")
        self.used_weight += weight

        try:
            lowered = terms.lower()
        except TermNodeCollision as exc:
            raise FoldLabellingRefused("refusal_term_node") from exc
        if lines.width_refused:
            raise FoldLabellingRefused("refusal_width")

        def record_lines() -> Iterator[RecordLine]:
            for body in bodies:
                yield record_line(body, records.ground_terms, terms)
            yield from terms.records()

        try:
            source = LabellingInput(lowered, _fold_lines(lines, record_lines(), terms))
            result = label(source, work_limit=self.labelling_units)
        except LabellingRefused as exc:
            if exc.kind == "width":
                raise FoldLabellingRefused("refusal_width") from exc
            if exc.kind == "limit":
                raise FoldLabellingRefused("refusal_limit") from exc
            raise
        missing = lines.held_rows or records.held_records
        return LabelledFold(
            result,
            None if missing else result.sequence.digest,
            {
                ref: result.labels[name].position
                for ref, name in terms.blank_sources.items()
            },
            {
                ref: result.labels[name].position
                for ref, name in terms.node_sources.items()
            },
        )

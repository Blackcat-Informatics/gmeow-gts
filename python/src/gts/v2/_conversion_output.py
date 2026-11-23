# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Prepare the canonical candidate for the independent P9 result read."""

from collections.abc import Callable
from dataclasses import dataclass

from ._content import UINT_MAX
from .canonical_graph import dictionary_terms
from .compaction import CompactionSource, FileEnd, ReadCoverage, Stopped, compact
from .conversion_fold import ConversionStop, FoldedSource, fold_source
from .conversion_source import SourceCapacity
from .dictionary import Triple
from .fold_labelling import FoldLabellingBudget, FoldLabellingRefused
from .frame import decode_frame
from .items import classify_item


@dataclass(frozen=True, slots=True)
class ConversionCapacity(SourceCapacity):
    labelled_lines: int = 1 << 26
    labelled_bytes_per_read: int = 1 << 37

    def __post_init__(self) -> None:
        SourceCapacity.__post_init__(self)
        for limit in (self.labelled_lines, self.labelled_bytes_per_read):
            if type(limit) is not int or not 0 <= limit <= UINT_MAX:
                raise ValueError("conversion capacity limits must be uint64 values")


_DEFAULT_CAPACITY = ConversionCapacity()


@dataclass(frozen=True, slots=True)
class ConversionCandidate:
    file: bytes
    expected_digest: bytes
    source: FoldedSource


def _canonical_folded_size(source: FoldedSource) -> int:
    """Count P3 folded-frame contributions before assigning dictionary IDs."""
    reachable = dictionary_terms(
        source.terms,
        source.lines.lines,
        source.records,
        (ref for ref, _ in source.values),
    )
    nested = sum(
        isinstance(term, Triple) and isinstance(reachable[term.object], Triple)
        for term in reachable.values()
    )
    # P9's record terms have dictionary IDs. A held blob has one declaration,
    # and each typed value appears once in the canonical term dictionary.
    return (
        len(source.lines.lines)
        + len(source.records.bindings)
        + len(source.records.suppression)
        + len(source.blobs.entries)
        + len(source.values)
        + nested
    )


def _frame_limit_exceeded(data: bytes, capacity: ConversionCapacity) -> bool:
    """Check the stored content, payload, and decoded length of written frames."""
    offset = 0
    while offset < len(data):
        item = classify_item(data, offset)
        if item.kind == "frame":
            frame = decode_frame(data, offset)
            if (
                len(frame.content) > capacity.stored_bytes_per_frame
                or len(frame.stored_payload) > capacity.stored_bytes_per_frame
                or frame.decoded_length > capacity.decoded_bytes_per_frame
            ):
                return True
        offset = item.end
    return False


def prepare_conversion(
    read: Callable[[], bytes], capacity: ConversionCapacity = _DEFAULT_CAPACITY
) -> ConversionCandidate | ConversionStop:
    """Read, label and compact a legacy file for P9 result verification."""
    source = fold_source(read, capacity)
    if isinstance(source, ConversionStop):
        return source
    labeller = FoldLabellingBudget(
        labelled_lines=capacity.labelled_lines,
        labelled_bytes_per_read=capacity.labelled_bytes_per_read,
        labelling_units=UINT_MAX,
    )
    try:
        labelled = labeller.label_fold(
            source.terms,
            source.lines,
            source.records,
            source.blobs,
            source.values,
            folded_size=_canonical_folded_size(source),
        )
    except FoldLabellingRefused:
        return ConversionStop("source_exceeds_capacity", 0)
    outcome = compact(
        CompactionSource(
            terms=source.terms,
            lines=source.lines,
            records=source.records,
            values=source.values,
            blobs=source.blobs,
            declarations=source.declarations,
            rebuilds={},
            coverage=ReadCoverage(FileEnd.NO_COMMIT, 0),
            content_digest=labelled.content_digest,
            blank_positions=labelled.blank_positions,
        ),
    ).outcome
    if isinstance(outcome, Stopped):
        if outcome.code == "labelling_refused":
            return ConversionStop("source_exceeds_capacity", 0)
        return ConversionStop("content_mismatch", 0)
    if _frame_limit_exceeded(outcome.file, capacity):
        return ConversionStop("source_exceeds_capacity", 0)
    assert labelled.content_digest is not None
    return ConversionCandidate(outcome.file, labelled.content_digest, source)

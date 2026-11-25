# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""The typed file result of a read of a GTS v2 input (P7 1.1 and 1.2)."""

from dataclasses import dataclass
from typing import Literal

from .content import ContentCapacity, ContentFinding, ContentRead
from .findings import (
    FindingRecord,
    finding_record,
    frame_findings,
    framing_findings,
    segment_findings,
)
from .segment_result import FrameResult, SegmentResult, frame_results, segment_results

type FileEnd = Literal[
    "not_read",
    "undelimitable",
    "torn",
    "no_commit",
    "after_commit",
    "at_authenticated_commit",
    "at_valid_commit",
]


@dataclass(frozen=True, slots=True)
class CompleteVisibility:
    """The `visible` variant `complete`, carrying its unauthenticated suppress count."""

    unauthenticated_suppress: int


@dataclass(frozen=True, slots=True)
class IncompleteVisibility:
    """The `visible` variant `incomplete`."""

    unfolded_suppress: int
    unopened_sealed: int
    unauthenticated_suppress: int


@dataclass(frozen=True, slots=True)
class FrameRecord:
    """A `frame_result` with the findings that stand in its list."""

    result: FrameResult
    findings: tuple[FindingRecord, ...]


@dataclass(frozen=True, slots=True)
class SegmentRecord:
    """A `segment_result` with its frames and the findings that stand in its list."""

    result: SegmentResult
    frames: tuple[FrameRecord, ...]
    findings: tuple[FindingRecord, ...]


@dataclass(frozen=True, slots=True)
class FileResult:
    """The P7 `file_result` (tag `2`) of a second-generation read.

    `content_digest` is `None` when it is absent. The `rebuilds` of each frame, the
    subject of a finding that carries one other than a signature algorithm, and the
    P7 sort order of each list of findings are not yet stated.
    """

    segments: tuple[SegmentRecord, ...]
    findings: tuple[FindingRecord, ...]
    content_digest: bytes | None
    visible: CompleteVisibility | IncompleteVisibility
    end: FileEnd
    left_out: int


_FRAMING_CODES = frozenset(("torn_tail", "undelimitable_input"))


def _no_segment_end(findings: tuple[ContentFinding, ...], input_size: int) -> FileEnd:
    """Return the first `end` of P7 1.2 that holds, for a file with no segment."""
    if any(
        finding.code == "read_failed"
        or (finding.code == "exceeds_capacity" and finding.reason == "item_count")
        for finding in findings
    ):
        return "not_read"
    if _runs_to_end(findings, "undelimitable_input", input_size):
        return "undelimitable"
    if _runs_to_end(findings, "torn_tail", input_size):
        return "torn"
    # A file with no segment has no last segment, so no commit holds.
    return "no_commit"


def _runs_to_end(
    findings: tuple[ContentFinding, ...], code: str, input_size: int
) -> bool:
    return any(
        finding.code == code
        and finding.offset is not None
        and finding.length is not None
        and finding.offset + finding.length == input_size
        for finding in findings
    )


def file_result(
    read: ContentRead, data: bytes, capacity: ContentCapacity
) -> FileResult:
    """Return the P7 `file_result` of a read of `data` made with `capacity`.

    Raise ValueError for a first-generation file, which keeps its legacy result.
    """
    if read.wire.opening.decision == "first_generation":
        raise ValueError(
            "a first-generation file keeps its legacy result, which is not built here"
        )
    if not read.wire.segments:
        if any(finding.segment is not None for finding in read.findings):
            raise ValueError("a file with no segment carries a segment finding")
        return FileResult(
            segments=(),
            findings=tuple(finding_record(finding) for finding in read.findings),
            content_digest=None,
            visible=CompleteVisibility(0),
            end=_no_segment_end(read.findings, len(data)),
            # Every torn tail is left out of the fold (P7, 1.2); a file with no
            # segment has no frame, item or damaged region to count.
            left_out=sum(1 for finding in read.findings if finding.code == "torn_tail"),
        )
    coverage = read.coverage
    if coverage is None:
        raise ValueError("a second-generation read states no coverage")
    openers = segment_results(read, data, capacity)
    segments = tuple(
        SegmentRecord(
            opener,
            tuple(
                FrameRecord(
                    frame_result,
                    frame_findings(read, ordinal, wire),
                )
                for frame_result, wire in zip(
                    frame_results(read, ordinal),
                    read.wire.segments[ordinal].frames,
                    strict=True,
                )
            ),
            segment_findings(read, ordinal),
        )
        for ordinal, opener in enumerate(openers)
    )
    visibility = read.visibility
    visible: CompleteVisibility | IncompleteVisibility = (
        CompleteVisibility(0)
        if visibility is None
        else (
            CompleteVisibility(visibility.unauthenticated_suppress)
            if visibility.complete
            else IncompleteVisibility(
                visibility.unfolded_suppress,
                visibility.unopened_sealed,
                visibility.unauthenticated_suppress,
            )
        )
    )
    return FileResult(
        segments=segments,
        findings=framing_findings(read)
        + tuple(
            finding_record(finding, read)
            for finding in read.findings
            if finding.segment is None and finding.code not in _FRAMING_CODES
        ),
        content_digest=None if read.fold is None else read.fold.content_digest,
        visible=visible,
        end=coverage.end.name.lower(),  # type: ignore[arg-type]
        left_out=coverage.left_out,
    )

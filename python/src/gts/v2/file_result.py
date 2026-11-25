# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed frame states of the GTS v2 file result (P7 section 1.1, P2 enumerations)."""

from dataclasses import dataclass
from typing import Literal

from .content import ContentFinding, ContentRead, OpaqueReason
from .payload import PayloadCheck
from .reader import WireFrame


@dataclass(frozen=True, slots=True)
class UncheckedPayload:
    """The `payload_state` variant `unchecked`, carrying its P2 reason."""

    reason: Literal["unknown_codec", "exceeds_capacity"]


type PayloadState = Literal["verified", "damaged"] | UncheckedPayload


@dataclass(frozen=True, slots=True)
class OpaqueFold:
    """The `fold_state` variant `opaque`, carrying its P2 reason."""

    reason: OpaqueReason


type FoldState = Literal["folded", "not_content"] | OpaqueFold

# Frame types that are not content unless opaque: commit (14), index (15), parity (16).
_NOT_CONTENT_TYPES = frozenset((14, 15, 16))

# The three size reasons of a payload check are the `exceeds_capacity` reason of
# the typed result (P2-28, and the R7 findings of P7).
_CAPACITY_REASONS = frozenset(("stored_size", "decoded_size", "total_decoded"))


def payload_state(check: PayloadCheck) -> PayloadState:
    """Return the P7 `payload_state` of a payload check.

    Raise ValueError when an unchecked payload carries no reason the P2
    enumeration admits, since no state can then be stated.
    """
    match check.state:
        case "verified" | "damaged":
            return check.state
        case "unchecked":
            if check.reason == "unknown_codec":
                return UncheckedPayload("unknown_codec")
            if check.reason in _CAPACITY_REASONS:
                return UncheckedPayload("exceeds_capacity")
            raise ValueError("an unchecked payload carries no P2 reason")


def fold_state(read: ContentRead, segment: int, frame: WireFrame) -> FoldState:
    """Return the P7 `fold_state` of a frame of a segment of a read.

    A frame with an opaque reason is `opaque` with that reason. Otherwise a
    commit, index or parity frame is `not_content` and every other frame is
    `folded` (P7 section 1.3).
    """
    reason = read.opaque_reasons.get((segment, frame.position))
    if reason is not None:
        return OpaqueFold(reason)
    if frame.frame_type in _NOT_CONTENT_TYPES:
        return "not_content"
    return "folded"


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
class NoSegmentFileResult:
    """The P7 `file_result` (tag `2`) of a file that holds no segment.

    `content_digest` is absent, since a file with no segment has none (P7, 1.2).
    """

    findings: tuple[ContentFinding, ...]
    visible: CompleteVisibility
    end: FileEnd
    left_out: int


def _file_end(findings: tuple[ContentFinding, ...], input_size: int) -> FileEnd:
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


def file_result(read: ContentRead, input_size: int) -> NoSegmentFileResult:
    """Return the P7 `file_result` of a read of an input of `input_size` bytes.

    Raise ValueError when the read holds a segment: the `segment_result` records
    of P7 1.1 are not yet built, so no complete file result can be stated.
    """
    if read.wire.opening.decision == "first_generation":
        raise ValueError(
            "a first-generation file keeps its legacy result, which is not built here"
        )
    if read.wire.segments:
        raise ValueError("the file result of a file with segments is not yet built")
    if any(finding.segment is not None for finding in read.findings):
        raise ValueError("a file with no segment carries a segment finding")
    return NoSegmentFileResult(
        findings=read.findings,
        visible=CompleteVisibility(0),
        end=_file_end(read.findings, input_size),
        # Every torn tail is left out of the fold (P7, 1.2); a file with no segment
        # has no frame, item or damaged region to count.
        left_out=sum(1 for finding in read.findings if finding.code == "torn_tail"),
    )

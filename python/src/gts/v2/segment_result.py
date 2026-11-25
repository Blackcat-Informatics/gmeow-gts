# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed opener of each segment of the GTS v2 read result (P7 1.1, P1 2.4, 2.4a)."""

from dataclasses import dataclass
from typing import Literal

from ._decode import WireError
from .content import ContentCapacity, ContentRead
from .reader import WireSegment
from .rebuild import unit_digest
from .resync import DecodedResync

type OpenerOutcome = Literal["readable", "unreadable", "links_unavailable"]
type PastCheck = Literal["absent", "not_checked", "matches", "differs"]


@dataclass(frozen=True, slots=True)
class HeaderOpener:
    """The `opener` variant `[1]`: a header."""


@dataclass(frozen=True, slots=True)
class ResyncOpener:
    """The `opener` variant `[2, reached, displaced, past, marks_ignored]`."""

    reached: Literal["walked", "scanned"]
    displaced: bool
    past: PastCheck
    marks_ignored: int


@dataclass(frozen=True, slots=True)
class SegmentResult:
    """The opener fields of a P7 `segment_result`; `None` stands for `absent`."""

    ordinal: int
    offset: int
    length: int
    opener: HeaderOpener | ResyncOpener
    header_identifier: bytes | None
    registry_version: int | None
    hash_algorithm: int | None
    profile: int | None


def opener_outcome(segment: WireSegment) -> OpenerOutcome:
    """Return the opener outcome of a segment (P1 2.4, Header outcome)."""
    value = segment.opener.value
    if isinstance(value, WireError):
        return "unreadable"
    if value.identifier is None:
        return "links_unavailable"
    return "readable"


class _PastCount:
    """The bytes hashed for past checks, kept apart from every total of the read."""

    def __init__(self, limit: int) -> None:
        self._limit = limit
        self._total = 0
        self._reach: dict[int, int] = {}
        self._exhausted = False

    def admit(self, start: int, place: int) -> bool:
        """Charge the range `[start, place)`; return False once the limit is passed."""
        if self._exhausted:
            return False
        charge = place - max(self._reach.get(start, start), start)
        if self._total + charge > self._limit:
            self._exhausted = True
            return False
        self._total += charge
        self._reach[start] = max(self._reach.get(start, start), place)
        return True


def _stated_offset(segment: WireSegment) -> int | None:
    value = segment.opener.value
    if isinstance(value, WireError):
        return getattr(value, "stated_offset", None)
    return value.offset if isinstance(value, DecodedResync) else None


def _past(
    segment: WireSegment, outcome: OpenerOutcome, data: bytes, count: _PastCount
) -> PastCheck:
    value = segment.opener.value
    if outcome == "unreadable" or not isinstance(value, DecodedResync):
        return "absent"
    if value.past is None:
        return "absent"
    place = segment.opener.item.offset
    if outcome == "links_unavailable" or place < value.offset:
        return "not_checked"
    start = place - value.offset
    if not count.admit(start, place):
        return "not_checked"
    return "matches" if unit_digest(data[start:place]) == value.past else "differs"


def _opener(
    segment: WireSegment, outcome: OpenerOutcome, data: bytes, count: _PastCount
) -> HeaderOpener | ResyncOpener:
    if segment.opener.kind == "header":
        return HeaderOpener()
    value = segment.opener.value
    stated = _stated_offset(segment)
    if isinstance(value, DecodedResync):
        displaced = value.displaced
        marks_ignored = value.marks_ignored
    else:
        # An item unreadable before its `offset` field is displaced (P7, 1.2).
        displaced = stated != segment.opener.item.offset
        marks_ignored = 0
    return ResyncOpener(
        "scanned" if segment.opener.scanned else "walked",
        displaced,
        _past(segment, outcome, data, count),
        marks_ignored,
    )


def segment_results(
    read: ContentRead, data: bytes, capacity: ContentCapacity
) -> tuple[SegmentResult, ...]:
    """Return the opener fields of each segment of a read of `data`, in file order."""
    count = _PastCount(capacity.decoded_bytes_per_read)
    results: list[SegmentResult] = []
    for ordinal, segment in enumerate(read.wire.segments):
        outcome = opener_outcome(segment)
        value = segment.opener.value
        header = None if isinstance(value, WireError) else value.header
        results.append(
            SegmentResult(
                ordinal,
                segment.opener.item.offset,
                segment.end_offset - segment.opener.item.offset,
                _opener(segment, outcome, data, count),
                None if isinstance(value, WireError) else value.identifier,
                None if header is None else header.registry_version,
                None if header is None else header.hash_algorithm,
                None if header is None else header.profile,
            )
        )
    return tuple(results)

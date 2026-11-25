# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed opener of each segment of the GTS v2 read result (P7 1.1, P1 2.4, 2.4a)."""

from dataclasses import dataclass
from typing import Literal

from ._decode import WireError
from .authentication import Authenticity
from .content import ContentCapacity, ContentRead
from .frame import FrameError
from .items import DamagedRegion, Item
from .reader import WireSegment
from .rebuild import unit_digest
from .resync import DecodedResync

type OpenerOutcome = Literal["readable", "unreadable", "links_unavailable"]
type IntegrityState = Literal["intact", "intact_unchecked", "damaged", "not_checked"]
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
    integrity: IntegrityState
    authenticity: Authenticity
    anchored_frames: int
    committed_frames: int
    authenticated_frames: int
    transparent_frames: tuple[int, ...]


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


# A closing item that fails its claim is not malformed; one that breaks its grammar is.
_CLAIM_REASONS = frozenset(("index_offsets", "index_locator"))
# The refusals by the total rebuilt and total formed lengths (P4, P3).
_REFUSED_TOTALS = frozenset(("total_rebuilt", "total_formed"))
_PARITY = 16
_DAMAGE_FINDINGS = frozenset(("malformed_value", "damaged_frame"))


def _damaged_item(item: Item | DamagedRegion) -> bool:
    """Return whether an item is a damaged region or an opaque item with a defect."""
    return isinstance(item, DamagedRegion) or (
        item.kind == "opaque" and item.reason != "unknown_item_kind"
    )


def _integrity(
    read: ContentRead, ordinal: int, segment: WireSegment, outcome: OpenerOutcome
) -> IntegrityState:
    """Return the P7 `integrity` of a segment (P7 1.3, INTEGRITY)."""
    if outcome == "links_unavailable":
        return "not_checked"
    if outcome == "unreadable":
        return "damaged"
    ordinary = [frame for frame in segment.frames if not frame.transparent]
    transparent = {frame.position for frame in segment.frames if frame.transparent}
    if (
        any(_damaged_item(item) for item in segment.items)
        or any(note.reason not in _CLAIM_REASONS for note in segment.closing)
        or any(
            isinstance(frame.envelope, FrameError) and frame.frame_type != _PARITY
            for frame in ordinary
        )
        or any(
            frame.link.state not in ("linked", "healed")
            or frame.payload.state == "damaged"
            for frame in ordinary
        )
        or any(
            finding.segment == ordinal
            and finding.code in _DAMAGE_FINDINGS
            and finding.position not in transparent
            for finding in read.findings
        )
    ):
        return "damaged"
    if any(frame.payload.state == "unchecked" for frame in ordinary) or any(
        finding.segment == ordinal
        and finding.code == "exceeds_capacity"
        and finding.reason in _REFUSED_TOTALS
        for finding in read.findings
    ):
        return "intact_unchecked"
    return "intact"


def _anchored(segment: WireSegment) -> int:
    """Count the ordinary frames before the first detached or unlinked one."""
    count = 0
    for frame in segment.frames:
        if frame.transparent:
            continue
        if frame.detached or frame.link.state not in ("linked", "healed"):
            break
        count += 1
    return count


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
        readable = outcome == "readable"
        last_commit = read.last_commits.get(ordinal, 0)
        authentication = read.authentication[ordinal]
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
                _integrity(read, ordinal, segment, outcome),
                authentication.authenticity,
                _anchored(segment) if readable else 0,
                sum(
                    1
                    for frame in segment.frames
                    if not frame.transparent and frame.position <= last_commit
                )
                if readable
                else 0,
                authentication.authenticated_frames if readable else 0,
                tuple(frame.position for frame in segment.frames if frame.transparent)
                if readable
                else (),
            )
        )
    return tuple(results)

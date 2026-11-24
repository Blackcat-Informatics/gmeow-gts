# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Whole-input item walk for the GTS v2 reader."""

from collections import deque
from dataclasses import InitVar, dataclass, field, replace
from typing import Literal

from ._decode import WireError, _Decoder
from .frame import _PARITY, Frame, FrameError, _decode_frame_at, _frame_field_offsets
from .framing import FramingReason, ItemFramingError, _generation_field, _head, item_end
from .header import DecodedHeader, decode_header
from .items import (
    ClosingFinding,
    DamagedRegion,
    Item,
    _check_closing_grammar,
    _classify_item_at,
    check_closing_items,
)
from .link import FrameLink, _inspect_frame_link, check_frame_link
from .parity import ParityCandidate, ParityRefusal, RecoveryResult, _ParityScanner
from .payload import PayloadBudget, PayloadCheck
from .resync import (
    RESYNC_MAX,
    DecodedResync,
    ResyncRefusal,
    ResyncScanner,
    decode_resync,
)

type _OpenerKind = Literal["header", "resync"]
type _Dispatch = (
    FramingReason
    | Literal[
        "empty_input",
        "first_generation",
        "unsupported_generation",
        "not_this_format",
        "item_count",
    ]
)


@dataclass(frozen=True, slots=True)
class _Boundary:
    offset: int
    end: int
    failure: FramingReason | None = None


@dataclass(frozen=True, slots=True)
class _Opener:
    kind: _OpenerKind
    item: _Boundary
    value: DecodedHeader | DecodedResync | WireError
    scanned: bool = False


@dataclass(frozen=True, slots=True)
class _Opening:
    decision: _Opener | _Dispatch
    damaged_prefix: _Boundary | None = None
    refusal: ResyncRefusal | None = None


@dataclass(frozen=True, slots=True)
class WireFrame:
    """Envelope, link and payload observations for a frame in a readable segment."""

    position: int
    frame_type: int | None
    transparent: bool
    envelope: Frame | FrameError
    link: FrameLink
    payload: PayloadCheck
    detached: bool = False


@dataclass(frozen=True, slots=True)
class WireSegment:
    """Items of a segment and the results of the wire-level checks."""

    opener: _Opener
    end_offset: int
    head: bytes | None
    items: tuple[Item | DamagedRegion, ...]
    frames: tuple[WireFrame, ...]
    closing: tuple[ClosingFinding, ...]
    framing_failure: _Boundary | None
    resync_refusals: tuple[ResyncRefusal, ...]
    parity_refusals: tuple[ParityRefusal, ...]


@dataclass(frozen=True, slots=True)
class WireRead:
    """Generation dispatch and item observations for a whole byte input."""

    opening: _Opening
    segments: tuple[WireSegment, ...] = ()
    item_limit_offset: int | None = None


@dataclass(slots=True)
class _Segment:
    opener: _Opener
    data: InitVar[bytes]
    budget: InitVar[PayloadBudget]
    head: bytes | None = field(init=False)
    predecessor: bytes | None = field(init=False)
    registry_version: int = field(init=False)
    parity: _ParityScanner | None = field(init=False)
    items: list[Item | DamagedRegion] = field(init=False)
    frames: list[WireFrame] = field(default_factory=list)
    refusals: list[ResyncRefusal] = field(default_factory=list)
    parity_refusals: list[ParityRefusal] = field(default_factory=list)
    pending: tuple[WireFrame, FrameLink] | None = None

    def __post_init__(self, data: bytes, budget: PayloadBudget) -> None:
        opener = self.opener
        self.items = [Item(opener.kind, opener.item.offset, opener.item.end)]
        self.head = None
        self.registry_version = 2
        if not isinstance(opener.value, WireError):
            self.head = opener.value.identifier
            if self.head is not None:
                # Unknown kinds fail framing only under a readable version 1 opener.
                self.registry_version = opener.value.header.registry_version
        self.predecessor = self.head
        self.parity = (
            None if self.head is None else _ParityScanner(data, self.head, budget)
        )

    def _save_frame(self, frame: WireFrame) -> None:
        self.frames.append(frame)
        if (
            self.parity is not None
            and frame.frame_type == _PARITY
            and frame.link.state != "broken"
        ):
            self.parity.remember(
                self.items[frame.position].offset,
                frame.envelope,
                newer=self.registry_version > 1,
            )

    def add_frame(self, frame: WireFrame, without_witness: FrameLink) -> None:
        if frame.link.effective_identifier is not None:
            self.head = frame.link.effective_identifier
            self.predecessor = self.head
        if frame.link.state == "healed":
            self.pending = frame, without_witness
        else:
            self._save_frame(frame)

    def resume(self, boundary: _Boundary, candidate: ParityCandidate) -> _Boundary:
        self.items.append(DamagedRegion(boundary.offset, candidate.offset))
        self.predecessor = candidate.anchor
        frame = candidate.frame
        item = Item("frame", candidate.offset, frame.next_offset)
        position = len(self.items)
        self.items.append(item)
        self._save_frame(
            WireFrame(
                position,
                _PARITY,
                True,
                frame,
                FrameLink("linked", frame.identifier, None),
                PayloadCheck("verified", None, decoded=frame.stored_payload),
            )
        )
        return _Boundary(item.offset, item.end)

    def settle_witness(self, kept: bool) -> None:
        if self.pending is None:
            return
        frame, without_witness = self.pending
        self.pending = None
        if not kept:
            frame = replace(frame, link=without_witness)
            self.head = without_witness.effective_identifier
            self.predecessor = self.head
        self._save_frame(frame)

    def finish(
        self, data: bytes, end: int, failure: _Boundary | None = None
    ) -> WireSegment:
        closing = (
            () if self.head is None else tuple(check_closing_items(data, self.items))
        )
        return WireSegment(
            self.opener,
            end,
            self.head,
            tuple(self.items),
            tuple(self.frames),
            closing,
            failure,
            tuple(self.refusals),
            tuple(self.parity_refusals),
        )


class _ItemWalk:
    """Delimit and count items, holding look-ahead without judging it."""

    def __init__(self, data: bytes, items_per_read: int) -> None:
        self._input = memoryview(data)
        self._limit = items_per_read
        self._count = 0
        self._offset = 0
        self._held: deque[_Boundary] = deque()

    @property
    def stopped_at(self) -> int | None:
        if self._count == self._limit and self._offset < len(self._input):
            return self._offset
        return None

    def _delimit(self, end: int) -> _Boundary | None:
        offset = self._offset
        if offset >= end or self._count == self._limit:
            return None
        failure = None
        try:
            next_offset = item_end(self._input[:end], offset)
        except ItemFramingError as error:
            if error.reason == "torn_tail" and end < len(self._input):
                return None
            failure = error.reason
            next_offset = len(self._input)

        if failure != "torn_tail":
            self._count += 1
        self._offset = next_offset
        return _Boundary(offset, next_offset, failure)

    def peek(self) -> _Boundary | None:
        if not self._held:
            boundary = self._delimit(len(self._input))
            if boundary is not None:
                self._held.append(boundary)
        return self._held[0] if self._held else None

    def take(self) -> _Boundary | None:
        if self._held:
            return self._held.popleft()
        return self._delimit(len(self._input))

    def recover(
        self,
        scanner: ResyncScanner,
        start: int,
        failed: _Boundary,
        *,
        parity: _ParityScanner | None = None,
    ) -> RecoveryResult:
        """Test the failed item's run after reaching its decision point."""
        if failed.failure is None:
            after = self.peek()
            if after is not None:
                # The next boundary can extend beyond the resync look-ahead.
                limit = min(len(self._input), failed.end + RESYNC_MAX)
                while (boundary := self._delimit(limit)) is not None:
                    self._held.append(boundary)
        if self._count == self._limit:
            return None

        # Resumption at parity adds a damaged region if the walk saw an incomplete item.
        slots = 2 if failed.failure == "torn_tail" else 1
        if (
            failed.failure is not None
            and parity is not None
            and self._limit - self._count >= slots
        ):
            result = parity.scan(scanner, start, failed.offset, failed.end)
        else:
            result = scanner.scan(start, failed.end)
        if isinstance(result, DecodedResync):
            self._resume(result.next_offset, 1)
        elif isinstance(result, ParityCandidate):
            self._resume(result.frame.next_offset, slots)
        return result

    def _resume(self, end: int, slots: int) -> None:
        self._count += slots
        self._held.clear()
        self._offset = end


def _walked_opener(data: bytes, item: _Boundary, kind: _OpenerKind) -> _Opener:
    value: DecodedHeader | DecodedResync | WireError
    try:
        decode = decode_header if kind == "header" else decode_resync
        value = decode(data, item.offset)
    except WireError as error:
        value = error
    return _Opener(kind, item, value)


def _scanned_opener(recovered: DecodedResync) -> _Opener:
    return _Opener(
        "resync",
        _Boundary(recovered.offset, recovered.next_offset),
        recovered,
        scanned=True,
    )


def _damage_before(data: bytes, start: int, end: int) -> _Boundary | None:
    if start < end:
        try:
            item_end(memoryview(data)[:end], start)
        except ItemFramingError as error:
            return _Boundary(start, end, error.reason)
    return None


def _open_input(
    data: bytes, walk: _ItemWalk, scanner: ResyncScanner, *, first_and_second: bool
) -> _Opening:
    if not data:
        return _Opening("empty_input")
    first = walk.take()
    if first is None:
        return _Opening("item_count")

    decision: _Opener | _Dispatch
    if first.failure is not None:
        decision = first.failure
    else:
        generation = _generation_field(data, first.end)
        if generation is None:
            decision = "not_this_format"
        elif generation.grammar == "first_generation":
            if generation.value == 1 and first_and_second:
                return _Opening("first_generation")
            return _Opening("unsupported_generation")
        elif generation.value < 2:
            decision = "not_this_format"
        elif generation.value > 2:
            return _Opening("unsupported_generation")
        else:
            _, _, kind_at = _head(data, 0, 0)
            _, kind, _ = _head(data, kind_at, 0)
            decision = _walked_opener(data, first, "resync" if kind == 5 else "header")
            if not isinstance(decision.value, WireError):
                return _Opening(decision)

    recovered = walk.recover(scanner, 0, first)
    if not isinstance(recovered, DecodedResync):
        return _Opening(
            decision,
            refusal=recovered if isinstance(recovered, ResyncRefusal) else None,
        )

    return _Opening(
        _scanned_opener(recovered), _damage_before(data, 0, recovered.offset)
    )


def _read_frame(
    data: bytes,
    walk: _ItemWalk,
    budget: PayloadBudget,
    segment: _Segment,
    offset: int,
    predecessor: bytes,
) -> tuple[WireFrame, FrameLink]:
    starts = _frame_field_offsets(data, offset)
    fields = _inspect_frame_link(data, offset, starts)
    link = without_witness = check_frame_link(fields, predecessor)
    if (
        link.state == "broken"
        and not fields.transparent
        and fields.stored_identifier is not None
        and link.computed_identifier is not None
    ):
        following = walk.peek()
        if (
            following is not None
            and following.failure is None
            and data.startswith(b"\x8b\x02", following.offset)
        ):
            witness = _inspect_frame_link(
                data, following.offset, _frame_field_offsets(data, following.offset)
            )
            link = check_frame_link(fields, predecessor, witness)

    newer = segment.registry_version > 1
    envelope: Frame | FrameError
    try:
        envelope = decoded = _decode_frame_at(data, offset, newer=newer)
    except FrameError as error:
        envelope = error
        payload = budget._check_invalid_envelope(data, starts, newer=newer)
        try:
            frame_type = _Decoder(data, starts[0], FrameError).uint()
        except FrameError:
            frame_type = None
    else:
        payload = budget.check(decoded)
        frame_type = decoded.frame_type
    frame = WireFrame(
        len(segment.items),
        frame_type,
        fields.transparent,
        envelope,
        link,
        payload,
        detached=not fields.transparent and predecessor != segment.head,
    )
    return frame, without_witness


def read_wire(
    data: bytes,
    *,
    items_per_read: int,
    stored_bytes_per_frame: int,
    decoded_bytes_per_frame: int,
    decoded_bytes_per_read: int,
    first_and_second: bool = True,
) -> WireRead:
    """Walk GTS v2 bytes and check openers, links, payloads and closing claims.

    A legacy dispatch is returned to the caller for the first-generation reader.
    The wire observations are inputs to parity recovery, content folding and
    construction of the typed read result.
    """
    if type(items_per_read) is not int or not 0 <= items_per_read < 1 << 64:
        raise ValueError("item limit must be a uint")
    budget = PayloadBudget(
        stored_bytes_per_frame=stored_bytes_per_frame,
        decoded_bytes_per_frame=decoded_bytes_per_frame,
        decoded_bytes_per_read=decoded_bytes_per_read,
    )
    walk = _ItemWalk(data, items_per_read)
    scanner = ResyncScanner(data, budget)
    opening = _open_input(data, walk, scanner, first_and_second=first_and_second)
    if not isinstance(opening.decision, _Opener):
        return WireRead(
            opening, item_limit_offset=0 if opening.decision == "item_count" else None
        )

    segments: list[WireSegment] = []
    segment = _Segment(opening.decision, data, budget)
    if opening.refusal is not None:
        segment.refusals.append(opening.refusal)
    previous = segment.opener.item
    while (boundary := walk.take()) is not None:
        item = None
        opener = None
        frame = None
        failed = boundary.failure is not None
        if not failed:
            item = _classify_item_at(
                data, boundary.offset, boundary.end, segment.registry_version
            )
            if item.kind in ("header", "resync"):
                opener = _walked_opener(data, boundary, item.kind)
                failed = isinstance(opener.value, WireError)
            elif item.kind == "opaque":
                failed = item.reason != "unknown_item_kind"
            elif segment.predecessor is not None:
                if item.kind == "frame":
                    frame, without_witness = _read_frame(
                        data, walk, budget, segment, item.offset, segment.predecessor
                    )
                    failed = (
                        isinstance(frame.envelope, FrameError)
                        or frame.link.state == "broken"
                    )
                else:
                    try:
                        _check_closing_grammar(data, item)
                    except WireError:
                        failed = True

        if failed:
            start = (
                boundary.offset if segment.pending is not None else previous.offset + 1
            )
            recovered = walk.recover(scanner, start, boundary, parity=segment.parity)
            if isinstance(recovered, DecodedResync):
                segment.settle_witness(kept=False)
                failure = _damage_before(data, boundary.offset, recovered.offset)
                end = recovered.offset
                if failure is not None:
                    if failure.failure == "undelimitable_input":
                        segment.items.append(DamagedRegion(failure.offset, failure.end))
                    else:
                        end = failure.offset
                segments.append(segment.finish(data, end, failure))
                segment = _Segment(_scanned_opener(recovered), data, budget)
                previous = segment.opener.item
                continue
            if isinstance(recovered, ResyncRefusal):
                segment.refusals.append(recovered)
            elif isinstance(recovered, ParityRefusal):
                segment.parity_refusals.append(recovered)
            elif isinstance(recovered, ParityCandidate):
                segment.settle_witness(kept=False)
                previous = segment.resume(boundary, recovered)
                continue

        segment.settle_witness(kept=True)
        if boundary.failure is not None:
            segments.append(segment.finish(data, boundary.offset, boundary))
            break
        if opener is not None:
            segments.append(segment.finish(data, boundary.offset))
            segment = _Segment(opener, data, budget)
        elif item is not None:
            segment.items.append(item)
            if frame is not None:
                segment.add_frame(frame, without_witness)
        previous = boundary
    else:
        segment.settle_witness(kept=True)
        segments.append(segment.finish(data, previous.end))
    return WireRead(opening, tuple(segments), walk.stopped_at)

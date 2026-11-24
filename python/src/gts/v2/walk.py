# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Walk segment boundaries and check the wire fields of Python v2 input."""

from dataclasses import dataclass

from ._item_walk import ItemCountReached, ItemWalk
from .dispatch import (
    DispatchFinding,
    FileDispatch,
    Opener,
    _decode_opener,
    _dispatch,
    _gap_finding,
)
from .frame import Frame, FrameError, _decode_framed
from .framing import ItemFramingError
from .header import DecodedHeader
from .items import (
    ClosingFinding,
    Item,
    ItemKind,
    _classify_framed_item,
    check_closing_items,
)
from .link import FrameLink, FrameLinkFields, check_frame_link, inspect_link_item
from .payload import PayloadBudget, PayloadCheck
from .resync import DecodedResync, ResyncError, ResyncRefusal, ResyncScanner


@dataclass(frozen=True, slots=True)
class WireFrame:
    item: Item
    frame: Frame | FrameError
    link: FrameLink
    payload: PayloadCheck | None


@dataclass(frozen=True, slots=True)
class WireSegment:
    offset: int
    length: int
    opener: Opener
    scanned: bool
    items: tuple[Item, ...]
    frames: tuple[WireFrame, ...]
    closing_findings: tuple[ClosingFinding, ...]


@dataclass(frozen=True, slots=True)
class WireRead:
    dispatch: FileDispatch
    segments: tuple[WireSegment, ...]
    framing_findings: tuple[DispatchFinding, ...]
    scan_refusals: tuple[ResyncRefusal, ...]


@dataclass(frozen=True, slots=True)
class _PendingFrame:
    fields: FrameLinkFields
    frame: Frame | FrameError
    payload: PayloadCheck | None


class _Segment:
    def __init__(self, item: Item, opener: Opener, *, scanned: bool) -> None:
        self.items = [item]
        self.opener = opener
        self.scanned = scanned
        self.frames: dict[int, _PendingFrame] = {}
        self.identifier = None
        self.registry = None
        if isinstance(opener, DecodedHeader | DecodedResync):
            self.identifier = opener.identifier
            if self.identifier is not None:
                self.registry = opener.header.registry_version

    def add(self, data: bytes, item: Item, budget: PayloadBudget) -> None:
        self.items.append(item)
        if item.kind != "frame":
            return
        fields, _ = inspect_link_item(data, item.offset)
        assert fields is not None and self.registry is not None
        frame: Frame | FrameError
        try:
            frame = _decode_framed(data, item.offset, self.registry)
        except FrameError as error:
            frame = error
            payload = None
        else:
            payload = budget.check(frame)
        self.frames[item.offset] = _PendingFrame(fields, frame, payload)

    def finish(self, data: bytes, end: int) -> WireSegment:
        start = self.items[0].offset
        predecessor = self.identifier
        frames = []
        closing: tuple[ClosingFinding, ...] = ()
        if predecessor is not None:
            for position, item in enumerate(self.items):
                current = self.frames.get(item.offset)
                if current is None:
                    continue
                following = (
                    self.frames.get(self.items[position + 1].offset)
                    if position + 1 < len(self.items)
                    else None
                )
                link = check_frame_link(
                    current.fields, predecessor, following.fields if following else None
                )
                if link.effective_identifier is not None:
                    predecessor = link.effective_identifier
                frames.append(WireFrame(item, current.frame, link, current.payload))
            closing = tuple(check_closing_items(data, self.items))
        return WireSegment(
            start,
            end - start,
            self.opener,
            self.scanned,
            tuple(self.items),
            tuple(frames),
            closing,
        )


def walk_file(
    data: bytes,
    budget: PayloadBudget,
    *,
    items_per_read: int = 1 << 26,
    first_and_second: bool = False,
) -> WireRead:
    """Walk complete items and resync at a loss of CBOR item boundaries.

    Frame events expose link checks and payload checks for decoded envelopes.
    Closing records use observed item boundaries. This wire pass does not
    perform parity recovery or scans triggered by a bad complete frame.
    """
    walk = ItemWalk(data, items_per_read)
    scanner = ResyncScanner(data, budget)
    dispatch = _dispatch(data, walk, scanner, first_and_second=first_and_second)
    findings = [] if dispatch.finding is None else [dispatch.finding]
    refusals = [] if dispatch.scan_refusal is None else [dispatch.scan_refusal]
    if dispatch.generation != 2:
        return WireRead(dispatch, (), tuple(findings), tuple(refusals))

    assert dispatch.opener_offset is not None and dispatch.opener is not None
    offset = dispatch.next_offset
    kind: ItemKind = (
        "resync"
        if isinstance(dispatch.opener, DecodedResync | ResyncError)
        else "header"
    )
    segment = _Segment(
        Item(kind, dispatch.opener_offset, offset),
        dispatch.opener,
        scanned=dispatch.scanned,
    )
    segments = []
    while offset < len(data):
        try:
            end = walk.take(offset)
        except ItemCountReached:
            findings.append(
                DispatchFinding(
                    "item_count", offset, len(data) - offset, items_per_read + 1
                )
            )
            break
        except ItemFramingError as error:
            candidate = (
                scanner.scan(segment.items[-1].offset + 1, len(data))
                if walk.scan_admitted(len(data))
                else None
            )
            if not isinstance(candidate, DecodedResync):
                if isinstance(candidate, ResyncRefusal):
                    refusals.append(candidate)
                findings.append(
                    DispatchFinding(error.reason, offset, len(data) - offset)
                )
                break
            walk.accept_resync()
            gap = _gap_finding(data, offset, candidate.offset)
            boundary = candidate.offset
            if gap is not None:
                findings.append(gap)
                if gap.reason == "undelimitable_input":
                    segment.items.append(Item("damaged_region", offset, boundary))
                else:
                    boundary = offset
            segments.append(segment.finish(data, boundary))
            offset = candidate.next_offset
            segment = _Segment(
                Item("resync", candidate.offset, offset), candidate, scanned=True
            )
            continue

        item = _classify_framed_item(data, offset, end, segment.registry)
        if item.kind in ("header", "resync"):
            segments.append(segment.finish(data, offset))
            opener = _decode_opener(data, offset, resync=item.kind == "resync")
            segment = _Segment(item, opener, scanned=False)
        else:
            segment.add(data, item, budget)
        offset = end
    segments.append(segment.finish(data, offset))
    return WireRead(dispatch, tuple(segments), tuple(findings), tuple(refusals))

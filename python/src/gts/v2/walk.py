# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Walk segment boundaries and check the wire fields of Python v2 input."""

from dataclasses import dataclass, replace

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
from .header import DecodedHeader, HeaderError
from .items import (
    ClosingFinding,
    Item,
    ItemKind,
    _classify_framed_item,
    _closing_grammar_failed,
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
    payload: PayloadCheck


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


class _Segment:
    def __init__(self, item: Item, opener: Opener, *, scanned: bool) -> None:
        self.items = [item]
        self.opener = opener
        self.scanned = scanned
        self.frames: list[WireFrame] = []
        self.provisional: tuple[FrameLinkFields, bytes] | None = None
        self.identifier = None
        self.registry = None
        if isinstance(opener, DecodedHeader | DecodedResync):
            self.identifier = opener.identifier
            if self.identifier is not None:
                self.registry = opener.header.registry_version
        self.predecessor = self.identifier

    def add(self, item: Item, frame: WireFrame | None = None) -> None:
        self.items.append(item)
        if frame is not None:
            self.frames.append(frame)
            if frame.link.effective_identifier is not None:
                self.predecessor = frame.link.effective_identifier

    def inspect(
        self, data: bytes, item: Item, budget: PayloadBudget, walk: ItemWalk
    ) -> tuple[WireFrame, tuple[FrameLinkFields, bytes] | None]:
        fields, _ = inspect_link_item(data, item.offset)
        predecessor = self.predecessor
        assert (
            fields is not None and self.registry is not None and predecessor is not None
        )
        link = check_frame_link(fields, predecessor)
        if (
            link.state == "broken"
            and not fields.transparent
            and fields.stored_identifier is not None
            and link.computed_identifier is not None
            and walk.peek(item.end) is not None
        ):
            witness, _ = inspect_link_item(data, item.end)
            link = check_frame_link(fields, predecessor, witness)
        frame: Frame | FrameError
        try:
            frame = _decode_framed(data, item.offset, self.registry)
        except FrameError as error:
            frame = error
            payload = budget._check_rejected_envelope(data, item.offset, self.registry)
        else:
            payload = budget.check(frame)
        provisional = (fields, predecessor) if link.state == "healed" else None
        return WireFrame(item, frame, link, payload), provisional

    def scan_start(self, failure: int) -> int:
        return failure if self.provisional is not None else self.items[-1].offset + 1

    def reject_witness(self) -> None:
        if self.provisional is not None:
            fields, predecessor = self.provisional
            self.frames[-1] = replace(
                self.frames[-1], link=check_frame_link(fields, predecessor)
            )
            self.provisional = None

    def finish(self, data: bytes, end: int) -> WireSegment:
        start = self.items[0].offset
        closing: tuple[ClosingFinding, ...] = ()
        if self.identifier is not None:
            closing = tuple(check_closing_items(data, self.items))
        return WireSegment(
            start,
            end - start,
            self.opener,
            self.scanned,
            tuple(self.items),
            tuple(self.frames),
            closing,
        )


def walk_file(
    data: bytes,
    budget: PayloadBudget,
    *,
    items_per_read: int = 1 << 26,
    first_and_second: bool = False,
) -> WireRead:
    """Walk items and recover at failed items or lost CBOR boundaries.

    Frame events expose independent link, envelope, and payload checks.
    Closing records use observed item boundaries. This wire pass does not
    perform parity recovery.
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
        framing_error = None
        item = None
        opener = None
        frame = None
        provisional = None
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
            framing_error = error
            end = len(data)
            failed = True
        else:
            # Outside a readable segment, a newer registry admits otherwise
            # unknown kinds for the framing test, but only openers are reported.
            item = _classify_framed_item(data, offset, end, segment.registry or 2)
            if item.kind in ("header", "resync"):
                opener = _decode_opener(data, offset, resync=item.kind == "resync")
                failed = isinstance(opener, HeaderError | ResyncError)
            elif segment.registry is None:
                failed = _classification_failed(item)
                item = Item("unchecked", offset, end)
            elif item.kind == "frame":
                frame, provisional = segment.inspect(data, item, budget, walk)
                failed = (
                    isinstance(frame.frame, FrameError) or frame.link.state == "broken"
                )
            else:
                failed = _classification_failed(item) or _closing_grammar_failed(
                    data, item
                )

        candidate = None
        if failed and walk.scan_admitted(end):
            result = scanner.scan(segment.scan_start(offset), end)
            if isinstance(result, ResyncRefusal):
                refusals.append(result)
            else:
                candidate = result
        if candidate is not None:
            walk.accept_resync()
            segment.reject_witness()
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

        if framing_error is not None:
            findings.append(
                DispatchFinding(framing_error.reason, offset, len(data) - offset)
            )
            break

        assert item is not None
        if opener is not None:
            segments.append(segment.finish(data, offset))
            segment = _Segment(item, opener, scanned=False)
        else:
            segment.add(item, frame)
            segment.provisional = provisional
        offset = end
    segments.append(segment.finish(data, offset))
    return WireRead(dispatch, tuple(segments), tuple(findings), tuple(refusals))


def _classification_failed(item: Item) -> bool:
    return item.kind == "opaque" and item.reason != "unknown_item_kind"

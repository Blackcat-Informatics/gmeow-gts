# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Confirm reconstructed bytes over disjoint runs of a segment (P5 §7.2)."""

from bisect import bisect_left, bisect_right
from collections import defaultdict, deque
from dataclasses import dataclass
from typing import TYPE_CHECKING

from ._decode import WireError
from .frame import FrameError, _decode_frame_at
from .framing import ItemFramingError, item_end
from .payload import PayloadBudget

if TYPE_CHECKING:
    from .reader import WireSegment


@dataclass(frozen=True, slots=True)
class _Patch:
    offset: int
    data: bytes

    def apply(self, target: bytearray, base: int = 0) -> None:
        start = self.offset - base
        target[start : start + len(self.data)] = self.data


@dataclass(frozen=True, slots=True)
class _Repair:
    position: int
    count: int
    digests_hold: bool
    data: tuple[_Patch, ...]
    parity: tuple[_Patch, ...]


def _walk_run(
    start: int,
    source: memoryview,
    working: memoryview,
    boundaries: set[int],
    opener_end: int,
    newer: bool,
    predecessor: bytes,
    budget: PayloadBudget,
) -> tuple[int, bool, bytes | None]:
    """Walk repaired items to the next surviving input boundary."""
    cursor = start
    last_ordinary = None
    while True:
        try:
            end = item_end(working, cursor)
        except ItemFramingError as error:
            return error.examined_end, False, None
        if cursor == 0:
            if end != opener_end or working[:end] != source[:end]:
                return end, False, None
        else:
            if working[cursor : cursor + 2] != b"\x8b\x02":
                return end, False, None
            try:
                frame = _decode_frame_at(working, cursor, newer=newer)
            except FrameError:
                return end, False, None
            if frame.computed_identifier(
                predecessor
            ) != frame.identifier or not budget.recheck(frame):
                return end, False, None
            if not frame.transparent:
                last_ordinary = predecessor = frame.identifier
        if end in boundaries:
            return end, True, last_ordinary
        cursor = end


def _rejected_spans(
    data: bytes, segment: "WireSegment", repairs: list[_Repair], budget: PayloadBudget
) -> set[int]:
    """Check the candidate copy once and propagate any failed run to its dependants."""
    rejected = {repair.position for repair in repairs if not repair.digests_hold}
    if not any(repair.data for repair in repairs):
        return rejected
    first = segment.items[0].offset
    end = segment.items[-1].end
    source = memoryview(data)[first:end]
    working = bytearray(source)
    starts = [item.offset - first for item in segment.items]
    touched: dict[int, set[int]] = defaultdict(set)
    for repair in repairs:
        for patch in repair.data:
            patch.apply(working, first)
            item_index = bisect_right(starts, patch.offset - first) - 1
            assert patch.offset + len(patch.data) <= segment.items[item_index].end
            touched[item_index].add(repair.position)
        for patch in repair.parity:
            patch.apply(working, first)

    boundaries = {*starts, len(working)}
    original_heads = {
        frame.position: identifier
        for frame in segment.frames
        if (identifier := frame.link.effective_identifier) is not None
    }
    opener = segment.opener.value
    assert not isinstance(opener, WireError) and opener.identifier is not None
    predecessor = opener.identifier
    newer = opener.header.registry_version > 1
    opener_end = segment.opener.item.end - first
    view = memoryview(working)

    run_count = covered_until = 0
    predecessor_run = None
    failed_since_predecessor = False
    runs_by_span: dict[int, set[int]] = defaultdict(set)
    dependants: dict[int, set[int]] = defaultdict(set)
    for index, cursor in enumerate(starts):
        if index < covered_until:
            continue
        if index not in touched:
            if (identifier := original_heads.get(index)) is not None:
                predecessor, predecessor_run = identifier, None
                failed_since_predecessor = False
            continue

        examined_end, holds, last_ordinary = _walk_run(
            cursor, source, view, boundaries, opener_end, newer, predecessor, budget
        )
        covered_until = bisect_left(starts, examined_end, index + 1)
        participants = {
            span
            for position in range(index, covered_until)
            for span in touched.get(position, ())
        }
        run = run_count
        run_count += 1
        for span in participants:
            runs_by_span[span].add(run)
        dependants[run].update(participants)
        if predecessor_run is not None:
            dependants[predecessor_run].update(participants)
        # Every failed run already removes its touching spans, so this prefix
        # dependency can seed removals without retaining all earlier failed runs.
        if not holds or failed_since_predecessor:
            rejected.update(participants)
        if not holds:
            failed_since_predecessor = True
        elif last_ordinary is not None:
            predecessor, predecessor_run = last_ordinary, run
            failed_since_predecessor = False

    pending = deque(rejected)
    while pending:
        for run in runs_by_span[pending.popleft()]:
            for span in dependants.pop(run, ()):
                if span not in rejected:
                    rejected.add(span)
                    pending.append(span)
    return rejected

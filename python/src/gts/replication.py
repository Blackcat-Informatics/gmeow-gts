# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Replication inventory helpers for the Python CLI.

These helpers report byte ranges, segment heads, and append deltas without
folding data into a new file. They are deliberately conservative: if the input
has a fatal parse error, torn append, segment diagnostic, or broken id/prev
chain, resumable ranges are withheld and the caller is told to scan instead.
"""

from __future__ import annotations

import json
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Literal, cast

import cbor2

from gts.model import Diagnostic, StreamableInfo
from gts.reader import read_segments
from gts.wire import (
    blake3_256,
    canonical,
    content_id,
    header_id,
    iter_items,
    unwrap_header,
)


@dataclass(frozen=True)
class FrameInventory:
    """Byte and chain metadata for one non-header frame."""

    item_index: int
    frame_index: int
    start: int
    end: int
    id: bytes
    frame_type: str
    valid: bool


@dataclass(frozen=True)
class SegmentInventory:
    """Inventory for one GTS segment and its frames."""

    index: int
    item_start: int
    item_end: int
    start: int
    end: int
    profile: str
    head: bytes | None
    frame_count: int
    layout: StreamableInfo
    diagnostics: list[Diagnostic]
    frames: list[FrameInventory]


@dataclass(frozen=True)
class ByteRange:
    """Half-open byte range, suitable for HTTP Range-style replication."""

    start: int
    end: int

    @property
    def length(self) -> int:
        """Number of bytes covered by the range."""
        return max(0, self.end - self.start)


MissingStatus = Literal["complete", "ranges", "unknown", "error"]


@dataclass(frozen=True)
class MissingResult:
    """Result of comparing a peer head against local segment/frame heads."""

    status: MissingStatus
    from_head: bytes
    ranges: list[ByteRange]
    scan_required: bool
    detail: str | None = None


@dataclass(frozen=True)
class Inventory:
    """A cleanly bounded view of the file suitable for replication decisions."""

    segments: list[SegmentInventory]
    fatal: Diagnostic | None
    torn: int | None
    clean_end: int
    item_count: int

    def has_problems(self) -> bool:
        """True when replication should not trust byte-range deltas."""
        return (
            self.fatal is not None
            or self.torn is not None
            or any(segment.diagnostics for segment in self.segments)
        )

    def problem_detail(self) -> str | None:
        """Return the first human-readable reason the inventory is not clean."""
        if self.fatal is not None:
            return f"{self.fatal.code}: {self.fatal.detail}"
        if self.torn is not None:
            return f"torn at offset {self.torn}"
        for segment in self.segments:
            if segment.diagnostics:
                diagnostic = segment.diagnostics[0]
                return f"{diagnostic.code}: {diagnostic.detail}"
        return None


def _as_text(value: object) -> str | None:
    return value if isinstance(value, str) else None


def _is_header_item(item: object) -> bool:
    if isinstance(item, cbor2.CBORTag):
        item = item.value
    return isinstance(item, Mapping) and "gts" in item and "t" not in item


def _item_end(
    items: list[tuple[int, object]],
    torn: int | None,
    data_len: int,
    index: int,
) -> int:
    if index + 1 < len(items):
        return items[index + 1][0]
    return torn if torn is not None else data_len


def _header_profile(item: object) -> str:
    try:
        header = unwrap_header(item)
    except ValueError:
        return "generic"
    return _as_text(header.get("prof")) or "generic"


def _header_stored_id(item: object) -> bytes | None:
    try:
        header = unwrap_header(item)
    except ValueError:
        return None
    value = header.get("id")
    return value if isinstance(value, bytes) else None


def _header_computed_id(item: object) -> bytes | None:
    try:
        return header_id(unwrap_header(item))
    except ValueError:
        return None


def _collect_frames(
    items: list[tuple[int, object]],
    torn: int | None,
    data_len: int,
    start: int,
    end: int,
) -> list[FrameInventory]:
    frames: list[FrameInventory] = []
    expected_prev = (
        _header_stored_id(items[start][1])
        or _header_computed_id(items[start][1])
        or b""
    )
    for item_index in range(start + 1, end):
        item_start, item = items[item_index]
        item_stop = _item_end(items, torn, data_len, item_index)
        frame_index = item_index - start - 1
        if not isinstance(item, Mapping):
            frames.append(
                FrameInventory(
                    item_index=item_index,
                    frame_index=frame_index,
                    start=item_start,
                    end=item_stop,
                    id=b"",
                    frame_type="<non-map>",
                    valid=False,
                )
            )
            continue

        # iter_items decodes arbitrary CBOR maps; after the Mapping check, treat
        # keys as wire field names and validate the actual values below.
        frame = cast(Mapping[str, object], item)
        computed = content_id(frame)
        stored = frame.get("id")
        stored_id = stored if isinstance(stored, bytes) else None
        frame_id = stored_id or computed
        prev = frame.get("prev")
        valid = (
            stored_id == computed and isinstance(prev, bytes) and prev == expected_prev
        )
        expected_prev = frame_id
        frames.append(
            FrameInventory(
                item_index=item_index,
                frame_index=frame_index,
                start=item_start,
                end=item_stop,
                id=frame_id,
                frame_type=_as_text(frame.get("t")) or "<unknown>",
                valid=valid,
            )
        )
    return frames


def inventory(data: bytes) -> Inventory:
    """Parse ``data`` into segment and frame byte-range inventory."""
    items, torn = iter_items(data)
    clean_end = torn if torn is not None else len(data)
    segments, _fs_torn, fatal = read_segments(data)
    if not items or fatal is not None:
        return Inventory(
            segments=[],
            fatal=fatal,
            torn=torn,
            clean_end=clean_end,
            item_count=len(items),
        )

    bounds = [index for index, (_, item) in enumerate(items) if _is_header_item(item)]
    if not bounds or bounds[0] != 0:
        return Inventory(
            segments=[],
            fatal=fatal,
            torn=torn,
            clean_end=clean_end,
            item_count=len(items),
        )

    ends = [*bounds[1:], len(items)]
    out: list[SegmentInventory] = []
    for index, (start_item, end_item) in enumerate(zip(bounds, ends, strict=True)):
        if index >= len(segments):
            break
        graph = segments[index]
        start = items[start_item][0]
        end = items[end_item][0] if end_item < len(items) else clean_end
        out.append(
            SegmentInventory(
                index=index,
                item_start=start_item,
                item_end=end_item,
                start=start,
                end=end,
                profile=graph.segment_profiles[0]
                if graph.segment_profiles
                else _header_profile(items[start_item][1]),
                head=graph.segment_heads[0] if graph.segment_heads else None,
                frame_count=max(0, end_item - start_item - 1),
                layout=graph.segment_streamable[0]
                if graph.segment_streamable
                else StreamableInfo(),
                diagnostics=list(graph.diagnostics),
                frames=_collect_frames(items, torn, len(data), start_item, end_item),
            )
        )
    return Inventory(
        segments=out,
        fatal=fatal,
        torn=torn,
        clean_end=clean_end,
        item_count=len(items),
    )


def _aggregate_digest(inv: Inventory) -> bytes:
    heads = [segment.head for segment in inv.segments if segment.head is not None]
    return blake3_256(canonical(["gts-segment-heads-v1", heads]))


def _optional_hex(value: bytes | None) -> str | None:
    return value.hex() if value is not None else None


def _diagnostic_json(diagnostic: Diagnostic) -> dict[str, object]:
    return {
        "code": diagnostic.code,
        "detail": diagnostic.detail,
        "frame_index": diagnostic.frame_index,
    }


def _layout_json(layout: StreamableInfo) -> dict[str, object]:
    return {
        "claimed": layout.claimed,
        "covered": layout.covered,
        "tail": layout.tail,
        "head": _optional_hex(layout.head),
    }


def _range_json(byte_range: ByteRange) -> dict[str, int]:
    return {
        "start": byte_range.start,
        "end": byte_range.end,
        "length": byte_range.length,
    }


def _dumps(doc: object) -> str:
    return json.dumps(doc, separators=(",", ":")) + "\n"


def heads_json(inv: Inventory) -> str:
    """Return the stable JSON representation of segment heads."""
    segment_heads = [
        segment.head.hex() for segment in inv.segments if segment.head is not None
    ]
    file_head = inv.segments[-1].head if inv.segments else None
    return _dumps(
        {
            "schema": "gts-replication-heads-v1",
            "clean": not inv.has_problems(),
            "segment_heads": segment_heads,
            "aggregate": {
                "schema": "gts-segment-heads-v1",
                "count": len(segment_heads),
                "digest": _aggregate_digest(inv).hex(),
                "file_head": _optional_hex(file_head),
            },
            "torn_at": inv.torn,
            "fatal": _diagnostic_json(inv.fatal) if inv.fatal is not None else None,
        }
    )


def segments_json(inv: Inventory) -> str:
    """Return the stable JSON representation of segment byte ranges."""
    return _dumps(
        {
            "schema": "gts-replication-segments-v1",
            "clean": not inv.has_problems(),
            "segments": [
                {
                    "index": segment.index,
                    "byte_range": _range_json(
                        ByteRange(start=segment.start, end=segment.end)
                    ),
                    "item_range": {
                        "start": segment.item_start,
                        "end": segment.item_end,
                    },
                    "profile": segment.profile,
                    "head": _optional_hex(segment.head),
                    "frame_count": segment.frame_count,
                    "layout": _layout_json(segment.layout),
                    "diagnostics": [
                        _diagnostic_json(diagnostic)
                        for diagnostic in segment.diagnostics
                    ],
                }
                for segment in inv.segments
            ],
            "item_count": inv.item_count,
            "torn_at": inv.torn,
            "fatal": _diagnostic_json(inv.fatal) if inv.fatal is not None else None,
        }
    )


def missing(inv: Inventory, from_head: bytes) -> MissingResult:
    """Return append ranges needed after ``from_head`` or request a scan."""
    if inv.has_problems():
        return MissingResult(
            status="error",
            from_head=from_head,
            ranges=[],
            scan_required=False,
            detail=inv.problem_detail(),
        )
    for segment in inv.segments:
        if segment.head == from_head:
            ranges = (
                [ByteRange(start=segment.end, end=inv.clean_end)]
                if segment.end < inv.clean_end
                else []
            )
            return MissingResult(
                status="ranges" if ranges else "complete",
                from_head=from_head,
                ranges=ranges,
                scan_required=False,
            )
        for frame in segment.frames:
            if frame.valid and frame.id == from_head:
                ranges = (
                    [ByteRange(start=frame.end, end=inv.clean_end)]
                    if frame.end < inv.clean_end
                    else []
                )
                return MissingResult(
                    status="ranges" if ranges else "complete",
                    from_head=from_head,
                    ranges=ranges,
                    scan_required=False,
                )
    return MissingResult(
        status="unknown",
        from_head=from_head,
        ranges=[],
        scan_required=True,
        detail="unknown peer head; scan required",
    )


def missing_json(result: MissingResult) -> str:
    """Return the stable JSON representation of a missing-range result."""
    return _dumps(
        {
            "schema": "gts-replication-missing-v1",
            "status": result.status,
            "from_head": result.from_head.hex(),
            "ranges": [_range_json(byte_range) for byte_range in result.ranges],
            "scan_required": result.scan_required,
            "detail": result.detail,
        }
    )


def resume_after(data: bytes, frame_id: bytes) -> bytes:
    """Return clean trailing bytes after a validated frame id."""
    inv = inventory(data)
    if inv.has_problems():
        msg = inv.problem_detail() or "input is not clean"
        raise ValueError(msg)
    for segment in inv.segments:
        for frame in segment.frames:
            if frame.valid and frame.id == frame_id:
                return data[frame.end : inv.clean_end]
    raise ValueError(f"frame {frame_id.hex()} not found")

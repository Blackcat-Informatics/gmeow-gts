# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P1 item classification, offsets, and locators (§§2.1, 2.2, 2.5–2.6)."""

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal

import cbor2

from ._decode import WireError, WireReason, _Decoder
from .framing import ItemFramingError, _head, item_end

_LOCATOR_PREFIX = b"\x82\x03\x1b"
_INDEX_PREFIX = b"\x8b\x02\x0f"
_LOCATOR_SIZE = 11
_UINT_MAX = (1 << 64) - 1

type ItemKind = Literal["header", "resync", "frame", "offsets", "locator", "opaque"]
type ItemReason = Literal[
    "wrong_type",
    "not_deterministic",
    "wrong_length",
    "unregistered_value",
    "unknown_item_kind",
]
type ClosingReason = WireReason | Literal["index_offsets", "index_locator"]


@dataclass(frozen=True, slots=True)
class Item:
    kind: ItemKind
    offset: int
    end: int
    reason: ItemReason | None = None


@dataclass(frozen=True, slots=True)
class ClosingFinding:
    position: int
    reason: ClosingReason


def _shortest_head_size(count: int | None) -> int:
    if count is None:
        return 0
    if count < 24:
        return 1
    for width in (1, 2, 4, 8):
        if count < 1 << (8 * width):
            return 1 + width
    raise AssertionError("array length exceeds uint")


def classify_item(data: bytes, offset: int = 0, *, registry_version: int = 1) -> Item:
    """Classify one complete item in a readable segment of a v2 file.

    This implements P1 §2.2 with a registry version 1 reader. The file reader
    decides the first item and the outcome of each opener separately.
    """
    end = item_end(data, offset)
    major, count, tag_at = _head(data, offset, offset)
    if major != 4 or count == 0:
        return Item("opaque", offset, end, "wrong_type")
    major, tag, after_tag = _head(data, tag_at, offset)
    if major != 0 or tag is None:
        return Item("opaque", offset, end, "wrong_type")
    if tag == 1:
        return Item("header", offset, end)
    if tag == 5:
        return Item("resync", offset, end)

    if tag_at - offset != _shortest_head_size(count) or (
        after_tag - tag_at != _shortest_head_size(tag)
    ):
        return Item("opaque", offset, end, "not_deterministic")
    if tag == 2:
        if count == 11:
            return Item("frame", offset, end)
        return Item("opaque", offset, end, "wrong_length")
    if tag == 3:
        return Item("locator", offset, end)
    if tag == 4:
        return Item("offsets", offset, end)
    if tag == 0 or registry_version <= 1:
        return Item("opaque", offset, end, "unregistered_value")
    return Item("opaque", offset, end, "unknown_item_kind")


def encode_offsets(item_lengths: Sequence[int]) -> bytes:
    """Write the lengths of earlier items in one segment, opener first."""
    if not item_lengths or any(
        type(length) is not int or not 1 <= length <= _UINT_MAX
        for length in item_lengths
    ):
        raise ValueError("offsets require the positive lengths of earlier items")
    return cbor2.dumps([4, list(item_lengths)])


def encode_locator(distance: int) -> bytes:
    """Write the fixed-width distance back to a segment's last index."""
    if type(distance) is not int or not 1 <= distance <= _UINT_MAX:
        raise ValueError("locator distance must be a positive uint")
    return _LOCATOR_PREFIX + distance.to_bytes(8, "big")


def recognize_locator(data: bytes) -> int | None:
    """Return an end-of-input index hint, to be confirmed by a forward walk."""
    offset = len(data) - _LOCATOR_SIZE
    if offset < 0 or not data.startswith(_LOCATOR_PREFIX, offset):
        return None
    distance = int.from_bytes(data[offset + len(_LOCATOR_PREFIX) :], "big")
    if not 1 <= distance <= offset:
        return None
    index_at = offset - distance
    if not data.startswith(_INDEX_PREFIX, index_at):
        return None
    try:
        end = item_end(data, index_at)
    except ItemFramingError:
        return None
    return index_at if end <= offset else None


def _offsets_hold(data: bytes, items: Sequence[Item], position: int) -> bool:
    decoder = _Decoder(data, items[position].offset, WireError)
    decoder.record(2)
    decoder.uint(minimum=4, maximum=4)
    count = decoder.array()
    holds = count == position
    for i in range(count):
        length = decoder.uint()
        if holds and length != items[i].end - items[i].offset:
            holds = False
    return holds


def _locator_distance(data: bytes, item: Item) -> int:
    decoder = _Decoder(data, item.offset, WireError)
    decoder.record(2)
    decoder.uint(minimum=3, maximum=3)
    offset = decoder.offset
    if data[offset] != 0x1B:
        decoder.uint()
        raise WireError("not_deterministic", offset)
    # A framed uint with this head has exactly eight argument bytes.
    return int.from_bytes(data[offset + 1 : item.end], "big")


def check_closing_items(data: bytes, items: Sequence[Item]) -> list[ClosingFinding]:
    """Check closing items against one segment's observed item sequence.

    The sequence starts with the opener and includes opaque and transparent
    items. Use actual item boundaries so storage claims cannot move the walk.
    The caller supplies a readable segment after framing and recovery.
    """
    results = []
    offsets_seen = False
    locator_seen = False
    last_index = None
    for position, item in enumerate(items):
        final = position == len(items) - 1
        if item.kind == "frame" and data.startswith(_INDEX_PREFIX, item.offset):
            last_index = item.offset
        elif item.kind == "offsets":
            followed_only_by_locator = (
                position == len(items) - 2 and items[-1].kind == "locator"
            )
            misplaced = (
                offsets_seen or locator_seen or not (final or followed_only_by_locator)
            )
            offsets_seen = True
            if misplaced:
                results.append(ClosingFinding(position, "unordered_or_repeated"))
                continue
            try:
                holds = _offsets_hold(data, items, position)
            except WireError as error:
                results.append(ClosingFinding(position, error.reason))
            else:
                if not holds:
                    results.append(ClosingFinding(position, "index_offsets"))
        elif item.kind == "locator":
            misplaced = locator_seen or not final
            locator_seen = True
            if misplaced:
                results.append(ClosingFinding(position, "unordered_or_repeated"))
                continue
            try:
                distance = _locator_distance(data, item)
            except WireError as error:
                results.append(ClosingFinding(position, error.reason))
            else:
                if not distance or item.offset - distance != last_index:
                    results.append(ClosingFinding(position, "index_locator"))
    return results

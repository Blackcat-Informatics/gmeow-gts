# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Whole-input item walk for the GTS v2 reader."""

from collections import deque
from dataclasses import dataclass
from typing import Literal

from ._decode import WireError
from .framing import FramingReason, ItemFramingError, _generation_field, _head, item_end
from .header import DecodedHeader, decode_header
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
        self, scanner: ResyncScanner, start: int, failed: _Boundary
    ) -> DecodedResync | ResyncRefusal | None:
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

        result = scanner.scan(start, failed.end)
        if isinstance(result, DecodedResync):
            self._count += 1
            self._held.clear()
            self._offset = result.next_offset
        return result


def _walked_opener(data: bytes, item: _Boundary, kind: _OpenerKind) -> _Opener:
    value: DecodedHeader | DecodedResync | WireError
    try:
        decode = decode_header if kind == "header" else decode_resync
        value = decode(data, item.offset)
    except WireError as error:
        value = error
    return _Opener(kind, item, value)


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
        return _Opening(decision, refusal=recovered)

    prefix = None
    if recovered.offset:
        try:
            item_end(memoryview(data)[: recovered.offset])
        except ItemFramingError as error:
            prefix = _Boundary(0, recovered.offset, error.reason)
    boundary = _Boundary(recovered.offset, recovered.next_offset)
    return _Opening(_Opener("resync", boundary, recovered, scanned=True), prefix)

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Share item-count and lookahead state between dispatch and the segment walk."""

from .framing import ItemFramingError, item_end
from .resync import RESYNC_MAX


class ItemCountReached(Exception):
    """The reader cannot delimit another item at this boundary."""


class ItemWalk:
    def __init__(self, data: bytes, limit: int) -> None:
        if type(limit) is not int or not 0 <= limit < 1 << 64:
            raise ValueError("items_per_read must be a uint")
        self._data = memoryview(data)
        self._limit = limit
        self._count = 0
        self._held: dict[int, int | ItemFramingError] = {}

    def take(self, offset: int) -> int:
        """Use a held boundary or delimit and charge the item at this offset."""
        known = self._held.pop(offset, None)
        if isinstance(known, ItemFramingError):
            raise known
        if known is not None:
            return known
        return self._delimit(offset, len(self._data))

    def peek(self, offset: int) -> int | None:
        """Hold the following item for a potential healing witness."""
        if offset >= len(self._data):
            return None
        known = self._held.get(offset)
        if isinstance(known, ItemFramingError):
            return None
        if known is not None:
            return known
        try:
            end = self._delimit(offset, len(self._data))
        except ItemCountReached:
            return None
        except ItemFramingError as error:
            self._held[offset] = error
            return None
        self._held[offset] = end
        return end

    def _delimit(self, offset: int, bound: int) -> int:
        if self._count >= self._limit:
            raise ItemCountReached
        try:
            end = item_end(self._data[:bound], offset)
        except ItemFramingError as error:
            self._count += int(error.reason == "undelimitable_input")
            raise
        self._count += 1
        return end

    def scan_admitted(self, after: int) -> bool:
        """Hold the boundaries required to decide a failed item's scan."""
        stop = min(len(self._data), after + RESYNC_MAX)
        position = after
        while position < stop and self._count < self._limit:
            # Hold the immediate lookahead item whole, even across the cap.
            bound = len(self._data) if position == after else stop
            known = self._held.get(position)
            if isinstance(known, ItemFramingError):
                break
            if known is not None:
                position = known
                continue
            try:
                end = self._delimit(position, bound)
            except ItemFramingError as error:
                if bound == len(self._data) or error.reason == "undelimitable_input":
                    self._held[position] = error
                break
            self._held[position] = end
            position = end
        return self._count < self._limit

    def accept_resync(self) -> None:
        """Keep prior charges and rewalk from the accepted opener onward."""
        self._count += 1
        self._held.clear()

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Deterministic wire field decoding shared by the P1 records."""

from collections.abc import Callable
from typing import Literal

from .framing import ItemFramingError, _head

type WireReason = Literal[
    "not_deterministic",
    "wrong_type",
    "wrong_length",
    "out_of_range",
    "unregistered_value",
    "forbidden_combination",
    "unordered_or_repeated",
    "inconsistent_counts",
]


class WireError(ValueError):
    """A malformed wire field and its position."""

    def __init__(self, reason: WireReason, offset: int) -> None:
        self.reason = reason
        self.offset = offset
        super().__init__(f"{reason} at offset {offset}")


class _Decoder:
    """A cursor over the deterministic GTS grammar, with checks in byte order."""

    def __init__(self, data: bytes, offset: int, error: type[WireError]) -> None:
        self.error = error
        self.data = data
        self.offset = offset

    def _take(self, size: int) -> bytes:
        start = self.offset
        end = start + size
        if end > len(self.data):
            raise self.error("wrong_length", start)
        self.offset = end
        return self.data[start:end]

    def _head(self, major_type: int) -> int:
        start = self.offset
        try:
            major, argument, self.offset = _head(self.data, start, start)
        except ItemFramingError as error:
            if error.reason == "torn_tail" and start < len(self.data):
                raise self.error("wrong_length", start + 1) from error
            raise self.error("wrong_length", start) from error

        additional = self.data[start] & 31
        if major <= 4 and (
            argument is None
            or (
                additional >= 24
                and argument < (24, 256, 65536, 4294967296)[additional - 24]
            )
        ):
            raise self.error("not_deterministic", start)
        if major == 7 and argument is None:
            raise self.error("wrong_length", start)
        if major != major_type or argument is None:
            raise self.error("wrong_type", start)
        return argument

    def record(self, length: int) -> None:
        start = self.offset
        if self._head(4) != length:
            raise self.error("wrong_length", start)

    def uint(
        self,
        *,
        minimum: int = 0,
        maximum: int = (1 << 64) - 1,
        reason: WireReason = "out_of_range",
    ) -> int:
        start = self.offset
        value = self._head(0)
        if not minimum <= value <= maximum:
            raise self.error(reason, start)
        return value

    def integer(self) -> int:
        if self.offset < len(self.data) and self.data[self.offset] >> 5 == 1:
            return -1 - self._head(1)
        return self._head(0)

    def option[T](self, decode: Callable[[], T]) -> T | None:
        start = self.offset
        length = self._head(4)
        if not length:
            raise self.error("wrong_length", start)
        tag = self.uint(minimum=1, maximum=2, reason="unregistered_value")
        if length != tag:
            raise self.error("wrong_length", start)
        return decode() if tag == 2 else None

    def boolean(self) -> bool:
        start = self.offset
        value = self._head(7)
        if self.offset != start + 1 or value not in (20, 21):
            raise self.error("wrong_type", start)
        return value == 21

    def bytes(self, length: int | None = None) -> memoryview:
        start = self.offset
        size = self._head(2)
        if length is not None and size != length:
            raise self.error("wrong_length", start)
        body = self.offset
        self.offset += size
        if self.offset > len(self.data):
            raise self.error("wrong_length", body)
        return memoryview(self.data)[body : self.offset]

    def text(self) -> None:
        self.text_value()

    def text_value(self) -> str:
        start = self.offset
        size = self._head(3)
        try:
            return self._take(size).decode("utf-8")
        except UnicodeDecodeError as error:
            raise self.error("wrong_type", start) from error

    def array(self) -> int:
        return self._head(4)

    def closed_item(self) -> None:
        """Check P1 §1.1 at every depth without materializing the value."""
        pending = 1
        while pending:
            major = self.data[self.offset] >> 5 if self.offset < len(self.data) else 0
            pending -= 1
            match major:
                case 2:
                    self.bytes()
                case 3:
                    self.text()
                case 4:
                    pending += self.array()
                case 7:
                    self.boolean()
                case 0 | 1:
                    self._head(major)
                case _:
                    raise self.error("wrong_type", self.offset)

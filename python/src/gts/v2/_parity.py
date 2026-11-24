# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P5 geometry, parity frame validation and placement."""

from dataclasses import dataclass
from typing import Literal

from blake3 import blake3

from ._decode import WireError, _Decoder
from .frame import Frame, FrameError
from .link import LinkState
from .payload import PayloadCheck

_SPAN_LENGTH = 1 << 22
_DATA_COLUMNS = 64
_PARITY_ROWS = 4


def _shard_size(span_size: int) -> int:
    return max(64, min(4096, 1 << ((span_size - 1) // _DATA_COLUMNS).bit_length()))


@dataclass(frozen=True, slots=True)
class ParityCheck:
    damaged: bool = False
    placement: Literal["placed", "stale"] | None = None
    newer_geometry: bool = False


class _ParityFrames:
    """Check parity in final segment order, with positions measured in S."""

    def __init__(self, opener: bytes, registry: int) -> None:
        self._opener = opener
        self._registry = registry
        self._seen_starts: set[int] = set()
        self._seen_short = False
        self._item_start = 0
        self._position = 0

    def advance(self, length: int) -> None:
        self._item_start = self._position
        self._position += length

    def check(
        self,
        content: memoryview,
        frame: Frame | FrameError,
        link: LinkState,
        predecessor: bytes,
        payload: PayloadCheck,
    ) -> ParityCheck:
        if link == "broken":
            return ParityCheck(damaged=True)
        decoder = _Decoder(content, 0, WireError)
        try:
            decoder.record(5)
            geometry = decoder.uint()
            first = decoder.uint()
            end = decoder.uint()
            anchor = decoder.bytes(32)
            table = decoder.bytes(32)
        except WireError:
            return ParityCheck(damaged=True)

        if geometry > 1 and self._registry > 1 and isinstance(frame, Frame):
            return ParityCheck(newer_geometry=True)

        span_size = end - first
        repeated = self._seen_short or first in self._seen_starts
        if not self._seen_short:
            if 0 < span_size < _SPAN_LENGTH:
                self._seen_short = True
                self._seen_starts.clear()
            else:
                self._seen_starts.add(first)
        if (
            geometry != 1
            or first % _SPAN_LENGTH
            or not 0 < span_size <= _SPAN_LENGTH
            or repeated
            or isinstance(frame, FrameError)
            or link != "linked"
            or anchor != predecessor
        ):
            return ParityCheck(damaged=True)
        if payload.state == "unchecked":
            return ParityCheck()
        if not self._table_matches(frame.stored_payload, table, span_size):
            return ParityCheck(damaged=True)

        placed = self._item_start < end <= self._position and (
            span_size == _SPAN_LENGTH or end == self._position
        )
        return ParityCheck(
            damaged=payload.state == "damaged",
            placement="placed" if placed else "stale",
        )

    def _table_matches(
        self, payload: memoryview, digest: memoryview, span_size: int
    ) -> bool:
        size = _shard_size(span_size)
        data_count = (span_size + size - 1) // size
        parity_count = (
            (data_count + _DATA_COLUMNS - 1) // _DATA_COLUMNS
        ) * _PARITY_ROWS
        decoder = _Decoder(payload, 0, WireError)
        try:
            decoder.record(3)
            table_start = decoder.offset
            for count in (data_count, parity_count):
                decoder.record(count)
                for _ in range(count):
                    decoder.bytes(16)
            table_end = decoder.offset
            decoder.bytes(parity_count * size)
        except WireError:
            return False
        if decoder.offset != len(payload):
            return False

        hasher = blake3(b"\x83\x58\x20", derive_key_context="gts2 parity table")
        hasher.update(self._opener)
        hasher.update(payload[table_start:table_end])
        return hasher.digest() == digest

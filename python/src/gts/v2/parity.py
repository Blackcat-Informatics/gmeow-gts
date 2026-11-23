# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Parity frame eligibility for resumption (P5 §§6.4–6.6, 7.2)."""

import re
from collections.abc import Set
from dataclasses import dataclass

from blake3 import blake3

from ._decode import WireError, _Decoder
from .frame import (
    Frame,
    _decode_frame_at,
    _frame_field_offsets,
    _identifier,
    _payload_digest,
)
from .items import _shortest_head_size
from .payload import PayloadBudget
from .resync import RESYNC_PATTERN, DecodedResync, ResyncRefusal, ResyncScanner

PARITY_PATTERN = b"\x8b\x02\x10\xf4\xf5"
_RECOVERY_PATTERN = re.compile(
    re.escape(RESYNC_PATTERN) + b"|" + re.escape(PARITY_PATTERN)
)
SPAN_LENGTH = 4194304


@dataclass(frozen=True, slots=True)
class _Layout:
    data_shards: int
    parity_shards: int
    shard_length: int

    @property
    def parity_bytes(self) -> int:
        return self.parity_shards * self.shard_length

    @property
    def payload_length(self) -> int:
        return (
            1
            + _shortest_head_size(self.data_shards)
            + 17 * self.data_shards
            + _shortest_head_size(self.parity_shards)
            + 17 * self.parity_shards
            + _shortest_head_size(self.parity_bytes)
            + self.parity_bytes
        )


def _layout(length: int) -> _Layout:
    shard_length = max(64, min(4096, 1 << ((length - 1) // 64).bit_length()))
    data_shards = (length + shard_length - 1) // shard_length
    stripes = (data_shards + 63) // 64
    return _Layout(data_shards, 4 * stripes, shard_length)


@dataclass(frozen=True, slots=True)
class ParityCandidate:
    offset: int
    first: int
    end: int
    anchor: bytes
    frame: Frame


@dataclass(frozen=True, slots=True)
class ParityRefusal:
    offset: int
    length: int
    count: int


type RecoveryResult = (
    DecodedResync | ResyncRefusal | ParityCandidate | ParityRefusal | None
)


def _read_content(decoder: _Decoder) -> tuple[int, int, int, bytes, bytes]:
    decoder.record(5)
    return (
        decoder.uint(),
        decoder.uint(),
        decoder.uint(),
        bytes(decoder.bytes(32)),
        bytes(decoder.bytes(32)),
    )


def _valid_table(
    data: bytes,
    payload_at: int,
    payload_end: int,
    layout: _Layout,
    opener: bytes,
    table_digest: bytes,
) -> bool:
    decoder = _Decoder(data, payload_at, WireError)
    try:
        decoder.record(3)
        first_list = decoder.offset
        for count in (layout.data_shards, layout.parity_shards):
            decoder.record(count)
            for _ in range(count):
                decoder.bytes(16)
        last_list = decoder.offset
        decoder.bytes(layout.parity_bytes)
    except WireError:
        return False
    if decoder.offset != payload_end:
        return False
    digest = blake3(b"\x83\x58\x20", derive_key_context="gts2 parity table")
    digest.update(opener)
    digest.update(memoryview(data)[first_list:last_list])
    return digest.digest() == table_digest


def check_parity_candidate(
    data: bytes,
    offset: int,
    *,
    opener: bytes,
    budget: PayloadBudget,
    prior_starts: Set[int],
    prior_short_span: bool,
) -> ParityCandidate | ParityRefusal | None:
    """Test a parity occurrence at its position in a damaged region.

    The caller supplies earlier parity spans from the current segment and
    orders tests of both resumption kinds by byte offset.
    """
    if not 0 <= offset <= len(data):
        raise ValueError("candidate offset is outside the input")
    if budget.total_refused or not data.startswith(PARITY_PATTERN, offset):
        return None
    # These five bytes fix the array, item kind, frame type and both flags.
    content_at = offset + len(PARITY_PATTERN)
    decoder = _Decoder(data, content_at, WireError)
    try:
        geometry, first, end, anchor, table_digest = _read_content(decoder)
        span = end - first
        if geometry != 1 or first % SPAN_LENGTH or not 1 <= span <= SPAN_LENGTH:
            return None
        if prior_short_span or first in prior_starts:
            return None
        layout = _layout(span)

        length = decoder.uint()
        digest = decoder.bytes(32)
        identity_end = decoder.offset
        decoder.record(1)
        decoder.uint(minimum=1, maximum=1)
        payload = decoder.bytes(layout.payload_length)
        if length != len(payload):
            return None
        payload_end = decoder.offset
        identifier = decoder.bytes(32)
        decoder.record(0)
    except WireError:
        return None

    identity = memoryview(data)[offset + 2 : identity_end]
    if _identifier(anchor, identity) != identifier:
        return None
    if not budget._allows_plain_candidate(length):
        return None
    refused = budget.charge_decoded(length)
    if refused is not None:
        return ParityRefusal(offset, decoder.offset - offset, refused)
    if _payload_digest(payload) != digest:
        return None
    if not _valid_table(
        data, payload_end - length, payload_end, layout, opener, table_digest
    ):
        return None
    return ParityCandidate(
        offset, first, end, anchor, _decode_frame_at(data, offset, newer=False)
    )


class _ParityScanner:
    """Track one segment's prior parity spans and interleave recovery tests."""

    def __init__(self, data: bytes, opener: bytes, budget: PayloadBudget) -> None:
        self._data = data
        self._opener = opener
        self._budget = budget
        self._starts: set[int] = set()
        self._short = False

    def remember(
        self, offset: int, envelope: Frame | WireError, *, newer: bool
    ) -> None:
        start = _frame_field_offsets(self._data, offset)[3]
        try:
            geometry, first, end, _, _ = _read_content(
                _Decoder(self._data, start, WireError)
            )
        except WireError:
            return
        if newer and geometry not in (0, 1) and isinstance(envelope, Frame):
            return
        self._starts.add(first)
        self._short |= 1 <= end - first < SPAN_LENGTH

    def scan(
        self, resync: ResyncScanner, start: int, damaged: int, end: int
    ) -> RecoveryResult:
        if self._budget.total_refused:
            return None
        # Neither marker contains an interior 8b, so the matches cannot overlap.
        for match in _RECOVERY_PATTERN.finditer(self._data, start, end):
            offset = match.start()
            result: RecoveryResult
            if match[0] == RESYNC_PATTERN:
                result = resync.scan(offset, offset + 1)
            elif offset >= damaged:
                result = check_parity_candidate(
                    self._data,
                    offset,
                    opener=self._opener,
                    budget=self._budget,
                    prior_starts=self._starts,
                    prior_short_span=self._short,
                )
            else:
                continue
            if result is not None:
                return result
        return None

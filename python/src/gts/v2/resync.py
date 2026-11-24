# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Resync opener records for GTS v2 (P1 §§2.4a–2.4b)."""

import codecs
from collections.abc import Sequence
from dataclasses import dataclass, replace

import cbor2
from blake3 import blake3

from ._decode import WireError, WireReason, _Decoder
from .framing import ItemFramingError, _head
from .header import (
    _BLAKE3_256,
    Header,
    _decode_lineage,
    _decode_opener_fields,
    _encode_opener_fields,
)
from .payload import PayloadBudget

RESYNC_MAX = 4096
_RESYNC_MARKS_MAX = 64
_CAPACITY_FIELDS = 9
_REGISTRY_FINDINGS = 48
_REGISTRY_REASONS = 92
_HEADER_CONTEXT = "gts2 header"
_MARKER = blake3(cbor2.dumps([5]), derive_key_context=_HEADER_CONTEXT).digest(length=16)
RESYNC_PATTERN = b"\x8b\x05\x02\x50" + _MARKER


class ResyncError(WireError):
    """An opener grammar error and the stated offset, when already read."""

    def __init__(self, reason: WireReason, offset: int) -> None:
        super().__init__(reason, offset)
        self.stated_offset: int | None = None


@dataclass(frozen=True, slots=True)
class ByteTarget:
    offset: int
    length: int


@dataclass(frozen=True, slots=True)
class AdvisoryMark:
    tag: int
    target: ByteTarget | None
    encoded: bytes


@dataclass(frozen=True, slots=True)
class DecodedResync:
    header: Header
    offset: int
    next_offset: int
    identifier: bytes | None
    stored_identifier: bytes
    past: bytes | None
    displaced: bool
    marks: tuple[AdvisoryMark, ...]
    marks_ignored: int


class _ResyncDecoder(_Decoder):
    """Apply the byte limit at field boundaries without losing earlier errors."""

    def __init__(self, data: bytes, offset: int) -> None:
        super().__init__(data, offset, ResyncError)
        self.limit = offset + RESYNC_MAX

    def _head(self, major_type: int) -> int:
        if self.offset >= self.limit:
            raise ResyncError("out_of_range", self.limit)
        return super()._head(major_type)

    def bytes(self, length: int | None = None) -> memoryview:
        result = super().bytes(length)
        if self.offset > self.limit:
            raise ResyncError("out_of_range", self.limit)
        return result

    def text(self) -> None:
        start = self.offset
        size = self._head(3)
        body = self.offset
        end = body + size
        if end <= self.limit:
            encoded = self._take(size)
        else:
            encoded = bytes(self.data[body : self.limit])
        try:
            # A partial code point at the cap is not an earlier UTF-8 error.
            codecs.utf_8_decode(encoded, "strict", end <= self.limit)
        except UnicodeDecodeError as error:
            raise ResyncError("wrong_type", start) from error
        if end > self.limit:
            raise ResyncError("out_of_range", self.limit)


def _target(decoder: _Decoder, stated_offset: int) -> ByteTarget | None:
    length = decoder.array()
    if length == 1:
        decoder.uint(minimum=1, maximum=1)
        return None
    if length != 3:
        raise WireError("wrong_length", decoder.offset)
    decoder.uint(minimum=2, maximum=2)
    offset = decoder.uint()
    size = decoder.uint(minimum=1)
    if offset + size > stated_offset:
        raise WireError("out_of_range", decoder.offset)
    return ByteTarget(offset, size)


def _applicable_mark(encoded: bytes, stated_offset: int) -> AdvisoryMark | None:
    # The closed grammar already holds. A mark outside the shape or code
    # assignments of its tag is ignored, including a tag the reader lacks.
    decoder = _Decoder(encoded, 0, WireError)
    length = decoder.array()
    tag = decoder.uint()
    if tag not in (1, 2, 3) or length != (4, 5, 2)[tag - 1]:
        return None
    try:
        target = _target(decoder, stated_offset)
        if tag == 1:
            decoder.uint(minimum=1, maximum=_CAPACITY_FIELDS)
            last_field = 0
            for _ in range(decoder.array()):
                decoder.record(2)
                last_field = decoder.uint(
                    minimum=last_field + 1, maximum=_CAPACITY_FIELDS
                )
                decoder.uint()
        else:
            if target is None:
                return None
            if tag == 2:
                decoder.uint(minimum=1, maximum=_REGISTRY_FINDINGS)
                decoder.option(
                    lambda: decoder.uint(minimum=1, maximum=_REGISTRY_REASONS)
                )
                decoder.uint()
    except WireError:
        return None
    return AdvisoryMark(tag, target, encoded)


def _read_marks(
    decoder: _ResyncDecoder, stated_offset: int
) -> tuple[tuple[AdvisoryMark, ...], int]:
    count = decoder.array()
    marks = []
    previous = b""
    for index in range(count):
        start = decoder.offset
        fields = decoder.array()
        if fields == 0:
            raise ResyncError("wrong_type", start)
        try:
            tag_major, _, _ = _head(decoder.data, decoder.offset, start)
        except ItemFramingError as error:
            raise ResyncError("wrong_length", decoder.offset) from error
        if tag_major != 0:
            raise ResyncError("wrong_type", start)
        if index >= _RESYNC_MARKS_MAX:
            raise ResyncError("out_of_range", start)

        # Complete CBOR items cannot be proper prefixes of each other. The
        # earlier mark's length is enough to decide order and duplication.
        prefix = bytes(decoder.data[start : start + len(previous)])
        if previous and prefix <= previous:
            raise ResyncError("unordered_or_repeated", start)

        for _ in range(fields):
            decoder.closed_item()
        if decoder.offset > decoder.limit:
            raise ResyncError("out_of_range", decoder.limit)
        previous = bytes(decoder.data[start : decoder.offset])
        mark = _applicable_mark(previous, stated_offset)
        if mark is not None:
            marks.append(mark)
    return tuple(marks), count - len(marks)


def _computed_identifier(body_fields: bytes | memoryview) -> bytes:
    # E(resync_body) consists of the array head and elements 0 through 9.
    hasher = blake3(b"\x8a", derive_key_context=_HEADER_CONTEXT)
    hasher.update(body_fields)
    return hasher.digest()


def _decode_prefix(
    decoder: _ResyncDecoder, *, scan: bool = False
) -> tuple[Header, int]:
    decoder.record(11)
    decoder.uint(minimum=5, maximum=5, reason="unregistered_value")
    decoder.uint(minimum=2, maximum=2)
    marker_at = decoder.offset
    if decoder.bytes() != _MARKER:
        raise ResyncError("out_of_range", marker_at)
    registry, algorithm, profile = _decode_opener_fields(
        decoder, require_known_hash=scan
    )
    digest_size = 32 if algorithm == _BLAKE3_256 else None
    lineage = _decode_lineage(decoder, digest_size)
    return Header(registry, algorithm, profile, lineage=lineage), decoder.uint()


def _decode_tail(
    decoder: _ResyncDecoder, offset: int, header: Header, stated_offset: int
) -> DecodedResync:
    digest_size = 32 if header.hash_algorithm == _BLAKE3_256 else None
    try:
        past = decoder.option(lambda: bytes(decoder.bytes(digest_size)))
        marks, marks_ignored = _read_marks(decoder, stated_offset)
        body_end = decoder.offset
        stored_identifier = bytes(decoder.bytes(digest_size))
    except ResyncError as error:
        if error.offset >= decoder.limit and error.reason != "out_of_range":
            capped = ResyncError("out_of_range", decoder.limit)
            capped.stated_offset = stated_offset
            raise capped from error
        error.stated_offset = stated_offset
        raise

    identifier = None
    if digest_size is not None:
        encoded_body = memoryview(decoder.data)[offset + 1 : body_end]
        identifier = _computed_identifier(encoded_body)
    return DecodedResync(
        header,
        stated_offset,
        decoder.offset,
        identifier,
        stored_identifier,
        past,
        stated_offset != offset,
        marks,
        marks_ignored,
    )


def decode_resync(data: bytes, offset: int = 0) -> DecodedResync:
    """Decode a resync item at a known boundary and compute its opener identifier.

    A walked item can be displaced or carry a different stored identifier.
    The recovery scan checks candidate acceptance separately.
    """
    if not 0 <= offset <= len(data):
        raise ValueError("resync offset is outside the input")
    decoder = _ResyncDecoder(data, offset)
    header, stated_offset = _decode_prefix(decoder)
    return _decode_tail(decoder, offset, header, stated_offset)


@dataclass(frozen=True, slots=True)
class ResyncRefusal:
    """The first candidate refused by the read total, at its input offset."""

    offset: int
    count: int


class ResyncScanner:
    """Test resync candidates in the reader's recovery runs (P1 §6.7)."""

    def __init__(self, data: bytes, budget: PayloadBudget) -> None:
        self._data = data
        self._budget = budget
        self._tested_until = 0
        self._last_start = 0

    def scan(self, start: int, end: int) -> DecodedResync | ResyncRefusal | None:
        """Find the first candidate starting in [start, end), or a refusal.

        Supply runs in ascending order of their starts. Runs may overlap and
        a candidate's final bytes may lie past the end of its run.
        """
        if not self._last_start <= start <= end <= len(self._data):
            raise ValueError("resync scan runs must start in order within the input")
        self._last_start = start
        if self._budget.total_refused:
            return None

        position = max(start, self._tested_until)
        search_end = min(len(self._data), end + len(RESYNC_PATTERN) - 1)
        while position < end:
            position = self._data.find(RESYNC_PATTERN, position, search_end)
            if position == -1:
                break
            self._tested_until = position + 1
            prefix = _ResyncDecoder(self._data, position)
            try:
                header, stated_offset = _decode_prefix(prefix, scan=True)
            except ResyncError:
                position = self._tested_until
                continue
            if stated_offset == position:
                refused = self._budget.charge_decoded(RESYNC_MAX)
                if refused is not None:
                    return ResyncRefusal(position, refused)
                candidate = self._candidate_tail(position, prefix.offset, header)
                if candidate is not None:
                    return candidate
            position = self._tested_until

        self._tested_until = max(self._tested_until, end)
        return None

    def _candidate_tail(
        self, position: int, tail_at: int, header: Header
    ) -> DecodedResync | None:
        data = self._data[position : position + RESYNC_MAX]
        decoder = _ResyncDecoder(data, 0)
        decoder.offset = tail_at - position
        try:
            candidate = _decode_tail(decoder, 0, header, position)
        except ResyncError:
            return None
        if candidate.identifier != candidate.stored_identifier:
            return None
        return replace(
            candidate, next_offset=position + candidate.next_offset, displaced=False
        )


def encode_resync(
    header: Header,
    *,
    offset: int,
    past: bytes | None = None,
    marks: Sequence[list[object]] = (),
) -> bytes:
    """Encode an opener in bytewise mark order using the caller's append data.

    The appender supplies the measured offset and the data for any advice.
    """
    if type(offset) is not int or not 0 <= offset < 1 << 64:
        raise ValueError("resync offset must be a uint")
    if header.canonical is not None:
        raise ValueError("a resync opener cannot claim canonical form")
    if past is not None and (not isinstance(past, bytes) or len(past) != 32):
        raise ValueError("past must be a 32-byte digest")
    if len(marks) > _RESYNC_MARKS_MAX:
        raise ValueError("resync mark count exceeds 64")
    registry, algorithm, profile, lineage = _encode_opener_fields(header)
    ordered = sorted(marks, key=cbor2.dumps)
    body = cbor2.dumps(
        [
            5,
            2,
            _MARKER,
            registry,
            algorithm,
            profile,
            lineage,
            offset,
            [1] if past is None else [2, past],
            ordered,
        ]
    )
    encoded = b"\x8b" + body[1:] + cbor2.dumps(_computed_identifier(body[1:]))
    decoded = decode_resync(encoded)
    if decoded.marks_ignored:
        raise ValueError("resync writer requires applicable marks")
    for mark in decoded.marks:
        if mark.tag == 3 and mark.target is not None:
            if mark.target.offset + mark.target.length != offset:
                raise ValueError("filler must end where the resync item begins")
    return encoded

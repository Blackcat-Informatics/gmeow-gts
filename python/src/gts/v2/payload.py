# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Capacity accounting and payload checks for GTS v2 (P1 §§3.5, 5.3–5.4)."""

from collections.abc import Iterator
from dataclasses import dataclass
from typing import Literal

import zstandard

from ._decode import _Decoder
from ._zstd import ZSTD_CODEC, zstd_header
from .frame import (
    CompressedStorage,
    Frame,
    FrameError,
    UnknownStorage,
    _frame_field_offsets,
    _payload_digest,
    _storage,
    _stored_check_parts,
)
from .framing import _head

_UINT_MAX = (1 << 64) - 1

type PayloadReason = Literal[
    "stored_size",
    "decoded_size",
    "total_decoded",
    "unknown_codec",
    "codec_constraint",
    "decode_failed",
    "decoded_length",
    "payload_digest",
]


@dataclass(frozen=True, slots=True)
class PayloadCheck:
    state: Literal["verified", "unchecked", "damaged"]
    stored_matches: bool | None
    reason: PayloadReason | None = None
    count: int | None = None
    decoded: bytes | memoryview | None = None


class PayloadBudget:
    """The stored, decoded and cumulative byte limits for one read.

    Use one instance across every segment and charge frames and recovery
    candidates in reader order. The stored check runs even when capacity
    refuses decoding.
    """

    def __init__(
        self,
        *,
        stored_bytes_per_frame: int,
        decoded_bytes_per_frame: int,
        decoded_bytes_per_read: int,
    ) -> None:
        limits = (
            stored_bytes_per_frame,
            decoded_bytes_per_frame,
            decoded_bytes_per_read,
        )
        if any(
            type(limit) is not int or not 0 <= limit <= _UINT_MAX for limit in limits
        ):
            raise ValueError("payload limits must be uint values")
        self._stored_limit = stored_bytes_per_frame
        self._decoded_limit = decoded_bytes_per_frame
        self._total_limit = decoded_bytes_per_read
        self._total_decoded = 0
        self._total_refused = False

    @property
    def total_refused(self) -> bool:
        return self._total_refused

    def charge_decoded(self, length: int) -> int | None:
        """Charge work against the read total or return the refused count."""
        if type(length) is not int or not 0 <= length <= _UINT_MAX:
            raise ValueError("decoded charge must be a uint")
        total = self._total_decoded + length
        if self._total_refused or total > self._total_limit:
            self._total_refused = True
            return min(total, _UINT_MAX)
        self._total_decoded = total
        return None

    def check(self, frame: Frame) -> PayloadCheck:
        """Check a frame whose payload fields have passed envelope validation."""
        return self._check_fields(
            frame.decoded_length,
            frame.payload_digest,
            frame.storage,
            frame.stored_payload,
            frame.check_storage(),
            len(frame.content),
            frame.encoded_signatures_size,
        )

    def _check_rejected_envelope(
        self, data: bytes, offset: int, registry_version: int
    ) -> PayloadCheck:
        starts = _frame_field_offsets(data, offset)
        try:
            storage = _storage(
                _Decoder(data, starts[6], FrameError), newer=registry_version > 1
            )
        except FrameError:
            return PayloadCheck("damaged", None)
        stored = _check_stored_field(data, starts[7], starts[8], storage)
        try:
            length = _Decoder(data, starts[4], FrameError).uint()
            digest = _Decoder(data, starts[5], FrameError).bytes(32)
            payload = _Decoder(data, starts[7], FrameError).bytes()
        except FrameError:
            return PayloadCheck("damaged", stored)
        return self._check_fields(
            length,
            digest,
            storage,
            payload,
            stored,
            starts[4] - starts[3],
            starts[10] - starts[9],
        )

    def _check_fields(
        self,
        length: int,
        digest: bytes | memoryview,
        storage: CompressedStorage | UnknownStorage | None,
        payload: memoryview,
        stored: bool | None,
        content_size: int,
        signatures_size: int,
    ) -> PayloadCheck:
        if not self._total_refused:
            for size in (len(payload), content_size, signatures_size):
                if size > self._stored_limit:
                    return PayloadCheck("unchecked", stored, "stored_size", size)

            if length > self._decoded_limit:
                return PayloadCheck("unchecked", stored, "decoded_size", length)
        refused = self.charge_decoded(length)
        if refused is not None:
            return PayloadCheck("unchecked", stored, "total_decoded", refused)

        decoded: bytes | memoryview
        if isinstance(storage, UnknownStorage):
            return PayloadCheck("unchecked", stored, "unknown_codec")
        if isinstance(storage, CompressedStorage):
            if storage.codec != ZSTD_CODEC:
                return PayloadCheck("unchecked", stored, "unknown_codec")
            if length == 0:
                return PayloadCheck("damaged", stored, "codec_constraint")
            header = zstd_header(length)
            if payload[: len(header)] != header:
                return PayloadCheck("damaged", stored, "codec_constraint")
            try:
                # The matched header fixes the content size and window. Require
                # the decoder to consume exactly one frame and the whole input.
                decoded = zstandard.ZstdDecompressor().decompress(
                    payload,
                    max_output_size=length,
                    read_across_frames=False,
                    allow_extra_data=False,
                )
            except (zstandard.ZstdError, OverflowError, MemoryError):
                return PayloadCheck("damaged", stored, "decode_failed")
            if len(decoded) != length:
                return PayloadCheck("damaged", stored, "decode_failed")
        else:
            decoded = payload
            if len(decoded) != length:
                return PayloadCheck("damaged", stored, "decoded_length")

        if _payload_digest(decoded) != digest:
            return PayloadCheck("damaged", stored, "payload_digest")
        return PayloadCheck("verified", stored, decoded=decoded)


def _check_stored_field(
    data: bytes,
    start: int,
    end: int,
    storage: CompressedStorage | UnknownStorage | None,
) -> bool | None:
    if not isinstance(storage, CompressedStorage):
        return None
    major, length, offset = _head(data, start, start)
    if major != 2:
        return None
    view = memoryview(data)
    parts = (
        _stored_chunks(view, offset, end - 1) if length is None else (view[offset:end],)
    )
    return _stored_check_parts(storage.codec, parts) == storage.check


def _stored_chunks(data: memoryview, offset: int, end: int) -> Iterator[memoryview]:
    while offset < end:
        _, length, body = _head(data, offset, offset)
        assert length is not None
        offset = body + length
        yield data[body:offset]

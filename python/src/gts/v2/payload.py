# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Capacity accounting and payload checks for GTS v2 (P1 §§3.5, 5.3–5.4)."""

from dataclasses import dataclass
from typing import Literal

import zstandard

from ._zstd import ZSTD_CODEC, zstd_header
from .frame import CompressedStorage, Frame, UnknownStorage

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

    Use one instance across every segment. Submit frames in reader order
    after checking their links. The stored check remains independent of
    payload acceptance and runs even when capacity refuses decoding.
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

    def check(self, frame: Frame) -> PayloadCheck:
        """Check a frame whose payload fields have passed envelope validation."""
        stored = frame.check_storage()
        length = frame.decoded_length
        total = self._total_decoded + length
        if self._total_refused:
            return PayloadCheck(
                "unchecked", stored, "total_decoded", min(total, _UINT_MAX)
            )

        for size in (
            len(frame.stored_payload),
            len(frame.content),
            frame.encoded_signatures_size,
        ):
            if size > self._stored_limit:
                return PayloadCheck("unchecked", stored, "stored_size", size)

        if length > self._decoded_limit:
            return PayloadCheck("unchecked", stored, "decoded_size", length)
        if total > self._total_limit:
            self._total_refused = True
            return PayloadCheck(
                "unchecked", stored, "total_decoded", min(total, _UINT_MAX)
            )
        self._total_decoded = total

        payload = frame.stored_payload
        storage = frame.storage
        decoded: bytes | memoryview
        if isinstance(storage, UnknownStorage):
            return PayloadCheck("unchecked", stored, "unknown_codec")
        if isinstance(storage, CompressedStorage):
            if storage.codec != ZSTD_CODEC:
                return PayloadCheck("unchecked", stored, "unknown_codec")
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

        if not frame.check_decoded_payload(decoded):
            return PayloadCheck("damaged", stored, "payload_digest")
        return PayloadCheck("verified", stored, decoded=decoded)

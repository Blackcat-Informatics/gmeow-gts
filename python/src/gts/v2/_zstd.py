# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""The shared P1 codec constraint and P7 storage choice for zstd."""

from zstandard import ZstdCompressor

from ._content import UINT_MAX

ZSTD_CODEC = 1
_MAGIC = b"\x28\xb5\x2f\xfd"


def zstd_header(length: int) -> bytes:
    """Generate the sole compressed frame header assigned to a decoded length."""
    if not 1 <= length <= UINT_MAX:
        raise ValueError("compressed payload length must be a positive uint")
    if length <= 255:
        return _MAGIC + b"\x20" + length.to_bytes(1, "little")
    if length <= 65791:
        return _MAGIC + b"\x60" + (length - 256).to_bytes(2, "little")
    if length <= 1 << 26:
        return _MAGIC + b"\xa0" + length.to_bytes(4, "little")
    if length < (1 << 32) - 1:
        return _MAGIC + b"\x80\x80" + length.to_bytes(4, "little")
    return _MAGIC + b"\xc0\x80" + length.to_bytes(8, "little")


def compressed_payload(payload: bytes, compressor: ZstdCompressor) -> bytes | None:
    """Choose compressed storage when the prescribed header and saving hold."""
    length = len(payload)
    if length < 64:
        return None
    encoded = compressor.compress(payload)
    if len(encoded) + 16 < length and encoded.startswith(zstd_header(length)):
        return encoded
    return None

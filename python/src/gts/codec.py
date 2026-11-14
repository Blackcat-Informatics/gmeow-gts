# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""The GTS transform catalog (§8).

Each catalog entry is a codec with a canonical ``name`` and a ``cls`` of ``encode``,
``compress`` or ``encrypt``. Decoding a chain requires a capability per codec: a
library (encode/compress) or a key (encrypt). The baseline implements the core
``identity``/``gzip``/``zstd`` codecs; an unknown codec or an ``encrypt`` codec
(no keys in the baseline) raises :class:`CodecUnavailableError`, which the reader folds
into an opaque node (§7.6, §8.3).
"""

from __future__ import annotations

import gzip
import io
from collections.abc import Callable
from dataclasses import dataclass
from typing import Literal

import zstandard

CodecClass = Literal["encode", "compress", "encrypt"]

#: A key-bound callback that reverses an ``encrypt`` codec (supplied by the reader).
Decryptor = Callable[[bytes], bytes]


class CodecUnavailableError(Exception):
    """Raised when a transform cannot be reversed.

    Attributes:
        reason: ``"unknown-codec"`` (no library) or ``"missing-key"`` (encrypt class).
    """

    def __init__(self, reason: str, detail: str) -> None:
        """Store the degradation ``reason`` alongside the human-readable detail."""
        super().__init__(detail)
        self.reason = reason


class PayloadDecodeError(ValueError):
    """A codec library cannot reverse a payload.

    The bytes are damaged or truncated, or the frame asks for more than this reader
    decodes (a zstd window above ``2**ZSTD_MAX_WINDOW_LOG``). The reader folds the frame
    as a ``damaged`` opaque node (§7.6) instead of raising.
    """


@dataclass(frozen=True)
class Codec:
    """A catalog entry (§5, §8.5)."""

    name: str
    cls: CodecClass


# The default catalog a writer emits; ids are file-local (§8.5 — match by name).
DEFAULT_CATALOG: dict[int, Codec] = {
    0: Codec("identity", "encode"),
    1: Codec("gzip", "compress"),
    2: Codec("zstd", "compress"),
    3: Codec("zstd-rsyncable", "compress"),
    7: Codec("cose-encrypt0", "encrypt"),
}

#: Largest zstd window, as a power of two, that this reader decodes (128 MiB), the
#: default limit of libzstd's streaming decoder. The frame-header check below refuses
#: every frame that DECLARES a larger window, so what is accepted depends on the frame
#: and not on the size of the caller's buffer: the library skips its own limit when the
#: whole content fits that buffer, and so decoded a few hand-built frames that declare a
#: large window over a small known content size. Those frames are now refused.
ZSTD_MAX_WINDOW_LOG = 27

_ZSTD_C = zstandard.ZstdCompressor()
_ZSTD_D = zstandard.ZstdDecompressor(max_window_size=1 << ZSTD_MAX_WINDOW_LOG)

#: Uncompressed block size for the zstd-rsyncable codec. Each block is an
#: independent zstd frame so that a local change only affects that block.
_RSYNCABLE_BLOCK_SIZE = 65536


def _zstd_compressor(level: int | None = None) -> zstandard.ZstdCompressor:
    """Return the default zstd compressor, or one using a caller-selected level."""
    if level is None:
        return _ZSTD_C
    return zstandard.ZstdCompressor(level=level)


def _encode_zstd_rsyncable(data: bytes, *, zstd_level: int | None = None) -> bytes:
    """Compress with periodic state resets for rsync/Git friendliness.

    Each block is compressed as an independent zstd frame and concatenated.
    A change inside one block therefore only affects that block's compressed
    bytes, keeping rsync/Git deltas small.
    """
    compressor = _zstd_compressor(zstd_level)
    out = io.BytesIO()
    view = memoryview(data)
    for i in range(0, len(data), _RSYNCABLE_BLOCK_SIZE):
        out.write(compressor.compress(view[i : i + _RSYNCABLE_BLOCK_SIZE]))
    return out.getvalue()


def _encode_one(name: str, data: bytes, *, zstd_level: int | None = None) -> bytes:
    """Apply a single codec by canonical name (encode direction)."""
    if name == "identity":
        return data
    if name == "gzip":
        return gzip.compress(data, mtime=0)
    if name == "zstd":
        return _zstd_compressor(zstd_level).compress(data)
    if name == "zstd-rsyncable":
        return _encode_zstd_rsyncable(data, zstd_level=zstd_level)
    msg = f"writer cannot encode with codec {name!r}"
    raise CodecUnavailableError("unknown-codec", msg)


_ZSTD_MAGIC = 0xFD2FB528
_ZSTD_SKIPPABLE_MAGIC = range(0x184D2A50, 0x184D2A60)
_ZSTD_MAX_BLOCK = 128 * 1024
_ZSTD_DICT_ID_BYTES = (0, 1, 2, 4)
_ZSTD_CONTENT_SIZE_BYTES = (0, 2, 4, 8)


def _zstd_window(descriptor: int) -> int:
    """Return the window in bytes that a frame's window descriptor byte declares."""
    base = 1 << (10 + (descriptor >> 3))
    return base + (base >> 3) * (descriptor & 7)


def check_zstd_frames(data: bytes) -> None:
    """Check that ``data`` is whole zstd frames this reader decodes, without decoding.

    Walks every frame header and block header (a few bytes per 128 KiB block), so a
    payload that is not zstd, ends inside a frame, or declares a window above
    ``2**ZSTD_MAX_WINDOW_LOG`` is found when the frame is folded rather than when its
    bytes are first read. Damage inside a block is not found here.

    Raises:
        PayloadDecodeError: the payload is not whole, decodable zstd frames.
    """
    n = len(data)
    pos = 0
    while pos < n:
        if n - pos < 4:
            msg = "zstd frame is truncated"
            raise PayloadDecodeError(msg)
        magic = int.from_bytes(data[pos : pos + 4], "little")
        if magic in _ZSTD_SKIPPABLE_MAGIC:
            if n - pos < 8:
                msg = "zstd skippable frame is truncated"
                raise PayloadDecodeError(msg)
            pos += 8 + int.from_bytes(data[pos + 4 : pos + 8], "little")
            if pos > n:
                msg = "zstd skippable frame is truncated"
                raise PayloadDecodeError(msg)
            continue
        if magic != _ZSTD_MAGIC:
            msg = "payload is not a zstd frame (unknown magic number)"
            raise PayloadDecodeError(msg)
        pos += 4
        if pos >= n:
            msg = "zstd frame is truncated"
            raise PayloadDecodeError(msg)
        flags = data[pos]
        pos += 1
        single_segment = bool(flags & 0x20)
        if flags & 0x08:
            msg = "zstd frame header sets a reserved bit"
            raise PayloadDecodeError(msg)
        window_byte = 0
        if not single_segment:
            if pos >= n:
                msg = "zstd frame is truncated"
                raise PayloadDecodeError(msg)
            window_byte = data[pos]
            pos += 1
        dict_bytes = _ZSTD_DICT_ID_BYTES[flags & 3]
        if pos + dict_bytes > n:
            msg = "zstd frame is truncated"
            raise PayloadDecodeError(msg)
        if any(data[pos : pos + dict_bytes]):
            msg = "zstd frame names a dictionary, which this reader does not implement"
            raise PayloadDecodeError(msg)
        pos += dict_bytes
        size_bytes = _ZSTD_CONTENT_SIZE_BYTES[flags >> 6] or (
            1 if single_segment else 0
        )
        if pos + size_bytes > n:
            msg = "zstd frame is truncated"
            raise PayloadDecodeError(msg)
        window = _zstd_window(window_byte)
        if single_segment:
            window = int.from_bytes(data[pos : pos + size_bytes], "little")
            if flags >> 6 == 1:
                window += 256
        pos += size_bytes
        if window > 1 << ZSTD_MAX_WINDOW_LOG:
            msg = (
                f"zstd frame declares a window of {window} bytes, above this reader's "
                f"limit of 2**{ZSTD_MAX_WINDOW_LOG}"
            )
            raise PayloadDecodeError(msg)
        while True:
            if pos + 3 > n:
                msg = "zstd frame is truncated"
                raise PayloadDecodeError(msg)
            header = int.from_bytes(data[pos : pos + 3], "little")
            block_type = (header >> 1) & 3
            size = header >> 3
            if block_type == 3 or size > _ZSTD_MAX_BLOCK:
                msg = "zstd block header is invalid"
                raise PayloadDecodeError(msg)
            pos += 3 + (1 if block_type == 1 else size)
            if pos > n:
                msg = "zstd frame is truncated"
                raise PayloadDecodeError(msg)
            if header & 1:
                break
        if flags & 0x04:
            pos += 4
            if pos > n:
                msg = "zstd frame is truncated"
                raise PayloadDecodeError(msg)


def _decode_zstd(data: bytes, *, checked: bool = False) -> bytes:
    """Decompress zstd bytes, including rsyncable streams with flush blocks.

    ``checked`` says the caller already ran :func:`check_zstd_frames` on ``data``.

    Raises:
        PayloadDecodeError: the payload is damaged, truncated or beyond this reader's
            window limit.
    """
    if not checked:
        check_zstd_frames(data)
    out = io.BytesIO()
    try:
        with _ZSTD_D.stream_reader(io.BytesIO(data), read_across_frames=True) as reader:
            while True:
                chunk = reader.read(131072)
                if not chunk:
                    break
                out.write(chunk)
    except zstandard.ZstdError as exc:
        msg = f"zstd decode failed: {exc}"
        raise PayloadDecodeError(msg) from exc
    return out.getvalue()


def _decode_one(codec: Codec, data: bytes, *, checked: bool = False) -> bytes:
    """Reverse a single codec, or raise :class:`CodecUnavailableError` (§8.3)."""
    if codec.cls == "encrypt":
        raise CodecUnavailableError(
            "missing-key", f"no key for encrypt codec {codec.name!r}"
        )
    if codec.name == "identity":
        return data
    if codec.name == "gzip":
        return gzip.decompress(data)
    if codec.name in ("zstd", "zstd-rsyncable"):
        return _decode_zstd(data, checked=checked)
    raise CodecUnavailableError("unknown-codec", f"unknown codec {codec.name!r}")


def encode_chain(
    chain: list[str],
    data: bytes,
    *,
    zstd_level: int | None = None,
) -> bytes:
    """Encode ``data`` through codec names in array order (§8.2).

    Args:
        chain: Codec names in the order applied by a writer.
        data: Payload bytes to transform.
        zstd_level: Optional per-frame zstd compression level used by ``zstd``
            and by each independent ``zstd-rsyncable`` block. ``None`` preserves
            the library default used before this option existed.
    """
    if zstd_level is not None and not any(
        name in ("zstd", "zstd-rsyncable") for name in chain
    ):
        msg = "zstd_level requires a zstd or zstd-rsyncable transform"
        raise ValueError(msg)
    for name in chain:
        data = _encode_one(name, data, zstd_level=zstd_level)
    return data


def decode_chain(
    chain: list[Codec],
    data: bytes,
    *,
    decrypt: Decryptor | None = None,
    checked: bool = False,
) -> bytes:
    """Reverse a resolved codec chain, last to first (§6.1, §8.2).

    ``checked`` says ``data`` already passed :func:`check_zstd_frames`, so a zstd codec
    applied last on encode is not walked again.

    ``encrypt``-class codecs are handed to ``decrypt`` (a key-bound callback the
    reader supplies); if none is given, the frame degrades to ``missing-key`` —
    keeping this module free of any crypto dependency.
    """
    for step, codec in enumerate(reversed(chain)):
        if codec.cls == "encrypt":
            if decrypt is None:
                raise CodecUnavailableError("missing-key", f"no key for {codec.name!r}")
            data = decrypt(data)
        else:
            data = _decode_one(codec, data, checked=checked and step == 0)
    return data

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Encode and place P5 geometry-1 parity frames over stored items."""

from collections.abc import Sequence
from functools import cache

import cbor2
from blake3 import blake3

from .frame import _PARITY, decode_frame, encode_plain_frame

_SPAN_LENGTH = 1 << 22
_DATA_COLUMNS = 64
_PARITY_ROWS = 4


def _field_tables() -> tuple[tuple[int, ...], tuple[int, ...]]:
    powers = []
    logs = [0] * 256
    value = 1
    for exponent in range(255):
        powers.append(value)
        logs[value] = exponent
        value <<= 1
        if value >= 256:
            value ^= 0x11D
    return tuple(powers), tuple(logs)


_POWERS, _LOGS = _field_tables()


@cache
def _products(coefficient: int) -> bytes:
    power = _LOGS[coefficient]
    return bytes([0]) + bytes(
        _POWERS[(power + _LOGS[value]) % 255] for value in range(1, 256)
    )


def _parity_shards(data: list[bytearray], size: int) -> list[bytes]:
    depth = (len(data) + _DATA_COLUMNS - 1) // _DATA_COLUMNS
    result = []
    for stripe in range(depth):
        columns = data[stripe::depth]
        for row in range(_PARITY_ROWS):
            shard = 0
            for column, block in enumerate(columns):
                # Multiply every byte by the Cauchy entry, then XOR in bulk.
                coefficient = _POWERS[-_LOGS[(255 - row) ^ column] % 255]
                shard ^= int.from_bytes(block.translate(_products(coefficient)))
            result.append(shard.to_bytes(size))
    return result


def _shard_digest(shard: bytes | bytearray) -> bytes:
    return blake3(shard, derive_key_context="gts2 shard").digest(length=16)


def _encode_span(span: bytearray, first: int, opener: bytes, anchor: bytes) -> bytes:
    length = len(span)
    size = max(64, min(4096, 1 << ((length - 1) // _DATA_COLUMNS).bit_length()))
    data = [span[offset : offset + size] for offset in range(0, length, size)]
    data[-1] = data[-1].ljust(size, b"\x00")
    parity = _parity_shards(data, size)
    data_digests = [_shard_digest(shard) for shard in data]
    parity_digests = [_shard_digest(shard) for shard in parity]
    table = blake3(
        cbor2.dumps([opener, data_digests, parity_digests]),
        derive_key_context="gts2 parity table",
    ).digest()
    return encode_plain_frame(
        anchor,
        _PARITY,
        [1, first, first + length, anchor, table],
        cbor2.dumps([data_digests, parity_digests, b"".join(parity)]),
    )


def _place_parity(
    items: Sequence[bytes], opener: bytes, closing_index: int | None
) -> tuple[list[bytes], int | None]:
    """Return the stored items with parity and the new position of the index."""
    result = [items[0]]
    pending = bytearray(items[0])
    first = 0
    index_position = None
    short_written = False
    for position in range(1, len(items)):
        item = items[position]
        frame = decode_frame(item)
        if frame.transparent:
            raise ValueError(
                "automatic parity requires a segment without existing parity"
            )
        if position == closing_index:
            index_position = len(result)
        result.append(item)
        if short_written:
            continue
        view = memoryview(item)
        offset = 0
        while offset < len(view):
            end = min(len(view), offset + _SPAN_LENGTH - len(pending))
            pending.extend(view[offset:end])
            offset = end
            if len(pending) == _SPAN_LENGTH:
                result.append(_encode_span(pending, first, opener, frame.identifier))
                first += _SPAN_LENGTH
                pending.clear()
        if position == closing_index and pending:
            result.append(_encode_span(pending, first, opener, frame.identifier))
            pending.clear()
            short_written = True
    return result, index_position

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Encode and place P5 geometry-1 parity frames over stored items."""

from collections.abc import Sequence

import cbor2
from blake3 import blake3

from ._parity_shards import _DATA_COLUMNS, _parity_shards, _shard_digest
from .frame import _PARITY, decode_frame, encode_plain_frame

_SPAN_LENGTH = 1 << 22


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

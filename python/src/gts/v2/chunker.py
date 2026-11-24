# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""The canonical GTS v2 chunker of P4 section 2."""

from collections.abc import Iterator
from struct import unpack

from blake3 import blake3

_WINDOW = 64
_MIN = 1 << 14
_SCALE = 1 << 16
_MAX = 1 << 18
_STRICT = 1 << 46
_LOOSE = 1 << 50
_REGISTER = (1 << 64) - 1
_GEAR = unpack(
    ">256Q", blake3(derive_key_context="gts2 gear table").digest(length=2048)
)


def chunks(data: bytes) -> Iterator[memoryview]:
    """Yield non-empty canonical chunks as spans of this byte string."""
    view = memoryview(data)
    start = 0
    while len(data) - start > _MIN:
        first = start + _MIN - 1
        limit = min(start + _MAX, len(data))
        fingerprint = 0
        for pos in range(first - _WINDOW + 1, first):
            fingerprint = ((fingerprint << 1) + _GEAR[data[pos]]) & _REGISTER
        end = limit
        for pos in range(first, limit):
            fingerprint = ((fingerprint << 1) + _GEAR[data[pos]]) & _REGISTER
            end = pos + 1
            if end == limit:
                break
            if fingerprint < (_STRICT if end - start < _SCALE else _LOOSE):
                break
        yield view[start:end]
        start = end
    if start < len(data):
        yield view[start:]

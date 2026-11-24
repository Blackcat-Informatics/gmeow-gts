# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS geometry-1 shard arithmetic (P5 §§6.1–6.4)."""

from collections.abc import Sequence
from functools import cache
from itertools import islice

from blake3 import blake3

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


def _multiply(left: int, right: int) -> int:
    if not left or not right:
        return 0
    return _POWERS[(_LOGS[left] + _LOGS[right]) % 255]


def _inverse(value: int) -> int:
    assert value
    return _POWERS[-_LOGS[value] % 255]


def _coefficient(row: int, column: int) -> int:
    return _inverse((255 - row) ^ column)


@cache
def _products(coefficient: int) -> bytes:
    return bytes(_multiply(coefficient, value) for value in range(256))


def _scale(shard: bytes | bytearray, coefficient: int) -> int:
    return int.from_bytes(shard.translate(_products(coefficient)))


def _combine(columns: Sequence[bytes | bytearray | None], row: int) -> int:
    shard = 0
    for column, block in enumerate(columns):
        if block is not None:
            shard ^= _scale(block, _coefficient(row, column))
    return shard


def _parity_shards(data: list[bytearray], size: int) -> list[bytes]:
    depth = (len(data) + _DATA_COLUMNS - 1) // _DATA_COLUMNS
    return [
        _combine(data[stripe::depth], row).to_bytes(size)
        for stripe in range(depth)
        for row in range(_PARITY_ROWS)
    ]


def _shard_digest(shard: bytes | bytearray) -> bytes:
    return blake3(shard, derive_key_context="gts2 shard").digest(length=16)


def _solve_columns(
    columns: list[bytes | bytearray | None],
    parity: Sequence[bytes | None],
    size: int,
) -> dict[int, bytes]:
    missing = [column for column, block in enumerate(columns) if block is None]
    held_rows = ((row, block) for row, block in enumerate(parity) if block is not None)
    equations = [
        (
            [_coefficient(row, column) for column in missing],
            int.from_bytes(block) ^ _combine(columns, row),
        )
        for row, block in islice(held_rows, len(missing))
    ]
    assert len(equations) == len(missing)
    # Leading square submatrices of these Cauchy rows are invertible.
    for column, (row, rhs) in enumerate(equations):
        factor = _inverse(row[column])
        row = [_multiply(value, factor) for value in row]
        rhs = _scale(rhs.to_bytes(size), factor)
        equations[column] = row, rhs
        for other, (other_row, other_rhs) in enumerate(equations):
            if other == column:
                continue
            factor = other_row[column]
            equations[other] = (
                [a ^ _multiply(b, factor) for a, b in zip(other_row, row, strict=True)],
                other_rhs ^ _scale(rhs.to_bytes(size), factor),
            )
    return {
        column: rhs.to_bytes(size)
        for column, (_, rhs) in zip(missing, equations, strict=True)
    }


def _solve_shards(
    covered: bytearray,
    parity: bytes,
    size: int,
    bad_data: set[int],
    bad_parity: set[int],
) -> tuple[dict[int, bytes], dict[int, bytes]] | None:
    """Solve every erased shard, or refuse the span if a stripe has over four."""
    depth = len(parity) // (size * _PARITY_ROWS)
    counts = [0] * depth
    for index in bad_data:
        counts[index % depth] += 1
    for index in bad_parity:
        counts[index // _PARITY_ROWS] += 1
    if max(counts) > _PARITY_ROWS:
        return None
    solved_data: dict[int, bytes] = {}
    solved_parity: dict[int, bytes] = {}
    data_count = (len(covered) + size - 1) // size
    for stripe, count in enumerate(counts):
        if not count:
            continue
        indices = range(stripe, data_count, depth)
        columns: list[bytes | bytearray | None] = [
            None
            if index in bad_data
            else covered[index * size : (index + 1) * size].ljust(size, b"\0")
            for index in indices
        ]
        first_row = stripe * _PARITY_ROWS
        rows = range(first_row, first_row + _PARITY_ROWS)
        blocks = [
            None if row in bad_parity else parity[row * size : (row + 1) * size]
            for row in rows
        ]
        if missing := _solve_columns(columns, blocks, size):
            for column, shard in missing.items():
                columns[column] = shard
                solved_data[indices[column]] = shard
        for row, block in enumerate(blocks):
            if block is None:
                solved_parity[rows[row]] = _combine(columns, row).to_bytes(size)
    return solved_data, solved_parity

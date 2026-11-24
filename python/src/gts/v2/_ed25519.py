# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Evaluate the public Ed25519 verification equation required by P6 §4.1."""

from hashlib import sha512
from typing import Literal

type Ed25519Error = Literal[
    "non_canonical_scalar", "small_order_key", "small_order_nonce", "equation_failed"
]
type _Point = tuple[int, int, int, int]

_P = (1 << 255) - 19
_L = (1 << 252) + 27742317777372353535851937790883648493
_D = -121665 * pow(121666, -1, _P) % _P
_SQRT_MINUS_ONE = pow(2, (_P - 1) // 4, _P)
_Y_MASK = (1 << 255) - 1
_IDENTITY = (0, 1, 1, 0)
_BX = 15112221349535400772501151409588531511454012693041857206046113283949847762202
_BY = 4 * pow(5, -1, _P) % _P
_BASE = (_BX, _BY, 1, _BX * _BY % _P)
_SMALL_ORDER = frozenset(
    (
        (1).to_bytes(32, "little"),
        (_P - 1).to_bytes(32, "little"),
        bytes(32),
        bytes(31) + b"\x80",
        bytes.fromhex(
            "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05"
        ),
        bytes.fromhex(
            "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85"
        ),
        bytes.fromhex(
            "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a"
        ),
        bytes.fromhex(
            "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa"
        ),
    )
)


def _decode(encoded: bytes) -> _Point | None:
    number = int.from_bytes(encoded, "little")
    y, sign = number & _Y_MASK, number >> 255
    if y >= _P:
        return None
    yy = y * y % _P
    # For the Ed25519 constant d, -1/d is a non-square, so v cannot vanish.
    v = (_D * yy + 1) % _P
    square = (yy - 1) * pow(v, -1, _P) % _P
    x = pow(square, (_P + 3) // 8, _P)
    if x * x % _P != square:
        x = x * _SQRT_MINUS_ONE % _P
    if x * x % _P != square or (x == 0 and sign):
        return None
    if x & 1 != sign:
        x = _P - x
    return x, y, 1, x * y % _P


def _encode(point: _Point) -> bytes:
    x, y, z, _ = point
    affine = pow(z, -1, _P)
    sign = (x * affine % _P) & 1
    return ((y * affine % _P) | (sign << 255)).to_bytes(32, "little")


def _add(left: _Point, right: _Point) -> _Point:
    """Add extended Edwards coordinates with the complete a=-1 formulas."""
    x1, y1, z1, t1 = left
    x2, y2, z2, t2 = right
    a = (y1 - x1) * (y2 - x2)
    b = (y1 + x1) * (y2 + x2)
    c = 2 * _D * t1 * t2
    d = 2 * z1 * z2
    e, f, g, h = b - a, d - c, d + c, b + a
    return e * f % _P, g * h % _P, f * g % _P, e * h % _P


def _multiply(point: _Point, scalar: int) -> _Point:
    result = _IDENTITY
    while scalar:
        if scalar & 1:
            result = _add(result, point)
        point = _add(point, point)
        scalar >>= 1
    return result


def verify_ed25519(
    public_key: bytes, message: bytes, value: bytes
) -> Ed25519Error | None:
    """Check a length-validated public key and signature in P6 order."""
    nonce = value[:32]
    scalar = int.from_bytes(value[32:], "little")
    if scalar >= _L:
        return "non_canonical_scalar"
    if public_key in _SMALL_ORDER:
        return "small_order_key"
    if nonce in _SMALL_ORDER:
        return "small_order_nonce"
    key = _decode(public_key)
    if key is None:
        return "equation_failed"
    challenge = (
        int.from_bytes(sha512(nonce + public_key + message).digest(), "little") % _L
    )
    x, y, z, t = _multiply(key, challenge)
    recovered_nonce = _add(_multiply(_BASE, scalar), (-x, y, z, -t))
    return None if _encode(recovered_nonce) == nonce else "equation_failed"

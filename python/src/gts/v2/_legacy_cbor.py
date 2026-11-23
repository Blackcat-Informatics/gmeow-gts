# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Preserve the generic CBOR values used by the P9 conversion reading."""

from __future__ import annotations

import struct
from collections.abc import Iterable
from dataclasses import dataclass, field
from itertools import batched
from typing import Literal

import cbor2

from gts.wire import blake3_256

from .framing import _head, item_end


@dataclass(frozen=True, slots=True)
class FloatValue:
    bits: int


@dataclass(frozen=True, slots=True)
class SimpleValue:
    number: int


@dataclass(frozen=True, slots=True)
class InvalidText:
    octets: bytes


@dataclass(frozen=True, slots=True)
class TaggedValue:
    number: int
    value: Value


@dataclass(frozen=True, slots=True)
class MapValue:
    entries: tuple[tuple[Value, Value], ...]


type Value = (
    None
    | bool
    | int
    | bytes
    | str
    | FloatValue
    | SimpleValue
    | InvalidText
    | TaggedValue
    | MapValue
    | tuple[Value, ...]
)
type FloatForm = Literal["key", "wide", "shortest"]


@dataclass(frozen=True, slots=True)
class LegacyItem:
    offset: int
    end: int
    value: Value


@dataclass(slots=True)
class _Group:
    major: int
    argument: int | None
    left: int | None
    children: list[Value] = field(default_factory=list)

    def finish(self) -> Value:
        if self.major == 4:
            return tuple(self.children)
        if self.major == 5:
            return MapValue(tuple(batched(self.children, 2, strict=True)))
        assert self.argument is not None
        return TaggedValue(self.argument, self.children[0])


def _wide_bits(additional: int, bits: int) -> int:
    if additional == 27:
        return bits
    frac_width, exp_width, letter = (10, 5, "e") if additional == 25 else (23, 8, "f")
    exponent = (1 << exp_width) - 1
    if (bits >> frac_width) & exponent == exponent:
        sign = bits >> (frac_width + exp_width)
        fraction = bits & ((1 << frac_width) - 1)
        return (sign << 63) | (0x7FF << 52) | (fraction << (52 - frac_width))
    number = struct.unpack(
        f">{letter}", bits.to_bytes((frac_width + exp_width + 1) // 8)
    )[0]
    return int.from_bytes(struct.pack(">d", number))


def _read_string(
    data: bytes, position: int, length: int | None, *, text: bool
) -> tuple[bytes | str | InvalidText, int]:
    chunks = []
    if length is None:
        while data[position] != 0xFF:
            _, length, position = _head(data, position, position)
            assert length is not None
            chunks.append(data[position : position + length])
            position += length
        position += 1
    else:
        chunks.append(data[position : position + length])
        position += length
    if not text:
        return b"".join(chunks), position
    try:
        return "".join(chunk.decode("utf-8") for chunk in chunks), position
    except UnicodeDecodeError:
        return InvalidText(b"".join(chunks)), position


def _simple(argument: int, additional: int) -> Value:
    if additional >= 25:
        return FloatValue(_wide_bits(additional, argument))
    match argument:
        case 20:
            return False
        case 21:
            return True
        case 22:
            return None
        case _:
            return SimpleValue(argument)


def decode_legacy_item(data: bytes, offset: int = 0) -> LegacyItem:
    """Read one item without tag coercions, numeric key collisions or a depth cap."""
    end = item_end(data, offset)
    position = offset
    groups: list[_Group] = []
    while position < end:
        additional = data[position] & 31
        major, argument, position = _head(data, position, offset)
        value: Value
        if major in (2, 3):
            value, position = _read_string(data, position, argument, text=major == 3)
        elif major in (4, 5, 6):
            left = (
                1
                if major == 6
                else None
                if argument is None
                else argument * (major - 3)
            )
            group = _Group(major, argument, left)
            if left != 0:
                groups.append(group)
                continue
            value = group.finish()
        elif argument is None:
            value = groups.pop().finish()
        elif major == 7:
            value = _simple(argument, additional)
        else:
            value = argument if major == 0 else -1 - argument

        while groups:
            parent = groups[-1]
            parent.children.append(value)
            if parent.left is None:
                break
            parent.left -= 1
            if parent.left:
                break
            value = groups.pop().finish()
        else:
            return LegacyItem(offset, end, value)
    raise AssertionError("complete CBOR item did not produce a value")


def _encode_head(major: int, number: int) -> bytes:
    encoded = cbor2.dumps(number)
    return bytes([(major << 5) | encoded[0]]) + encoded[1:]


def _encode_float(bits: int, form: FloatForm) -> bytes:
    wide = bits.to_bytes(8)
    if form != "key":
        if (bits >> 52) & 0x7FF == 0x7FF:
            if bits & ((1 << 52) - 1):
                return b"\xf9\x7e\x00"
            return b"\xf9\xfc\x00" if bits >> 63 else b"\xf9\x7c\x00"
        if form == "shortest":
            number = struct.unpack(">d", wide)[0]
            for additional, letter in ((25, "e"), (26, "f")):
                try:
                    narrow = struct.pack(f">{letter}", number)
                except OverflowError:
                    continue
                if _wide_bits(additional, int.from_bytes(narrow)) == bits:
                    return bytes([0xE0 | additional]) + narrow
    return b"\xfb" + wide


@dataclass(slots=True)
class _MapOrder:
    entries: list[tuple[bytearray, Value]]


def encode_legacy_value(value: Value, *, floats: FloatForm = "key") -> bytes:
    """Encode a source value for key comparison or one P9 identity preimage."""
    encoded = bytearray()
    pending: list[tuple[Value | _MapOrder | bytearray, bytearray]] = [(value, encoded)]
    while pending:
        item, target = pending.pop()
        match item:
            case tuple():
                target.extend(_encode_head(4, len(item)))
                pending.extend((child, target) for child in reversed(item))
            case MapValue(entries):
                target.extend(_encode_head(5, len(entries)))
                if len(entries) == 1:
                    ((key, child),) = entries
                    pending.extend(((child, target), (key, target)))
                elif entries:
                    keys = [(bytearray(), child) for _, child in entries]
                    pending.append((_MapOrder(keys), target))
                    pending.extend(
                        (key, buf)
                        for (key, _), (buf, _) in zip(entries, keys, strict=True)
                    )
            case _MapOrder(entries):
                entries.sort(key=lambda entry: entry[0])
                for buf, child in reversed(entries):
                    pending.extend(((child, target), (buf, target)))
            case TaggedValue(number, child):
                target.extend(_encode_head(6, number))
                pending.append((child, target))
            case FloatValue(bits):
                target.extend(_encode_float(bits, floats))
            case SimpleValue(number):
                target.extend(
                    bytes([0xE0 | number]) if number < 24 else bytes([0xF8, number])
                )
            case InvalidText(octets):
                target.extend(_encode_head(3, len(octets)))
                target.extend(octets)
            case bytearray():
                target.extend(item)
            case _:
                target.extend(cbor2.dumps(item))
    return bytes(encoded)


def merge_legacy_maps(maps: Iterable[MapValue]) -> MapValue:
    """Apply the shallow merge, comparing keys by their encoded bytes."""
    by_key = {}
    for value in maps:
        for key, child in value.entries:
            by_key[encode_legacy_value(key)] = (key, child)
    return MapValue(tuple(by_key[key] for key in sorted(by_key)))


def source_id_matches(item: MapValue, *, header: bool = False) -> bool:
    """Judge a source ID against both item-wide float encodings of P9."""
    stored = [value for key, value in item.entries if key == "id"]
    if len(stored) != 1 or not isinstance(stored[0], bytes):
        return False
    excluded = ("id",) if header else ("id", "sig")
    content = MapValue(tuple((k, v) for k, v in item.entries if k not in excluded))
    forms: tuple[FloatForm, ...] = ("wide", "shortest")
    return any(
        blake3_256(encode_legacy_value(content, floats=form)) == stored[0]
        for form in forms
    )

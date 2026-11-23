# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""CBOR item boundaries and the first-item generation field (P1 §§2.3, 6)."""

from dataclasses import dataclass
from typing import Literal

type FramingReason = Literal["torn_tail", "undelimitable_input"]
type GenerationGrammar = Literal["record", "first_generation"]

# Positive stack entries count outstanding children of a definite item.
# Negative entries describe the current position inside an indefinite item.
_MAP_VALUE = -1
_MAP_KEY = -2
_ARRAY = -3
_BYTES = -4
_TEXT = -5


class ItemFramingError(ValueError):
    """A framing failure with the offset of the first byte of the item."""

    def __init__(self, reason: FramingReason, offset: int) -> None:
        self.reason = reason
        self.offset = offset
        super().__init__(f"{reason} at offset {offset}")


@dataclass(frozen=True, slots=True)
class GenerationField:
    grammar: GenerationGrammar
    value: int


def _head(
    data: bytes | memoryview, offset: int, item_start: int
) -> tuple[int, int | None, int]:
    """Read a CBOR head without applying the deterministic GTS grammar."""
    if offset == len(data):
        raise ItemFramingError("torn_tail", item_start)
    major, additional = divmod(data[offset], 32)
    offset += 1
    if additional < 24:
        return major, additional, offset
    if additional <= 27:
        end = offset + (1 << (additional - 24))
        if end > len(data):
            raise ItemFramingError("torn_tail", item_start)
        argument = int.from_bytes(data[offset:end], "big")
        if major == 7 and additional == 24 and argument < 32:
            raise ItemFramingError("undelimitable_input", item_start)
        return major, argument, end
    if additional == 31 and major in (2, 3, 4, 5, 7):
        return major, None, offset
    raise ItemFramingError("undelimitable_input", item_start)


def item_end(data: bytes, offset: int = 0) -> int:
    """Walk one CBOR item at a known boundary and return its exclusive end.

    This walk checks well-formedness without allocating decoded values. The
    field decoder checks UTF-8 and deterministic encoding when required.
    """
    if not 0 <= offset <= len(data):
        raise ValueError("item offset is outside the input")
    item_start = offset
    parents = [1]
    while parents:
        parent = parents[-1]
        # P1 §6.3 decides a bad chunk from its initial byte even if its
        # announced head or body is cut short.
        if parent in (_BYTES, _TEXT) and offset < len(data):
            major, additional = divmod(data[offset], 32)
            chunk_major = 2 if parent == _BYTES else 3
            if data[offset] != 0xFF and (major != chunk_major or additional == 31):
                raise ItemFramingError("undelimitable_input", item_start)

        major, argument, offset = _head(data, offset, item_start)
        if major == 7 and argument is None:
            if parent >= 0 or parent == _MAP_VALUE:
                raise ItemFramingError("undelimitable_input", item_start)
            parents.pop()
        else:
            if parent > 0:
                parents[-1] -= 1
            elif parent in (_MAP_KEY, _MAP_VALUE):
                parents[-1] = _MAP_VALUE if parent == _MAP_KEY else _MAP_KEY

            if argument is None:
                parents.append((_BYTES, _TEXT, _ARRAY, _MAP_KEY)[major - 2])
            elif major in (2, 3):
                offset += argument
                if offset > len(data):
                    raise ItemFramingError("torn_tail", item_start)
            elif major in (4, 5) and argument:
                parents.append(argument if major == 4 else 2 * argument)
            elif major == 6:
                parents.append(1)

        while parents and parents[-1] == 0:
            parents.pop()
    return offset


def _is_version_key(data: bytes, offset: int) -> bool:
    major, length, body = _head(data, offset, offset)
    if major != 3:
        return False
    if length is not None:
        return length == 1 and data[body] == ord("v")
    # The item walk has already checked the chunks and the terminating break.
    found = False
    while data[body] != 0xFF:
        _, length, body = _head(data, body, offset)
        if length != 0:
            if length != 1 or found or data[body] != ord("v"):
                return False
            found = True
            body += 1
    return found


def generation_field(data: bytes) -> GenerationField | None:
    """Read the generation field of a complete first item.

    Return None when neither prefix grammar applies. This function does not
    recover from a torn first item or decide whether a generation is supported.
    """
    if not data:
        return None
    end = item_end(data)
    major, count, position = _head(data, 0, 0)
    if major == 4 and (count is None or count >= 2):
        kind_major, kind, position = _head(data, position, 0)
        if kind_major == 0 and kind in (1, 5):
            value_major, value, _ = _head(data, position, 0)
            if value_major == 0 and value is not None:
                return GenerationField("record", value)
        return None

    if major == 6 and count == 55799:
        major, count, position = _head(data, position, 0)
    if major != 5:
        return None

    map_end = end - 1 if count is None else end
    while position < map_end:
        value_position = item_end(data, position)
        if _is_version_key(data, position):
            value_major, value, _ = _head(data, value_position, 0)
            if value_major == 0 and value is not None:
                return GenerationField("first_generation", value)
            return None
        position = item_end(data, value_position)
    return None

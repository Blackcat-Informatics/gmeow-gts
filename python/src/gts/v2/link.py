# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P1 frame link inputs and decisions after item framing (P1 §§3.4, 5.1–5.2)."""

from dataclasses import dataclass
from typing import Literal

from ._decode import _Decoder
from .frame import FrameError, _frame_field_offsets, _identifier, _identity
from .framing import ItemFramingError, _byte_string_chunks, item_end

type LinkState = Literal["linked", "healed", "broken"]


@dataclass(frozen=True, slots=True)
class FrameLinkFields:
    offset: int
    next_offset: int
    transparent: bool
    identity_fields: memoryview | None
    stored_identifier: bytes | None


@dataclass(frozen=True, slots=True)
class FrameLink:
    state: LinkState
    computed_identifier: bytes | None
    effective_identifier: bytes | None


def _stored_digest(data: bytes, offset: int) -> bytes | None:
    # P1 §3.4 compares a 32-byte value even when its encoding is malformed.
    # The item walk has already checked the bounds and the chunk heads.
    if data[offset] >> 5 != 2:
        return None
    digest = bytearray()
    for chunk in _byte_string_chunks(data, offset):
        if len(digest) + len(chunk) > 32:
            return None
        digest.extend(chunk)
    return bytes(digest) if len(digest) == 32 else None


def inspect_link_item(
    data: bytes, offset: int = 0
) -> tuple[FrameLinkFields | None, int]:
    """Walk one item and retain the fields needed for a link check.

    A complete non-frame item returns None and its end. The envelope test uses
    the deterministic array head and item kind. Other malformed fields leave
    the item a frame. The caller settles framing and recovery before using an
    item as a witness.
    """
    if not 0 <= offset <= len(data):
        raise ValueError("item offset is outside the input")
    if not data.startswith(b"\x8b\x02", offset):
        return None, item_end(data, offset)

    try:
        starts = _frame_field_offsets(data, offset)
    except ItemFramingError as error:
        raise ItemFramingError(error.reason, offset, error.examined_end) from error
    return _inspect_frame_link(data, offset, starts), starts[-1]


def _inspect_frame_link(
    data: bytes, offset: int, starts: tuple[int, ...]
) -> FrameLinkFields:
    # Elements one through six are exactly the identity field run.
    identity_start, identity_end = starts[0], starts[6]
    decoder = _Decoder(data, identity_start, FrameError)
    try:
        _identity(decoder, newer=None)
    except FrameError:
        # With value checks deferred, this is a type, length or encoding error.
        identity = None
    else:
        identity = memoryview(data)[identity_start:identity_end]

    next_offset = starts[-1]
    return FrameLinkFields(
        offset=offset,
        next_offset=next_offset,
        transparent=data[starts[2]] == 0xF5,
        identity_fields=identity,
        stored_identifier=_stored_digest(data, starts[8]),
    )


def check_frame_link(
    frame: FrameLinkFields, predecessor: bytes, witness: FrameLinkFields | None = None
) -> FrameLink:
    """Resolve a frame link against its predecessor and optional next item.

    Supply a witness only after framing accepts the next item as a frame in
    the same segment. A recovery boundary or read limit supplies no witness.
    The returned effective identifier advances the predecessor when present.
    """
    computed = (
        _identifier(predecessor, frame.identity_fields)
        if frame.identity_fields is not None
        else None
    )
    stored = frame.stored_identifier
    state: LinkState = "broken"
    if computed is not None and computed == stored:
        state = "linked"
    elif (
        not frame.transparent
        and stored is not None
        and computed is not None
        and witness is not None
        and witness.offset == frame.next_offset
        and witness.identity_fields is not None
        and _identifier(computed, witness.identity_fields) == witness.stored_identifier
    ):
        state = "healed"
    effective = None
    if not frame.transparent:
        effective = computed if state == "healed" or stored is None else stored
    return FrameLink(state, computed, effective)

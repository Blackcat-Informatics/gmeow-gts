# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Decode a GTS v2 pack directory and its member bodies (P4 §4)."""

from dataclasses import dataclass
from enum import IntEnum

from ._content import content_code
from ._decode import WireError, _Decoder, _immutable_octets
from .rebuild import Entry, EntryListError, decode_entries


class PackError(WireError):
    """The first content violation, relative to the start of the payload."""


class MemberKind(IntEnum):
    CHUNK = 1
    BLOB = 2
    RECIPE = 3
    VALUE = 4


@dataclass(frozen=True, slots=True)
class PackMember:
    kind: MemberKind
    digest: bytes
    media_type: str | None
    total_length: int
    body: memoryview
    entries: tuple[Entry, ...]


@dataclass(frozen=True, slots=True)
class _DirectoryEntry:
    kind: MemberKind
    digest: bytes
    media_type: str | None
    body_length: int
    total_length: int
    entry_count: int


def _media_type(cursor: _Decoder, registry_version: int) -> str | None:
    start = cursor.offset
    size = cursor.array()
    if size == 0:
        raise cursor.error("wrong_length", start)
    tag = content_code(cursor, 2, registry_version)
    if size != tag:
        raise cursor.error("wrong_length", start)
    return cursor.text_value() if tag == 2 else None


def _decode_member(cursor: _Decoder, registry_version: int) -> _DirectoryEntry:
    start = cursor.offset
    size = cursor.array()
    if size == 0:
        raise PackError("wrong_length", start)
    kind = MemberKind(content_code(cursor, 4, registry_version))
    if size != 3:
        raise PackError("wrong_length", start)
    cursor.record((1, 2, 4, 3)[kind - 1])
    digest = bytes(cursor.bytes(32))
    declaration = kind >= MemberKind.RECIPE
    total_length = cursor.uint() if declaration else 0
    media_type = (
        _media_type(cursor, registry_version)
        if kind in (MemberKind.BLOB, MemberKind.RECIPE)
        else None
    )
    entry_count = cursor.uint() if declaration else 0
    body_length = cursor.uint()
    return _DirectoryEntry(
        kind,
        digest,
        media_type,
        body_length,
        total_length if declaration else body_length,
        entry_count,
    )


def decode_pack(
    payload: bytes | memoryview,
    *,
    member_count: int,
    unit_length: int,
    registry_version: int = 1,
) -> tuple[PackMember, ...]:
    """Validate the outer record and each member before returning the pack.

    Member digests are checked separately so a bad unit does not hide the
    other members.
    """
    if member_count == 0:
        raise PackError("out_of_range", 0)
    payload = _immutable_octets(payload)
    cursor = _Decoder(payload, 0, PackError)
    cursor.record(2)
    directory = [
        _decode_member(cursor, registry_version) for _ in range(cursor.array())
    ]
    bodies = cursor.bytes()
    if cursor.offset != len(payload):
        raise PackError("wrong_length", cursor.offset)
    if len(directory) != member_count:
        raise PackError("inconsistent_counts", 0)
    if sum(entry.body_length for entry in directory) != len(bodies):
        raise PackError("inconsistent_counts", 0)
    if (
        sum(entry.body_length for entry in directory if entry.kind <= MemberKind.BLOB)
        != unit_length
    ):
        raise PackError("inconsistent_counts", 0)

    members = []
    body_start = cursor.offset - len(bodies)
    offset = 0
    for entry in directory:
        end = offset + entry.body_length
        body = bodies[offset:end]
        entries: tuple[Entry, ...] = ()
        if entry.kind == MemberKind.CHUNK and not body:
            raise PackError("out_of_range", body_start + offset)
        if entry.kind >= MemberKind.RECIPE:
            try:
                entries = decode_entries(
                    body,
                    kind="recipe" if entry.kind == MemberKind.RECIPE else "value",
                    entry_count=entry.entry_count,
                    total_length=entry.total_length,
                )
            except EntryListError as error:
                raise PackError(
                    error.reason, body_start + offset + error.offset
                ) from error
        members.append(
            PackMember(
                entry.kind,
                entry.digest,
                entry.media_type,
                entry.total_length,
                body,
                entries,
            )
        )
        offset = end
    return tuple(members)

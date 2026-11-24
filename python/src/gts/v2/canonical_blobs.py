# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Canonical value packs, blob packs, and external declarations of P4."""

from collections.abc import Iterable, Iterator, Mapping
from dataclasses import dataclass

import cbor2

from .blobs import BlobEntry, BlobName, BlobSet
from .canonical_graph import EncodedFrame
from .chunker import chunks
from .pack import MemberKind
from .rebuild import unit_digest

_PACK_BYTES = 1 << 24
type _Media = tuple[int] | tuple[int, str]
type _Unit = tuple[bytes, memoryview]


@dataclass(frozen=True, slots=True)
class _Member:
    kind: MemberKind
    content: list[object]
    body: bytes | memoryview


@dataclass(frozen=True, slots=True)
class _Blob:
    entry: BlobEntry
    units: tuple[_Unit, ...]
    media: tuple[_Media, ...]


def _units(data: bytes) -> Iterator[_Unit]:
    for part in chunks(data):
        yield unit_digest(part), part


def _entries(units: tuple[_Unit, ...]) -> bytes:
    return cbor2.dumps([(digest, len(part)) for digest, part in units])


def _pack(members: list[_Member]) -> EncodedFrame:
    unit_length = sum(
        len(item.body) for item in members if item.kind <= MemberKind.BLOB
    )
    directory = [[item.kind, item.content, len(item.body)] for item in members]
    bodies = b"".join(item.body for item in members)
    return 13, [len(members), unit_length], cbor2.dumps([directory, bodies])


def _packs(members: Iterable[_Member]) -> Iterator[EncodedFrame]:
    batch: list[_Member] = []
    used = 0
    for member in members:
        length = len(member.body)
        if batch and used + length > _PACK_BYTES:
            yield _pack(batch)
            batch = []
            used = 0
        batch.append(member)
        used += length
    if batch:
        yield _pack(batch)


def _value_members(values: Iterable[tuple[bytes, bytes]]) -> Iterator[_Member]:
    emitted: set[bytes] = set()
    for digest, data in values:
        units = tuple(_units(data))
        for unit, body in units:
            if unit not in emitted:
                emitted.add(unit)
                yield _Member(MemberKind.CHUNK, [unit], body)
        yield _Member(
            MemberKind.VALUE, [digest, len(data), len(units)], _entries(units)
        )


def _blob_members(blobs: Iterable[_Blob]) -> Iterator[_Member]:
    live = [blob for blob in blobs if blob.entry.present]
    single = {
        blob.entry.name[1]: blob.media[0] for blob in live if len(blob.units) == 1
    }
    emitted: set[bytes] = set()
    for blob in live:
        for unit, body in blob.units:
            if unit in emitted:
                continue
            emitted.add(unit)
            if unit in single:
                yield _Member(MemberKind.BLOB, [unit, single[unit]], body)
            else:
                yield _Member(MemberKind.CHUNK, [unit], body)
        if len(blob.units) > 1:
            yield _Member(
                MemberKind.RECIPE,
                [blob.entry.name[1], blob.entry.length, blob.media[0], len(blob.units)],
                _entries(blob.units),
            )


def _plan_blobs(blobs: BlobSet, verified: Mapping[BlobName, bytes]) -> Iterator[_Blob]:
    for entry in sorted(blobs.entries, key=lambda item: cbor2.dumps(item.name)):
        media: tuple[_Media, ...] = tuple(
            (2, text) for text in sorted(entry.media_types, key=cbor2.dumps)
        ) or ((1,),)
        units: tuple[_Unit, ...] = ()
        if entry.present:
            data = verified.get(entry.name)
            if data is None:
                raise ValueError(
                    "canonical blob region requires bytes for every present blob"
                )
            if len(data) != entry.length or (1, unit_digest(data)) != entry.name:
                raise ValueError(
                    "present blob bytes do not match its length and digest"
                )
            units = tuple(_units(data)) or ((entry.name[1], memoryview(data)),)
        yield _Blob(entry, units, media)


def canonical_value_frames(value_octets: Iterable[bytes]) -> Iterator[EncodedFrame]:
    """Yield the packs of the canonical value region."""
    values = sorted(
        ((unit_digest(data), data) for data in set(value_octets)),
        key=lambda pair: pair[0],
    )
    if any(not data for _, data in values):
        raise ValueError("canonical value octets must be non-empty")
    return _packs(_value_members(values))


def canonical_blob_frames(
    blobs: BlobSet, verified: Mapping[BlobName, bytes]
) -> Iterator[EncodedFrame]:
    """Yield the packs and external frames of the canonical blob region."""
    return _blob_frames(tuple(_plan_blobs(blobs, verified)))


def _blob_frames(plans: tuple[_Blob, ...]) -> Iterator[EncodedFrame]:
    yield from _packs(_blob_members(plans))
    for blob in plans:
        media = blob.media[1:] if blob.entry.present else blob.media
        for option in media:
            yield 12, [blob.entry.name, blob.entry.length, option], b""


def canonical_binary_frames(
    value_octets: Iterable[bytes], blobs: BlobSet, verified: Mapping[BlobName, bytes]
) -> Iterator[EncodedFrame]:
    """Yield the canonical value region, blob region, and external frames."""
    value_frames = canonical_value_frames(value_octets)
    blob_frames = canonical_blob_frames(blobs, verified)
    yield from value_frames
    yield from blob_frames

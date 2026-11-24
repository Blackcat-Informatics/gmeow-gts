# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Validate and rebuild GTS v2 recipes and values (P4 §§3, 6)."""

from dataclasses import dataclass, field
from itertools import islice
from typing import Literal

from blake3 import blake3

from ._decode import WireError, _Decoder, _immutable_octets

_MAX_UINT = (1 << 64) - 1
_UNIT_CONTEXT = "gts2 unit"

type _Octets = bytes | memoryview
type RebuildKind = Literal["recipe", "value"]
type RebuildFailure = Literal["chunk", "blob_length", "blob_digest"]


class EntryListError(WireError):
    """The first content violation in a recipe or value declaration."""


@dataclass(frozen=True, slots=True)
class Entry:
    digest: bytes
    length: int


def decode_entries(
    payload: bytes | memoryview,
    *,
    kind: RebuildKind,
    entry_count: int,
    total_length: int,
) -> tuple[Entry, ...]:
    """Validate the decoded entry list against its enclosing declaration."""
    if entry_count < (2 if kind == "recipe" else 1):
        raise EntryListError("out_of_range", 0)
    payload = _immutable_octets(payload)
    cursor = _Decoder(payload, 0, EntryListError)
    length = cursor.array()
    entries = []
    for _ in range(length):
        cursor.record(2)
        entries.append(Entry(bytes(cursor.bytes(32)), cursor.uint()))
    if cursor.offset != len(payload):
        raise EntryListError("wrong_length", cursor.offset)
    if length != entry_count:
        raise EntryListError("inconsistent_counts", 0)
    if any(entry.length == 0 for entry in entries):
        raise EntryListError("out_of_range", 0)
    actual_length = sum(entry.length for entry in entries)
    if actual_length > _MAX_UINT:
        raise EntryListError("out_of_range", 0)
    if actual_length != total_length:
        raise EntryListError("inconsistent_counts", 0)
    return tuple(entries)


def unit_digest(data: _Octets) -> bytes:
    """Return the BLAKE3 name of the given unit, blob or value octets."""
    return blake3(data, derive_key_context=_UNIT_CONTEXT).digest()


@dataclass(frozen=True, slots=True)
class _VerifiedUnit:
    position: int
    data: _Octets


class UnitIndex:
    """The first verified unit per digest in one segment."""

    def __init__(self) -> None:
        self._units: dict[bytes, _VerifiedUnit] = {}

    def add(self, position: int, digest: bytes, data: _Octets) -> bool:
        """Check one copy and index it if verified. Add units in file order.

        Pass only units whose enclosing payload passed its own checks. A false
        result means the copy has the P4 payload_digest finding. Index all units
        of a pack before processing any declaration in that pack.
        """
        data = _immutable_octets(data)
        if unit_digest(data) != digest:
            return False
        self._units.setdefault(digest, _VerifiedUnit(position, data))
        return True

    def _at_or_before(self, position: int, digest: bytes) -> _Octets | None:
        """Find the first verified copy when it is in range for a declaration."""
        unit = self._units.get(digest)
        if unit is None or unit.position > position:
            return None
        return unit.data

    def first_position(self, digest: bytes) -> int | None:
        """Return the first verified copy's position, if there is one."""
        unit = self._units.get(digest)
        return None if unit is None else unit.position


@dataclass(frozen=True, slots=True)
class VerifiedRebuild:
    """Verified octets held as references to their stored units."""

    _parts: tuple[_Octets, ...]

    def to_bytes(self) -> bytes:
        """Materialize the concatenation for a caller that needs one byte string."""
        return b"".join(self._parts)


@dataclass(frozen=True, slots=True)
class IncompleteRebuild:
    reason: RebuildFailure
    later_unit_missing: bool | None = field(default=None, compare=False)

    @property
    def all_units_present(self) -> bool:
        if self.reason == "blob_length" and self.later_unit_missing is None:
            raise ValueError("recipe length failure lacks unit availability")
        return self.reason != "chunk" and not self.later_unit_missing


@dataclass(frozen=True, slots=True)
class RefusedRebuild:
    count: int


@dataclass(frozen=True, slots=True)
class UncheckedRebuild:
    """The access mode does not run rebuilds."""


type RebuildResult = (
    VerifiedRebuild | IncompleteRebuild | RefusedRebuild | UncheckedRebuild
)


class RebuildBudget:
    """Charge folded declarations in file order across a read."""

    def __init__(self, limit: int, *, enabled: bool = True) -> None:
        if type(limit) is not int or not 0 <= limit <= _MAX_UINT:
            raise ValueError("rebuild capacity is outside uint64")
        self._limit = limit
        self._enabled = enabled
        self._total = 0
        self._refusing = False

    def rebuild(
        self,
        index: UnitIndex,
        *,
        position: int,
        digest: bytes,
        total_length: int,
        entries: tuple[Entry, ...],
        kind: RebuildKind | None = None,
    ) -> RebuildResult:
        """Charge a folded declaration with validated entries, then rebuild it."""
        if type(total_length) is not int or not 0 <= total_length <= _MAX_UINT:
            raise ValueError("rebuild charge is outside uint64")
        if not self._enabled:
            return UncheckedRebuild()
        charge = self._total + total_length
        if self._refusing or charge > self._limit:
            self._refusing = True
            return RefusedRebuild(min(charge, _MAX_UINT))
        self._total = charge

        hasher = blake3(derive_key_context=_UNIT_CONTEXT)
        parts = []
        for ordinal, entry in enumerate(entries):
            unit = index._at_or_before(position, entry.digest)
            if unit is None:
                return IncompleteRebuild("chunk")
            if len(unit) != entry.length:
                # Keep the first P4 failure. P7 compaction also needs to
                # distinguish whether this declaration has every unit.
                missing = (
                    any(
                        index._at_or_before(position, following.digest) is None
                        for following in islice(entries, ordinal + 1, None)
                    )
                    if kind == "recipe"
                    else None
                )
                return IncompleteRebuild("blob_length", missing)
            parts.append(unit)
            hasher.update(unit)
        if hasher.digest() != digest:
            return IncompleteRebuild("blob_digest")
        return VerifiedRebuild(tuple(parts))

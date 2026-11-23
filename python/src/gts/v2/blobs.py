# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Collect folded GTS v2 blobs, units and values (P4 §§3–6)."""

from bisect import bisect_left
from collections.abc import Iterable
from dataclasses import dataclass, field
from typing import Literal

import cbor2

from ._content import ContentError, content_code
from ._decode import _Decoder, _immutable_octets
from .frame import Frame
from .pack import MemberKind, PackMember, decode_pack
from .rebuild import (
    IncompleteRebuild,
    RebuildBudget,
    RebuildResult,
    RefusedRebuild,
    UncheckedRebuild,
    UnitIndex,
    VerifiedRebuild,
    decode_entries,
)

type BlobName = tuple[int, bytes]
type BlobState = Literal["complete", "incomplete", "unchecked", "external"]


@dataclass(frozen=True, slots=True)
class ExternalBlob:
    name: BlobName
    length: int
    media_type: str | None


type BlobContent = PackMember | tuple[PackMember, ...] | ExternalBlob


def decode_blob_content(
    frame: Frame, decoded: bytes | memoryview, *, registry_version: int = 1
) -> BlobContent:
    """Interpret P4 content after the envelope, link and payload checks pass."""
    kind = frame.frame_type
    decoder = _Decoder(frame.content, 0, ContentError)
    match kind:
        case 12:
            decoder.record(3)
            decoder.record(2)
            algorithm = content_code(decoder, 1, registry_version)
            name = (algorithm, bytes(decoder.bytes(32)))
            return ExternalBlob(
                name, decoder.uint(), decoder.option(decoder.text_value)
            )
        case 13:
            decoder.record(2)
            return decode_pack(
                decoded,
                member_count=decoder.uint(),
                unit_length=decoder.uint(),
                registry_version=registry_version,
            )
        case 8 | 9 | 10 | 11:
            member_kind = MemberKind(kind - 7)
        case _:
            raise ValueError("frame is outside the blob part")

    decoder.record((1, 2, 4, 3)[member_kind - 1])
    digest = bytes(decoder.bytes(32))
    is_rebuild = member_kind >= MemberKind.RECIPE
    length = decoder.uint() if is_rebuild else frame.decoded_length
    media = (
        decoder.option(decoder.text_value)
        if member_kind in (MemberKind.BLOB, MemberKind.RECIPE)
        else None
    )
    body = memoryview(_immutable_octets(decoded))
    entries = (
        decode_entries(
            body,
            kind="recipe" if member_kind == MemberKind.RECIPE else "value",
            entry_count=decoder.uint(),
            total_length=length,
        )
        if is_rebuild
        else ()
    )
    return PackMember(member_kind, digest, media, length, body, entries)


@dataclass(frozen=True, slots=True)
class BlobLocation:
    segment: int
    position: int
    member: int | None = None


@dataclass(frozen=True, slots=True)
class BlobDeclaration:
    location: BlobLocation
    name: BlobName
    length: int
    media_type: str | None
    state: BlobState
    _payload: bytes | memoryview | VerifiedRebuild | None = None

    def bytes(self) -> bytes | None:
        """Deliver the declaration's bytes when the unit or rebuild verified."""
        if isinstance(self._payload, VerifiedRebuild):
            return self._payload.to_bytes()
        return None if self._payload is None else bytes(self._payload)


@dataclass(frozen=True, slots=True)
class BlobFinding:
    location: BlobLocation
    code: Literal[
        "damaged_frame", "missing_dependency", "exceeds_capacity", "unreferenced_unit"
    ]
    reason: (
        Literal[
            "payload_digest", "chunk", "blob_length", "blob_digest", "total_rebuilt"
        ]
        | None
    ) = None
    count: int | None = None


@dataclass(frozen=True, slots=True)
class BlobSegment:
    declarations: tuple[BlobDeclaration, ...]
    rebuilds: tuple[tuple[BlobLocation, RebuildResult], ...]
    findings: tuple[BlobFinding, ...]


@dataclass(frozen=True, slots=True)
class ValueSelection:
    declared: bool
    rebuilt: VerifiedRebuild | None
    refused: bool = False


@dataclass(frozen=True, slots=True)
class _ValueCandidate:
    location: BlobLocation
    outcome: RebuildResult


@dataclass(slots=True)
class _ValueCandidates:
    declared: list[_ValueCandidate] = field(default_factory=list)
    verified: list[_ValueCandidate] = field(default_factory=list)
    refused: list[_ValueCandidate] = field(default_factory=list)

    def add(self, location: BlobLocation, outcome: RebuildResult) -> None:
        candidate = _ValueCandidate(location, outcome)
        self._append_first(self.declared, candidate)
        if isinstance(outcome, VerifiedRebuild):
            self._append_first(self.verified, candidate)
        elif isinstance(outcome, RefusedRebuild):
            self._append_first(self.refused, candidate)

    @staticmethod
    def _append_first(
        candidates: list[_ValueCandidate], candidate: _ValueCandidate
    ) -> None:
        if (
            not candidates
            or candidates[-1].location.segment != candidate.location.segment
        ):
            candidates.append(candidate)

    @staticmethod
    def before(
        candidates: list[_ValueCandidate],
        first_segment: int,
        boundary: tuple[int, int],
    ) -> _ValueCandidate | None:
        index = bisect_left(candidates, first_segment, key=lambda c: c.location.segment)
        if index == len(candidates):
            return None
        candidate = candidates[index]
        where = candidate.location
        return candidate if (where.segment, where.position) < boundary else None


class BlobRead:
    """Collect P4 data with one rebuild budget over all segments of a read."""

    def __init__(self, rebuilt_bytes_per_read: int, *, whole: bool = True) -> None:
        self._whole = whole
        self._budget = RebuildBudget(rebuilt_bytes_per_read, enabled=whole)
        self._values: dict[tuple[bytes, int], _ValueCandidates] = {}

    def value_before(
        self,
        digest: bytes,
        length: int,
        *,
        segment: int,
        position: int,
        first_segment: int = 0,
    ) -> ValueSelection:
        """Select an already rebuilt value in the requested segment run.

        Report whether any candidate was declared and whether any of their
        rebuilds was refused in the whole-file read. The first eligible
        segment defaults to the start of the file.
        """
        candidates = self._values.get((digest, length))
        if candidates is None:
            return ValueSelection(False, None)
        before = (segment, position)
        declared = candidates.before(candidates.declared, first_segment, before)
        verified = candidates.before(candidates.verified, first_segment, before)
        refused = candidates.before(candidates.refused, first_segment, before)
        rebuilt = None if verified is None else verified.outcome
        assert rebuilt is None or isinstance(rebuilt, VerifiedRebuild)
        return ValueSelection(declared is not None, rebuilt, refused is not None)

    def fold_segment(
        self,
        segment: int,
        frames: Iterable[tuple[int, BlobContent]],
        *,
        algorithm: int = 1,
    ) -> BlobSegment:
        """Collect folded P4 frames in item order, processing segments in file order."""
        units = UnitIndex()
        chunks: list[tuple[bytes, BlobLocation]] = []
        referenced: set[bytes] = set()
        declarations: list[BlobDeclaration] = []
        rebuilds: list[tuple[BlobLocation, RebuildResult]] = []
        findings: list[BlobFinding] = []
        records: list[tuple[BlobLocation, PackMember | ExternalBlob]] = []
        for position, content in frames:
            if isinstance(content, tuple):
                records.extend(
                    (BlobLocation(segment, position, index), member)
                    for index, member in enumerate(content)
                )
            else:
                records.append((BlobLocation(segment, position), content))

        # Index the segment's units before any rebuild. Their positions limit
        # which declarations can use them, including every member of one pack.
        verified_blobs: dict[BlobLocation, bytes | memoryview] = {}
        for location, record in records:
            if isinstance(record, ExternalBlob) or record.kind > MemberKind.BLOB:
                continue
            data = _immutable_octets(record.body)
            verified = units.add(location.position, record.digest, data)
            if not verified:
                findings.append(
                    BlobFinding(location, "damaged_frame", "payload_digest")
                )
            if record.kind == MemberKind.CHUNK:
                chunks.append((record.digest, location))
            else:
                referenced.add(record.digest)
                if verified:
                    verified_blobs[location] = data

        for location, record in records:
            if isinstance(record, ExternalBlob):
                declarations.append(
                    BlobDeclaration(
                        location,
                        record.name,
                        record.length,
                        record.media_type,
                        "external",
                    )
                )
                continue
            if record.kind == MemberKind.CHUNK:
                continue
            if record.kind == MemberKind.BLOB:
                stored = verified_blobs.get(location)
                declarations.append(
                    BlobDeclaration(
                        location,
                        (algorithm, record.digest),
                        record.total_length,
                        record.media_type,
                        "complete" if stored is not None else "incomplete",
                        stored,
                    )
                )
                continue
            referenced.update(entry.digest for entry in record.entries)
            outcome = self._budget.rebuild(
                units,
                position=location.position,
                digest=record.digest,
                total_length=record.total_length,
                entries=record.entries,
            )
            rebuilds.append((location, outcome))
            state = _rebuild_state(location, outcome, findings)
            if record.kind == MemberKind.RECIPE:
                declarations.append(
                    BlobDeclaration(
                        location,
                        (algorithm, record.digest),
                        record.total_length,
                        record.media_type,
                        state,
                        outcome if isinstance(outcome, VerifiedRebuild) else None,
                    )
                )
            else:
                key = (record.digest, record.total_length)
                candidates = self._values.get(key)
                if candidates is None:
                    candidates = self._values[key] = _ValueCandidates()
                candidates.add(location, outcome)

        if self._whole:
            findings.extend(
                BlobFinding(location, "unreferenced_unit")
                for digest, location in chunks
                if digest not in referenced
            )
        return BlobSegment(tuple(declarations), tuple(rebuilds), tuple(findings))


def _rebuild_state(
    location: BlobLocation, outcome: RebuildResult, findings: list[BlobFinding]
) -> BlobState:
    if isinstance(outcome, RefusedRebuild):
        findings.append(
            BlobFinding(location, "exceeds_capacity", "total_rebuilt", outcome.count)
        )
        return "unchecked"
    if isinstance(outcome, IncompleteRebuild):
        code: Literal["missing_dependency", "damaged_frame"] = (
            "missing_dependency" if outcome.reason == "chunk" else "damaged_frame"
        )
        findings.append(BlobFinding(location, code, outcome.reason))
        return "incomplete"
    if isinstance(outcome, UncheckedRebuild):
        return "unchecked"
    return "complete"


@dataclass(frozen=True, slots=True)
class BlobEntry:
    name: BlobName
    length: int
    media_types: tuple[str, ...]
    state: BlobState

    @property
    def present(self) -> bool:
        return self.state != "external"


@dataclass(frozen=True, slots=True)
class BlobSet:
    entries: tuple[BlobEntry, ...]
    length_conflicts: tuple[BlobLocation, ...]


def blob_set(declarations: Iterable[BlobDeclaration]) -> BlobSet:
    """Compute deterministic blob entries and the locations of length conflicts."""
    by_name: dict[BlobName, list[BlobDeclaration]] = {}
    for declaration in declarations:
        by_name.setdefault(declaration.name, []).append(declaration)
    entries = []
    conflicts: list[BlobLocation] = []
    for name in sorted(by_name, key=cbor2.dumps):
        group = by_name[name]
        shortest = min(d.length for d in group)
        conflicts.extend(d.location for d in group if d.length != shortest)
        media = {d.media_type for d in group if d.media_type is not None}
        states = {d.state for d in group}
        state: BlobState = "external"
        if "complete" in states:
            state = "complete"
        elif "unchecked" in states:
            state = "unchecked"
        elif "incomplete" in states:
            state = "incomplete"
        entries.append(
            BlobEntry(name, shortest, tuple(sorted(media, key=cbor2.dumps)), state)
        )
    return BlobSet(tuple(entries), tuple(conflicts))

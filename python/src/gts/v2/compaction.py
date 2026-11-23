# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Write a P7 canonical segment from a whole-read fold."""

from collections import defaultdict
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import IntEnum
from itertools import chain
from typing import Literal

import cbor2

from ._content import UINT_MAX
from ._fold_record_lines import record_bodies
from .blobs import BlobDeclaration, BlobLocation, BlobName, BlobSet
from .canonical_blobs import canonical_binary_frames
from .canonical_graph import canonical_graph
from .canonical_records import canonical_record_frames
from .dictionary import TermPool
from .fold_labelling import RefusalReason
from .header import Header
from .index_claims import IndexStatistics
from .lines import LineResult
from .rebuild import IncompleteRebuild, RebuildResult, RefusedRebuild, UncheckedRebuild
from .record_fold import RecordFold
from .terms import TypedValue
from .writer import SegmentWriter

type _Code = Literal[
    "exceeds_capacity",
    "opaque_term",
    "labelling_refused",
    "missing_dependency",
    "damaged_frame",
]
type _Reason = (
    RefusalReason
    | Literal["total_formed", "total_rebuilt", "chunk", "blob_length", "blob_digest"]
)
_CODES: dict[_Code, int] = {
    "damaged_frame": 13,
    "missing_dependency": 19,
    "exceeds_capacity": 20,
    "opaque_term": 25,
    "labelling_refused": 26,
}
_REASONS: dict[_Reason, int] = {
    "blob_length": 14,
    "blob_digest": 15,
    "chunk": 24,
    "refusal_width": 32,
    "refusal_limit": 33,
    "refusal_size": 85,
    "total_rebuilt": 89,
    "total_formed": 90,
    "refusal_weight": 91,
    "refusal_term_node": 92,
}


class FileEnd(IntEnum):
    AT_AUTHENTICATED_COMMIT = 1
    AT_VALID_COMMIT = 2
    AFTER_COMMIT = 3
    NO_COMMIT = 4
    TORN = 5
    UNDELIMITABLE = 6
    NOT_READ = 7


@dataclass(frozen=True, slots=True)
class ReadCoverage:
    """The file-level coverage of the whole read that produced the fold."""

    end: FileEnd
    left_out: int

    def __post_init__(self) -> None:
        if not isinstance(self.end, FileEnd):
            raise ValueError("read coverage requires a file_end")
        if type(self.left_out) is not int or not 0 <= self.left_out <= UINT_MAX:
            raise ValueError("left_out must be a uint")

    @property
    def covers_content(self) -> bool:
        return self.left_out == 0 and self.end < FileEnd.TORN


@dataclass(frozen=True, slots=True, kw_only=True)
class CompactionSource:
    """The fold and observations from the same whole read."""

    terms: TermPool
    lines: LineResult
    records: RecordFold
    values: frozenset[tuple[int, TypedValue]]
    blobs: BlobSet
    declarations: tuple[BlobDeclaration, ...]
    rebuilds: Mapping[BlobLocation, RebuildResult]
    coverage: ReadCoverage
    content_digest: bytes | None
    blank_positions: Mapping[int, int] | None
    formed_refused: bool = False
    labelling_refusal: RefusalReason | None = None


@dataclass(frozen=True, slots=True)
class Written:
    file: bytes


@dataclass(frozen=True, slots=True)
class Stopped:
    code: _Code
    reason: _Reason | None = None


@dataclass(frozen=True, slots=True)
class CompactionResult:
    outcome: Written | Stopped
    coverage: ReadCoverage

    def encode(self) -> bytes:
        """Encode the P7 compaction_result record. Coverage is delivered beside it."""
        match self.outcome:
            case Written(file):
                return cbor2.dumps([1, file])
            case Stopped(code, reason):
                option = [1] if reason is None else [2, _REASONS[reason]]
                return cbor2.dumps([2, _CODES[code], option])


def _failed_blob(
    declarations: Sequence[BlobDeclaration],
    rebuilds: Mapping[BlobLocation, RebuildResult],
    length: int,
) -> Stopped:
    rebuilt = [
        (item, rebuilds.get(item.location))
        for item in declarations
        if item.state != "external"
    ]
    if not rebuilt:
        raise ValueError("present blob lacks a folded declaration")
    if any(isinstance(outcome, RefusedRebuild) for _, outcome in rebuilt):
        return Stopped("exceeds_capacity", "total_rebuilt")
    if any(
        isinstance(outcome, UncheckedRebuild)
        or (outcome is None and item.state == "unchecked")
        for item, outcome in rebuilt
    ):
        raise ValueError("compaction requires whole-read rebuild outcomes")
    available = [
        (item, outcome)
        for item, outcome in rebuilt
        if item.state == "complete"
        or (isinstance(outcome, IncompleteRebuild) and outcome.all_units_present)
    ]
    if not available and any(outcome is not None for _, outcome in rebuilt):
        return Stopped("missing_dependency", "chunk")
    if any(
        item.length != length
        or (isinstance(outcome, IncompleteRebuild) and outcome.reason == "blob_length")
        for item, outcome in available
    ):
        return Stopped("damaged_frame", "blob_length")
    return Stopped("damaged_frame", "blob_digest")


def _blob_bytes(source: CompactionSource) -> dict[BlobName, bytes] | Stopped:
    by_name: dict[BlobName, list[BlobDeclaration]] = defaultdict(list)
    for declaration in source.declarations:
        by_name[declaration.name].append(declaration)
    verified = {}
    for entry in sorted(source.blobs.entries, key=lambda blob: cbor2.dumps(blob.name)):
        if not entry.present:
            continue
        declarations = by_name[entry.name]
        for declaration in declarations:
            if declaration.state == "complete" and declaration.length == entry.length:
                body = declaration.bytes()
                if body is None:
                    raise ValueError("verified blob declaration lacks its bytes")
                verified[entry.name] = body
                break
        else:
            return _failed_blob(declarations, source.rebuilds, entry.length)
    return verified


def _write(
    source: CompactionSource, verified: Mapping[BlobName, bytes], profile: int | None
) -> Written:
    blank_positions = source.blank_positions
    digest = source.content_digest
    if blank_positions is None or digest is None:
        raise ValueError("admitted fold lacks its labelling")
    graph = canonical_graph(
        source.terms, source.lines.lines, source.records, source.values, blank_positions
    )
    writer = SegmentWriter(
        Header(profile=profile, canonical=1), collect_index=True, compress=True
    )
    for frame in chain(
        canonical_binary_frames(graph.value_octets, source.blobs, verified),
        graph.frames(),
        canonical_record_frames(source.records, graph.term_ids),
    ):
        writer.append(*frame)
    writer.commit()
    statistics = IndexStatistics(
        len(source.lines.lines),
        sum(1 for _ in record_bodies(source.records, source.blobs, source.values)),
        len(blank_positions),
    )
    writer.append_index(statistics, content_digest=digest)
    return Written(writer.to_bytes(offsets=True, locator=True, parity=True))


def compact(source: CompactionSource, profile: int | None = None) -> CompactionResult:
    """Compact the supplied fold and return the source coverage beside the outcome."""
    if source.formed_refused:
        outcome: Written | Stopped = Stopped("exceeds_capacity", "total_formed")
    elif source.lines.held_rows or source.records.held_records:
        outcome = Stopped("opaque_term")
    elif source.labelling_refusal is not None:
        outcome = Stopped("labelling_refused", source.labelling_refusal)
    else:
        verified = _blob_bytes(source)
        outcome = (
            verified
            if isinstance(verified, Stopped)
            else _write(source, verified, profile)
        )
    return CompactionResult(outcome, source.coverage)

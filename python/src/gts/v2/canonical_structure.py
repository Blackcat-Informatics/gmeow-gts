# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare the structure of a claimed segment with its own fold (P7 §4.6)."""

from collections.abc import Collection, Mapping, Sequence
from dataclasses import dataclass
from typing import Literal

import cbor2

from ._fold_record_lines import record_bodies
from .blobs import BlobName, BlobSet
from .canonical_blobs import canonical_blob_frames, canonical_value_frames
from .canonical_graph import EncodedFrame, canonical_graph
from .canonical_records import canonical_record_frames
from .dictionary import TermPool
from .frame import _COMMIT, _PARITY, Frame
from .framing import ItemFramingError, item_end
from .header import Header, decode_header, encode_header
from .index_claims import IndexStatistics, _IndexClaims
from .items import ItemKind
from .pack import MemberKind, PackMember
from .parity_write import _SPAN_LENGTH
from .range import RangeAccumulator
from .record_fold import RecordFold
from .rows import Row
from .terms import BlankTerm, TypedValue

type StructureCheck = Literal[
    "header",
    "frame_sequence",
    "terms",
    "rows",
    "value_region",
    "blob_region",
    "bindings",
    "suppression",
    "commit",
    "index",
]
_VALUE_PACK = 0
_BLOB_PACK = _PACK = 13
_INDEX = 15
_ORDER = {
    kind: rank
    for rank, kind in enumerate(
        (
            _VALUE_PACK,
            _BLOB_PACK,
            12,
            *range(1, 8),
            _COMMIT,
            _INDEX,
            "offsets",
            "locator",
        )
    )
}
_REQUIRED_AFTER = {_INDEX: _COMMIT, "offsets": _INDEX, "locator": "offsets"}
_CHECKS: dict[int, StructureCheck] = {
    _VALUE_PACK: "value_region",
    1: "terms",
    2: "rows",
    3: "rows",
    4: "rows",
    5: "rows",
    6: "bindings",
    7: "suppression",
    8: "blob_region",
    9: "blob_region",
    10: "blob_region",
    11: "value_region",
    12: "blob_region",
    _BLOB_PACK: "blob_region",
    _COMMIT: "commit",
    _INDEX: "index",
}
_STANDALONE = frozenset((8, 9, 10, 11))


def _wrong_kind(kind: int | None) -> StructureCheck:
    if kind is not None and kind in _STANDALONE:
        return _CHECKS[kind]
    return "frame_sequence"


@dataclass(frozen=True, slots=True)
class StructureDifference:
    """The first difference found on the canonical check's structure level."""

    position: int
    check: StructureCheck


@dataclass(frozen=True, slots=True, kw_only=True)
class StructureItem:
    """One item and its whole-read observations.

    A recognized frame type survives an opaque envelope. Give pack members
    only for folded packs. Set counted_failure for an ordinary frame that
    is opaque or has a finding counted by P7 §4.6.
    """

    kind: ItemKind | Literal["damaged_region"]
    size: int
    frame_type: int | None = None
    envelope: Frame | None = None
    decoded: bytes | memoryview | None = None
    pack_members: tuple[PackMember, ...] | None = None
    counted_failure: bool = False


@dataclass(frozen=True, slots=True, kw_only=True)
class StructureFold:
    """The segment's own fold with the bytes of every present blob."""

    terms: TermPool
    rows: Collection[Row]
    records: RecordFold
    values: frozenset[tuple[int, TypedValue]]
    blobs: BlobSet
    verified_blobs: Mapping[BlobName, bytes]
    labelling_succeeded: bool


class _Mismatch(Exception):
    def __init__(self, position: int, check: StructureCheck) -> None:
        self.difference = StructureDifference(position, check)


def _pack_kinds(items: Sequence[StructureItem]) -> dict[int, int]:
    # A pack of chunks and an unfolded pack inherit the next decisive region.
    decisive = {
        MemberKind.VALUE: _VALUE_PACK,
        MemberKind.BLOB: _BLOB_PACK,
        MemberKind.RECIPE: _BLOB_PACK,
    }
    region = _BLOB_PACK
    kinds: dict[int, int] = {}
    for position in range(len(items) - 1, 0, -1):
        item = items[position]
        if item.frame_type == _PACK:
            for member in item.pack_members or ():
                if member.kind in decisive:
                    region = decisive[member.kind]
                    break
            kinds[position] = region
    return kinds


def _index_fields(payload: bytes | memoryview) -> tuple[memoryview, ...]:
    data = memoryview(payload)
    if data[:1] != b"\x86":
        return ()
    fields: list[memoryview] = []
    offset = 1
    try:
        for _ in range(6):
            end = item_end(data, offset)
            fields.append(data[offset:end])
            offset = end
    except ItemFramingError:
        return ()
    return tuple(fields) if offset == len(data) else ()


def _index_matches(actual: bytes | memoryview, expected: bytes, labelled: bool) -> bool:
    fields = _index_fields(actual)
    if not fields:
        return False
    canonical = _index_fields(expected)
    if labelled and (
        fields[2] != canonical[2]
        or fields[4][:4] != b"\x82\x02\x58\x20"
        or len(fields[4]) != 36
    ):
        return False
    return all(fields[index] == canonical[index] for index in (0, 1, 3, 5))


class _Comparison:
    def __init__(self, items: Sequence[StructureItem]) -> None:
        self.items = items
        self.pack_kinds = _pack_kinds(items)
        self.position = 1
        self.pending = items[0].size

    def kind(self, position: int, item: StructureItem) -> int | None:
        return self.pack_kinds.get(position, item.frame_type)

    def take(self, kind: ItemKind, frame_kind: int | None = None) -> StructureItem:
        position = self.position
        if position == len(self.items):
            raise _Mismatch(position, "frame_sequence")
        item = self.items[position]
        actual = self.kind(position, item)
        if item.kind != kind or actual != frame_kind:
            raise _Mismatch(position, _wrong_kind(actual))
        self.position += 1
        return item

    def parity_due(self, size: int, *, closing: bool) -> int:
        spans, self.pending = divmod(self.pending + size, _SPAN_LENGTH)
        if closing and self.pending:
            spans += 1
            self.pending = 0
        return spans

    def frame(
        self, kind: int, encoded: EncodedFrame, *, labelled: bool = False
    ) -> bytes:
        position = self.position
        item = self.take("frame", kind)
        envelope, decoded = item.envelope, item.decoded
        _, content, payload = encoded
        if envelope is None or decoded is None:
            raise ValueError("structure check requires the verified ordinary frame")
        matches = (
            _index_matches(decoded, payload, labelled)
            if kind == _INDEX
            else decoded == payload
        )
        if envelope.content != cbor2.dumps(content) or not matches:
            raise _Mismatch(position, _CHECKS[kind])
        for _ in range(self.parity_due(item.size, closing=kind == _INDEX)):
            self.take("frame", _PARITY)
        return envelope.identifier

    def finish(self) -> None:
        self.take("offsets")
        self.take("locator")
        if self.position < len(self.items):
            kind = self.kind(self.position, self.items[self.position])
            raise _Mismatch(self.position, _wrong_kind(kind))


def _first_with_incomplete_fold(
    comparison: _Comparison, failed: int
) -> StructureDifference:
    items = comparison.items
    closing_index = next(
        (
            position
            for position in range(len(items) - 1, 0, -1)
            if items[position].frame_type == _INDEX
        ),
        None,
    )
    previous: int | str = "header"
    spans = 0
    for position in range(1, failed):
        item = items[position]
        frame_kind = comparison.kind(position, item)
        if spans:
            if item.kind != "frame" or frame_kind != _PARITY:
                return StructureDifference(position, _wrong_kind(frame_kind))
            spans -= 1
            continue
        if (
            frame_kind == _PARITY
            and item.kind == "frame"
            and previous == _INDEX
            and comparison.pending
        ):
            # An append can preserve the short span after its earlier index.
            comparison.pending = 0
            continue
        if frame_kind in _STANDALONE:
            return StructureDifference(position, _wrong_kind(frame_kind))
        kind: int | str | None = frame_kind if item.kind == "frame" else item.kind
        if (
            kind is None
            or kind not in _ORDER
            or _ORDER[kind] < _ORDER.get(previous, -1)
            or (kind == _COMMIT and previous == _COMMIT)
            or (kind in _REQUIRED_AFTER and _REQUIRED_AFTER[kind] != previous)
        ):
            return StructureDifference(position, "frame_sequence")
        previous = kind
        if item.kind == "frame":
            spans = comparison.parity_due(item.size, closing=position == closing_index)
    kind = comparison.kind(failed, items[failed])
    return StructureDifference(
        failed,
        _CHECKS.get(kind, "frame_sequence") if kind is not None else "frame_sequence",
    )


def _compare_fold(
    comparison: _Comparison, source: StructureFold, opener: bytes
) -> None:
    graph = canonical_graph(
        source.terms, source.rows, source.records, source.values, None
    )
    claims = _IndexClaims()
    leaves = RangeAccumulator(opener)
    regions = (
        (_VALUE_PACK, canonical_value_frames(graph.value_octets)),
        (_BLOB_PACK, canonical_blob_frames(source.blobs, source.verified_blobs)),
        (None, graph.frames()),
        (None, canonical_record_frames(source.records, graph.term_ids)),
    )
    for region, frames in regions:
        for encoded in frames:
            kind, _, _ = encoded
            observed_kind = region if kind == _PACK and region is not None else kind
            leaves.append(comparison.frame(observed_kind, encoded))
            claims.add(*encoded)
    root = leaves.root()
    commit: EncodedFrame = (_COMMIT, [root.count - 1, root.digest], b"")
    head = comparison.frame(_COMMIT, commit)
    leaves.append(head)
    claims.add(*commit)
    stats = IndexStatistics(
        len(source.rows),
        sum(1 for _ in record_bodies(source.records, source.blobs, source.values)),
        sum(isinstance(term, BlankTerm) for term in graph.terms),
    )
    root = leaves.root()
    digest = bytes(32) if source.labelling_succeeded else None
    payload = claims.payload(stats, root.digest, digest)
    comparison.frame(
        _INDEX,
        (_INDEX, [root.count - 1, head], payload),
        labelled=source.labelling_succeeded,
    )
    comparison.finish()


def check_structure(
    header: Header, items: Sequence[StructureItem], source: StructureFold | None
) -> StructureDifference | None:
    """Judge a claimed segment once the whole read permits this level.

    Capacity and failed reads preempt this comparison. A counted frame
    failure permits a sequence check without reconstructing the own fold.
    """
    expected = Header(profile=header.profile, canonical=1)
    if header != expected or not items or items[0].kind != "header":
        return StructureDifference(0, "header")
    try:
        opener = decode_header(encode_header(expected)).identifier
    except ValueError:
        return StructureDifference(0, "header")
    assert opener is not None
    comparison = _Comparison(items)
    for position, item in enumerate(items):
        if item.counted_failure and item.frame_type != _PARITY:
            return _first_with_incomplete_fold(comparison, position)
    if source is None:
        raise ValueError("structure check requires the segment's own fold")
    try:
        _compare_fold(comparison, source, opener)
    except _Mismatch as mismatch:
        return mismatch.difference
    return None

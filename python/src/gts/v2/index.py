# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Validate the grammar of a decoded GTS v2 index (P5 §§4.1, 4.3)."""

from collections.abc import Mapping, Sequence, Set
from dataclasses import dataclass
from typing import Literal

import cbor2

from ._content import ContentError, UnknownContentCode, content_code
from ._decode import _Decoder
from .frame import _LAST_FRAME_TYPE, Frame
from .index_claims import IndexStatistics, _IndexClaims
from .pack import PackMember
from .range import RangeAccumulator
from .reader import WireFrame
from .rows import _PAYLOAD_SIZES, Row, _graph


@dataclass(frozen=True, slots=True)
class IndexPayload:
    type_runs: tuple[tuple[int, int], ...]
    graph_directory: memoryview
    graph_directory_size: tuple[int, int]
    statistics: memoryview
    range_root: bytes
    content_digest: bytes | None
    pack_members: memoryview
    pack_member_count: int


def check_index_payload(
    payload: bytes | memoryview, *, registry_version: int = 1, after_stop: bool = False
) -> IndexPayload:
    """Check the index payload grammar and retain its prefix and digest claims."""
    view = memoryview(payload)
    reader = _Decoder(view, 0, ContentError)

    def frame_type() -> int:
        if after_stop and registry_version > 1:
            return reader.uint(minimum=1, reason="unregistered_value")
        return content_code(reader, _LAST_FRAME_TYPE, registry_version)

    reader.record(6)
    runs = []
    for _ in range(reader.array()):
        reader.record(2)
        runs.append((frame_type(), reader.uint()))

    directory_at = reader.offset
    graph_count = reader.array()
    range_count = 0
    for _ in range(graph_count):
        reader.record(2)
        graph_at = reader.offset
        try:
            _graph(reader, registry_version)
        except UnknownContentCode:
            if not after_stop:
                raise
            reader.offset = graph_at
            fields = reader.array()
            reader.uint()
            for _ in range(fields - 1):
                reader.closed_item()
        graph_ranges = reader.array()
        range_count += graph_ranges
        for _ in range(graph_ranges):
            reader.record(4)
            frame_type()
            reader.uint()
            reader.uint()
            reader.uint()

    directory = view[directory_at : reader.offset]
    statistics_at = reader.offset
    reader.record(4)
    reader.uint()
    reader.uint()
    reader.uint()
    for _ in range(reader.array()):
        reader.record(2)
        reader.uint()
        reader.uint()

    statistics = view[statistics_at : reader.offset]
    root = bytes(reader.bytes(32))
    digest = reader.option(lambda: bytes(reader.bytes(32)))
    members_at = reader.offset
    member_count = reader.array()
    for _ in range(member_count):
        reader.record(3)
        reader.bytes(32)
        reader.uint()
        reader.uint()
    if reader.offset != len(payload):
        raise ContentError("wrong_length", reader.offset)
    return IndexPayload(
        tuple(runs),
        directory,
        (graph_count, range_count),
        statistics,
        root,
        digest,
        view[members_at:],
        member_count,
    )


type IndexReason = Literal[
    "index_head", "index_root", "index_directory", "index_members", "index_digest"
]


@dataclass(frozen=True, slots=True)
class ClosingIndex:
    position: int
    claim: IndexPayload
    directory_failed: bool
    predicates: tuple[tuple[int, int], ...]

    def disagrees(
        self, statistics: IndexStatistics, digest: bytes | None
    ) -> tuple[IndexReason, ...]:
        reasons: list[IndexReason] = []
        if not self.directory_failed and self.claim.statistics != cbor2.dumps(
            statistics.record(self.predicates)
        ):
            reasons.append("index_directory")
        if self.claim.content_digest != digest:
            reasons.append("index_digest")
        return tuple(reasons)


def check_prefix_claims(
    opener: bytes,
    frames: Sequence[WireFrame],
    indexes: Mapping[int, IndexPayload],
    rows: Mapping[int, tuple[Row, ...]],
    packs: Mapping[int, tuple[PackMember, ...]],
    opaque: Set[int],
) -> tuple[list[tuple[int, IndexReason]], ClosingIndex | None]:
    """Check structural claims wherever an index stands and locate its own fold."""
    findings: list[tuple[int, IndexReason]] = []
    if not indexes:
        return findings, None
    history = RangeAccumulator(opener)
    head = opener
    claims = _IndexClaims()
    last = next(
        (frame.position for frame in reversed(frames) if not frame.transparent), None
    )
    last_index = max(indexes)
    closing = None
    graph_available = packs_available = fold_available = True
    for frame in frames:
        position = frame.position
        if position in indexes:
            payload = indexes[position]
            envelope = frame.envelope
            assert isinstance(envelope, Frame)
            content = _Decoder(envelope.content, 0, ContentError)
            content.record(2)
            count, claimed_head = content.uint(), content.bytes(32)
            root = history.root()
            if count != root.count - 1 or claimed_head != head:
                findings.append((position, "index_head"))
            if payload.range_root != root.digest:
                findings.append((position, "index_root"))
            directory_failed = (
                claims.type_runs != list(payload.type_runs)
                or graph_available
                and (
                    payload.graph_directory_size != claims.graph_directory_size
                    or payload.graph_directory != cbor2.dumps(claims.graph_directory())
                )
            )
            if directory_failed:
                findings.append((position, "index_directory"))
            if packs_available and (
                payload.pack_member_count != claims.pack_member_count
                or payload.pack_members != cbor2.dumps(claims.pack_members())
            ):
                findings.append((position, "index_members"))
            if position == last and fold_available:
                closing = ClosingIndex(
                    position, payload, directory_failed, claims.predicates()
                )
            if position == last_index:
                break
        if not frame.transparent and position in opaque:
            fold_available = False
        if (identifier := frame.link.effective_identifier) is not None:
            head = identifier
            history.append(identifier)
            kind = frame.frame_type
            if kind in _PAYLOAD_SIZES and position not in rows:
                graph_available = False
            if kind == 13 and position not in packs:
                packs_available = False
            claims.observe(kind, rows.get(position, ()), packs.get(position, ()))
    return findings, closing

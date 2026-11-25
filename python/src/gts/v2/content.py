# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Whole-input graph and blob content reading for GTS v2."""

from collections.abc import Iterable, Mapping
from dataclasses import dataclass, field
from typing import Literal, cast

from ._content import UnknownContentCode
from ._decode import WireError, _Decoder
from ._fold_record_lines import record_bodies
from ._labelling_result import ComponentStatistics, InputStatistics
from ._labelling_structure import Diagnostic
from ._read_coverage import SegmentCoverage, file_coverage
from .authentication import (
    AuthenticationFrame,
    SegmentAuthentication,
    authenticate_segment,
)
from .blobs import (
    BlobContent,
    BlobDeclaration,
    BlobRead,
    BlobSegment,
    BlobSet,
    StateValueScope,
    blob_set,
    decode_blob_content,
)
from .canonical_check import (
    ITEM_LIMIT,
    NOT_CLAIMED,
    CanonicalResult,
    CheckState,
    Difference,
    UncheckedReason,
    checked,
)
from .canonical_labelling import BlankOrigin, check_canonical_labelling
from .canonical_structure import StructureFold, StructureItem, check_structure
from .compaction import CompactionResult, CompactionSource, ReadCoverage
from .compaction import compact as compact_fold
from .dictionary import TermDictionary, TermPool, TermResolution
from .fold_labelling import (
    FoldLabellingBudget,
    FoldLabellingRefused,
    LabelledFold,
    RefusalReason,
    folded_size,
)
from .frame import Frame, FrameError, Signature, _frame_field_offsets, _signatures
from .graph import GraphContent, GraphFold, GraphFrame
from .index import ClosingIndex, IndexPayload, check_index_payload, check_prefix_claims
from .index_claims import IndexStatistics
from .lineage import SegmentLineage, check_lineages
from .lines import LineFold, LineResult
from .range import RangeAccumulator
from .reader import WireFrame, WireRead, WireSegment, read_wire
from .rebuild import RefusedRebuild
from .record_fold import (
    RecordFold,
    RecordFrame,
    RecordLocation,
    RecordScope,
    ResolvedBinding,
    fold_records,
)
from .records import BlobReference, decode_bindings, decode_suppression
from .rows import DecodedRows, Row
from .signatures import SignatureVerifier
from .terms import (
    BlankTerm,
    DecodedTerms,
    FormedBudget,
    FormedCapacityError,
    LiteralTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
    WireTerm,
)

_LAST_FRAME = 16

# The `opaque_reason` values of P2, in the precedence order of P2-30.
OpaqueReason = Literal[
    "malformed",
    "chain_break",
    "stopped",
    "damaged",
    "unsupported_critical",
    "unknown_codec",
    "exceeds_capacity",
    "unknown_frame_type",
]
_OPAQUE_PRECEDENCE: tuple[OpaqueReason, ...] = (
    "malformed",
    "chain_break",
    "stopped",
    "damaged",
    "unsupported_critical",
    "unknown_codec",
    "exceeds_capacity",
    "unknown_frame_type",
)


@dataclass(frozen=True, slots=True)
class ContentCapacity:
    stored_bytes_per_frame: int = 1 << 26
    decoded_bytes_per_frame: int = 1 << 26
    decoded_bytes_per_read: int = 1 << 36
    items_per_read: int = 1 << 26
    rebuilt_bytes_per_read: int = 1 << 37
    formed_bytes_per_read: int = 1 << 37
    labelled_lines: int = 1 << 26
    labelled_bytes_per_read: int = 1 << 37
    labelling_units: int = 0


_DEFAULT_CAPACITY = ContentCapacity()


@dataclass(frozen=True, slots=True)
class ContentFinding:
    segment: int | None
    position: int | None
    code: str
    reason: str | None = None
    count: int | None = None
    offset: int | None = None
    length: int | None = None


@dataclass(frozen=True, slots=True)
class VisibleDataset:
    lines: frozenset[Row]
    bindings: frozenset[ResolvedBinding]
    blob_declarations: tuple[BlobDeclaration, ...]

    @property
    def named_graphs(self) -> frozenset[int]:
        return frozenset(row.graph for row in self.lines if row.graph is not None)


@dataclass(frozen=True, slots=True)
class LabellingReport:
    content_digest: bytes | None
    components: tuple[ComponentStatistics, ...]
    statistics: InputStatistics
    diagnostic: tuple[Diagnostic, ...]


@dataclass(frozen=True, slots=True)
class ContentFold:
    terms: TermPool
    lines: LineResult
    blobs: BlobSet
    values: frozenset[tuple[int, TypedValue]]
    records: RecordFold
    blob_declarations: tuple[BlobDeclaration, ...]
    labelling: LabellingReport | None = None
    blank_positions: Mapping[int, int] | None = None

    @property
    def content_digest(self) -> bytes | None:
        return None if self.labelling is None else self.labelling.content_digest

    @property
    def visible_dataset(self) -> VisibleDataset:
        hidden = self.records.hidden_blobs
        return VisibleDataset(
            self.lines.lines - self.records.hidden_lines,
            self.records.bindings - self.records.hidden_bindings,
            tuple(
                declaration
                for declaration in self.blob_declarations
                if BlobReference(*declaration.name) not in hidden
            ),
        )


@dataclass(frozen=True, slots=True)
class Visibility:
    unfolded_suppress: int
    unopened_sealed: int
    unauthenticated_suppress: int

    @property
    def complete(self) -> bool:
        return self.unfolded_suppress == self.unopened_sealed == 0


@dataclass(frozen=True, slots=True)
class GraphState:
    """The raw fold named by a commit at its first eligible occurrence."""

    identifier: bytes
    commit: RecordLocation
    first_segment: int
    fold: ContentFold


@dataclass(frozen=True, slots=True)
class ContentRead:
    wire: WireRead
    lineages: tuple[SegmentLineage, ...]
    blob_segments: tuple[BlobSegment | None, ...]
    findings: tuple[ContentFinding, ...]
    fold: ContentFold | None
    states: tuple[GraphState, ...] = ()
    authentication: tuple[SegmentAuthentication, ...] = ()
    visibility: Visibility | None = None
    own_labelling: tuple[LabellingReport | None, ...] = ()
    coverage: ReadCoverage | None = None
    canonical: tuple[CanonicalResult, ...] = ()
    # The P2 opaque reason of each opaque frame, keyed by (segment ordinal, position).
    opaque_reasons: dict[tuple[int, int], OpaqueReason] = field(default_factory=dict)

    @property
    def repaired_copy(self) -> bytes | None:
        """Return the input with every accepted parity repair applied."""
        return self.wire.repaired_copy

    def compact(self, profile: int | None = None) -> CompactionResult:
        """Compact this whole read's fold with its source observations."""
        fold = self.fold
        if fold is None or self.coverage is None:
            raise ValueError("compaction requires a second-generation whole-read fold")
        refusal = next(
            (
                finding.reason
                for finding in self.findings
                if finding.segment is None and finding.code == "labelling_refused"
            ),
            None,
        )
        if refusal not in (
            None,
            "refusal_size",
            "refusal_weight",
            "refusal_term_node",
            "refusal_width",
            "refusal_limit",
        ):
            raise ValueError("unknown file labelling refusal")
        return compact_fold(
            CompactionSource(
                terms=fold.terms,
                lines=fold.lines,
                records=fold.records,
                values=fold.values,
                blobs=fold.blobs,
                declarations=fold.blob_declarations,
                rebuilds={
                    location: outcome
                    for segment in self.blob_segments
                    if segment is not None
                    for location, outcome in segment.rebuilds
                },
                coverage=self.coverage,
                content_digest=fold.content_digest,
                blank_positions=fold.blank_positions,
                formed_refused=any(
                    finding.reason == "total_formed" for finding in self.findings
                ),
                labelling_refusal=cast(RefusalReason | None, refusal),
            ),
            profile,
        )


@dataclass(frozen=True, slots=True)
class _GraphAt:
    position: int
    frame: GraphFrame
    in_doubt: bool
    hidden_before: int


@dataclass(slots=True)
class _SegmentContent:
    graph: list[_GraphAt]
    blobs: list[tuple[int, BlobContent]]
    records: list[RecordFrame]
    first_ineligible: int | None
    opaque_positions: set[int]
    index_digests: dict[int, bytes | None]
    closing_index: ClosingIndex | None
    opaque_reasons: dict[int, OpaqueReason]


@dataclass(frozen=True, slots=True)
class _StatePrefix:
    first_segment: int
    commit: RecordLocation

    def includes(self, segment: int, position: int) -> bool:
        return (
            self.first_segment <= segment < self.commit.segment
            or segment == self.commit.segment
            and position <= self.commit.position
        )

    @property
    def boundary(self) -> tuple[int, int]:
        return self.commit.segment, self.commit.position


@dataclass(frozen=True, slots=True)
class _GraphResult:
    pool: TermPool
    dictionaries: dict[int, TermDictionary]
    lines: LineResult
    values: frozenset[tuple[int, TypedValue]]

    def with_records(
        self,
        blobs: BlobSet,
        records: RecordFold,
        declarations: tuple[BlobDeclaration, ...],
        labelled: LabelledFold | None = None,
    ) -> ContentFold:
        return ContentFold(
            self.pool,
            self.lines,
            blobs,
            self.values,
            records,
            declarations,
            None if labelled is None else _labelling_report(labelled),
            None if labelled is None else labelled.blank_positions,
        )


def _commits(
    segment: WireSegment,
    ordinal: int,
    opener: bytes,
    stored_limit: int,
    findings: list[ContentFinding],
) -> tuple[int, list[tuple[int, bytes]], set[int]]:
    history = RangeAccumulator(opener)
    last_commit = 0
    tail_start = 0
    tail_count = 0
    candidates: list[tuple[int, bytes]] = []
    valid: set[int] = set()
    for wire_frame in segment.frames:
        frame = wire_frame.envelope
        position = wire_frame.position
        if (
            isinstance(frame, Frame)
            and frame.frame_type == 14
            and len(frame.content) <= stored_limit
        ):
            decoder = _Decoder(frame.content, 0, WireError)
            decoder.record(2)
            count, root = decoder.uint(), decoder.bytes(32)
            expected = history.root()
            count_holds = count == expected.count - 1
            root_holds = root == expected.digest
            if not count_holds:
                findings.append(
                    ContentFinding(
                        ordinal,
                        position,
                        "commit_count_mismatch",
                        count=expected.count - 1,
                    )
                )
            if not root_holds:
                findings.append(
                    ContentFinding(ordinal, position, "commit_root_mismatch")
                )
            if count_holds and root_holds and wire_frame.link.state != "broken":
                last_commit = position
                valid.add(position)
                tail_count = 0
                if wire_frame.payload.state == "verified":
                    identifier = wire_frame.link.effective_identifier
                    assert identifier is not None
                    candidates.append((position, identifier))

        identifier = wire_frame.link.effective_identifier
        if identifier is not None:
            history.append(identifier)
            if position != last_commit and wire_frame.frame_type != 15:
                if not tail_count:
                    tail_start = position
                tail_count += 1
    if tail_count:
        findings.append(
            ContentFinding(ordinal, tail_start, "uncommitted_tail", count=tail_count)
        )
    return last_commit, candidates, valid


def _raw_content_and_flags(
    data: bytes, segment: WireSegment, wire_frame: WireFrame
) -> tuple[memoryview, bool | None]:
    envelope = wire_frame.envelope
    if isinstance(envelope, Frame):
        return envelope.content, envelope.critical
    starts = _frame_field_offsets(data, segment.items[wire_frame.position].offset)
    content = memoryview(data)[starts[3] : starts[4]]
    if (
        starts[2] - starts[1] == starts[3] - starts[2] == 1
        and data[starts[1]] in (0xF4, 0xF5)
        and data[starts[2]] in (0xF4, 0xF5)
    ):
        return content, data[starts[1]] == 0xF5
    return content, None


def _read_signatures(
    data: bytes,
    segment: WireSegment,
    frame: WireFrame,
    stored_limit: int,
    registry: int,
) -> tuple[Signature, ...]:
    if frame.transparent or frame.link.effective_identifier is None:
        return ()
    if isinstance(frame.envelope, Frame):
        return (
            frame.envelope.signatures
            if frame.envelope.encoded_signatures_size <= stored_limit
            else ()
        )
    starts = _frame_field_offsets(data, segment.items[frame.position].offset)
    start, end = starts[-2:]
    if end - start > stored_limit:
        return ()
    # P6 judges signatures on ordinary frames even if another field is invalid.
    try:
        return _signatures(_Decoder(data, start, FrameError), newer=registry > 1)
    except FrameError:
        return ()


def _record_content_error(
    error: WireError | UnknownContentCode | FormedCapacityError,
    ordinal: int,
    position: int,
    findings: list[ContentFinding],
) -> bool:
    if isinstance(error, UnknownContentCode):
        findings.append(ContentFinding(ordinal, position, "unsupported_critical"))
        return True
    if isinstance(error, FormedCapacityError):
        findings.append(
            ContentFinding(
                ordinal, position, "exceeds_capacity", "total_formed", error.count
            )
        )
    else:
        findings.append(
            ContentFinding(ordinal, position, "malformed_value", error.reason)
        )
    return False


def _error_reason(
    error: WireError | UnknownContentCode | FormedCapacityError,
) -> OpaqueReason:
    """Return the opaque reason that a content error gives its frame (P2)."""
    if isinstance(error, UnknownContentCode):
        return "unsupported_critical"
    if isinstance(error, FormedCapacityError):
        return "exceeds_capacity"
    return "malformed"


def _blocks_state_prefix(frame: WireFrame) -> bool:
    if frame.transparent:
        return False
    if frame.link.state == "broken":
        return True
    if frame.frame_type == _LAST_FRAME:
        # Ordinary parity is damaged but remains not_content under P7.
        return False
    return (
        not isinstance(frame.envelope, Frame)
        or frame.payload.state != "verified"
        or not 1 <= frame.envelope.frame_type < _LAST_FRAME
    )


def _read_segment_content(
    data: bytes,
    segment: WireSegment,
    ordinal: int,
    formed: FormedBudget,
    stored_limit: int,
    last_commit: int,
    findings: list[ContentFinding],
) -> _SegmentContent:
    opener = segment.opener.value
    assert not isinstance(opener, WireError) and opener.identifier is not None
    registry = opener.header.registry_version
    graph = GraphContent(formed, registry_version=registry)
    graph_frames: list[_GraphAt] = []
    blob_frames: list[tuple[int, BlobContent]] = []
    record_frames: list[RecordFrame] = []
    doubt_at = next(
        (
            position
            for position, item in enumerate(segment.items)
            if item.kind in ("opaque", "damaged_region")
        ),
        len(segment.items),
    )
    stopped = False
    hidden_before = 0
    first_ineligible = None
    opaque_positions: set[int] = set()
    # The reasons each frame's own decision sites give, before the segment-level ones.
    applies: dict[int, set[OpaqueReason]] = {}
    stopped_at: int | None = None
    index_digests: dict[int, bytes | None] = {}
    checked_indexes: dict[int, IndexPayload] = {}
    for frame in segment.frames:
        position = frame.position
        kind = frame.frame_type
        if isinstance(frame.envelope, FrameError):
            # An envelope that fails its grammar is a malformed value (P1, 2.2).
            applies.setdefault(position, set()).add("malformed")
        previous_cutoff = hidden_before
        if frame.link.state == "broken" and not frame.transparent:
            hidden_before = graph.next_id
        stopped_ordinary = stopped and not frame.transparent and kind != _LAST_FRAME
        if first_ineligible is None and (
            stopped_ordinary or _blocks_state_prefix(frame)
        ):
            first_ineligible = position
        decoded = (
            frame.payload.decoded
            if isinstance(frame.envelope, Frame) and frame.link.state != "broken"
            else None
        )
        if kind == 16:
            if position in segment.parity.future_geometry:
                opaque_positions.add(position)
                applies.setdefault(position, set()).add("unknown_frame_type")
                if not stopped and decoded is not None:
                    findings.append(
                        ContentFinding(
                            ordinal,
                            position,
                            "unknown_enumeration_value",
                            "unknown_frame_type",
                        )
                    )
            continue
        opaque_positions.add(position)
        if kind is None or kind == 0:
            applies.setdefault(position, set()).add("malformed")
            continue
        if kind == 14:
            if decoded is not None and not stopped:
                opaque_positions.remove(position)
            continue
        if kind == 15:
            if (
                not stopped
                and isinstance(frame.envelope, Frame)
                and frame.link.state != "broken"
                and _payload_refused(frame)
            ):
                index_digests[position] = None
            if decoded is not None:
                try:
                    index = check_index_payload(
                        decoded, registry_version=registry, after_stop=stopped
                    )
                except WireError as error:
                    _record_content_error(error, ordinal, position, findings)
                    applies.setdefault(position, set()).add(_error_reason(error))
                    if first_ineligible is None:
                        first_ineligible = position
                except UnknownContentCode:
                    if not stopped:
                        findings.append(
                            ContentFinding(
                                ordinal,
                                position,
                                "unknown_enumeration_value",
                                "unknown_frame_type",
                            )
                        )
                    applies.setdefault(position, set()).add("unknown_frame_type")
                    if first_ineligible is None:
                        first_ineligible = position
                else:
                    checked_indexes[position] = index
                    if not stopped:
                        opaque_positions.remove(position)
                        if index.content_digest is not None:
                            index_digests[position] = index.content_digest
            continue
        if kind > _LAST_FRAME:
            _, critical = _raw_content_and_flags(data, segment, frame)
            # A critical unknown type stops the fold only from registry version 2;
            # version 1 reports it as an unknown type.
            if registry > 1 and critical is True:
                findings.append(
                    ContentFinding(ordinal, position, "unsupported_critical")
                )
                applies.setdefault(position, set()).add("unsupported_critical")
                stopped = True
                stopped_at = position if stopped_at is None else stopped_at
            else:
                applies.setdefault(position, set()).add("unknown_frame_type")
                if registry > 1 and critical is False:
                    findings.append(
                        ContentFinding(
                            ordinal,
                            position,
                            "unknown_enumeration_value",
                            "unknown_frame_type",
                        )
                    )
            continue
        if stopped:
            continue
        if kind <= 5:
            content, _ = _raw_content_and_flags(data, segment, frame)
            graph_frame = graph.read(
                kind, content if len(content) <= stored_limit else None, decoded
            )
            if graph_frame.error is not None:
                stopped = _record_content_error(
                    graph_frame.error, ordinal, position, findings
                )
                applies.setdefault(position, set()).add(
                    _error_reason(graph_frame.error)
                )
                if stopped and stopped_at is None:
                    stopped_at = position
                if first_ineligible is None and not frame.transparent:
                    first_ineligible = position
            rows = graph_frame.contribution
            if isinstance(rows, DecodedTerms | DecodedRows):
                opaque_positions.remove(position)
            if isinstance(rows, DecodedRows) and rows.order_holds is False:
                findings.append(
                    ContentFinding(ordinal, position, "declared_order_violated")
                )
            graph_frames.append(
                _GraphAt(
                    position,
                    graph_frame,
                    position >= doubt_at,
                    previous_cutoff if position >= last_commit else 0,
                )
            )
        elif isinstance(frame.envelope, Frame) and decoded is not None:
            try:
                if kind in (6, 7):
                    decode_records = (
                        decode_bindings if kind == 6 else decode_suppression
                    )
                    records = decode_records(
                        frame.envelope.content, decoded, registry_version=registry
                    )
                    record_frames.append(
                        RecordFrame(
                            RecordLocation(ordinal, position),
                            records,
                            graph.next_id,
                            previous_cutoff if position >= last_commit else 0,
                            position >= doubt_at,
                        )
                    )
                    opaque_positions.remove(position)
                    continue
                blob_content = decode_blob_content(
                    frame.envelope, decoded, registry_version=registry
                )
            except (WireError, UnknownContentCode) as error:
                stopped = _record_content_error(error, ordinal, position, findings)
                applies.setdefault(position, set()).add(_error_reason(error))
                if stopped and stopped_at is None:
                    stopped_at = position
                if first_ineligible is None and not frame.transparent:
                    first_ineligible = position
            else:
                blob_frames.append((position, blob_content))
                opaque_positions.remove(position)
    closing_index = None
    if checked_indexes:
        prefix_findings, closing_index = check_prefix_claims(
            opener.identifier,
            segment.frames,
            checked_indexes,
            {
                event.position: batch.rows
                for event in graph_frames
                if isinstance(batch := event.frame.contribution, DecodedRows)
            },
            {
                position: blob
                for position, blob in blob_frames
                if isinstance(blob, tuple)
            },
            opaque_positions,
        )
        findings.extend(
            ContentFinding(ordinal, position, "index_claim_failed", reason)
            for position, reason in prefix_findings
        )
    frames = {frame.position: frame for frame in segment.frames}
    opaque_reasons: dict[int, OpaqueReason] = {}
    for position in sorted(opaque_positions):
        frame = frames[position]
        found: set[OpaqueReason] = set(applies.get(position, ()))
        if frame.link.state == "broken" and not frame.transparent:
            found.add("chain_break")
        if (
            stopped_at is not None
            and position > stopped_at
            and not frame.transparent
            and frame.frame_type != _LAST_FRAME
        ):
            found.add("stopped")
        if frame.payload.state == "damaged":
            found.add("damaged")
        elif _payload_refused(frame):
            found.add(
                "unknown_codec"
                if frame.payload.reason == "unknown_codec"
                else "exceeds_capacity"
            )
        reason: OpaqueReason | None = next(
            (r for r in _OPAQUE_PRECEDENCE if r in found), None
        )
        if reason is None:
            raise ValueError(f"opaque frame at position {position} gives no P2 reason")
        opaque_reasons[position] = reason
    return _SegmentContent(
        graph_frames,
        blob_frames,
        record_frames,
        first_ineligible,
        opaque_positions,
        index_digests,
        closing_index,
        opaque_reasons,
    )


def _record_scope(
    wire: WireRead,
    dictionaries: dict[int, TermDictionary],
    blobs: BlobSet,
    prefix: _StatePrefix | None = None,
) -> RecordScope:
    holders: dict[bytes, list[int]] = {}
    frames: set[bytes] = set()
    commits: set[bytes] = set()
    for ordinal in dictionaries:
        segment = wire.segments[ordinal]
        for frame in segment.frames:
            if prefix is not None and not prefix.includes(ordinal, frame.position):
                break
            identifier = frame.link.effective_identifier
            if identifier is None:
                continue
            frames.add(identifier)
            if frame.frame_type == 14:
                commits.add(identifier)
            if frame.link.state != "broken":
                owners = holders.setdefault(identifier, [])
                # A pair asks for the first holder other than its own segment.
                if len(owners) < 2 and (not owners or owners[-1] != ordinal):
                    owners.append(ordinal)
    return RecordScope(
        dictionaries,
        {name: tuple(owners) for name, owners in holders.items()},
        {BlobReference(*entry.name) for entry in blobs.entries},
        frames,
        set(),
        commits,
    )


def _values_at(
    frame: _GraphAt,
    segment: int,
    blobs: BlobRead,
    cache: dict[ValueReference, bytes],
    state: StateValueScope | None,
    *,
    own: bool,
) -> tuple[dict[ValueReference, bytes | None], set[ValueReference]]:
    batch = frame.frame.contribution
    values: dict[ValueReference, bytes | None] = {}
    refused: set[ValueReference] = set()
    if not isinstance(batch, DecodedTerms):
        return values, refused
    seen: set[ValueReference] = set()
    for term in batch.terms:
        value = _value_reference(term)
        if value is None or value in seen:
            continue
        seen.add(value)
        selected = blobs.value_before(
            value.digest,
            value.length,
            segment=segment,
            position=frame.position,
            first_segment=segment if own else 0,
            state=state,
        )
        if selected.refused:
            refused.add(value)
        if selected.rebuilt is not None:
            octets = cache.get(value)
            if octets is None:
                octets = cache[value] = selected.rebuilt.to_bytes()
            values[value] = octets
        elif selected.declared:
            values[value] = None
    return values, refused


def _value_reference(term: WireTerm) -> ValueReference | None:
    match term:
        case (
            LiteralTerm(lexical=ValueReference() as value)
            | TypedValueTerm(value=ValueReference() as value)
        ):
            return value
        case _:
            return None


def _fold_graph(
    graphs: Iterable[tuple[int, list[_GraphAt]]],
    blob_read: BlobRead,
    cache: dict[ValueReference, bytes],
    findings: list[ContentFinding] | None,
    prefix: _StatePrefix | None = None,
    *,
    own: bool = False,
) -> _GraphResult | None:
    pool = TermPool()
    lines = LineFold()
    typed_values: set[tuple[int, TypedValue]] = set()
    missing: set[ValueReference] = set()
    dictionaries: dict[int, TermDictionary] = {}
    state = (
        None
        if prefix is None
        else StateValueScope(prefix.first_segment, prefix.boundary)
    )
    for ordinal, frames in graphs:
        fold = GraphFold(pool, 0 if own else ordinal, lines)
        dictionaries[ordinal] = fold.dictionary
        for event in frames:
            if prefix is not None and not prefix.includes(ordinal, event.position):
                break
            values, refused = _values_at(
                event, ordinal, blob_read, cache, state, own=own
            )
            result = fold.add(
                event.frame,
                values=values,
                in_doubt=event.in_doubt,
                hidden_before=event.hidden_before,
            )
            if result is None:
                continue
            if prefix is None and findings is not None:
                findings.extend(
                    ContentFinding(ordinal, event.position, code)
                    for code in sorted(result.findings)
                )
            if not isinstance(result, TermResolution):
                continue
            typed_values.update(result.values)
            if prefix is not None:
                batch = event.frame.contribution
                assert isinstance(batch, DecodedTerms)
                if refused and any(
                    _value_reference(term) in refused
                    and fold.dictionary.resolved_id(batch.id_base + index) is None
                    for index, term in enumerate(batch.terms)
                ):
                    return None
                continue
            if findings is None:
                continue
            if result.order_holds is False:
                findings.append(
                    ContentFinding(ordinal, event.position, "declared_order_violated")
                )
            for value in result.missing_values:
                if value not in missing:
                    missing.add(value)
                    findings.append(
                        ContentFinding(
                            ordinal, event.position, "missing_dependency", "value"
                        )
                    )
    return _GraphResult(pool, dictionaries, lines.result(), frozenset(typed_values))


def _labelling_report(labelled: LabelledFold) -> LabellingReport:
    result = labelled.labelling
    return LabellingReport(
        labelled.content_digest, result.components, result.statistics, result.diagnostic
    )


def _label_fold(
    budget: FoldLabellingBudget,
    size: int,
    graph: _GraphResult,
    records: RecordFold,
    blobs: BlobSet,
    findings: list[ContentFinding],
    ordinal: int | None,
) -> LabelledFold | None:
    try:
        return budget.label_fold(
            graph.pool, graph.lines, records, blobs, graph.values, folded_size=size
        )
    except FoldLabellingRefused as exc:
        findings.append(ContentFinding(ordinal, None, "labelling_refused", exc.reason))
        return None


def _claims_canonical(segment: WireSegment) -> bool:
    opener = segment.opener.value
    return (
        segment.opener.kind == "header"
        and not isinstance(opener, WireError)
        and opener.identifier is not None
        and opener.header.canonical == 1
    )


def _payload_refused(frame: WireFrame) -> bool:
    return frame.payload.state == "unchecked" and frame.payload.reason in (
        "stored_size",
        "decoded_size",
        "total_decoded",
    )


def _blank_origins(
    segment: _SegmentContent, dictionary: TermDictionary
) -> Iterable[BlankOrigin]:
    for event in segment.graph:
        batch = event.frame.contribution
        if isinstance(batch, DecodedTerms):
            for local, term in enumerate(batch.terms, batch.id_base):
                if isinstance(term, BlankTerm):
                    ref = dictionary.resolved_id(local)
                    if ref is not None:
                        yield BlankOrigin(ref, event.position)


_COUNTED_FINDINGS = {
    "commit_count_mismatch",
    "commit_root_mismatch",
    "damaged_frame",
    "declared_order_violated",
    "forward_reference",
    "ill_formed_term",
    "malformed_value",
    "opaque_term",
    "position_constraint",
    "unreferenced_unit",
}


def _canonical_levels(
    wire: WireSegment,
    content: _SegmentContent,
    fold: ContentFold,
    rebuilt: BlobSegment,
    labelled: LabelledFold | None,
    origins: Iterable[BlankOrigin],
    findings: Iterable[ContentFinding],
) -> CanonicalResult:
    opener = wire.opener.value
    assert not isinstance(opener, WireError)
    refused_payloads = {
        frame.position for frame in wire.frames if _payload_refused(frame)
    }
    refused_rebuilds = {
        location
        for location, outcome in rebuilt.rebuilds
        if isinstance(outcome, RefusedRebuild)
    }
    blob_locations = {item.location for item in rebuilt.declarations}
    formed = any(item.reason == "total_formed" for item in findings)
    structure_reason: UncheckedReason | None = None
    structure = None
    if refused_payloads or refused_rebuilds or formed:
        structure_reason = "exceeds_capacity"
    else:
        counted = content.opaque_positions | {
            finding.position
            for finding in findings
            if finding.position is not None
            and (
                finding.code in _COUNTED_FINDINGS
                or finding.code == "index_claim_failed"
                and finding.reason
                not in ("index_offsets", "index_locator", "index_digest")
                or finding.code == "missing_dependency"
                and finding.reason in ("chunk", "value")
            )
        }
        counted.update(location.position for location in fold.blobs.length_conflicts)
        counted.update(
            frame.position for frame in wire.frames if frame.link.state == "healed"
        )
        frames = {frame.position: frame for frame in wire.frames}
        packs = {
            position: blob
            for position, blob in content.blobs
            if isinstance(blob, tuple)
        }
        lengths = {entry.name: entry.length for entry in fold.blobs.entries}
        verified = {
            item.name: body
            for item in rebuilt.declarations
            if item.length == lengths[item.name] and (body := item.bytes()) is not None
        }
        items = tuple(
            StructureItem(
                kind=item.kind,
                size=item.end - item.offset,
                frame_type=frame.frame_type if frame is not None else None,
                envelope=frame.envelope
                if frame is not None and isinstance(frame.envelope, Frame)
                else None,
                decoded=frame.payload.decoded if frame is not None else None,
                pack_members=packs.get(position),
                counted_failure=position in counted,
            )
            for position, item in enumerate(wire.items)
            for frame in (frames.get(position),)
        )
        structure = check_structure(
            opener.header,
            items,
            StructureFold(
                terms=fold.terms,
                rows=fold.lines.lines,
                records=fold.records,
                values=fold.values,
                blobs=fold.blobs,
                verified_blobs=verified,
                labelling_succeeded=labelled is not None,
            ),
        )
    index_position = max(content.index_digests, default=0)
    index_digest = content.index_digests.get(index_position)
    content_refused = any(
        frame.position in refused_payloads and frame.frame_type not in (14, 15, 16)
        for frame in wire.frames
    )
    labelling_reason: UncheckedReason | None = None
    labelling = None
    if (
        formed
        or content_refused
        or refused_rebuilds - blob_locations
        or (index_position and index_digest is None)
    ):
        labelling_reason = "exceeds_capacity"
    elif labelled is None:
        labelling_reason = "labelling_refused"
    else:
        labelling = check_canonical_labelling(
            fold.terms,
            fold.lines.lines,
            fold.records,
            fold.values,
            labelled,
            origins,
            None if index_digest is None else (index_position, index_digest),
        )
    difference: Difference | None = structure
    if labelling is not None and (
        structure is None or labelling.position < structure.position
    ):
        difference = labelling
    return CanonicalResult(
        True,
        checked(structure, structure_reason),
        checked(labelling, labelling_reason),
        CheckState(
            "unchecked", "exceeds_capacity" if refused_payloads else "encoder_not_held"
        ),
        difference,
    )


def _own_labelling(
    wire: WireRead,
    ordinal: int,
    segment: _SegmentContent,
    rebuilt: BlobSegment,
    blob_read: BlobRead,
    cache: dict[ValueReference, bytes],
    budget: FoldLabellingBudget,
    findings: list[ContentFinding],
    segment_findings_at: int,
) -> tuple[LabellingReport | None, CanonicalResult]:
    wire_segment = wire.segments[ordinal]
    claimed = _claims_canonical(wire_segment)
    declarations = rebuilt.declarations
    size = folded_size(
        ((ordinal, (event.frame for event in segment.graph)),),
        segment.records,
        len(declarations),
    )
    admitted = True
    try:
        budget.check_size(size)
    except FoldLabellingRefused as exc:
        findings.append(ContentFinding(ordinal, None, "labelling_refused", exc.reason))
        if not claimed:
            return None, NOT_CLAIMED
        admitted = False
    own_findings: list[ContentFinding] = []
    graph = _fold_graph(
        ((ordinal, segment.graph),),
        blob_read,
        cache,
        own_findings if claimed else None,
        own=True,
    )
    assert graph is not None
    blobs = blob_set(declarations)
    records = fold_records(
        segment.records,
        _record_scope(wire, graph.dictionaries, blobs),
        graph.pool,
        graph.lines.lines,
        check_referents=False,
    )
    labelled = (
        _label_fold(budget, size, graph, records, blobs, findings, ordinal)
        if admitted
        else None
    )
    report = None if labelled is None else _labelling_report(labelled)
    closing = segment.closing_index
    if labelled is not None and closing is not None:
        statistics = IndexStatistics(
            len(graph.lines.lines),
            sum(1 for _ in record_bodies(records, blobs, graph.values)),
            len(labelled.blank_positions),
        )
        findings.extend(
            ContentFinding(ordinal, closing.position, "index_claim_failed", reason)
            for reason in closing.disagrees(statistics, labelled.content_digest)
        )
    if not claimed:
        return report, NOT_CLAIMED
    own_findings.extend(findings[segment_findings_at:])
    own_findings.extend(
        ContentFinding(ordinal, note.location.position, note.code, note.reason)
        for note in records.findings
    )
    canonical = _canonical_levels(
        wire_segment,
        segment,
        graph.with_records(blobs, records, declarations),
        rebuilt,
        labelled,
        _blank_origins(segment, graph.dictionaries[ordinal]),
        own_findings,
    )
    return report, canonical


def read_content(
    data: bytes,
    capacity: ContentCapacity = _DEFAULT_CAPACITY,
    *,
    verifier: SignatureVerifier | None = None,
    first_and_second: bool = True,
) -> ContentRead:
    """Read wire bytes and fold the supported graph and blob content.

    The result retains generation dispatch, damage, and graph states named by
    resolved bindings, and reports authentication and suppression separately.

    ``first_and_second`` is passed to ``read_wire`` and defaults to true: a
    first-generation input is reported as a ``first_generation`` decision for
    the legacy reader. With false it is refused as ``unsupported_generation``.
    """
    wire = read_wire(
        data,
        items_per_read=capacity.items_per_read,
        stored_bytes_per_frame=capacity.stored_bytes_per_frame,
        decoded_bytes_per_frame=capacity.decoded_bytes_per_frame,
        decoded_bytes_per_read=capacity.decoded_bytes_per_read,
        first_and_second=first_and_second,
    )
    lineages = check_lineages(wire.segments)
    formed = FormedBudget(capacity.formed_bytes_per_read)
    blob_read = BlobRead(capacity.rebuilt_bytes_per_read)
    labelling_budget = FoldLabellingBudget(
        labelled_lines=capacity.labelled_lines,
        labelled_bytes_per_read=capacity.labelled_bytes_per_read,
        labelling_units=capacity.labelling_units,
    )
    verifier = SignatureVerifier() if verifier is None else verifier
    segments: dict[int, _SegmentContent] = {}
    coverage_observations: dict[int, SegmentCoverage] = {}
    blob_segments: list[BlobSegment | None] = []
    candidate_states: dict[bytes, list[_StatePrefix]] = {}
    authentication: list[SegmentAuthentication] = []
    own_reports: list[LabellingReport | None] = [None] * len(wire.segments)
    canonical_reports = [NOT_CLAIMED] * len(wire.segments)
    cache: dict[ValueReference, bytes] = {}
    unfolded = unopened = unauthenticated = 0
    run_is_usable = False
    findings: list[ContentFinding] = []
    opaque_reasons: dict[tuple[int, int], OpaqueReason] = {}
    for ordinal, segment in enumerate(wire.segments):
        segment_findings_at = len(findings)
        opener = segment.opener.value
        if isinstance(opener, WireError) or opener.identifier is None:
            blob_segments.append(None)
            authentication.append(
                SegmentAuthentication((), frozenset(), 0, "unauthenticated")
            )
            continue
        last_commit, candidates, valid_commits = _commits(
            segment,
            ordinal,
            opener.identifier,
            capacity.stored_bytes_per_frame,
            findings,
        )
        findings.extend(
            ContentFinding(
                ordinal,
                note.position,
                note.code,
                note.reason,
                note.count,
                note.offset,
                note.length,
            )
            for note in segment.parity.findings
        )
        segment_content = _read_segment_content(
            data,
            segment,
            ordinal,
            formed,
            capacity.stored_bytes_per_frame,
            last_commit,
            findings,
        )
        opaque_reasons.update(
            ((ordinal, position), reason)
            for position, reason in segment_content.opaque_reasons.items()
        )
        registry = opener.header.registry_version
        authenticity = authenticate_segment(
            opener.identifier,
            (
                AuthenticationFrame(
                    frame.position,
                    frame.frame_type,
                    frame.link,
                    _read_signatures(
                        data, segment, frame, capacity.stored_bytes_per_frame, registry
                    ),
                    frame.transparent,
                    frame.detached,
                    frame.position in valid_commits,
                )
                for frame in segment.frames
            ),
            verifier,
        )
        authentication.append(authenticity)
        coverage_observations[ordinal] = SegmentCoverage(
            len(segment_content.opaque_positions),
            last_commit,
            any(
                position == last_commit
                and any(result.authenticated for result in signatures)
                for position, signatures in authenticity.signatures
            ),
        )
        folded = {record.location.position for record in segment_content.records}
        for frame in segment.frames:
            if frame.frame_type == 7:
                if frame.position not in folded:
                    unfolded += 1
                elif frame.position not in authenticity.authenticated_positions:
                    unauthenticated += 1
            elif registry >= 2 and frame.frame_type == 18:
                # Registry-2 sealing is not opened by this reader.
                unopened += 1
        run_start = lineages[ordinal].run_start
        if run_start == ordinal:
            run_is_usable = True
        elif run_start is None:
            run_is_usable = False
        first_ineligible = segment_content.first_ineligible
        if run_is_usable:
            assert run_start is not None
            for position, identifier in candidates:
                if first_ineligible is not None and position >= first_ineligible:
                    break
                prefix = _StatePrefix(run_start, RecordLocation(ordinal, position))
                candidate_states.setdefault(identifier, []).append(prefix)
            run_is_usable = first_ineligible is None
        segments[ordinal] = segment_content
        rebuilt = blob_read.fold_segment(
            ordinal, segment_content.blobs, algorithm=opener.header.hash_algorithm
        )
        blob_segments.append(rebuilt)
        findings.extend(
            ContentFinding(
                ordinal,
                note.location.position,
                note.code,
                note.reason,
                note.count,
            )
            for note in rebuilt.findings
        )
        if wire.item_limit_offset is None:
            report, canonical = _own_labelling(
                wire,
                ordinal,
                segment_content,
                rebuilt,
                blob_read,
                cache,
                labelling_budget,
                findings,
                segment_findings_at,
            )
            own_reports[ordinal] = report
            canonical_reports[ordinal] = canonical
            if canonical.first_difference is not None:
                difference = canonical.first_difference
                findings.append(
                    ContentFinding(
                        ordinal,
                        difference.position,
                        "canonical_violated",
                        difference.check,
                    )
                )
        elif _claims_canonical(segment):
            canonical_reports[ordinal] = ITEM_LIMIT

    coverage = file_coverage(wire, coverage_observations)
    first_item = wire.opening.first_item
    if first_item is not None and isinstance(wire.opening.decision, str):
        # P2 `not_this_format` and `unsupported_generation`: the first item is the
        # finding's subject, at offset 0 with its byte length.
        findings.append(
            ContentFinding(
                None,
                None,
                wire.opening.decision,
                offset=first_item.offset,
                length=first_item.end - first_item.offset,
            )
        )
    if not wire.segments or wire.item_limit_offset is not None:
        return ContentRead(
            wire,
            lineages,
            tuple(blob_segments),
            tuple(findings),
            None,
            authentication=tuple(authentication),
            own_labelling=tuple(own_reports),
            coverage=coverage,
            canonical=tuple(canonical_reports),
            opaque_reasons=opaque_reasons,
        )

    declarations = tuple(
        declaration
        for segment in blob_segments
        if segment is not None
        for declaration in segment.declarations
    )
    file_size = folded_size(
        (
            (ordinal, (event.frame for event in segment.graph))
            for ordinal, segment in segments.items()
        ),
        (record for segment in segments.values() for record in segment.records),
        len(declarations),
    )
    file_size_admitted = True
    try:
        labelling_budget.check_size(file_size)
    except FoldLabellingRefused as exc:
        findings.append(ContentFinding(None, None, "labelling_refused", exc.reason))
        file_size_admitted = False

    graph = _fold_graph(
        ((ordinal, segment.graph) for ordinal, segment in segments.items()),
        blob_read,
        cache,
        findings,
    )
    assert graph is not None
    blobs = blob_set(declarations)
    findings.extend(
        ContentFinding(loc.segment, loc.position, "conflicting_declaration")
        for loc in blobs.length_conflicts
    )
    states: dict[bytes, GraphState | None] = {}

    def resolve_state(identifier: bytes) -> bool:
        if identifier in states:
            return states[identifier] is not None
        for prefix in candidate_states.get(identifier, ()):
            ordinals = range(prefix.first_segment, prefix.commit.segment + 1)
            state_graph = _fold_graph(
                ((ordinal, segments[ordinal].graph) for ordinal in ordinals),
                blob_read,
                cache,
                findings,
                prefix,
            )
            if state_graph is None:
                continue
            state_declarations = tuple(
                blob_read.state_declaration(declaration, commit=prefix.boundary)
                for ordinal in ordinals
                if (segment := blob_segments[ordinal]) is not None
                for declaration in segment.declarations
                if prefix.includes(ordinal, declaration.location.position)
            )
            state_blobs = blob_set(state_declarations)
            state_records = fold_records(
                (
                    record
                    for ordinal in ordinals
                    for record in segments[ordinal].records
                    if prefix.includes(ordinal, record.location.position)
                ),
                _record_scope(wire, state_graph.dictionaries, state_blobs, prefix),
                state_graph.pool,
                state_graph.lines.lines,
                check_referents=False,
            )
            states[identifier] = GraphState(
                identifier,
                prefix.commit,
                prefix.first_segment,
                state_graph.with_records(
                    state_blobs, state_records, state_declarations
                ),
            )
            return True
        states[identifier] = None
        return False

    records = fold_records(
        (record for segment in segments.values() for record in segment.records),
        _record_scope(wire, graph.dictionaries, blobs),
        graph.pool,
        graph.lines.lines,
        resolve_state=resolve_state,
    )
    findings.extend(
        ContentFinding(
            note.location.segment, note.location.position, note.code, note.reason
        )
        for note in records.findings
    )
    labelled = (
        _label_fold(labelling_budget, file_size, graph, records, blobs, findings, None)
        if file_size_admitted
        else None
    )
    return ContentRead(
        wire,
        lineages,
        tuple(blob_segments),
        tuple(findings),
        graph.with_records(blobs, records, declarations, labelled),
        tuple(state for state in states.values() if state is not None),
        tuple(authentication),
        Visibility(unfolded, unopened, unauthenticated),
        tuple(own_reports),
        coverage,
        tuple(canonical_reports),
        opaque_reasons=opaque_reasons,
    )

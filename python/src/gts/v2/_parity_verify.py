# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Judge parity frames, shard digests, and unprotected bytes (P5 §§6.6–6.7)."""

from bisect import bisect_right
from dataclasses import dataclass
from typing import TYPE_CHECKING

import cbor2

from ._decode import WireError, _Decoder
from ._parity_repair import _Patch, _rejected_spans, _Repair
from ._parity_shards import _shard_digest, _solve_shards
from .frame import _PARITY, Frame, _frame_field_offsets
from .items import DamagedRegion, Item
from .parity import SPAN_LENGTH, _Layout, _layout, _read_content, _valid_table
from .payload import PayloadBudget

if TYPE_CHECKING:
    from .reader import WireFrame, WireSegment


@dataclass(frozen=True, slots=True)
class ParityFinding:
    code: str
    position: int | None
    reason: str | None = None
    count: int | None = None
    offset: int | None = None
    length: int | None = None


@dataclass(frozen=True, slots=True)
class SegmentParity:
    findings: tuple[ParityFinding, ...] = ()
    future_geometry: frozenset[int] = frozenset()


@dataclass(frozen=True, slots=True)
class _Span:
    first: int
    end: int
    layout: _Layout
    payload: memoryview


@dataclass(frozen=True, slots=True)
class _ShardFailures:
    data_digests: list[bytes]
    parity_digests: list[bytes]
    parity: bytes
    data: set[int]
    parity_indices: set[int]
    unexplained: set[int]

    @property
    def stale(self) -> bool:
        return bool(self.data) and self.data == self.unexplained


def _payload_finding(frame: "WireFrame", item: Item) -> ParityFinding:
    check = frame.payload
    return ParityFinding(
        "exceeds_capacity" if check.state == "unchecked" else "damaged_frame",
        frame.position,
        check.reason,
        check.count,
        item.offset,
        item.end - item.offset,
    )


class _ParityFrames:
    def __init__(self, data: bytes, root: bytes, registry: int) -> None:
        self.data = data
        self.root = root
        self.registry = registry
        self.starts: set[int] = set()
        self.short = False

    def judge(
        self, frame: "WireFrame", item: Item, predecessor: bytes
    ) -> _Span | ParityFinding | None:
        """Determine whether a known parity frame supplies a usable table."""
        damaged = ParityFinding("parity_damaged", frame.position)
        if frame.link.state == "broken":
            return damaged
        envelope = frame.envelope
        if isinstance(envelope, Frame):
            raw = envelope.content
        else:
            fields = _frame_field_offsets(self.data, item.offset)
            raw = memoryview(self.data)[fields[3] : fields[4]]
        try:
            geometry, first, end, anchor, table = _read_content(
                _Decoder(raw, 0, WireError)
            )
        except WireError:
            return damaged
        if geometry > 1 and self.registry > 1 and isinstance(envelope, Frame):
            return None
        span = end - first
        repeated = self.short or first in self.starts
        self.starts.add(first)
        self.short |= 1 <= span < SPAN_LENGTH
        if (
            repeated
            or frame.link.state != "linked"
            or not isinstance(envelope, Frame)
            or geometry != 1
            or first % SPAN_LENGTH
            or not 1 <= span <= SPAN_LENGTH
            or anchor != predecessor
        ):
            return damaged
        if frame.payload.state == "unchecked":
            return _payload_finding(frame, item)
        layout = _layout(span)
        payload = envelope.stored_payload
        if not _valid_table(payload, 0, len(payload), layout, self.root, table):
            return damaged
        return _Span(first, end, layout, payload)


def _passes(frame: "WireFrame | None") -> bool:
    return (
        frame is not None
        and isinstance(frame.envelope, Frame)
        and frame.link.state == "linked"
        and (
            frame.payload.state == "verified"
            if frame.envelope.storage is None
            else frame.payload.stored_matches is True
        )
    )


def check_parity(
    data: bytes, segment: "WireSegment", budget: PayloadBudget
) -> tuple[SegmentParity, list[_Patch]]:
    """Judge parity on the input and verify the candidate repairs separately."""
    findings: list[ParityFinding] = []
    future: set[int] = set()
    if not any(frame.frame_type == _PARITY for frame in segment.frames):
        return SegmentParity(), []
    opener = segment.opener.value
    assert not isinstance(opener, WireError) and opener.identifier is not None
    predecessor = opener.identifier
    parity_frames = _ParityFrames(data, predecessor, opener.header.registry_version)
    frames = {frame.position: frame for frame in segment.frames}
    first_ordinary = next(
        (frame for frame in segment.frames if not frame.transparent), None
    )
    opener_passes = first_ordinary is not None and first_ordinary.link.state in (
        "linked",
        "healed",
    )
    source = memoryview(data)
    parts: list[memoryview] = []
    starts: list[int] = []
    offsets: list[int] = []
    failed_starts: list[int] = []
    failed_ends: list[int] = []
    repairs: list[_Repair] = []
    stream_end = protected_end = 0
    judged = False
    for position, item in enumerate(segment.items):
        frame = frames.get(position)
        anchor_predecessor = predecessor
        if frame is not None and frame.link.effective_identifier is not None:
            predecessor = frame.link.effective_identifier

        if frame is not None and frame.frame_type == _PARITY:
            assert isinstance(item, Item)
            if position and isinstance(segment.items[position - 1], DamagedRegion):
                # Resumption supplies the predecessor in the candidate's anchor.
                assert isinstance(frame.envelope, Frame)
                _, _, _, anchor, _ = _read_content(
                    _Decoder(frame.envelope.content, 0, WireError)
                )
                anchor_predecessor = predecessor = anchor
            checked = parity_frames.judge(frame, item, anchor_predecessor)
            if checked is None:
                future.add(position)
                if frame.payload.state != "verified":
                    findings.append(_payload_finding(frame, item))
                continue
            judged = True
            if isinstance(checked, ParityFinding):
                findings.append(checked)
            else:
                if frame.payload.state == "damaged":
                    findings.append(ParityFinding("parity_damaged", position))
                first, end = checked.first, checked.end
                if not starts[-1] < end <= stream_end or (
                    end - first < SPAN_LENGTH and end != stream_end
                ):
                    findings.append(ParityFinding("parity_stale", position))
                    continue
                covered = bytearray()
                for index in range(
                    bisect_right(starts, first) - 1,
                    bisect_right(starts, end - 1),
                ):
                    start = starts[index]
                    covered.extend(parts[index][max(first - start, 0) : end - start])
                failures = _shard_findings(
                    covered,
                    checked.payload,
                    checked.layout.shard_length,
                    first,
                    position,
                    failed_starts,
                    failed_ends,
                    findings,
                )
                if not failures.stale:
                    protected_end = max(protected_end, end)
                    if failures.data != failures.unexplained or failures.parity_indices:
                        repair = _solve_span(
                            checked,
                            failures,
                            covered,
                            position,
                            item.offset,
                            data,
                            starts,
                            offsets,
                            parts,
                        )
                        if repair is None:
                            findings.append(
                                ParityFinding(
                                    "repair_failed", position, "too_many_erasures"
                                )
                            )
                        else:
                            repairs.append(repair)

        if (
            frame is not None
            and frame.transparent
            or item.kind in ("offsets", "locator")
        ):
            continue
        starts.append(stream_end)
        parts.append(source[item.offset : item.end])
        offsets.append(item.offset)
        stream_end += item.end - item.offset
        if not (opener_passes if position == 0 else _passes(frame)):
            failed_starts.append(starts[-1])
            failed_ends.append(stream_end)

    patches: list[_Patch] = []
    if repairs:
        rejected = _rejected_spans(data, segment, repairs, budget)
        for repair in repairs:
            if repair.position in rejected:
                findings.append(
                    ParityFinding("repair_failed", repair.position, "verify_failed")
                )
            else:
                patches.extend(repair.data)
                patches.extend(repair.parity)
                findings.append(
                    ParityFinding("repaired", repair.position, count=repair.count)
                )
    unprotected = max(protected_end, len(parts[0]))
    if judged and unprotected < stream_end:
        index = bisect_right(starts, unprotected) - 1
        findings.append(
            ParityFinding(
                "unprotected_tail",
                None,
                offset=offsets[index] + unprotected - starts[index],
                length=stream_end - unprotected,
            )
        )
    return SegmentParity(tuple(findings), frozenset(future)), patches


def _shard_findings(
    covered: bytearray,
    payload: memoryview,
    size: int,
    first: int,
    position: int,
    failed_starts: list[int],
    failed_ends: list[int],
    findings: list[ParityFinding],
) -> _ShardFailures:
    data_digests, parity_digests, parity = cbor2.loads(payload)
    failed_data: set[int] = set()
    unexplained: set[int] = set()
    for shard_index, digest in enumerate(data_digests):
        offset = shard_index * size
        shard = covered[offset : offset + size].ljust(size, b"\0")
        if _shard_digest(shard) != digest:
            failed_data.add(shard_index)
            start = first + offset
            index = bisect_right(failed_ends, start)
            if index == len(failed_starts) or failed_starts[index] >= start + size:
                unexplained.add(shard_index)
    failed_parity = {
        row
        for row, digest in enumerate(parity_digests)
        if _shard_digest(parity[row * size : (row + 1) * size]) != digest
    }
    if failed_data:
        findings.append(
            ParityFinding("parity_mismatch", position, "data", len(failed_data))
        )
    if failed_parity:
        findings.append(
            ParityFinding("parity_mismatch", position, "parity", len(failed_parity))
        )
    if failed_data and unexplained == failed_data:
        findings.append(ParityFinding("parity_stale", position))
    elif unexplained:
        findings.append(
            ParityFinding("parity_unexplained", position, count=len(unexplained))
        )
    return _ShardFailures(
        data_digests, parity_digests, parity, failed_data, failed_parity, unexplained
    )


def _solve_span(
    span: _Span,
    failures: _ShardFailures,
    covered: bytearray,
    position: int,
    parity_offset: int,
    data: bytes,
    starts: list[int],
    offsets: list[int],
    parts: list[memoryview],
) -> _Repair | None:
    size = span.layout.shard_length
    solved = _solve_shards(
        covered, failures.parity, size, failures.data, failures.parity_indices
    )
    if solved is None:
        return None
    solved_data, solved_parity = solved
    digests_hold = True
    patches = []
    for index, shard in solved_data.items():
        digests_hold &= _shard_digest(shard) == failures.data_digests[index]
        if index in failures.unexplained:
            continue
        start = span.first + index * size
        end = min(start + size, span.end)
        for part in range(
            bisect_right(starts, start) - 1, bisect_right(starts, end - 1)
        ):
            part_start = starts[part]
            left = max(start, part_start)
            right = min(end, part_start + len(parts[part]))
            patches.append(
                _Patch(
                    offsets[part] + left - part_start,
                    shard[left - start : right - start],
                )
            )
    parity_patches = []
    if solved_parity:
        payload_end = _frame_field_offsets(data, parity_offset)[8]
        parity_start = payload_end - len(failures.parity)
        for row, shard in solved_parity.items():
            digests_hold &= _shard_digest(shard) == failures.parity_digests[row]
            parity_patches.append(_Patch(parity_start + row * size, shard))
    return _Repair(
        position,
        len(solved_data) - len(failures.unexplained) + len(solved_parity),
        digests_hold,
        tuple(patches),
        tuple(parity_patches),
    )

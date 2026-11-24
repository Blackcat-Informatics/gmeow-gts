# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Combine wire and fold observations for P7 §1.2."""

from collections.abc import Mapping
from dataclasses import dataclass

from .compaction import FileEnd, ReadCoverage
from .framing import FramingReason
from .reader import WireRead

_FRAMING_ENDS: dict[FramingReason, FileEnd] = {
    "torn_tail": FileEnd.TORN,
    "undelimitable_input": FileEnd.UNDELIMITABLE,
}


@dataclass(frozen=True, slots=True)
class SegmentCoverage:
    opaque_frames: int
    last_commit: int
    authenticating_commit: bool


def _file_end(wire: WireRead, last: SegmentCoverage | None) -> FileEnd:
    if wire.item_limit_offset is not None:
        return FileEnd.NOT_READ
    decision = wire.opening.decision
    if not wire.segments:
        if decision in ("torn_tail", "undelimitable_input"):
            return _FRAMING_ENDS[decision]
        return FileEnd.NO_COMMIT
    segment = wire.segments[-1]
    failure = segment.framing_failure
    if failure is not None and failure.failure is not None:
        return _FRAMING_ENDS[failure.failure]
    if last is None or not last.last_commit:
        return FileEnd.NO_COMMIT
    if any(
        frame.position > last.last_commit
        and not frame.transparent
        and frame.frame_type != 15
        for frame in segment.frames
    ) or any(
        segment.items[position].kind in ("opaque", "damaged_region")
        for position in range(last.last_commit + 1, len(segment.items))
    ):
        return FileEnd.AFTER_COMMIT
    if last.authenticating_commit:
        return FileEnd.AT_AUTHENTICATED_COMMIT
    return FileEnd.AT_VALID_COMMIT


def file_coverage(
    wire: WireRead, observations: Mapping[int, SegmentCoverage]
) -> ReadCoverage | None:
    """Report omitted items and end for a second-generation whole read."""
    decision = wire.opening.decision
    if decision == "first_generation":
        return None
    prefix = wire.opening.damaged_prefix
    # Before the first opener there is no segment to contain a damaged
    # region. A torn prefix counts by its framing finding.
    left_out = int(
        decision == "torn_tail"
        or (prefix is not None and prefix.failure == "torn_tail")
    )
    for ordinal, segment in enumerate(wire.segments):
        observation = observations.get(ordinal)
        if observation is None:
            left_out += len(segment.items) - 1
        else:
            left_out += observation.opaque_frames + sum(
                item.kind in ("opaque", "damaged_region") for item in segment.items
            )
        failure = segment.framing_failure
        # Resumptions record a damaged region as an item. An undelimitable
        # suffix with no resumption affects the file end.
        if failure is not None and failure.failure == "torn_tail":
            left_out += 1
    return ReadCoverage(
        _file_end(wire, observations.get(len(wire.segments) - 1)), left_out
    )

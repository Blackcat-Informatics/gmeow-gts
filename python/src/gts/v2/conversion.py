# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Convert first-generation GTS to canonical v2 with a detached P9 proof.

Result admission reads two places. ``ContentRead.findings`` carries content-level
findings only. Wire-level outcomes (a torn tail, undelimitable input, scan refusals
and a broken link) stay in the wire read structures of ``ContentRead.wire``: the
opening decision and damaged prefix, each segment's framing failure and items, and
each frame's link and payload states. Admission consults both, so a caller that
inspects a read does the same.
"""

from collections.abc import Callable

from ._conversion_output import _DEFAULT_CAPACITY, ConversionCapacity
from ._conversion_result import Converted, ResultRead, ResultSegment, _convert
from ._decode import WireError
from .blobs import BlobLocation
from .content import ContentCapacity, ContentRead, read_content
from .conversion_fold import ConversionStop
from .items import DamagedRegion
from .reader import WireSegment
from .rebuild import RebuildResult, RefusedRebuild, UncheckedRebuild

_ERROR_FINDINGS = frozenset(
    (
        "unsupported_registry_version",
        "unsupported_critical",
        "malformed_value",
        "chain_break",
        "damaged_frame",
        "commit_count_mismatch",
        "commit_root_mismatch",
        "declared_order_violated",
        "forward_reference",
        "position_constraint",
        "ill_formed_term",
        "canonical_violated",
        "parity_mismatch",
        "repair_failed",
        "conflicting_declaration",
        "profile_violation",
    )
)
# Error-severity codes of the P2 finding table that admission checks elsewhere, with
# the check. A test compares this table and ``_ERROR_FINDINGS`` with ``docs/v2/P2.md``.
_SEPARATE_ERROR_CHECKS = {
    "proof_malformed": "detached proof checks in gts.v2.range",
    "proof_failed": "detached proof checks in gts.v2.range",
    "signature_invalid": "signature outcomes in _read_result",
}
_CAPACITY_REASONS = ("stored_size", "decoded_size", "total_decoded")


def _rebuilds(
    read: ContentRead, ordinal: int
) -> tuple[tuple[BlobLocation, RebuildResult], ...]:
    segment = read.blob_segments[ordinal]
    return () if segment is None else segment.rebuilds


def _capacity_refused(read: ContentRead) -> bool:
    return (
        read.wire.item_limit_offset is not None
        or any(
            frame.payload.state == "unchecked"
            and frame.payload.reason in _CAPACITY_REASONS
            for segment in read.wire.segments
            for frame in segment.frames
        )
        or any(
            finding.code == "labelling_refused"
            or finding.code == "exceeds_capacity"
            and finding.reason == "total_formed"
            for finding in read.findings
        )
        or any(
            isinstance(outcome, RefusedRebuild)
            for ordinal in range(len(read.wire.segments))
            for _, outcome in _rebuilds(read, ordinal)
        )
    )


def _intact(read: ContentRead, segment: WireSegment, ordinal: int) -> bool:
    opener = segment.opener.value
    return (
        not isinstance(opener, WireError)
        and opener.identifier is not None
        and not any(
            isinstance(item, DamagedRegion) or item.reason is not None
            for item in segment.items
        )
        and all(
            frame.transparent
            or (
                not isinstance(frame.envelope, WireError)
                and frame.link.state != "broken"
                and frame.payload.state == "verified"
            )
            for frame in segment.frames
        )
        and all(
            note.segment != ordinal
            or (
                note.code not in ("malformed_value", "damaged_frame")
                and note.reason != "total_formed"
            )
            for note in read.findings
        )
        and all(
            not isinstance(outcome, RefusedRebuild | UncheckedRebuild)
            for _, outcome in _rebuilds(read, ordinal)
        )
        and all(
            finding.reason in ("index_offsets", "index_locator")
            for finding in segment.closing
        )
    )


def _read_result(data: bytes, capacity: ConversionCapacity) -> ResultRead:
    """Project the independent reader's observations for P9 result admission."""
    read = read_content(
        data,
        ContentCapacity(
            stored_bytes_per_frame=capacity.stored_bytes_per_frame,
            decoded_bytes_per_frame=capacity.decoded_bytes_per_frame,
            decoded_bytes_per_read=capacity.decoded_bytes_per_read,
            items_per_read=capacity.items_per_read,
            rebuilt_bytes_per_read=capacity.rebuilt_bytes_per_read,
            formed_bytes_per_read=capacity.formed_bytes_per_read,
            labelled_lines=capacity.labelled_lines,
            labelled_bytes_per_read=capacity.labelled_bytes_per_read,
            labelling_units=capacity.labelling_units,
        ),
    )
    refused = _capacity_refused(read)
    if refused or read.wire.opening.decision == "first_generation":
        return ResultRead(
            segments=None,
            content_digest=None,
            visible_complete=False,
            capacity_refused=refused,
            error_or_fatal=False,
        )
    error = (
        read.wire.opening.damaged_prefix is not None
        or any(
            segment.framing_failure is not None
            and segment.framing_failure.failure == "undelimitable_input"
            for segment in read.wire.segments
        )
        or any(finding.code in _ERROR_FINDINGS for finding in read.findings)
        or any(
            signature.outcome.status == "invalid"
            for segment in read.authentication
            for _, signatures in segment.signatures
            for signature in signatures
        )
    )
    segments = []
    for ordinal, (segment, canonical, report) in enumerate(
        zip(read.wire.segments, read.canonical, read.own_labelling, strict=True)
    ):
        intact = _intact(read, segment, ordinal)
        verified = canonical.structure.state == "verified"
        segments.append(
            ResultSegment(
                canonical_claimed=canonical.claimed,
                structure_verified=verified,
                labelling_verified=canonical.labelling.state == "verified",
                intact=intact,
                content_digest=None if report is None else report.content_digest,
            )
        )
    return ResultRead(
        segments=tuple(segments),
        content_digest=None if read.fold is None else read.fold.content_digest,
        visible_complete=read.visibility is not None and read.visibility.complete,
        capacity_refused=False,
        error_or_fatal=error,
    )


def convert(
    read: Callable[[], bytes], capacity: ConversionCapacity = _DEFAULT_CAPACITY
) -> Converted | ConversionStop:
    """Read the source twice and return verified canonical v2 bytes and a proof."""
    return _convert(read, _read_result, capacity)

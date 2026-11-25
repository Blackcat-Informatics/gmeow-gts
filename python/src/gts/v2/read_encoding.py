# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Encode the typed file result of a read as the P7 `file_result` records."""

from .canonical_check import CanonicalResult, CheckState, Difference
from .file_result import FoldState, OpaqueFold, PayloadState
from .findings import REASON_CODES, FindingRecord
from .segment_result import (
    FrameResult,
    HeaderOpener,
    RebuildRecord,
    ResyncOpener,
    SegmentResult,
)
from .typed_file import (
    CompleteVisibility,
    FileResult,
    IncompleteVisibility,
    SegmentRecord,
)

# Each table below is the code of one P2 enumeration, as its section 4 states it.
_LINK = {"linked": 1, "healed": 2, "broken": 3}
_STORED = {"matches": 1, "mismatch": 2}
_REACHED = {"walked": 1, "scanned": 2}
_PAST = {"absent": 1, "matches": 2, "differs": 3, "not_checked": 4}
_INTEGRITY = {"intact": 1, "intact_unchecked": 2, "damaged": 3, "not_checked": 4}
_AUTHENTICITY = {"authenticated": 1, "partly_authenticated": 2, "unauthenticated": 3}
_SIGNATURE_STATUS = {"valid": 1, "invalid": 2, "unverified": 3}
_UNCHECKED = {
    "unknown_codec": 1,
    "exceeds_capacity": 2,
    "labelling_refused": 3,
    "encoder_not_held": 4,
    "not_claimed": 5,
    "access": 6,
    "read_failed": 7,
}
_OPAQUE = {
    "damaged": 1,
    "malformed": 2,
    "unknown_frame_type": 3,
    "unsupported_critical": 4,
    "stopped": 5,
    "chain_break": 6,
    "exceeds_capacity": 7,
    "unknown_codec": 8,
}
_CANONICAL_CHECK = {
    "header": 1,
    "frame_sequence": 2,
    "terms": 3,
    "rows": 4,
    "value_region": 5,
    "blob_region": 6,
    "bindings": 7,
    "suppression": 8,
    "commit": 9,
    "index": 10,
    "labelling": 11,
    "content_digest": 12,
}
_FILE_END = {
    "at_authenticated_commit": 1,
    "at_valid_commit": 2,
    "after_commit": 3,
    "no_commit": 4,
    "torn": 5,
    "undelimitable": 6,
    "not_read": 7,
}
_REBUILD = {"verified": 1, "incomplete": 2, "refused": 3, "not_run": 4}


def _option(value: object | None) -> list[object]:
    """Return an `option(T)`: `[1]` when absent, else `[2, value]`."""
    if value is None:
        return [1]
    return [2, value]


def _check(state: CheckState) -> list[object]:
    """Return a `check_state` or a `storage_check_state` (`failed` is `differs`)."""
    if state.state == "unchecked":
        assert state.reason is not None
        return [2, _UNCHECKED[state.reason]]
    return [1] if state.state == "verified" else [3]


def _canonical(result: CanonicalResult) -> list[object]:
    first: list[object] = [1]
    if result.first_difference is not None:
        difference: Difference = result.first_difference
        first = [2, [_CANONICAL_CHECK[difference.check], difference.position]]
    return [
        result.claimed,
        _check(result.structure),
        _check(result.labelling),
        _check(result.storage),
        first,
    ]


def _payload(state: PayloadState) -> list[object]:
    if state == "verified":
        return [1]
    if state == "damaged":
        return [2]
    return [3, _UNCHECKED[state.reason]]


def _fold(state: FoldState) -> list[object]:
    if state == "folded":
        return [1]
    if state == "not_content":
        return [2]
    assert isinstance(state, OpaqueFold)
    return [3, _OPAQUE[state.reason]]


def _rebuild(record: RebuildRecord) -> list[object]:
    if record.state == "incomplete":
        assert record.reason is not None
        state: list[object] = [2, REASON_CODES[record.reason]]
    else:
        state = [_REBUILD[record.state]]
    return [_option(record.member), state]


def _frame(frame: FrameResult, findings: tuple[FindingRecord, ...]) -> list[object]:
    return [
        frame.position,
        frame.offset,
        frame.length,
        _option(frame.frame_type),
        _LINK[frame.link],
        _payload(frame.payload),
        _option(None if frame.stored is None else _STORED[frame.stored]),
        [
            [
                _SIGNATURE_STATUS[signature.outcome.status],
                _option(
                    None
                    if signature.outcome.reason is None
                    else REASON_CODES[signature.outcome.reason]
                ),
                signature.algorithm,
                signature.key_id,
                signature.coverage_first,
                signature.coverage_end,
                signature.authenticated,
            ]
            for signature in frame.signatures
        ],
        _fold(frame.fold),
        [_rebuild(record) for record in frame.rebuilds],
        [_finding(finding) for finding in findings],
    ]


def _subject(subject: tuple[int, ...]) -> list[object]:
    """Return a P2 `subject_kind`; a profile subject carries `check` `absent`."""
    tag = subject[0]
    if tag == 4:
        return [4, subject[1], [1]]
    return [tag, subject[1]]


def _finding(finding: FindingRecord) -> list[object]:
    return [
        finding.code,
        _option(finding.reason),
        _option(None if finding.subject is None else _subject(finding.subject)),
        _option(finding.position),
        _option(finding.offset),
        _option(finding.length),
        _option(finding.count),
    ]


def _opener(opener: HeaderOpener | ResyncOpener) -> list[object]:
    if isinstance(opener, HeaderOpener):
        return [1]
    return [
        2,
        _REACHED[opener.reached],
        opener.displaced,
        _PAST[opener.past],
        opener.marks_ignored,
    ]


def _segment(record: SegmentRecord) -> list[object]:
    result: SegmentResult = record.result
    return [
        result.ordinal,
        result.offset,
        result.length,
        _opener(result.opener),
        _option(result.header_identifier),
        _option(result.registry_version),
        _option(result.hash_algorithm),
        _option(result.profile),
        _INTEGRITY[result.integrity],
        _AUTHENTICITY[result.authenticity],
        result.anchored_frames,
        result.committed_frames,
        result.authenticated_frames,
        list(result.transparent_frames),
        _canonical(result.canonical),
        _option(result.content_digest),
        [_frame(frame.result, frame.findings) for frame in record.frames],
        [_finding(finding) for finding in record.findings],
    ]


def _visible(
    visible: CompleteVisibility | IncompleteVisibility,
) -> list[object]:
    if isinstance(visible, CompleteVisibility):
        return [1, visible.unauthenticated_suppress]
    return [
        2,
        visible.unfolded_suppress,
        visible.unopened_sealed,
        visible.unauthenticated_suppress,
    ]


def encode_file(file: FileResult) -> list[object]:
    """Return the second-generation `file_result` (tag `2`) as its P7 record.

    The result is deterministic CBOR when given to `cbor2.dumps(..., canonical=True)`.
    The first-generation tag is not encoded here: the typed reader does not produce it.
    """
    return [
        2,
        [_segment(segment) for segment in file.segments],
        [_finding(finding) for finding in file.findings],
        _option(file.content_digest),
        _visible(file.visible),
        _FILE_END[file.end],
        file.left_out,
    ]

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed findings of the GTS v2 read result (P2 section 5.1 and 6.1, P7 1.1)."""

from collections.abc import Iterable
from dataclasses import dataclass

import cbor2

from ._decode import WireError
from .content import ContentFinding, ContentRead
from .frame import FrameError
from .items import DamagedRegion, Item
from .reader import WireFrame
from .resync import RESYNC_MAX

_PARITY = 16
_REGISTRY_VERSION = 1

# The finding codes of registry version 1 (P2, 5.1).
FINDING_CODES = {
    "empty_input": 1,
    "not_this_format": 2,
    "unsupported_generation": 3,
    "unsupported_registry_version": 4,
    "unknown_enumeration_value": 5,
    "unsupported_critical": 6,
    "malformed_value": 7,
    "undelimitable_input": 8,
    "torn_tail": 9,
    "read_failed": 10,
    "chain_break": 11,
    "healed_link": 12,
    "damaged_frame": 13,
    "stored_check_mismatch": 14,
    "commit_count_mismatch": 15,
    "commit_root_mismatch": 16,
    "uncommitted_tail": 17,
    "index_claim_failed": 18,
    "missing_dependency": 19,
    "exceeds_capacity": 20,
    "declared_order_violated": 21,
    "forward_reference": 22,
    "position_constraint": 23,
    "ill_formed_term": 24,
    "opaque_term": 25,
    "labelling_refused": 26,
    "canonical_violated": 27,
    "suppression_unmatched": 28,
    "binding_unanchored": 29,
    "parity_damaged": 30,
    "parity_mismatch": 31,
    "parity_unexplained": 32,
    "parity_stale": 33,
    "repair_failed": 34,
    "repaired": 35,
    "unprotected_tail": 36,
    "proof_malformed": 37,
    "proof_failed": 38,
    "proof_holds": 39,
    "signature_invalid": 40,
    "signature_unverified": 41,
    "lineage_begins": 42,
    "lineage_mismatch": 43,
    "unreferenced_unit": 44,
    "conflicting_declaration": 45,
    "profile_violation": 46,
    "conversion_stopped": 47,
    "profile_not_checked": 48,
}

# The reasons of registry version 1 (P2, 6.1).
REASON_CODES = {
    "unknown_codec": 1,
    "unknown_frame_type": 2,
    "not_deterministic": 3,
    "wrong_type": 4,
    "wrong_length": 5,
    "out_of_range": 6,
    "unregistered_value": 7,
    "unordered_or_repeated": 8,
    "inconsistent_counts": 9,
    "forbidden_combination": 10,
    "codec_constraint": 11,
    "payload_digest": 12,
    "decoded_length": 13,
    "blob_length": 14,
    "blob_digest": 15,
    "decode_failed": 16,
    "index_head": 17,
    "index_root": 18,
    "index_offsets": 19,
    "index_digest": 20,
    "index_members": 21,
    "index_directory": 22,
    "index_locator": 23,
    "chunk": 24,
    "term": 25,
    "value": 26,
    "blob": 27,
    "frame": 28,
    "state": 29,
    "decoded_size": 30,
    "stored_size": 31,
    "refusal_width": 32,
    "refusal_limit": 33,
    "quad": 34,
    "reifier": 35,
    "graph": 36,
    "reifier_binding": 37,
    "annotation": 38,
    "binding": 39,
    "data": 40,
    "parity": 41,
    "too_many_erasures": 42,
    "verify_failed": 43,
    "count": 44,
    "length": 45,
    "order": 46,
    "inclusion": 47,
    "consistency_old_root": 48,
    "consistency_new_root": 49,
    "consistency_diverged": 50,
    "range": 51,
    "consistency": 52,
    "unknown_signature_algorithm": 53,
    "key_not_held": 54,
    "head": 55,
    "ordinal": 56,
    "key_mismatch": 57,
    "value_length": 58,
    "non_canonical_scalar": 59,
    "small_order_key": 60,
    "small_order_nonce": 61,
    "equation_failed": 62,
    "composite_classical_failed": 63,
    "composite_post_quantum_failed": 64,
    "header": 65,
    "frame_sequence": 66,
    "terms": 67,
    "rows": 68,
    "value_region": 69,
    "blob_region": 70,
    "bindings": 71,
    "suppression": 72,
    "commit": 73,
    "index": 74,
    "labelling": 75,
    "content_digest": 76,
    "source_damaged": 77,
    "source_not_decodable": 78,
    "unrepresentable_term": 79,
    "unrepresentable_frame": 80,
    "source_not_deterministic": 81,
    "source_exceeds_capacity": 82,
    "content_mismatch": 83,
    "unknown_item_kind": 84,
    "refusal_size": 85,
    "total_decoded": 86,
    "item_count": 87,
    "state_unavailable": 88,
    "total_rebuilt": 89,
    "total_formed": 90,
    "refusal_weight": 91,
    "refusal_term_node": 92,
}


@dataclass(frozen=True, slots=True)
class FindingRecord:
    """A P7 `finding`; `None` stands for `absent`."""

    code: int
    reason: int | None
    position: int | None
    offset: int | None
    length: int | None
    count: int | None
    subject: tuple[int, ...] | None = None
    # The step of the P7 procedure that produced the finding (P7 1.4, ORDER).
    step: int = 0


# The step of the procedure of P7 section 3 that produces each finding code. A code
# that several steps produce takes the step of its commonest producer here, and the
# producers of the others state theirs.
_STEPS = {
    "empty_input": 1,
    "not_this_format": 1,
    "unsupported_generation": 1,
    "undelimitable_input": 2,
    "torn_tail": 2,
    "read_failed": 2,
    "unsupported_registry_version": 3,
    "lineage_begins": 3,
    "lineage_mismatch": 3,
    "malformed_value": 11,
    "unknown_enumeration_value": 11,
    "unsupported_critical": 11,
    "chain_break": 5,
    "healed_link": 5,
    "stored_check_mismatch": 6,
    "damaged_frame": 7,
    "parity_damaged": 8,
    "parity_mismatch": 8,
    "parity_unexplained": 8,
    "parity_stale": 8,
    "repair_failed": 9,
    "repaired": 9,
    "unprotected_tail": 10,
    "declared_order_violated": 15,
    "forward_reference": 15,
    "position_constraint": 15,
    "ill_formed_term": 15,
    "opaque_term": 15,
    "commit_count_mismatch": 13,
    "commit_root_mismatch": 13,
    "uncommitted_tail": 13,
    "unreferenced_unit": 14,
    "conflicting_declaration": 14,
    "missing_dependency": 17,
    "suppression_unmatched": 17,
    "binding_unanchored": 17,
    "labelling_refused": 18,
    "index_claim_failed": 19,
    "signature_invalid": 20,
    "signature_unverified": 20,
    "profile_violation": 21,
    "profile_not_checked": 21,
    "canonical_violated": 22,
}
_CAPACITY_STEPS = {
    "item_count": 2,
    "stored_size": 7,
    "decoded_size": 7,
    "total_decoded": 7,
    "total_rebuilt": 14,
    "total_formed": 11,
}


def _step(code: str, reason: str | None) -> int:
    if code == "exceeds_capacity":
        return _CAPACITY_STEPS.get(str(reason), 11)
    if code == "unknown_enumeration_value" and reason in (
        "unknown_frame_type",
        "unknown_item_kind",
    ):
        # The items step (R4) judges an envelope's frame type and an item's kind.
        return 4
    if code == "missing_dependency" and reason == "chunk":
        return 11
    return _STEPS[code]


def sort_findings(records: Iterable[FindingRecord]) -> tuple[FindingRecord, ...]:
    """Sort findings by step, then position (absent first), then encoding (P7 1.4)."""

    def key(record: FindingRecord) -> tuple[int, int, int, bytes]:
        encoded = cbor2.dumps(
            [
                record.code,
                [1] if record.reason is None else [2, record.reason],
                [1] if record.subject is None else [2, list(record.subject)],
                [1] if record.position is None else [2, record.position],
                [1] if record.offset is None else [2, record.offset],
                [1] if record.length is None else [2, record.length],
                [1] if record.count is None else [2, record.count],
            ]
        )
        position = record.position
        return record.step, 0 if position is None else 1, position or 0, encoded

    return tuple(sorted(records, key=key))


# The subject kind of a frame type (P2, `subject_kind` tag 2).
_SUBJECT_FRAME_TYPE = 2
# The subject kind of a profile (P2, `subject_kind` tag 4).
_SUBJECT_PROFILE = 4
# The subject kind of an item kind (P2, `subject_kind` tag 5).
_SUBJECT_ITEM_KIND = 5
# The subject kind of a codec (P2, `subject_kind` tag 1).
_SUBJECT_CODEC = 1


def _subject(
    read: ContentRead | None, segment: int | None, finding: ContentFinding
) -> tuple[int, ...] | None:
    """Return the typed subject of a finding whose subject this reader states.

    Frame-type subjects come from the frame at the finding's position, and the
    profile subject from the header of its segment. Other subjects (codec, item
    kind, the profile's check) are not yet stated, so their findings carry none.
    """
    if read is None or segment is None:
        return None
    if finding.code == "unsupported_critical" or (
        finding.code == "unknown_enumeration_value"
        and finding.reason == "unknown_frame_type"
    ):
        frame = next(
            (
                frame
                for frame in read.wire.segments[segment].frames
                if frame.position == finding.position
            ),
            None,
        )
        if frame is not None and frame.frame_type is not None:
            return (_SUBJECT_FRAME_TYPE, frame.frame_type)
    if finding.code == "profile_not_checked":
        opener = read.wire.segments[segment].opener.value
        header = getattr(opener, "header", None)
        if header is not None and header.profile is not None:
            return (_SUBJECT_PROFILE, header.profile)
    return None


def finding_record(
    finding: ContentFinding,
    read: ContentRead | None = None,
) -> FindingRecord:
    """Return the typed record of a finding of the content read.

    The subject is stated when `read` is given and the finding's subject is one
    this reader states. Raise KeyError when the finding's code or reason is not one
    of registry version 1.
    """
    offset, length = finding.offset, finding.length
    if (
        finding.code == "malformed_value"
        and read is not None
        and finding.segment is not None
        and finding.position is not None
        and offset is None
    ):
        # A malformed item is reported with the bytes of the item at its position.
        item = read.wire.segments[finding.segment].items[finding.position]
        offset, length = item.offset, item.end - item.offset
    return FindingRecord(
        FINDING_CODES[finding.code],
        None if finding.reason is None else REASON_CODES[finding.reason],
        finding.position,
        offset,
        length,
        finding.count,
        subject=_subject(read, finding.segment, finding),
        step=_step(finding.code, finding.reason),
    )


_PAYLOAD_DAMAGE = frozenset(("decode_failed", "decoded_length", "payload_digest"))


# The payload checks that a size limit refuses; each is an `exceeds_capacity` reason.
_PAYLOAD_SIZE = ("stored_size", "decoded_size", "total_decoded")


def frame_findings(
    read: ContentRead, segment: int, frame: WireFrame
) -> tuple[FindingRecord, ...]:
    """Return the findings that the wire checks give one frame, then those of the read.

    The wire checks are the link, the payload, the stored check and the signatures
    of the frame. A parity frame has its own findings and none of these.
    """
    item = read.wire.segments[segment].items[frame.position]
    offset, length = item.offset, item.end - item.offset
    found: list[FindingRecord] = []
    if frame.frame_type != _PARITY:
        reason = frame.payload.reason
        malformed: str | None = None
        if isinstance(frame.envelope, FrameError):
            malformed = frame.envelope.reason
        elif frame.payload.state == "damaged" and reason == "codec_constraint":
            malformed = reason
        # The envelope and the codec constraint are the item's `malformed_value`;
        # a content error at the position is reported by the read itself.
        if malformed is not None and not any(
            finding.code == "malformed_value"
            and finding.segment == segment
            and finding.position == frame.position
            for finding in read.findings
        ):
            found.append(
                _record(
                    "malformed_value",
                    malformed,
                    frame.position,
                    offset,
                    length,
                    7 if malformed == "codec_constraint" else 4,
                )
            )
        if malformed is None and frame.link.state == "broken":
            found.append(_record("chain_break", None, frame.position, offset, length))
        elif frame.link.state == "healed":
            found.append(_record("healed_link", None, frame.position, offset, length))
        if frame.payload.state == "damaged" and reason in _PAYLOAD_DAMAGE:
            found.append(
                _record("damaged_frame", reason, frame.position, offset, length)
            )
        if frame.payload.state == "unchecked" and reason in _PAYLOAD_SIZE:
            found.append(
                FindingRecord(
                    FINDING_CODES["exceeds_capacity"],
                    REASON_CODES[str(reason)],
                    frame.position,
                    offset,
                    length,
                    frame.payload.count,
                    step=_step("exceeds_capacity", reason),
                )
            )
        if frame.payload.state == "unchecked" and reason == "unknown_codec":
            # The codec is the subject, and the payload check (R7) judges it.
            codec = frame.payload.codec
            if codec is None:
                raise ValueError("an unknown codec carries its codec")
            found.append(
                FindingRecord(
                    FINDING_CODES["unknown_enumeration_value"],
                    REASON_CODES["unknown_codec"],
                    frame.position,
                    None,
                    None,
                    None,
                    subject=(_SUBJECT_CODEC, codec),
                    step=7,
                )
            )
        if frame.payload.stored_matches is False:
            found.append(
                _record("stored_check_mismatch", None, frame.position, offset, length)
            )
    signatures = dict(read.authentication[segment].signatures).get(frame.position, ())
    for signature in signatures:
        status = signature.outcome.status
        if status != "valid":
            found.append(
                FindingRecord(
                    FINDING_CODES[f"signature_{status}"],
                    REASON_CODES[str(signature.outcome.reason)],
                    frame.position,
                    None,
                    None,
                    None,
                    (3, signature.algorithm),
                    step=20,
                )
            )
    found.extend(
        finding_record(finding, read)
        for finding in read.findings
        if finding.segment == segment and finding.position == frame.position
    )
    return sort_findings(found)


def _record(
    code: str,
    reason: str | None,
    position: int,
    offset: int,
    length: int,
    step: int | None = None,
) -> FindingRecord:
    return FindingRecord(
        FINDING_CODES[code],
        None if reason is None else REASON_CODES[reason],
        position,
        offset,
        length,
        None,
        step=_step(code, reason) if step is None else step,
    )


def framing_findings(read: ContentRead) -> tuple[FindingRecord, ...]:
    """Return the framing findings of the file that no segment holds (P7 1.4, item 3).

    These are the `torn_tail` and `undelimitable_input` of the bytes before the first
    opener and of the bytes that end a segment where framing is lost.
    """
    boundaries = []
    prefix = read.wire.opening.damaged_prefix
    if prefix is not None:
        boundaries.append(prefix)
    boundaries.extend(
        segment.framing_failure
        for segment in read.wire.segments
        if segment.framing_failure is not None
    )
    spans = {
        (FINDING_CODES[boundary.failure], boundary.offset, boundary.end)
        for boundary in boundaries
        if boundary.failure in ("torn_tail", "undelimitable_input")
    }
    # A region that a parity resumption passed over is undelimitable too.
    spans.update(
        (FINDING_CODES["undelimitable_input"], item.offset, item.end)
        for segment in read.wire.segments
        for item in segment.items
        if isinstance(item, DamagedRegion)
    )
    return sort_findings(
        FindingRecord(code, None, None, offset, end - offset, None, step=2)
        for code, offset, end in spans
    )


_LINEAGE_REASONS = frozenset(("head", "ordinal"))


def segment_findings(read: ContentRead, segment: int) -> tuple[FindingRecord, ...]:
    """Return the findings that stand in the list of a segment (P7 1.4, item 2).

    These are the findings of the opener, of the lineage, of an item that is not a
    frame, and the findings of the read that name no frame of the segment.
    """
    wire = read.wire.segments[segment]
    found: list[FindingRecord] = []
    opener = wire.opener.value
    if not isinstance(opener, WireError):
        if opener.header.registry_version > _REGISTRY_VERSION:
            found.append(
                FindingRecord(
                    FINDING_CODES["unsupported_registry_version"],
                    None,
                    None,
                    None,
                    None,
                    None,
                    step=3,
                )
            )
        for label in read.lineages[segment].findings:
            if label in _LINEAGE_REASONS:
                found.append(_segment_record("lineage_mismatch", label))
            else:
                found.append(_segment_record(label, None))
    else:
        item = wire.opener.item
        found.append(
            _record(
                "malformed_value",
                opener.reason,
                0,
                item.offset,
                item.end - item.offset,
                3,
            )
        )
    frames = {frame.position for frame in wire.frames}
    # The items of a segment whose opener is not readable are delimited, not judged.
    judged = not isinstance(opener, WireError) and opener.identifier is not None
    if (
        judged
        and not isinstance(opener, WireError)
        and opener.header.profile is not None
    ):
        # The reader implements no profile, so every declared profile is not checked.
        found.append(
            FindingRecord(
                FINDING_CODES["profile_not_checked"],
                None,
                None,
                None,
                None,
                None,
                subject=(_SUBJECT_PROFILE, opener.header.profile),
                step=_step("profile_not_checked", None),
            )
        )
    for position, item in enumerate(wire.items if judged else ()):
        if not isinstance(item, Item) or item.kind != "opaque":
            continue
        if item.reason == "unknown_item_kind":
            found.append(_item_kind_record(position, item.tag))
        elif item.reason is not None:
            found.append(
                _record(
                    "malformed_value",
                    item.reason,
                    position,
                    item.offset,
                    item.end - item.offset,
                    4,
                )
            )
    for note in wire.closing if judged else ():
        if note.reason in _CLAIM_REASONS:
            found.append(_claim_record(note.reason, note.position))
        else:
            item = wire.items[note.position]
            found.append(
                _record(
                    "malformed_value",
                    note.reason,
                    note.position,
                    item.offset,
                    item.end - item.offset,
                    4,
                )
            )
    for refusal in wire.resync_refusals:
        # The refusal stands at the item that holds its offset, or at the next item
        # when the offset falls between items: the items that end by it come before.
        position = sum(1 for item in wire.items if item.end <= refusal.offset)
        found.append(
            FindingRecord(
                FINDING_CODES["exceeds_capacity"],
                REASON_CODES["total_decoded"],
                position,
                refusal.offset,
                RESYNC_MAX,
                refusal.count,
                step=_step("exceeds_capacity", "total_decoded"),
            )
        )
    found.extend(
        finding_record(finding, read)
        for finding in read.findings
        if finding.segment == segment and finding.position not in frames
    )
    return sort_findings(found)


# The closing reasons that claim the offsets or the locator of an index frame.
_CLAIM_REASONS = ("index_offsets", "index_locator")


def _claim_record(reason: str, position: int) -> FindingRecord:
    """Return the finding of an index claim that fails at an item of the segment."""
    return FindingRecord(
        FINDING_CODES["index_claim_failed"],
        REASON_CODES[reason],
        position,
        None,
        None,
        None,
        step=_step("index_claim_failed", None),
    )


def _item_kind_record(position: int, tag: int | None) -> FindingRecord:
    """Return the finding of an opaque item whose kind this reader does not know.

    Its subject is the item's CBOR tag, the item kind (P2 `subject_kind` tag 5).
    """
    if tag is None:
        raise ValueError("an unknown item kind carries its tag")
    return FindingRecord(
        FINDING_CODES["unknown_enumeration_value"],
        REASON_CODES["unknown_item_kind"],
        position,
        None,
        None,
        None,
        subject=(_SUBJECT_ITEM_KIND, tag),
        step=_step("unknown_enumeration_value", "unknown_item_kind"),
    )


def _segment_record(code: str, reason: str | None) -> FindingRecord:
    return FindingRecord(
        FINDING_CODES[code],
        None if reason is None else REASON_CODES[reason],
        None,
        None,
        None,
        None,
        step=_step(code, reason),
    )

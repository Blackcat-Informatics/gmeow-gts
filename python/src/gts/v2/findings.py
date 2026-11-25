# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed findings of the GTS v2 read result (P2 section 5.1 and 6.1, P7 1.1)."""

from dataclasses import dataclass

from ._decode import WireError
from .content import ContentFinding, ContentRead
from .frame import FrameError
from .items import DamagedRegion, Item
from .reader import WireFrame

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


def finding_record(finding: ContentFinding) -> FindingRecord:
    """Return the typed record of a finding of the content read.

    Raise KeyError when the finding's code or reason is not one of registry version 1.
    """
    return FindingRecord(
        FINDING_CODES[finding.code],
        None if finding.reason is None else REASON_CODES[finding.reason],
        finding.position,
        finding.offset,
        finding.length,
        finding.count,
    )


_PAYLOAD_DAMAGE = frozenset(("decode_failed", "decoded_length", "payload_digest"))


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
                _record("malformed_value", malformed, frame.position, offset, length)
            )
        if malformed is None and frame.link.state == "broken":
            found.append(_record("chain_break", None, frame.position, offset, length))
        elif frame.link.state == "healed":
            found.append(_record("healed_link", None, frame.position, offset, length))
        if frame.payload.state == "damaged" and reason in _PAYLOAD_DAMAGE:
            found.append(
                _record("damaged_frame", reason, frame.position, offset, length)
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
                )
            )
    found.extend(
        finding_record(finding)
        for finding in read.findings
        if finding.segment == segment and finding.position == frame.position
    )
    return tuple(found)


def _record(
    code: str, reason: str | None, position: int, offset: int, length: int
) -> FindingRecord:
    return FindingRecord(
        FINDING_CODES[code],
        None if reason is None else REASON_CODES[reason],
        position,
        offset,
        length,
        None,
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
    return tuple(
        FindingRecord(code, None, None, offset, end - offset, None)
        for code, offset, end in sorted(spans, key=lambda span: (span[1], span[0]))
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
                "malformed_value", opener.reason, 0, item.offset, item.end - item.offset
            )
        )
    frames = {frame.position for frame in wire.frames}
    # The items of a segment whose opener is not readable are delimited, not judged.
    judged = not isinstance(opener, WireError) and opener.identifier is not None
    for position, item in enumerate(wire.items if judged else ()):
        if (
            isinstance(item, Item)
            and item.kind == "opaque"
            and item.reason not in (None, "unknown_item_kind")
        ):
            found.append(
                _record(
                    "malformed_value",
                    item.reason,
                    position,
                    item.offset,
                    item.end - item.offset,
                )
            )
    found.extend(
        _record(
            "malformed_value",
            note.reason,
            note.position,
            wire.items[note.position].offset,
            wire.items[note.position].end - wire.items[note.position].offset,
        )
        for note in (wire.closing if judged else ())
        if note.reason not in ("index_offsets", "index_locator")
    )
    found.extend(
        finding_record(finding)
        for finding in read.findings
        if finding.segment == segment and finding.position not in frames
    )
    return tuple(found)


def _segment_record(code: str, reason: str | None) -> FindingRecord:
    return FindingRecord(
        FINDING_CODES[code],
        None if reason is None else REASON_CODES[reason],
        None,
        None,
        None,
        None,
    )

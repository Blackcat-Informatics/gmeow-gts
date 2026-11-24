# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Drive v2 graph and blob checks through the whole-input reader."""

import json
from collections import Counter
from dataclasses import replace
from pathlib import Path

import cbor2
import pytest

from gts.v2._content import ContentError
from gts.v2.content import ContentCapacity, read_content
from gts.v2.dictionary import Blank
from gts.v2.frame import _identifier, _payload_digest, decode_frame, encode_plain_frame
from gts.v2.header import Header, Lineage, decode_header
from gts.v2.index import check_index_payload
from gts.v2.range import RangeAccumulator
from gts.v2.rebuild import unit_digest
from gts.v2.rows import QuadRow, encode_rows
from gts.v2.signatures import HeldKey, SignatureVerifier
from gts.v2.terms import IriTerm, TypedValueTerm, ValueReference, encode_terms
from gts.v2.writer import SegmentWriter

_VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2"
_CODES = {
    5: "unknown_enumeration_value",
    6: "unsupported_critical",
    7: "malformed_value",
    13: "damaged_frame",
    15: "commit_count_mismatch",
    16: "commit_root_mismatch",
    17: "uncommitted_tail",
    19: "missing_dependency",
    20: "exceeds_capacity",
    21: "declared_order_violated",
    22: "forward_reference",
    23: "position_constraint",
    24: "ill_formed_term",
    25: "opaque_term",
    28: "suppression_unmatched",
    29: "binding_unanchored",
    44: "unreferenced_unit",
    45: "conflicting_declaration",
}
_REASONS = {
    2: "unknown_frame_type",
    8: "unordered_or_repeated",
    12: "payload_digest",
    14: "blob_length",
    15: "blob_digest",
    24: "chunk",
    25: "term",
    26: "value",
    27: "blob",
    28: "frame",
    29: "state",
    32: "refusal_width",
    33: "refusal_limit",
    34: "quad",
    35: "reifier",
    36: "graph",
    37: "reifier_binding",
    38: "annotation",
    39: "binding",
    85: "refusal_size",
    88: "state_unavailable",
    89: "total_rebuilt",
    90: "total_formed",
    91: "refusal_weight",
    92: "refusal_term_node",
}


def _read_results(part: str, name: str):
    folder = _VECTORS / part
    for case in json.loads((folder / f"{name}.json").read_text())["cases"]:
        raw = (folder / case["input_file"]).read_bytes()
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        (
            stored,
            decoded,
            units,
            lines,
            total_decoded,
            items,
            rebuilt,
            formed,
            weight,
            _,
        ) = report[3]
        capacity = ContentCapacity(
            stored, decoded, total_decoded, items, rebuilt, formed, lines, weight, units
        )
        yield raw, report[-1], capacity


def _contents(part: str, name: str):
    for raw, file_result, capacity in _read_results(part, name):
        yield raw, file_result[1], capacity


def _digest(option: list) -> bytes | None:
    return option[1] if option[0] == 2 else None


def _labelling_refusals(file_result: list):
    for segment in file_result[1]:
        for finding in segment[-1]:
            if finding[0] == 26:
                yield segment[0], None, _REASONS[finding[1][1]]
    for finding in file_result[2]:
        if finding[0] == 26:
            yield None, None, _REASONS[finding[1][1]]


@pytest.mark.parametrize(
    ("name", "check", "position"),
    [
        ("canonical__labels-automorphic", None, None),
        ("canonical__check__first-difference__terms", "terms", 2),
        ("canonical__check__first-difference__labelling", "labelling", 2),
        ("canonical__check__first-difference__content_digest", "content_digest", 8),
        ("canonical__check__index-above-capacity", None, None),
        ("canonical__check__not-claimed", None, None),
    ],
)
def test_reader_reports_both_canonical_levels(
    name: str, check: str | None, position: int | None
) -> None:
    states = {1: "verified", 2: "unchecked", 3: "failed"}
    for raw, segments, capacity in _contents("P7", name):
        result = read_content(raw, capacity)
        (segment,) = segments
        (canonical,) = result.canonical
        claim, structure, labelling, _, _ = segment[14]
        assert canonical.claimed == claim
        assert canonical.structure.state == states[structure[0]]
        assert canonical.labelling.state == states[labelling[0]]
        violations = [
            finding
            for finding in result.findings
            if finding.code == "canonical_violated"
        ]
        if check is None:
            assert canonical.first_difference is None
            assert not violations
        else:
            difference = canonical.first_difference
            assert difference is not None
            assert (difference.check, difference.position) == (check, position)
            assert [(finding.reason, finding.position) for finding in violations] == [
                (check, position)
            ]


def test_bad_index_prefix_claim_precedes_a_later_opaque_index() -> None:
    writer = SegmentWriter(Header(canonical=1))
    prefix = RangeAccumulator(writer.head)
    prefix.append(writer.commit())
    wrong_count = cbor2.dumps(
        [[[14, 1]], [], [0, 0, 0, []], prefix.root().digest, [1], []]
    )
    writer.append(15, [2, writer.head], wrong_count)
    writer.append(15, [2, writer.head], b"\x80")
    result = read_content(writer.to_bytes(parity=True, offsets=True, locator=True))
    assert any(
        finding.code == "index_claim_failed"
        and finding.position == 2
        and finding.reason == "index_head"
        for finding in result.findings
    )
    difference = result.canonical[0].first_difference
    assert difference is not None
    assert (difference.check, difference.position) == ("index", 2)


_INDEX_REASONS = {
    17: "index_head",
    18: "index_root",
    20: "index_digest",
    21: "index_members",
    22: "index_directory",
}


@pytest.mark.parametrize(
    "name",
    [
        "index-each-claim",
        "index-nonterminal-claims",
        "index-opaque-covered",
        "index-labelling-refused",
        "index-after-stop",
        "index-last-unfolded",
        "index-absent-equal",
        "index-covers-frame-of-no-type",
    ],
)
def test_read_index_claims_match_p5_findings(name: str) -> None:
    for raw, segments, capacity in _contents("P5", name):
        expected = Counter(
            (segment[0], finding[3][1], _INDEX_REASONS[finding[1][1]])
            for segment in segments
            for frame in segment[-2]
            for finding in frame[-1]
            if finding[0] == 18 and finding[1][1] in _INDEX_REASONS
        )
        result = read_content(raw, capacity)
        assert (
            Counter(
                (finding.segment, finding.position, finding.reason)
                for finding in result.findings
                if finding.code == "index_claim_failed"
            )
            == expected
        )


def test_index_statistics_count_predicates_of_held_quads() -> None:
    # P5 counts rows in their decoded frames even when the fold holds them.
    for raw, _, capacity in _contents("P5", "index-digest-absent"):
        result = read_content(raw, capacity)
        assert result.fold is not None
        assert not result.fold.lines.lines
        assert result.fold.content_digest is None
        assert any(
            (finding.code, finding.reason, finding.position)
            == ("index_claim_failed", "index_directory", 3)
            for finding in result.findings
        )


def test_closing_index_checks_digest_claims_after_a_transparent_stop() -> None:
    opener, head, transparent = _transparent_stop()
    root = RangeAccumulator(head).root().digest
    payload = cbor2.dumps([[], [], [1, 0, 0, []], root, [2, bytes(32)], []])
    index = encode_plain_frame(head, 15, [0, head], payload)
    result = read_content(opener + transparent + index)
    assert any(finding.code == "unsupported_critical" for finding in result.findings)
    assert {
        (finding.position, finding.reason)
        for finding in result.findings
        if finding.code == "index_claim_failed"
    } == {(2, "index_directory"), (2, "index_digest")}


_PARITY_CODES = {
    30: "parity_damaged",
    31: "parity_mismatch",
    32: "parity_unexplained",
    33: "parity_stale",
    36: "unprotected_tail",
}


@pytest.mark.parametrize(
    "part, name",
    [
        ("P1", "parity-later-version-payload-damaged"),
        ("P5", "parity-above-capacity-and-damaged"),
        ("P5", "parity-above-capacity-wrong-table"),
        ("P5", "parity-condition-six-first-by-case"),
        ("P5", "parity-damaged-shard"),
        ("P5", "parity-unexplained"),
        ("P5", "recovery-parity-in-resync-segment"),
        ("P5", "commit-count-mismatch"),
    ],
)
def test_reader_matches_parity_diagnostics(part: str, name: str) -> None:
    def value(option: list) -> int | None:
        return option[1] if option[0] == 2 else None

    reasons = {
        None: None,
        12: "payload_digest",
        30: "decoded_size",
        40: "data",
        41: "parity",
    }
    codes = {13: "damaged_frame", 20: "exceeds_capacity", **_PARITY_CODES}
    for raw, segments, capacity in _contents(part, name):
        expected = Counter()
        for segment in segments:
            report = [
                *segment[-1],
                *(f for frame in segment[-2] if frame[3] == [2, 16] for f in frame[-1]),
            ]
            for code, reason, _, position, offset, length, count in report:
                if code in codes:
                    expected[
                        (
                            segment[0],
                            codes[code],
                            value(position),
                            reasons[value(reason)],
                            value(count),
                            value(offset),
                            value(length),
                        )
                    ] += 1
        result = read_content(raw, capacity)
        parity_positions = {
            (ordinal, frame.position)
            for ordinal, segment in enumerate(result.wire.segments)
            for frame in segment.frames
            if frame.frame_type == 16
        }
        assert (
            Counter(
                (f.segment, f.code, f.position, f.reason, f.count, f.offset, f.length)
                for f in result.findings
                if f.code in _PARITY_CODES.values()
                or f.code in ("damaged_frame", "exceeds_capacity")
                and (f.segment, f.position) in parity_positions
            )
            == expected
        )


@pytest.mark.parametrize(
    "part, name",
    [
        ("P3", "labelling-held-fold"),
        ("P3", "labelling-limit-not-guaranteed"),
        ("P3", "labelling-weight-after-size"),
        ("P3", "labelling-width-and-limit"),
        ("P3", "lowering-deep"),
        ("P3", "bindings-leaf-deep"),
        ("P3", "bindings-unnamed-pair"),
        ("P4", "rebuild-value-first-verified"),
    ],
)
def test_whole_and_own_labelling_match_the_read_result(part: str, name: str) -> None:
    for raw, expected, capacity in _read_results(part, name):
        result = read_content(raw, capacity)
        assert result.fold is not None
        assert result.fold.content_digest == _digest(expected[3])
        actual_own = tuple(
            None if report is None else report.content_digest
            for report in result.own_labelling
        )
        assert actual_own == tuple(_digest(segment[15]) for segment in expected[1])
        actual_refusals = Counter(
            (finding.segment, finding.position, finding.reason)
            for finding in result.findings
            if finding.code == "labelling_refused"
        )
        assert actual_refusals == Counter(_labelling_refusals(expected))


@pytest.mark.parametrize(
    "weight, refused_segments", [(24896, ()), (24895, (None,)), (12447, (0, None))]
)
def test_own_fold_consumes_its_weight_before_the_file_fold(
    weight: int, refused_segments: tuple[int | None, ...]
) -> None:
    ((raw, expected, capacity),) = _read_results("P3", "labelling-weight-example")
    result = read_content(raw, replace(capacity, labelled_bytes_per_read=weight))
    assert result.fold is not None
    assert len(result.fold.lines.lines) == 2
    (own,) = result.own_labelling
    assert (None if own is None else own.content_digest) == (
        None if 0 in refused_segments else _digest(expected[1][0][15])
    )
    assert result.fold.content_digest == (
        None if None in refused_segments else _digest(expected[3])
    )
    assert tuple(
        (finding.segment, finding.position, finding.reason)
        for finding in result.findings
        if finding.code == "labelling_refused"
    ) == tuple((segment, None, "refusal_weight") for segment in refused_segments)


@pytest.mark.parametrize(
    "part, name",
    [
        ("P1", "frame-unknown-critical"),
        ("P1", "frame-unknown-noncritical"),
        ("P1", "parity-geometry-newer"),
        ("P3", "annotations-repeat-after-resolution"),
        ("P3", "rows-opaque-term-held"),
        ("P3", "terms-id-base-after-opaque"),
        ("P3", "terms-literal-ref-first-reaching-frame"),
        ("P3", "rows-order-canonical-across-frames"),
        ("P3", "bindings-referent-blob-resolved"),
        ("P3", "bindings-referent-other-segment"),
        ("P3", "bindings-referent-state-failing-commit"),
        ("P3", "bindings-forward-reference"),
        ("P3", "bindings-unnamed-pair"),
        ("P3", "bindings-leaf-deep"),
        ("P3", "suppress-order-free"),
        ("P3", "suppress-not-folded"),
        ("P3", "suppress-unmatched-once"),
        ("P3", "suppress-undefined-and-unreadable"),
        ("P3", "suppress-annotation-any-count"),
        ("P4", "rebuild-unit-after-recipe"),
        ("P4", "rebuild-charge-order"),
        ("P4", "blobset-order-free"),
        ("P5", "commit-count-mismatch"),
        ("P5", "commit-root-mismatch"),
        ("P5", "commit-none"),
    ],
)
def test_whole_content_findings(part: str, name: str) -> None:
    for raw, file_result, capacity in _read_results(part, name):
        result = read_content(raw, capacity)
        assert result.fold is not None
        expected = Counter(
            (segment, position, "labelling_refused", reason, None)
            for segment, position, reason in _labelling_refusals(file_result)
        )
        for segment in file_result[1]:
            ordinal = segment[0]
            findings = [*segment[-1], *(f for frame in segment[-2] for f in frame[-1])]
            for code, reason, _, position, _, _, count in findings:
                if code not in _CODES or position[0] != 2:
                    continue
                reason_code = reason[1] if reason[0] == 2 else None
                if code in (13, 20) and reason_code not in (14, 15, 89, 90):
                    continue
                expected[
                    (
                        ordinal,
                        position[1],
                        _CODES[code],
                        _REASONS[reason_code] if reason_code else None,
                        count[1] if count[0] == 2 else None,
                    )
                ] += 1
        actual = Counter(
            (f.segment, f.position, f.code, f.reason, f.count)
            for f in result.findings
            if f.code not in _PARITY_CODES.values()
        )
        assert actual == expected


@pytest.mark.parametrize(
    "name, resolves",
    [
        ("rebuild-value-first-verified", True),
        ("rebuild-value-none-verified", False),
    ],
)
def test_literal_value_selection_across_segments(name: str, resolves: bool) -> None:
    for raw, _, capacity in _contents("P4", name):
        result = read_content(raw, capacity)
        assert result.fold is not None
        assert len(result.blob_segments) == 2
        assert bool(result.fold.lines.lines) == resolves
        assert result.fold.lines.held_rows is not resolves
        assert all(f.reason != "value" for f in result.findings)


def test_item_limit_preserves_wire_prefix_without_returning_a_file_fold() -> None:
    raw, _, _ = next(_contents("P3", "rows-order-canonical-across-frames"))
    result = read_content(raw, ContentCapacity(items_per_read=2))
    assert result.wire.item_limit_offset is not None
    assert result.fold is None
    assert result.own_labelling == (None,)
    assert all(finding.code != "labelling_refused" for finding in result.findings)


@pytest.mark.parametrize(
    "name", ["header-outcome-unreadable-first", "header-hash-newer"]
)
def test_unreadable_segments_produce_an_empty_file_fold(name: str) -> None:
    raw, _, capacity = next(_contents("P1", name))
    result = read_content(raw, capacity)
    assert result.blob_segments == (None,)
    assert result.fold is not None
    assert not result.fold.lines.lines and not result.fold.blobs.entries


def test_large_content_element_cannot_reserve_dictionary_ids() -> None:
    terms_record, terms_payload = encode_terms([IriTerm(b"urn:a")])
    row_type, row_record, row_payload = encode_rows([QuadRow(0, 0, 0)])
    stored_limit = max(len(terms_payload), len(row_payload))
    excess = [0, [[1, 1], [3, 1]] * 8, 2, 64]
    assert len(cbor2.dumps(excess)) > stored_limit
    writer = SegmentWriter()
    writer.append(1, excess, b"")
    writer.append(1, terms_record, terms_payload)
    writer.append(row_type, row_record, row_payload)
    result = read_content(
        writer.to_bytes(), ContentCapacity(stored_bytes_per_frame=stored_limit)
    )
    assert result.fold is not None
    assert len(result.fold.lines.lines) == 1


def test_state_reference_resolves_to_its_verified_commit() -> None:
    raw, _, capacity = next(_contents("P3", "bindings-referent-state-resolved"))
    result = read_content(raw, capacity)
    (state,) = result.states
    assert state.first_segment == state.commit.segment == 0
    assert not state.fold.lines.lines
    assert not state.fold.records.bindings
    assert not any(f.reason in ("state", "state_unavailable") for f in result.findings)


def _with_state_reference(raw: bytes, identifier: bytes) -> bytes:
    writer = SegmentWriter()
    terms_record, terms_payload = encode_terms([IriTerm(b"urn:a")])
    writer.append(1, terms_record, terms_payload)
    writer.append(6, [1], cbor2.dumps([[[1, 0], [3, identifier]]]))
    return raw + writer.to_bytes()


@pytest.mark.parametrize(
    "name, segment, start, line_count, blank_segment",
    [
        ("state-chained-lineage", 1, 0, 1, None),
        ("state-no-lineage-local", 1, 1, 0, None),
        ("state-same-name-first-naming", 1, 1, 0, None),
        ("state-same-name-first-holder-three", 1, 1, 1, 1),
        ("state-ordinals-kept", 2, 1, 1, 1),
    ],
)
def test_graph_state_uses_first_available_commit_and_preserves_ordinals(
    name: str, segment: int, start: int, line_count: int, blank_segment: int | None
) -> None:
    raw, _, capacity = next(_contents("P3", name))
    before = read_content(raw, capacity)
    assert not before.states
    commit = before.wire.segments[-1].frames[-1].link.effective_identifier
    assert commit is not None
    (state,) = read_content(_with_state_reference(raw, commit), capacity).states
    assert state.identifier == commit
    assert (state.commit.segment, state.first_segment) == (segment, start)
    assert len(state.fold.lines.lines) == line_count
    if blank_segment is not None:
        (row,) = state.fold.lines.lines
        assert isinstance(row, QuadRow)
        blank = state.fold.terms.term(row.subject)
        assert isinstance(blank, Blank) and blank.segment == blank_segment


@pytest.mark.parametrize(
    "name, larger_allowance",
    [("state-rebuild-refused-capacity", 64), ("state-rebuild-refused-mixed", 128)],
)
def test_graph_state_uses_original_rebuild_allowance(
    name: str, larger_allowance: int
) -> None:
    raw, _, capacity = next(_contents("P3", name))
    commit = (
        read_content(raw, capacity)
        .wire.segments[-1]
        .frames[-1]
        .link.effective_identifier
    )
    assert commit is not None
    raw = _with_state_reference(raw, commit)

    smaller = read_content(raw, capacity)
    assert not smaller.states
    assert any(f.reason == "state_unavailable" for f in smaller.findings)
    larger = read_content(
        raw, replace(capacity, rebuilt_bytes_per_read=larger_allowance)
    )
    (state,) = larger.states
    assert state.identifier == commit and len(state.fold.lines.lines) == 1


@pytest.mark.parametrize("later_candidate_has_unit", [False, True])
def test_graph_state_distinguishes_missing_units_from_a_refused_rebuild(
    later_candidate_has_unit: bool,
) -> None:
    data = b"data"
    name = unit_digest(data)
    writer = SegmentWriter()
    recipe = cbor2.dumps([[name, len(data)]])
    writer.append(11, [name, len(data), 1], recipe)
    if later_candidate_has_unit:
        writer.append(8, [name], data)
        writer.append(11, [name, len(data), 1], recipe)
    terms_record, terms_payload = encode_terms(
        [IriTerm(b"urn:s"), TypedValueTerm(ValueReference(name, len(data)))]
    )
    writer.append(1, terms_record, terms_payload)
    frame_type, row_content, row_payload = encode_rows([QuadRow(0, 0, 1)])
    writer.append(frame_type, row_content, row_payload)
    commit = writer.commit()

    result = read_content(
        _with_state_reference(writer.to_bytes(), commit),
        ContentCapacity(rebuilt_bytes_per_read=0),
    )
    if later_candidate_has_unit:
        assert not result.states
        assert any(f.reason == "state_unavailable" for f in result.findings)
    else:
        (state,) = result.states
        assert state.identifier == commit and state.fold.lines.held_rows


def test_refused_value_is_projected_at_each_commit_cutoff() -> None:
    data = b"data"
    name = unit_digest(data)
    writer = SegmentWriter()
    writer.append(11, [name, len(data), 1], cbor2.dumps([[name, len(data)]]))
    terms_record, terms_payload = encode_terms(
        [IriTerm(b"urn:s"), TypedValueTerm(ValueReference(name, len(data)))]
    )
    writer.append(1, terms_record, terms_payload)
    frame_type, row_content, row_payload = encode_rows([QuadRow(0, 0, 1)])
    writer.append(frame_type, row_content, row_payload)
    before_unit = writer.commit()
    writer.append(8, [name], data)
    after_unit = writer.commit()

    raw = _with_state_reference(writer.to_bytes(), before_unit)
    result = read_content(
        _with_state_reference(raw, after_unit),
        ContentCapacity(rebuilt_bytes_per_read=0),
    )
    (state,) = result.states
    assert state.identifier == before_unit and state.fold.lines.held_rows
    assert [f.segment for f in result.findings if f.reason == "state_unavailable"] == [
        2
    ]


def test_commit_in_segment_without_a_run_names_no_state() -> None:
    writer = SegmentWriter(Header(lineage=Lineage(bytes(32), 1)))
    commit = writer.commit()
    terms_record, terms_payload = encode_terms([IriTerm(b"urn:a")])
    writer.append(1, terms_record, terms_payload)
    writer.append(6, [1], cbor2.dumps([[[1, 0], [3, commit]]]))
    result = read_content(writer.to_bytes())
    assert result.fold is not None
    assert len(result.fold.records.bindings) == 1
    assert any(
        f.code == "missing_dependency" and f.reason == "state_unavailable"
        for f in result.findings
    )


@pytest.mark.parametrize(
    "name",
    [
        "state-payload-damaged",
        "state-signature-duplicate",
        "state-opaque-noncontent-before-commit",
        "state-parity-ordinary-broken-link",
        "state-earlier-segment-frame-broken",
    ],
)
def test_state_referent_is_unavailable_after_an_ineligible_frame(name: str) -> None:
    raw, _, capacity = next(_contents("P3", name))
    original = read_content(raw, capacity)
    commit = original.wire.segments[-1].frames[-1]
    assert commit.frame_type == 14 and commit.link.effective_identifier is not None

    result = read_content(
        _with_state_reference(raw, commit.link.effective_identifier), capacity
    )
    assert result.fold is not None
    assert any(
        f.code == "missing_dependency" and f.reason == "state_unavailable"
        for f in result.findings
    )


def test_malformed_index_in_a_commit_prefix_prevents_a_state() -> None:
    writer = SegmentWriter()
    writer.append(15, [0, writer.head], cbor2.dumps([]))
    commit = writer.commit()
    terms_record, terms_payload = encode_terms([IriTerm(b"urn:a")])
    writer.append(1, terms_record, terms_payload)
    writer.append(6, [1], cbor2.dumps([[[1, 0], [3, commit]]]))

    result = read_content(writer.to_bytes())
    assert result.fold is not None
    assert {(item.position, item.code, item.reason) for item in result.findings} >= {
        (1, "malformed_value", "wrong_length"),
        (4, "missing_dependency", "state_unavailable"),
    }


def _transparent_stop() -> tuple[bytes, bytes, bytes]:
    first_header = cbor2.dumps([1, 2, 2, 1, [1], [1], [1]])
    first_head = decode_header(first_header).identifier
    assert first_head is not None
    identity = [17, True, True, [0], 0, _payload_digest(b"")]
    fields = b"".join(map(cbor2.dumps, identity))
    transparent = cbor2.dumps(
        [2, *identity, [1], b"", _identifier(first_head, fields), []]
    )
    return first_header, first_head, transparent


@pytest.mark.parametrize("ordinary_after_stop", [False, True])
def test_transparent_stop_blocks_state_via_a_stopped_ordinary_frame(
    ordinary_after_stop: bool,
) -> None:
    first_header, first_head, transparent = _transparent_stop()
    stopped_frame = b""
    if ordinary_after_stop:
        root = RangeAccumulator(first_head).root().digest
        stopped_frame = encode_plain_frame(first_head, 14, [0, root], b"")
        first_head = decode_frame(stopped_frame).identifier
    later = SegmentWriter(Header(lineage=Lineage(first_head, 1)))
    commit = later.commit()
    terms_record, terms_payload = encode_terms([IriTerm(b"urn:a")])
    later.append(1, terms_record, terms_payload)
    later.append(6, [1], cbor2.dumps([[[1, 0], [3, commit]]]))

    raw = first_header + transparent + stopped_frame + later.to_bytes()
    result = read_content(raw)
    if ordinary_after_stop:
        assert not result.states
        assert any(f.reason == "state_unavailable" for f in result.findings)
    else:
        (state,) = result.states
        assert state.identifier == commit and state.first_segment == 0


@pytest.mark.parametrize("directory", [[], [[[99, 0], []]]])
def test_stopped_index_checks_grammar_after_a_future_code(directory: list) -> None:
    payload = cbor2.dumps([[[99, 1]], directory, [], bytes(32), [1], []])
    with pytest.raises(ContentError) as error:
        check_index_payload(payload, registry_version=2, after_stop=True)
    assert error.value.reason == "wrong_length"


@pytest.mark.parametrize(
    "name",
    [
        "lineage-matches",
        "lineage-head-mismatch",
        "lineage-ordinal-mismatch",
        "lineage-ordinal-after-absent",
        "lineage-first-present",
        "lineage-begins-links-unavailable",
        "lineage-previous-unreadable",
        "resync-isolation-checked-lineage-unavailable",
    ],
)
def test_lineage_findings(name: str) -> None:
    reasons = {55: "head", 56: "ordinal"}
    for raw, segments, capacity in _contents("P1", name):
        result = read_content(raw, capacity)
        expected = [
            tuple(
                "lineage_begins" if code == 42 else reasons[reason[1]]
                for code, reason, *_ in segment[-1]
                if code in (42, 43)
            )
            for segment in segments
        ]
        assert [comparison.findings for comparison in result.lineages] == expected


def test_resync_opener_starts_a_new_graph_state_run() -> None:
    raw, _, capacity = next(
        _contents("P1", "resync-isolation-checked-lineage-unavailable")
    )
    result = read_content(raw, capacity)
    assert [lineage.run_start for lineage in result.lineages] == [0, None, 2]


def test_suppress_graph_keeps_line_mentioning_graph_name() -> None:
    raw, _, capacity = next(_contents("P3", "suppress-graph-not-mention"))
    result = read_content(raw, capacity)
    assert result.fold is not None
    raw_lines = result.fold.lines.lines
    assert len(raw_lines) == 1
    assert not result.fold.records.hidden_lines


def test_malformed_suppression_is_reported_before_semantic_resolution() -> None:
    writer = SegmentWriter()
    writer.append(7, [1], b"\x80")
    result = read_content(writer.to_bytes())
    assert result.fold is not None
    assert any(
        f.code == "malformed_value" and f.reason == "inconsistent_counts"
        for f in result.findings
    )


@pytest.mark.parametrize(
    "part, name",
    [
        ("P3", "suppress-term"),
        ("P3", "suppress-unauthenticated"),
        ("P3", "suppress-not-folded"),
        ("P3", "suppress-by-untrusted-party"),
        ("P3", "suppress-blob-hides-declaration-keeps-set"),
        ("P6", "cov-healed"),
        ("P6", "cov-broken-signed-frame"),
        ("P6", "proc-opaque-frame"),
        ("P6", "sig-detached-frame"),
        ("P6", "sig-commit-after-detached-frame"),
        ("P6", "parity-transparent-not-bool-signed"),
        ("P6", "auth-commit-malformed"),
        ("P6", "auth-payload-damaged"),
    ],
)
def test_signature_coverage_authentication_and_visibility(part: str, name: str) -> None:
    folder = _VECTORS / part
    for case in json.loads((folder / f"{name}.json").read_text())["cases"]:
        raw = (folder / case["input_file"]).read_bytes()
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        stored, decoded, _, _, total_decoded, items, rebuilt, formed, _, _ = report[3]
        capacity = ContentCapacity(
            stored, decoded, total_decoded, items, rebuilt, formed
        )
        trusted, held = report[5]
        verifier = SignatureVerifier(
            trusted_keys=trusted, held_keys=(HeldKey(*key) for key in held)
        )
        result = read_content(raw, capacity, verifier=verifier)
        assert result.visibility is not None
        kind, *counts = report[-1][4]
        assert result.visibility.complete == (kind == 1)
        assert result.visibility.unauthenticated_suppress == counts[-1]
        if kind == 2:
            assert (
                result.visibility.unfolded_suppress,
                result.visibility.unopened_sealed,
            ) == tuple(counts[:2])
        for actual, segment in zip(result.authentication, report[-1][1], strict=True):
            assert actual.authenticated_frames == segment[12]
            assert (
                actual.authenticity
                == {
                    1: "authenticated",
                    2: "partly_authenticated",
                    3: "unauthenticated",
                }[segment[9]]
            )
            expected = {
                frame[0]: [tuple(s[4:]) for s in frame[7]]
                for frame in segment[-2]
                if frame[7]
            }
            assert {
                position: [
                    (s.coverage_first, s.coverage_end, s.authenticated)
                    for s in signatures
                ]
                for position, signatures in actual.signatures
            } == expected


def test_hidden_blob_declarations_remain_in_the_raw_blob_set() -> None:
    raw, _, capacity = next(
        _contents("P3", "suppress-blob-hides-declaration-keeps-set")
    )
    result = read_content(raw, capacity)
    assert result.fold is not None
    assert result.fold.blobs.entries and result.fold.blob_declarations
    assert not result.fold.visible_dataset.blob_declarations

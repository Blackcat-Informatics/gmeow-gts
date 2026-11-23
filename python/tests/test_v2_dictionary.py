# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check term resolution and line contributions against P3."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2.dictionary import LanguageLiteral, TermDictionary, TermPool
from gts.v2.lines import LineFold
from gts.v2.rows import AnnotationRow, DecodedRows, EmptyGraphRow, QuadRow, decode_rows
from gts.v2.terms import (
    BlankTerm,
    DecodedTerms,
    FormedBudget,
    IriTerm,
    LiteralTerm,
    TripleTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
    decode_terms,
    reserved_term_runs,
)

P3 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P3"
_FINDINGS = {
    22: "forward_reference",
    23: "position_constraint",
    24: "ill_formed_term",
    25: "opaque_term",
}


@pytest.mark.parametrize(
    "name",
    [
        "terms-text-not-utf8",
        "terms-unreadable-first-condition",
        "terms-datatype-forward",
        "terms-triple-forward",
        "terms-triple-subject-literal",
        "terms-triple-predicate-not-iri",
        "terms-triple-component-unreadable",
        "terms-duplicate-canonical-kinds",
        "terms-canonical-kind-order",
        "terms-canonical-literal-order",
        "terms-canonical-triple-depth",
        "terms-canonical-skipped-term",
        "terms-canonical-across-frames",
        "terms-canonical-follows-opaque",
        "rows-forward-reference",
        "rows-opaque-term-held",
        "rows-position-graph-literal",
        "rows-position-subject-literal",
        "rows-position-predicate-blank",
        "reifier-bindings-triple-not-triple",
    ],
)
def test_term_and_row_findings_match_published_cases(name: str) -> None:
    cases = json.loads((P3 / f"{name}.json").read_text())["cases"]
    for case in cases:
        data = (P3 / case["input_file"]).read_bytes()
        result = cbor2.loads(bytes.fromhex(case["read_result"]))[-1]
        capacity = case["capacity"]
        formed = FormedBudget(capacity["formed_bytes_per_read"])
        budget = PayloadBudget(
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
        )
        pool = TermPool()
        for segment in result[1]:
            dictionary = TermDictionary(pool, segment[0])
            for record in segment[-2]:
                _, offset, _, kind, link, payload_state, *_ = record
                if kind[0] != 2 or kind[1] not in range(1, 6):
                    continue
                frame = decode_frame(data, offset, registry_version=segment[5][1])
                if (
                    link == 3
                    or payload_state[0] != 1
                    or any(f[0] == 7 for f in record[-1])
                ):
                    if frame.frame_type == 1:
                        dictionary.reserve(
                            reserved_term_runs(
                                frame.content, expected_base=dictionary.next_id
                            )
                        )
                    continue
                payload = budget.check(frame).decoded
                assert payload is not None
                allowed = {f[0] for f in record[-1]}
                expected = {_FINDINGS[code] for code in allowed if code in _FINDINGS}
                if frame.frame_type == 1:
                    batch = decode_terms(
                        frame.content,
                        payload,
                        expected_base=dictionary.next_id,
                        budget=formed,
                    )
                    read = dictionary.add(batch)
                    assert read.findings == expected
                    assert (read.order_holds is False) == (21 in allowed)
                else:
                    rows = decode_rows(
                        frame.frame_type,
                        frame.content,
                        payload,
                        written_alike=dictionary.written_alike,
                    )
                    assert dictionary.resolve_rows(rows).findings == expected


def test_scope_and_value_form_are_independent_of_wire_ids() -> None:
    pool = TermPool()
    first = TermDictionary(pool, 0)
    second = TermDictionary(pool, 1)
    batch = DecodedTerms(
        0,
        1,
        False,
        (
            BlankTerm(b"\xff"),
            IriTerm(b"http://www.w3.org/2001/XMLSchema#integer"),
            LiteralTerm(b"4", datatype=1),
            TypedValueTerm(TypedValue(3, 4)),
        ),
    )
    first.add(batch)
    second.add(batch)
    assert first.resolved_id(0) != second.resolved_id(0)
    assert first.resolved_id(2) == first.resolved_id(3) == second.resolved_id(2)
    assert first.written_alike(2) != first.written_alike(3)


def test_value_resolution_keeps_digest_and_length_together() -> None:
    digest = bytes(32)
    available = ValueReference(digest, 2)
    absent = ValueReference(digest, 3)
    failed = ValueReference(digest, 4)
    dictionary = TermDictionary(TermPool(), 0)
    decoded = DecodedTerms(
        0, 1, False, tuple(map(TypedValueTerm, (available, absent, failed, absent)))
    )
    result = dictionary.add(decoded, values={available: b"xy", failed: None})
    assert result.missing_values == (absent,)
    assert result.values == ((dictionary.resolved_id(0), TypedValue(5, b"xy")),)
    assert dictionary.resolved_id(2) is None


def test_repeated_lexical_reference_shares_the_rebuilt_text() -> None:
    pool = TermPool()
    dictionary = TermDictionary(pool, 0)
    octets = b"a" * 65536
    ref = ValueReference(bytes(32), len(octets))
    terms = (LiteralTerm(ref, language="en"), LiteralTerm(ref, language="fr"))
    dictionary.add(DecodedTerms(0, 1, False, terms), values={ref: octets})
    first, second = (dictionary.resolved_id(i) for i in (0, 1))
    assert first is not None and second is not None
    english, french = pool.term(first), pool.term(second)
    assert isinstance(english, LanguageLiteral) and isinstance(french, LanguageLiteral)
    assert english.lexical is french.lexical


def test_rows_hold_nested_references_to_earlier_ids_after_a_broken_link() -> None:
    dictionary = TermDictionary(TermPool(), 0)
    dictionary.add(
        DecodedTerms(0, 1, False, (IriTerm(b"s"), IriTerm(b"p"), IriTerm(b"o")))
    )
    dictionary.add(
        DecodedTerms(3, 1, False, (TripleTerm(0, 1, 2), IriTerm(b"s2"), IriTerm(b"p2")))
    )
    batch = DecodedRows(2, (QuadRow(4, 5, 3), QuadRow(4, 5, 40)), "unordered", None)
    result = dictionary.resolve_rows(batch, hidden_before=3)
    assert result.rows == ()
    assert result.findings == {"opaque_term", "forward_reference"}


def test_opaque_term_runs_reserve_ids_without_materializing_them() -> None:
    dictionary = TermDictionary(TermPool(), 0)
    maximum = (1 << 64) - 1
    dictionary.reserve(((1, maximum),))
    dictionary.add(DecodedTerms(maximum, 1, False, (IriTerm(b"last"),)))
    assert dictionary.next_id == maximum + 1
    batch = DecodedRows(2, (QuadRow(maximum, 0, 0),), None, None)
    assert dictionary.resolve_rows(batch).findings == {"opaque_term"}


def test_canonical_triple_depth_uses_opaque_leaf_kind_from_runs() -> None:
    dictionary = TermDictionary(TermPool(), 0)
    dictionary.reserve(((1, 1),))
    batch = DecodedTerms(
        1,
        1,
        True,
        (IriTerm(b"p"), IriTerm(b"s"), TripleTerm(2, 1, 1), TripleTerm(0, 1, 1)),
    )
    assert dictionary.add(batch).order_holds is False


def test_line_fold_sums_annotations_and_removes_populated_empty_graphs() -> None:
    dictionary = TermDictionary(TermPool(), 0)
    dictionary.add(
        DecodedTerms(0, 1, False, tuple(IriTerm(bytes([c])) for c in b"spog"))
    )
    fold = LineFold()
    fold.add(dictionary.resolve_rows(DecodedRows(5, (EmptyGraphRow(3),), None, None)))
    maximum = (1 << 64) - 1
    fold.add(
        dictionary.resolve_rows(
            DecodedRows(4, (AnnotationRow(maximum, 0, 1, 2, 3),), None, None)
        )
    )
    fold.add(
        dictionary.resolve_rows(
            DecodedRows(4, (AnnotationRow(1, 0, 1, 2, 3),), None, None)
        )
    )
    result = fold.result()
    assert result.width_refused
    assert result.lines == {AnnotationRow(maximum, 0, 1, 2, 3)}

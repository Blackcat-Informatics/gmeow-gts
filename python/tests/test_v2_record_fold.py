# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check file-scoped binding resolution and monotone suppression."""

from gts.v2.dictionary import Blank, TermDictionary, TermPool
from gts.v2.record_fold import (
    GroundRef,
    RecordFinding,
    RecordFrame,
    RecordLocation,
    RecordScope,
    fold_records,
)
from gts.v2.records import (
    Binding,
    BlankPair,
    BlobReference,
    LineTarget,
    StateReference,
    SuppressionEntry,
    SuppressionKind,
    TripleLeaf,
)
from gts.v2.rows import AnnotationRow, QuadRow
from gts.v2.terms import BlankTerm, DecodedTerms, IriTerm, TripleTerm, WireTerm


def _dictionary(pool: TermPool, ordinal: int, *terms: WireTerm) -> TermDictionary:
    dictionary = TermDictionary(pool, ordinal)
    dictionary.add(DecodedTerms(0, 64, False, terms))
    return dictionary


def test_pair_uses_first_other_frame_holder_without_falling_through() -> None:
    pool = TermPool()
    dictionaries = {
        0: _dictionary(pool, 0, BlankTerm(b"a")),
        1: _dictionary(pool, 1, BlankTerm(b"b")),
        2: _dictionary(pool, 2, BlankTerm(b"c")),
    }
    identifier = bytes(32)
    blob = BlobReference(1, identifier)
    scope = RecordScope(
        dictionaries, {identifier: (0, 1)}, {blob}, {identifier}, set(), set()
    )
    pair = Binding((BlankPair(identifier, b"b"),), blob)
    frames = (
        RecordFrame(RecordLocation(0, 3), (pair,), 1),
        RecordFrame(RecordLocation(1, 3), (Binding((0,), blob),), 1),
        RecordFrame(RecordLocation(2, 3), (pair,), 1),
    )
    result = fold_records(frames, scope, pool, ())
    named = {
        pool.term(binding.term)
        for binding in result.bindings
        if isinstance(binding.term, int)
    }
    assert named == {Blank(1, b"b")}
    assert sum(isinstance(binding.term, GroundRef) for binding in result.bindings) == 1
    assert result.findings == (
        RecordFinding(RecordLocation(2, 3), "binding_unanchored"),
    )


def test_forward_reference_wins_across_the_entire_suppression_entry() -> None:
    pool = TermPool()
    dictionary = TermDictionary(pool, 0)
    dictionary.reserve(((1, 1),))
    dictionary.add(DecodedTerms(1, 64, False, (IriTerm(b"urn:a"),)))
    scope = RecordScope({0: dictionary}, {}, set(), set(), set(), set())
    early = RecordLocation(0, 2)
    entry = SuppressionEntry(SuppressionKind.QUAD, LineTarget(((0,), (1,), (0,)), None))
    result = fold_records((RecordFrame(early, (entry,), 1),), scope, pool, ())
    assert result.findings == (RecordFinding(early, "forward_reference"),)
    assert not result.held_records
    assert not result.suppression


def test_missing_referents_count_by_name_even_at_the_same_frame() -> None:
    pool = TermPool()
    dictionary = _dictionary(pool, 0, IriTerm(b"urn:subject"))
    available = StateReference(b"a" * 32)
    unavailable = StateReference(b"u" * 32)
    one, two = BlobReference(1, b"1" * 32), BlobReference(1, b"2" * 32)
    scope = RecordScope(
        {0: dictionary},
        {},
        set(),
        set(),
        {available.identifier},
        {available.identifier, unavailable.identifier},
    )
    where = RecordLocation(0, 2)
    frame = RecordFrame(
        where,
        tuple(Binding((0,), ref) for ref in (one, two, one, available, unavailable)),
        1,
    )
    result = fold_records((frame,), scope, pool, ())
    assert len(result.bindings) == 4
    assert (
        result.findings.count(RecordFinding(where, "missing_dependency", "blob")) == 2
    )
    assert (
        result.findings.count(
            RecordFinding(where, "missing_dependency", "state_unavailable")
        )
        == 1
    )
    assert len(result.findings) == 3


def test_suppression_matches_nested_terms_and_every_rule_for_a_hidden_line() -> None:
    pool = TermPool()
    dictionary = _dictionary(
        pool,
        0,
        IriTerm(b"urn:s"),
        IriTerm(b"urn:p"),
        IriTerm(b"urn:o"),
        IriTerm(b"urn:g"),
        TripleTerm(0, 1, 2),
    )
    terms = tuple(dictionary.resolved_id(term) for term in range(5))
    assert all(isinstance(term, int) for term in terms)
    subject, predicate, object_id, graph, nested = terms
    quad = QuadRow(subject, predicate, nested)
    annotation = AnnotationRow(42, subject, predicate, object_id, graph)
    blob = BlobReference(1, bytes(32))
    scope = RecordScope({0: dictionary}, {}, {blob}, set(), set(), set())
    rules = (
        SuppressionEntry(SuppressionKind.TERM, (2,)),
        SuppressionEntry(SuppressionKind.GRAPH, (3,)),
        SuppressionEntry(
            SuppressionKind.ANNOTATION, LineTarget(((0,), (1,), (2,)), (3,))
        ),
        SuppressionEntry(SuppressionKind.BLOB, blob),
    )
    result = fold_records(
        (RecordFrame(RecordLocation(0, 5), rules, 5),), scope, pool, (quad, annotation)
    )
    assert result.hidden_lines == {quad, annotation}
    assert result.hidden_blobs == {blob}
    assert len(result.suppression) == 4
    assert not result.findings


def test_term_rule_matches_a_named_component_of_an_unnamed_binding() -> None:
    pool = TermPool()
    dictionary = _dictionary(pool, 0, IriTerm(b"urn:p"), IriTerm(b"urn:o"))
    blob = BlobReference(1, bytes(32))
    binding = Binding((TripleLeaf(0), BlankPair(bytes(32), b"a"), 0, 1), blob)
    scope = RecordScope({0: dictionary}, {}, {blob}, set(), set(), set())
    source = RecordLocation(0, 2)
    where = RecordLocation(0, 3)
    rules = (
        SuppressionEntry(SuppressionKind.TERM, (1,)),
        SuppressionEntry(SuppressionKind.BINDING, binding),
    )
    result = fold_records(
        (RecordFrame(source, (binding,), 2), RecordFrame(where, rules, 2)),
        scope,
        pool,
        (),
    )
    assert result.hidden_bindings == result.bindings
    assert result.findings == (
        RecordFinding(source, "binding_unanchored"),
        RecordFinding(where, "suppression_unmatched", "binding"),
    )

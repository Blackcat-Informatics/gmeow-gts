# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the P3 labelled set and capacity decisions before L1."""

from dataclasses import replace

import cbor2
import pytest
from blake3 import blake3

from gts.v2 import _fold_terms
from gts.v2._labelling_result import label
from gts.v2.blobs import BlobEntry, BlobSet
from gts.v2.dictionary import (
    Blank,
    Iri,
    LanguageLiteral,
    TermPool,
    Triple,
    TypedLiteral,
)
from gts.v2.fold_labelling import FoldLabellingBudget, FoldLabellingRefused, folded_size
from gts.v2.graph import GraphFrame, TermReservation
from gts.v2.labelling import Iri as LabellingIri
from gts.v2.labelling import LabellingInput, LabellingTerms, Literal, RecordLine
from gts.v2.lines import LineResult
from gts.v2.record_fold import (
    GroundRef,
    GroundTriple,
    RecordFold,
    RecordFrame,
    RecordLocation,
    ResolvedBinding,
    ResolvedSuppression,
)
from gts.v2.records import (
    Binding,
    BlankPair,
    FrameReference,
    SuppressionKind,
    TripleLeaf,
)
from gts.v2.rows import DecodedRows, QuadRow
from gts.v2.terms import (
    DecodedTerms,
    TripleTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
)

_NO_RECORDS = RecordFold(
    frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
)
_NO_BLOBS = BlobSet((), ())


def _budget(weight: int, size: int = 10) -> FoldLabellingBudget:
    return FoldLabellingBudget(
        labelled_lines=size, labelled_bytes_per_read=weight, labelling_units=0
    )


def test_size_counts_folded_reservations_before_resolving_ids_or_merging_rows() -> None:
    row = QuadRow(0, 1, 2)
    first = DecodedTerms(
        0,
        1,
        False,
        (
            TypedValueTerm(TypedValue(2, 1)),
            TripleTerm(0, 0, 10),
            TripleTerm(0, 0, 30),
            TypedValueTerm(ValueReference(b"v" * 32, 24)),
        ),
    )
    later = DecodedTerms(10, 1, False, (TripleTerm(0, 0, 0),))
    other = DecodedTerms(30, 1, False, (TripleTerm(0, 0, 0),))
    graphs = (
        (
            0,
            (
                GraphFrame(first),
                GraphFrame(DecodedRows(2, (row, row), None, None)),
                GraphFrame(later),
                GraphFrame(TermReservation(((4, 1),))),
            ),
        ),
        (1, (GraphFrame(other),)),
    )
    referent = FrameReference(b"f" * 32)
    entries = (
        Binding((TripleLeaf(0), 10, 99, TripleLeaf(0), 1, 99, 30), referent),
        Binding((1,), referent),
    )
    records = (RecordFrame(RecordLocation(0, 1), entries, 4),)
    assert folded_size(graphs, records, blob_declarations=3) == 13


def test_worked_nested_fold_weight_and_latched_refusal() -> None:
    # P3 section 6.5 gives the weighted two-line example and these read limits.
    pool = TermPool()
    x = pool._intern(Iri("X" * 1024))
    inner = pool._intern(Triple(x, x, x))
    outer = pool._intern(Triple(x, x, inner))
    lines = LineResult(
        frozenset((QuadRow(x, x, inner), QuadRow(x, x, outer))), False, False
    )
    budget = _budget(24895, 3)
    first = budget.label_fold(pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=3)
    assert budget.used_weight == 12448
    assert first.term_node_positions == {inner: 0}
    assert [len(line) for line in first.labelling.sequence.encodings] == [
        5168,
        4144,
        3136,
    ]
    with pytest.raises(FoldLabellingRefused, match="refusal_weight"):
        budget.label_fold(pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=3)
    empty = LineResult(frozenset(), False, False)
    with pytest.raises(FoldLabellingRefused, match="refusal_size"):
        budget.label_fold(pool, empty, _NO_RECORDS, _NO_BLOBS, (), folded_size=4)
    with pytest.raises(FoldLabellingRefused, match="refusal_weight"):
        budget.label_fold(pool, empty, _NO_RECORDS, _NO_BLOBS, (), folded_size=0)
    assert budget.used_weight == 12448
    exact = _budget(24896)
    for _ in range(2):
        assert (
            exact.label_fold(pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=3)
            == first
        )
    assert exact.used_weight == 24896


def test_nested_source_uses_ordinal_label_and_full_literal_form() -> None:
    pool = TermPool()
    subject = pool._intern(Blank(7, b"\x00opaque"))
    predicate = pool._intern(Iri("p"))
    object_id = pool._intern(LanguageLiteral("é", "EN", "rtl"))
    inner = pool._intern(Triple(subject, predicate, object_id))
    outer = pool._intern(Triple(subject, predicate, inner))
    lines = LineResult(
        frozenset(
            (QuadRow(subject, predicate, inner), QuadRow(subject, predicate, outer))
        ),
        False,
        False,
    )
    result = _budget(4096).label_fold(
        pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=3
    )
    context = "gts2 nested term"
    blank = cbor2.dumps([7, b"\x00opaque"])
    components = [
        cbor2.dumps([0, blake3(blank, derive_key_context=context).digest()]),
        cbor2.dumps(cbor2.dumps([1, "p"])),
        cbor2.dumps(cbor2.dumps([2, "é", 4, "EN"])),
    ]
    source = cbor2.dumps(
        [blake3(cbor2.dumps(components), derive_key_context=context).digest()]
    )
    assert set(result.labelling.labels) == {blank, source}
    assert result.term_node_positions == {
        inner: result.labelling.labels[source].position
    }
    assert result.blank_positions == {subject: result.labelling.labels[blank].position}


def test_fold_labels_hidden_rows_and_all_non_nested_record_kinds() -> None:
    pool = TermPool()
    s, p, g = (pool._intern(Iri(text)) for text in ("s", "p", "g"))
    value = TypedValue(2, 3)
    literal = pool._intern(TypedLiteral(*value.literal()))
    row = QuadRow(s, p, literal, g)
    frame = FrameReference(bytes(range(32)))
    pair = BlankPair(bytes(32), b"not named")
    blob = BlobEntry((1, b"b" * 32), 24, ("type/x",), "incomplete")
    binding = ResolvedBinding(GroundRef(1), frame)
    suppression = ResolvedSuppression(SuppressionKind.QUAD, (s, p, literal), g)
    records = replace(
        _NO_RECORDS,
        bindings=frozenset((binding,)),
        suppression=frozenset((suppression,)),
        ground_terms=(pair, GroundTriple(GroundRef(0), p, literal)),
        hidden_lines=frozenset((row,)),
        hidden_bindings=frozenset((binding,)),
    )
    lines = LineResult(frozenset((row,)), False, False)
    budget = _budget(4096)
    result = budget.label_fold(
        pool, lines, records, BlobSet((blob,), ()), ((literal, value),), folded_size=5
    )

    terms = LabellingTerms()
    ls, lp, lg = (terms.add(LabellingIri(text)) for text in (b"s", b"p", b"g"))
    ll = terms.add(Literal(*(text.encode() for text in value.literal())))
    slot = [1]
    payloads = (
        (
            [1, [[3, [2, pair.frame, pair.label], slot, slot], [2, frame.identifier]]],
            (lp, ll),
        ),
        ([2, [2, slot, slot, slot, [2, slot]]], (ls, lp, ll, lg)),
        ([3, [1, [1, b"b" * 32], 24, ["type/x"]]], ()),
        ([4, [2, slot]], (ll,)),
    )
    expected = label(
        LabellingInput(
            terms,
            (
                QuadRow(ls, lp, ll, lg),
                *(RecordLine(cbor2.dumps(payload), args) for payload, args in payloads),
            ),
        )
    )
    assert result.labelling == expected
    assert result.content_digest == expected.sequence.digest
    assert budget.used_weight == sum(map(len, expected.sequence.encodings))
    held = _budget(4096).label_fold(
        pool,
        replace(lines, held_rows=True),
        records,
        BlobSet((blob,), ()),
        ((literal, value),),
        folded_size=5,
    )
    assert held.content_digest is None
    assert held.labelling == expected


def test_weight_is_admitted_before_collisions_and_width() -> None:
    pool = TermPool()
    x, y = (pool._intern(Iri(name)) for name in ("x", "y"))
    a = pool._intern(Triple(x, x, x))
    b = pool._intern(Triple(y, x, x))
    outer_a = pool._intern(Triple(x, x, a))
    outer_b = pool._intern(Triple(x, x, b))
    lines = LineResult(
        frozenset((QuadRow(x, x, outer_a), QuadRow(x, x, outer_b))), False, True
    )

    class ConstantHash:
        def digest(self) -> bytes:
            return bytes(32)

    with pytest.MonkeyPatch.context() as patch:
        patch.setattr(_fold_terms, "blake3", lambda *_args, **_kwargs: ConstantHash())
        at_cap = _budget(0)
        with pytest.raises(FoldLabellingRefused, match="refusal_weight"):
            at_cap.label_fold(pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=4)
        enough = _budget(4096)
        with pytest.raises(FoldLabellingRefused, match="refusal_term_node"):
            enough.label_fold(pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=4)
        assert enough.used_weight > 0
    enough = _budget(4096)
    with pytest.raises(FoldLabellingRefused, match="refusal_width"):
        enough.label_fold(pool, lines, _NO_RECORDS, _NO_BLOBS, (), folded_size=4)
    assert enough.used_weight > 0


def test_unnamed_triple_slots_are_written_at_arbitrary_leaf_depth() -> None:
    pool = TermPool()
    iri = pool._intern(Iri("p"))
    pair = BlankPair(bytes(32), b"?")
    depth = 1050
    triples = tuple(GroundTriple(iri, iri, GroundRef(i)) for i in range(depth))
    referent = FrameReference(b"f" * 32)
    records = replace(
        _NO_RECORDS,
        bindings=frozenset((ResolvedBinding(GroundRef(depth), referent),)),
        ground_terms=(pair, *triples),
    )
    budget = _budget(100_000, depth)
    result = budget.label_fold(
        pool,
        LineResult(frozenset(), False, False),
        records,
        _NO_BLOBS,
        (),
        folded_size=depth,
    )
    (line,) = result.labelling.sequence.encodings
    payload = (
        bytes.fromhex("820182")
        + bytes.fromhex("840381018101") * depth
        + cbor2.dumps([2, pair.frame, pair.label])
        + cbor2.dumps([2, referent.identifier])
    )
    assert line[9 : 9 + int.from_bytes(line[1:9], "big")] == payload
    assert budget.used_weight == len(line)
    assert not result.term_node_positions

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the graph region of a canonical GTS v2 segment."""

from dataclasses import replace
from pathlib import Path

from gts.v2.blobs import BlobSet
from gts.v2.canonical_graph import CanonicalGraph, canonical_graph
from gts.v2.dictionary import (
    Blank,
    Iri,
    LanguageLiteral,
    TermPool,
    Triple,
    TypedLiteral,
)
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.frame import decode_frame
from gts.v2.header import decode_header
from gts.v2.lines import LineResult
from gts.v2.payload import PayloadBudget
from gts.v2.rebuild import unit_digest
from gts.v2.record_fold import RecordFold, ResolvedBinding, ResolvedSuppression
from gts.v2.records import BlobReference, SuppressionKind
from gts.v2.rows import AnnotationRow, QuadRow
from gts.v2.terms import (
    BlankTerm,
    IriTerm,
    LiteralTerm,
    TripleTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
)
from gts.v2.writer import SegmentWriter

_VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P3"
_NO_RECORDS = RecordFold(
    frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
)
_NO_BLOBS = BlobSet((), ())


def _graph_frames(data: bytes) -> list[tuple[int, bytes, bytes]]:
    budget = PayloadBudget(
        stored_bytes_per_frame=1 << 26,
        decoded_bytes_per_frame=1 << 26,
        decoded_bytes_per_read=1 << 36,
    )
    frames = []
    offset = decode_header(data).next_offset
    while offset < len(data):
        frame = decode_frame(data, offset)
        if frame.frame_type == 15:
            break
        if 1 <= frame.frame_type <= 5:
            check = budget.check(frame)
            assert check.decoded is not None
            frames.append(
                (frame.frame_type, bytes(frame.content), bytes(check.decoded))
            )
        offset = frame.next_offset
    return frames


def _assert_graph_region(graph: CanonicalGraph, name: str) -> None:
    writer = SegmentWriter()
    for kind, content, payload in graph.frames():
        writer.append(kind, content, payload)
    expected = (_VECTORS / f"{name}.canonical.bin").read_bytes()
    assert _graph_frames(writer.to_bytes()) == _graph_frames(expected)


def test_canonical_dictionary_keeps_hidden_terms_and_datatypes() -> None:
    pool = TermPool()
    p, b, a = (pool._intern(Iri(name)) for name in ("p", "b", "a"))
    literal = pool._intern(TypedLiteral("x", "dt"))
    blank = pool._intern(Blank(0, b"source label"))
    unused = pool._intern(Iri("unused"))
    hidden = QuadRow(a, p, b)
    lines = LineResult(frozenset((QuadRow(a, p, literal), hidden)), False, False)
    records = replace(
        _NO_RECORDS,
        bindings=frozenset((ResolvedBinding(blank, BlobReference(1, bytes(32))),)),
        suppression=frozenset((ResolvedSuppression(SuppressionKind.QUAD, (a, p, b)),)),
        hidden_lines=frozenset((hidden,)),
    )
    labelled = FoldLabellingBudget(
        labelled_lines=4, labelled_bytes_per_read=4096, labelling_units=0
    ).label_fold(pool, lines, records, _NO_BLOBS, (), folded_size=4)
    graph = canonical_graph(pool, lines.lines, records, (), labelled.blank_positions)
    assert unused not in graph.term_ids
    assert graph.terms[graph.term_ids[blank]] == BlankTerm(b"\x00")
    _assert_graph_region(graph, "canonical-dictionary-membership")


def test_rows_use_dictionary_ids_and_preserve_annotation_counts() -> None:
    pool = TermPool()
    a, g, o, p, q, reifier = (
        pool._intern(Iri(name)) for name in ("a", "g", "o", "p", "q", "r1")
    )
    z, w = (pool._intern(Blank(0, name)) for name in (b"z", b"w"))
    literal = pool._intern(TypedLiteral("v", "d"))
    rows = (
        QuadRow(a, p, o, g),
        QuadRow(z, p, o),
        AnnotationRow(3, reifier, q, literal),
        QuadRow(a, p, o),
        QuadRow(w, p, o),
    )
    graph = canonical_graph(pool, rows, _NO_RECORDS, (), {w: 0, z: 1})
    _assert_graph_region(graph, "canonical-rows-sort")


def test_triples_remain_full_dictionary_terms_with_earlier_components() -> None:
    pool = TermPool()
    p = pool._intern(Iri("p"))
    blank = pool._intern(Blank(3, b"z"))
    inner = pool._intern(Triple(blank, p, blank))
    outer = pool._intern(Triple(blank, p, inner))
    graph = canonical_graph(
        pool, [QuadRow(blank, p, outer)], _NO_RECORDS, (), {blank: 24}
    )
    assert graph.terms == (
        IriTerm(b"p"),
        BlankTerm(b"\x18\x18"),
        TripleTerm(1, 0, 1),
        TripleTerm(1, 0, 2),
    )
    assert graph.rows == (QuadRow(1, 0, 3),)


def test_literal_and_typed_value_thresholds_share_equal_octets() -> None:
    pool = TermPool()
    p = pool._intern(Iri("p"))
    data = b"a" * (1 << 20)
    language = pool._intern(LanguageLiteral(data.decode(), "en", None))
    inline_language = pool._intern(LanguageLiteral(data[:-1].decode(), "en", None))
    value = TypedValue(5, data)
    typed = pool._intern(TypedLiteral(*value.literal()))
    edge = TypedValue(5, b"b" * (1 << 19))
    typed_edge = pool._intern(TypedLiteral(*edge.literal()))
    inline = TypedValue(5, b"c" * ((1 << 19) - 1))
    typed_inline = pool._intern(TypedLiteral(*inline.literal()))
    graph = canonical_graph(
        pool,
        (QuadRow(p, p, language), QuadRow(p, p, inline_language)),
        _NO_RECORDS,
        ((typed, value), (typed_edge, edge), (typed_inline, inline)),
        {},
    )
    shared = ValueReference(unit_digest(data), len(data))
    assert graph.terms[graph.term_ids[language]] == LiteralTerm(shared, language="en")
    assert graph.terms[graph.term_ids[inline_language]] == LiteralTerm(
        data[:-1], language="en"
    )
    assert graph.terms[graph.term_ids[typed]] == TypedValueTerm(shared)
    assert graph.terms[graph.term_ids[typed_edge]] == TypedValueTerm(
        ValueReference(unit_digest(edge.value), len(edge.value))
    )
    assert graph.terms[graph.term_ids[typed_inline]] == TypedValueTerm(inline)
    assert set(graph.value_octets) == {data, edge.value}
    assert len(graph.value_octets) == 2
    assert all(term.text == b"p" for term in graph.terms if isinstance(term, IriTerm))


def test_terms_budget_cuts_at_the_limit_and_isolates_a_large_term() -> None:
    pool = TermPool()
    a = pool._intern(Iri("a" * ((1 << 22) - 8)))
    b = pool._intern(Iri("b" * ((1 << 22) + 1)))
    c = pool._intern(Iri("c"))
    graph = canonical_graph(pool, (QuadRow(a, c, b),), _NO_RECORDS, (), {})
    content = [record for kind, record, _ in graph.frames() if kind == 1]
    assert content == [[base, [[1, 1]], 1, 64] for base in range(3)]

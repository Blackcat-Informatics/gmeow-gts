# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""`nquads → gts` inverse-of-fold transform (#14)."""

from __future__ import annotations

from pathlib import Path

import pytest

from gts import Term, TermKind, Writer, from_nquads, read, to_nquads
from gts.cli import main
from gts.from_nquads import NQuadsParseError
from gts.model import RDF_DIR_LANG_STRING

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors"
RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"


def _roundtrip(gts_bytes: bytes) -> bool:
    nq1 = to_nquads(read(gts_bytes))
    nq2 = to_nquads(read(from_nquads(nq1)))
    return sorted(nq1.splitlines()) == sorted(nq2.splitlines())


@pytest.mark.parametrize(
    "vector",
    sorted(p.name for p in VECTORS_DIR.glob("*.gts")),
)
def test_fold_roundtrips_through_from_nquads(vector: str) -> None:
    """fold → from-nq → fold reproduces the projection for every pure graph."""
    data = (VECTORS_DIR / vector).read_bytes()
    try:
        nq = to_nquads(read(data))
    except Exception:  # noqa: BLE001 - vectors with damaged frames are not graphs
        pytest.skip("vector does not fold to a graph")
    if not nq.strip():
        pytest.skip("empty fold")
    assert _roundtrip(data)


def test_named_graph_reifier_and_annotation_roundtrip() -> None:
    """Named graphs, reifiers, and annotations all survive the round-trip."""
    w = Writer(profile="dist")
    w.add_terms(
        [
            Term(TermKind.IRI, "https://ex/s"),
            Term(TermKind.IRI, "https://ex/p"),
            Term(TermKind.IRI, "https://ex/o"),
            Term(TermKind.IRI, "https://ex/g"),
            Term(TermKind.IRI, "https://ex/conf"),
            Term(TermKind.LITERAL, "0.9"),
        ]
    )
    w.add_quads([(0, 1, 2, 3)])
    w.add_reifies([(0, (0, 1, 2), None)])
    w.add_annot([(0, 4, 5, None)])
    assert _roundtrip(w.to_bytes())


def test_literals_lang_and_datatype_roundtrip() -> None:
    """Language-tagged and datatyped literals survive."""
    xsd_int = "http://www.w3.org/2001/XMLSchema#integer"
    nq = (
        '<https://ex/s> <https://ex/label> "Cat"@en .\n'
        f'<https://ex/s> <https://ex/n> "42"^^<{xsd_int}> .\n'
        "_:b0 <https://ex/p> <https://ex/s> .\n"
    )
    out = to_nquads(read(from_nquads(nq)))
    assert sorted(out.splitlines()) == sorted(nq.strip().splitlines())


def test_directional_language_literals_roundtrip() -> None:
    nq = '<https://ex/s> <https://ex/label> "Cat"@en--ltr .\n'
    graph = read(from_nquads(nq))
    literal = next(
        term
        for term in graph.terms
        if term.kind is TermKind.LITERAL and term.value == "Cat"
    )
    assert literal.lang == "en"
    assert literal.direction == "ltr"
    assert graph.datatype_iri(literal) == RDF_DIR_LANG_STRING
    assert sorted(to_nquads(graph).splitlines()) == sorted(nq.strip().splitlines())


def test_compact_blank_node_and_language_tag_delimiters_roundtrip() -> None:
    """A final '.' delimiter is not part of blank node labels or language tags."""
    nq = (
        "<https://ex/s> <https://ex/p> _:b0.\n"
        '<https://ex/s> <https://ex/label> "Cat"@en.\n'
    )
    expected = (
        "<https://ex/s> <https://ex/p> _:b0 .\n"
        '<https://ex/s> <https://ex/label> "Cat"@en .\n'
    )
    out = to_nquads(read(from_nquads(nq)))
    assert sorted(out.splitlines()) == sorted(expected.strip().splitlines())


def test_quoted_triple_adjacent_delimiters_roundtrip() -> None:
    """Quoted triple close delimiters are not consumed into node labels."""
    lang_reifier = f"<https://ex/r2> <{RDF_REIFIES}> "
    nq = (
        f"<https://ex/r1> <{RDF_REIFIES}> <<( _:b0 <https://ex/p> _:b1)>> .\n"
        f"{lang_reifier}"
        '<<( <https://ex/s> <https://ex/p> "Cat"@en)>> .\n'
    )
    expected = (
        f"<https://ex/r1> <{RDF_REIFIES}> <<( _:b0 <https://ex/p> _:b1 )>> .\n"
        f"{lang_reifier}"
        '<<( <https://ex/s> <https://ex/p> "Cat"@en )>> .\n'
    )
    out = to_nquads(read(from_nquads(nq)))
    assert sorted(out.splitlines()) == sorted(expected.strip().splitlines())


def test_writer_allows_multiple_reifiers_for_the_same_statement() -> None:
    writer = Writer(profile="dist")
    writer.add_terms(
        [
            Term(TermKind.IRI, "https://ex/r1"),
            Term(TermKind.IRI, "https://ex/r2"),
            Term(TermKind.IRI, "https://ex/s"),
            Term(TermKind.IRI, "https://ex/p"),
            Term(TermKind.IRI, "https://ex/o"),
        ]
    )
    writer.add_quads([(2, 3, 4, None)])
    writer.add_reifies([(0, (2, 3, 4), None), (1, (2, 3, 4), None)])

    graph = read(writer.to_bytes())
    assert graph.reifiers == [(0, (2, 3, 4), None), (1, (2, 3, 4), None)]
    out = to_nquads(graph)
    assert out.count(RDF_REIFIES) == 2
    assert "<https://ex/r1>" in out
    assert "<https://ex/r2>" in out


def test_from_nquads_preserves_multiple_reifiers_for_the_same_statement() -> None:
    nq = (
        f"<https://ex/r1> <{RDF_REIFIES}> "
        "<<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n"
        f"<https://ex/r2> <{RDF_REIFIES}> "
        "<<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n"
    )
    graph = read(from_nquads(nq))
    assert len(graph.reifiers) == 2
    assert len({spo for _rid, spo, _graph_name in graph.reifiers}) == 1
    assert sorted(to_nquads(graph).splitlines()) == sorted(nq.strip().splitlines())


def test_cli_from_nq_inverts_fold(tmp_path: Path) -> None:
    """`gts from-nq` writes a GTS that folds to the input N-Quads."""
    src = (VECTORS_DIR / "11-datatype-defaulting.gts").read_bytes()
    nq = to_nquads(read(src))
    nq_path = tmp_path / "in.nq"
    nq_path.write_text(nq, encoding="utf-8")
    out_path = tmp_path / "out.gts"
    assert main(["from-nq", str(nq_path), "-o", str(out_path)]) == 0
    assert sorted(to_nquads(read(out_path.read_bytes())).splitlines()) == sorted(
        nq.splitlines()
    )


def test_cli_from_nq_rejects_malformed() -> None:
    """Malformed N-Quads raise a parse error inside the transform."""
    with pytest.raises(NQuadsParseError):
        from_nquads("<https://ex/s> <https://ex/p> .\n")  # only 2 terms


def test_rejects_empty_blank_node_label_and_language_tag() -> None:
    """Delimiter fixes must not turn empty tokens into valid terms."""
    with pytest.raises(NQuadsParseError):
        from_nquads("<https://ex/s> <https://ex/p> _: .\n")
    with pytest.raises(NQuadsParseError):
        from_nquads('<https://ex/s> <https://ex/p> "Cat"@ .\n')

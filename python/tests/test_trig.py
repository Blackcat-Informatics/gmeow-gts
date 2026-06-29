# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""TriG import/export tests."""

from __future__ import annotations

from pathlib import Path

import pytest

from gts import Term, TermKind, Writer, from_trig, read, to_nquads, to_trig
from gts.cli import main
from gts.trig import TriGParseError

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors"
RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"


def _sorted_lines(text: str) -> list[str]:
    return sorted(
        line for line in text.splitlines() if line and not line.startswith("@prefix")
    )


def _roundtrip(gts_bytes: bytes) -> bool:
    folded = read(gts_bytes)
    nq1 = to_nquads(folded)
    nq2 = to_nquads(read(from_trig(to_trig(folded))))
    return _sorted_lines(nq1) == _sorted_lines(nq2)


@pytest.mark.parametrize("vector", sorted(p.name for p in VECTORS_DIR.glob("*.gts")))
def test_fold_roundtrips_through_trig(vector: str) -> None:
    data = (VECTORS_DIR / vector).read_bytes()
    folded = read(data)
    if not to_trig(folded).strip():
        pytest.skip("empty fold")
    assert _roundtrip(data)


def test_to_trig_groups_named_graphs_and_keeps_reifiers() -> None:
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

    folded = read(w.to_bytes())
    trig = to_trig(folded)
    assert "@prefix rdf:" in trig
    assert (
        "<https://ex/g> {\n  <https://ex/s> <https://ex/p> <https://ex/o> .\n}" in trig
    )
    assert "rdf:reifies <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> ." in trig
    assert _roundtrip(w.to_bytes())


def test_parses_prefixes_graph_blocks_reifiers_and_literals() -> None:
    trig = """@prefix ex: <https://ex/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g {
  ex:s ex:label "Cat"@en .
  ex:s ex:n "42"^^xsd:integer .
}
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9" .
"""
    out = to_nquads(read(from_trig(trig)))
    expected = (
        '<https://ex/s> <https://ex/label> "Cat"@en <https://ex/g> .\n'
        '<https://ex/s> <https://ex/n> "42"^^'
        "<http://www.w3.org/2001/XMLSchema#integer> <https://ex/g> .\n"
        f"<https://ex/r> <{RDF_REIFIES}> "
        "<<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n"
        '<https://ex/r> <https://ex/confidence> "0.9" .\n'
    )
    assert _sorted_lines(out) == _sorted_lines(expected)


def test_trig_preserves_directional_language_literals() -> None:
    trig = """@prefix ex: <https://ex/> .

ex:s ex:label "Cat"@en--rtl .
"""
    graph = read(from_trig(trig))
    assert '"Cat"@en--rtl' in to_nquads(graph)
    assert '"Cat"@en--rtl' in to_trig(graph)
    assert _roundtrip(from_trig(trig))


def test_parses_graph_keyword_and_a_predicate() -> None:
    trig = """PREFIX ex: <https://ex/>
GRAPH ex:g {
  ex:s a ex:Thing .
}
"""
    out = to_nquads(read(from_trig(trig)))
    assert _sorted_lines(out) == [
        "<https://ex/s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
        "<https://ex/Thing> <https://ex/g> ."
    ]


def test_prefixed_names_stop_before_quoted_triple_close() -> None:
    trig = """@prefix ex: <https://ex/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:r rdf:reifies <<( ex:s ex:p ex:o)>> .
"""
    out = to_nquads(read(from_trig(trig)))
    assert _sorted_lines(out) == [
        f"<https://ex/r> <{RDF_REIFIES}> "
        "<<( <https://ex/s> <https://ex/p> <https://ex/o> )>> ."
    ]


def test_rejects_malformed_or_unsupported_trig() -> None:
    with pytest.raises(TriGParseError, match="terminate statement"):
        from_trig("@prefix ex: <https://ex/> .\nex:s ex:p ex:o\n")
    with pytest.raises(TriGParseError, match="shorthand"):
        from_trig("@prefix ex: <https://ex/> .\nex:s ex:p ex:o ; ex:q ex:r .\n")


def test_cli_from_trig_inverts_to_trig(tmp_path: Path) -> None:
    src = (VECTORS_DIR / "11-datatype-defaulting.gts").read_bytes()
    src_path = tmp_path / "src.gts"
    src_path.write_bytes(src)
    trig_path = tmp_path / "in.trig"
    out_path = tmp_path / "out.gts"

    assert main(["to-trig", str(src_path)]) == 0
    trig_path.write_text(to_trig(read(src)), encoding="utf-8")
    assert main(["from-trig", str(trig_path), "-o", str(out_path)]) == 0
    assert _sorted_lines(to_nquads(read(out_path.read_bytes()))) == _sorted_lines(
        to_nquads(read(src))
    )


def test_cli_from_trig_rejects_invalid_utf8(tmp_path: Path, capsys: object) -> None:
    bad = tmp_path / "bad.trig"
    bad.write_bytes(b"\xff")
    assert main(["from-trig", str(bad)]) == 1
    captured = capsys.readouterr()
    assert "invalid UTF-8" in captured.err


def test_requested_module_import_path() -> None:
    from gts.trig import from_trig as module_from_trig
    from gts.trig import to_trig as module_to_trig

    assert module_from_trig is from_trig
    assert module_to_trig is to_trig

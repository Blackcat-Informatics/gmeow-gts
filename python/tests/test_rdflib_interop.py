# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Optional rdflib interop contract."""

from __future__ import annotations

from typing import Any

import pytest

from gts import Term, TermKind, Writer, read, to_nquads
from gts.rdf import RDF12UnsupportedError, from_rdflib, to_rdflib

rdflib = pytest.importorskip("rdflib")
XSD = pytest.importorskip("rdflib.namespace").XSD


def _nquads(dataset: Any) -> list[str]:
    text = dataset.serialize(format="nquads")
    if isinstance(text, bytes):
        text = text.decode("utf-8")
    return sorted(line for line in text.splitlines() if line.strip())


def test_rdflib_dataset_roundtrips_through_gts() -> None:
    dataset = rdflib.Dataset()
    graph = dataset.graph(rdflib.URIRef("https://example.org/graph"))
    cat = rdflib.URIRef("https://example.org/Cat")
    graph.add(
        (
            cat,
            rdflib.URIRef("http://www.w3.org/2000/01/rdf-schema#label"),
            rdflib.Literal("Cat", lang="en"),
        )
    )
    graph.add(
        (
            cat,
            rdflib.URIRef("https://example.org/lives"),
            rdflib.Literal(9, datatype=XSD.integer),
        )
    )

    gts_bytes = from_rdflib(dataset)
    folded = read(gts_bytes)
    assert not folded.diagnostics
    assert sorted(to_nquads(folded).splitlines()) == _nquads(dataset)
    assert _nquads(to_rdflib(folded)) == _nquads(dataset)


def test_rdflib_export_refuses_rdf12_quoted_triples_without_lossy_flag() -> None:
    writer = Writer(profile="dist")
    writer.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/claim"),
            Term(TermKind.IRI, "https://example.org/subject"),
            Term(TermKind.IRI, "https://example.org/predicate"),
            Term(TermKind.LITERAL, "object"),
        ]
    )
    writer.add_reifies([(0, (1, 2, 3), None)])
    folded = read(writer.to_bytes())

    with pytest.raises(RDF12UnsupportedError):
        to_rdflib(folded)

    lossy = to_rdflib(folded, allow_rdf12_lossy=True)
    assert _nquads(lossy) == []


def test_rdflib_export_allows_literal_text_that_mentions_quoted_triples() -> None:
    writer = Writer(profile="dist")
    writer.add_terms(
        [
            Term(TermKind.IRI, "https://example.org/note"),
            Term(TermKind.IRI, "https://example.org/text"),
            Term(TermKind.LITERAL, 'literal text says "<<( not syntax"'),
        ]
    )
    writer.add_quads([(0, 1, 2, None)])
    folded = read(writer.to_bytes())

    dataset = to_rdflib(folded)
    lines = _nquads(dataset)
    assert len(lines) == 1
    assert "literal text says" in lines[0]

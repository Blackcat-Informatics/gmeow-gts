# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Bounded nested-GTS Full Reader behavior."""

from __future__ import annotations

import json
from pathlib import Path

from gts import GTS_MEDIA_TYPE, Term, TermKind, Writer, read_nested
from gts.wire import digest_str

EX = "https://example.org/"
VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "security"


def _tiny_graph(label: str = "nested") -> bytes:
    writer = Writer(profile="dist")
    writer.add_terms(
        [
            Term(TermKind.IRI, EX + label),
            Term(TermKind.IRI, EX + "label"),
            Term(TermKind.LITERAL, label),
        ]
    )
    writer.add_quads([(0, 1, 2, None)])
    return writer.to_bytes()


def _bundle(child: bytes) -> bytes:
    writer = Writer(profile="bundle")
    writer.add_blob(child, mt=GTS_MEDIA_TYPE)
    return writer.to_bytes()


def test_read_nested_exposes_subgraph_by_blob_digest() -> None:
    child = _tiny_graph("child")
    outer = _bundle(child)

    result = read_nested(outer)

    digest = digest_str(child)
    assert digest in result.subgraphs
    assert result.subgraphs[digest].quads == [(0, 1, 2, None)]
    assert not [d for d in result.diagnostics if d.code == "RecursionLimit"]


def test_read_nested_stops_at_recursion_limit() -> None:
    grandchild = _tiny_graph("grandchild")
    child = _bundle(grandchild)
    outer = _bundle(child)

    result = read_nested(outer, max_depth=1)

    assert digest_str(child) in result.subgraphs
    assert digest_str(grandchild) not in result.subgraphs
    assert "RecursionLimit" in [d.code for d in result.diagnostics]


def test_read_nested_stops_at_decoded_size_budget() -> None:
    child = _tiny_graph("oversized")
    outer = _bundle(child)

    result = read_nested(outer, max_decoded_bytes=len(child) - 1)

    assert digest_str(child) not in result.subgraphs
    assert "RecursionLimit" in [d.code for d in result.diagnostics]


def test_nested_recursion_security_vector_descriptor() -> None:
    vector = json.loads((VECTORS_DIR / "nested-recursion-limit.json").read_text())
    assert vector["id"] == "nested-recursion-limit"
    assert vector["expected_diagnostics"] == ["RecursionLimit"]

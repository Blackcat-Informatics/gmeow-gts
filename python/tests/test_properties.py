# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Bounded property tests for GTS reader and projection invariants."""

from __future__ import annotations

import os

from hypothesis import given, settings
from hypothesis import strategies as st

from gts import Term, TermKind, Writer, read, to_nquads

CAT = "https://example.org/Cat"
LABEL = "http://www.w3.org/2000/01/rdf-schema#label"
PROPERTY_EXAMPLES = int(os.environ.get("GTS_PROPERTY_EXAMPLES", "40"))
PROPERTY_SETTINGS = settings(
    database=None,
    deadline=None,
    derandomize=True,
    max_examples=PROPERTY_EXAMPLES,
)


def _diag_codes(graph: object) -> list[str]:
    return [d.code for d in graph.diagnostics]  # type: ignore[attr-defined]


@PROPERTY_SETTINGS
@given(st.binary(max_size=512))
def test_reader_refuses_arbitrary_bytes_without_raising(data: bytes) -> None:
    graph = read(data)

    for diagnostic in graph.diagnostics:
        assert diagnostic.code
        assert diagnostic.detail

    # Refusal still yields a graph object that projection code can handle.
    to_nquads(graph)


def _label_log(labels: list[str]) -> bytes:
    writer = Writer(profile="dist")
    writer.add_terms(
        [
            Term(TermKind.IRI, CAT),
            Term(TermKind.IRI, LABEL),
            *[Term(TermKind.LITERAL, label, lang="en") for label in labels],
        ]
    )
    writer.add_quads([(0, 1, idx, None) for idx in range(2, len(labels) + 2)])
    return writer.to_bytes()


@PROPERTY_SETTINGS
@given(
    st.lists(
        st.text(alphabet=st.characters(codec="utf-8"), max_size=24),
        min_size=1,
        max_size=5,
    )
)
def test_writer_decode_projection_and_torn_append_are_deterministic(
    labels: list[str],
) -> None:
    first = _label_log(labels)
    second = _label_log(labels)

    assert first == second

    graph = read(first)
    assert _diag_codes(graph) == []
    assert len(graph.quads) == len(labels)

    projection = to_nquads(graph)
    assert projection == to_nquads(read(second))

    torn = read(first + b"\xa3")
    assert "TornAppendError" in _diag_codes(torn)
    assert torn.quads == graph.quads
    assert to_nquads(torn) == projection

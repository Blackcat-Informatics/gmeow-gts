# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Optional rdflib interop for RDF 1.1 datasets.

The adapter is deliberately narrow: rdflib handles the RDF 1.1 Dataset surface
well, while GTS can carry RDF 1.2 quoted-triple terms. Strict export refuses
quoted-triple syntax instead of silently changing the graph.
"""

from __future__ import annotations

from typing import Any

from gts.from_nquads import from_nquads
from gts.model import Graph
from gts.nquads import to_nquads


class RDF12UnsupportedError(ValueError):
    """Raised when an RDF 1.2 feature cannot be represented in rdflib RDF 1.1."""


def _as_text(value: object) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8")
    return str(value)


def _strict_rdf11(text: str) -> str:
    quoted = [line for line in text.splitlines() if _has_quoted_triple(line)]
    if quoted:
        raise RDF12UnsupportedError(
            "rdflib RDF 1.1 interop cannot represent RDF 1.2 quoted-triple "
            "terms; pass allow_rdf12_lossy=True to drop quoted-triple lines"
        )
    return text


def _lossy_rdf11(text: str) -> str:
    lines = [line for line in text.splitlines() if not _has_quoted_triple(line)]
    return "\n".join(lines) + ("\n" if lines else "")


def _has_quoted_triple(line: str) -> bool:
    outside: list[str] = []
    in_literal = False
    escaped = False
    for ch in line:
        if in_literal:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_literal = False
            continue
        if ch == '"':
            in_literal = True
        else:
            outside.append(ch)
    return "<<(" in "".join(outside)


def to_rdflib(graph: Graph, *, allow_rdf12_lossy: bool = False) -> Any:
    """Project a folded GTS graph into an ``rdflib.Dataset``.

    Args:
        graph: Folded GTS graph.
        allow_rdf12_lossy: When false, reject quoted-triple syntax. When true,
            drop N-Quads lines containing quoted triples and parse the remaining
            RDF 1.1-compatible dataset.

    Returns:
        An ``rdflib.Dataset``. Import happens lazily so plain GTS installs do
        not depend on rdflib.
    """
    from rdflib import Dataset

    text = to_nquads(graph)
    text = _lossy_rdf11(text) if allow_rdf12_lossy else _strict_rdf11(text)
    dataset = Dataset()
    if text.strip():
        dataset.parse(data=text, format="nquads")
    return dataset


def from_rdflib(dataset: Any) -> bytes:
    """Build a GTS file from an ``rdflib.Graph`` or ``rdflib.Dataset``.

    ``Dataset`` inputs preserve graph names through N-Quads. Plain ``Graph``
    inputs serialize as N-Triples, which is accepted as default-graph input by
    :func:`gts.from_nquads.from_nquads`.
    """
    try:
        serialized = dataset.serialize(format="nquads")
    except Exception:  # noqa: BLE001 - rdflib Graphs reject the nquads format
        serialized = dataset.serialize(format="nt")
    return from_nquads(_as_text(serialized))


to_rdflib_dataset = to_rdflib
from_rdflib_dataset = from_rdflib

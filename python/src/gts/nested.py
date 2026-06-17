# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Bounded nested-GTS discovery for Full Reader callers."""

from __future__ import annotations

from dataclasses import dataclass, field

from gts.crypto import KeyProvider
from gts.model import Diagnostic, Graph
from gts.reader import read

GTS_MEDIA_TYPE = "application/vnd.blackcat.gts+cbor-seq"


@dataclass
class NestedReadResult:
    """A root fold plus nested folds addressed by containing blob digest."""

    graph: Graph
    subgraphs: dict[str, Graph] = field(default_factory=dict)
    diagnostics: list[Diagnostic] = field(default_factory=list)


def read_nested(
    data: bytes,
    *,
    keys: KeyProvider | None = None,
    max_depth: int = 3,
    max_decoded_bytes: int = 16 * 1024 * 1024,
) -> NestedReadResult:
    """Read a GTS file and boundedly recurse into nested-GTS blobs.

    Baseline readers treat nested GTS as ordinary blobs. Full Reader callers can
    use this helper to expose subgraphs by blob digest while enforcing the
    recursion and decoded-size budgets required by §12.1/§18.
    """
    if max_depth < 0:
        msg = "max_depth must be >= 0"
        raise ValueError(msg)
    if max_decoded_bytes < 0:
        msg = "max_decoded_bytes must be >= 0"
        raise ValueError(msg)

    remaining = max_decoded_bytes
    subgraphs: dict[str, Graph] = {}

    def visit(blob: bytes, depth: int) -> Graph:
        nonlocal remaining
        graph = read(blob, keys=keys)
        for digest, meta in list(graph.blob_meta.items()):
            if meta.get("mt") != GTS_MEDIA_TYPE or digest not in graph.blobs:
                continue
            if digest in subgraphs:
                continue
            try:
                nested_bytes = graph.blobs[digest]
            except Exception as exc:  # noqa: BLE001 - lazy blob decode is untrusted input
                graph.diagnostics.append(
                    Diagnostic(
                        "DamagedFrame",
                        f"nested GTS blob {digest} could not be decoded: {exc}",
                        None,
                    )
                )
                continue
            if depth >= max_depth:
                graph.diagnostics.append(
                    Diagnostic(
                        "RecursionLimit",
                        f"nested GTS blob {digest} exceeds max depth {max_depth}",
                        None,
                    )
                )
                continue
            if len(nested_bytes) > remaining:
                graph.diagnostics.append(
                    Diagnostic(
                        "RecursionLimit",
                        "nested GTS decoded-size budget exceeded at "
                        f"{digest}: {len(nested_bytes)} > {remaining}",
                        None,
                    )
                )
                continue
            remaining -= len(nested_bytes)
            child = visit(nested_bytes, depth + 1)
            subgraphs[digest] = child
        return graph

    root = visit(data, 0)
    graphs = [root, *subgraphs.values()]
    diagnostics = [diag for graph in graphs for diag in graph.diagnostics]
    return NestedReadResult(graph=root, subgraphs=subgraphs, diagnostics=diagnostics)

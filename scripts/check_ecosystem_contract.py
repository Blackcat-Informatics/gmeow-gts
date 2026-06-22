#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Guard the ecosystem integration contract against documentation drift."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs" / "GTS-ECOSYSTEM-INTEGRATIONS.md"

REQUIRED_DOC_MARKERS = [
    "<!-- gts-ecosystem-contract:v1 -->",
    "## Status Matrix",
    "| Rust RDF |",
    "| Python RDF/data |",
    "| TypeScript browser |",
    "| Go services |",
    "| Tar-compatible archives |",
    "## Tar-Compatible Archive Bridge",
    "## Python: rdflib And Data Frames",
    "RDF12UnsupportedError",
    "gts.to_rdflib(graph, allow_rdf12_lossy=True)",
    "## Rust: RDF Crates",
    "gmeow_gts::rdf::{to_rdf_dataset, from_rdf_dataset}",
    "gmeow_gts::oxigraph::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}",
    "gmeow_gts::sophia::{to_sophia_dataset, from_sophia_dataset}",
    "Writer::from_store",
    'features = ["oxigraph-adapter"]',
    'features = ["sophia-adapter"]',
    "to_rdf_dataset_lossy",
    "Rio remains deferred",
    "## TypeScript: Browser And Range Fetch",
    "ReadableStream<Uint8Array>",
    "## Go: Services And Object Stores",
    "reader.ReadFrom(ctx, io.Reader, reader.Options)",
    "## Replication And Service Boundaries",
    "## Contract Guard",
]

LINK_TARGETS = [
    ROOT / "README.md",
    ROOT / "docs" / "gts-reference.md",
    ROOT / "rust" / "README.md",
    ROOT / "python" / "README.md",
    ROOT / "ts" / "README.md",
    ROOT / "go" / "README.md",
]


def fail(message: str) -> None:
    print(f"check_ecosystem_contract: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> int:
    if not DOC.is_file():
        fail(f"missing contract document: {DOC.relative_to(ROOT)}")
    text = DOC.read_text(encoding="utf-8")
    for marker in REQUIRED_DOC_MARKERS:
        if marker not in text:
            fail(f"{DOC.relative_to(ROOT)} missing marker: {marker}")
    for path in LINK_TARGETS:
        if not path.is_file():
            fail(f"missing link target: {path.relative_to(ROOT)}")
        if "GTS-ECOSYSTEM-INTEGRATIONS.md" not in path.read_text(encoding="utf-8"):
            fail(f"{path.relative_to(ROOT)} does not link the ecosystem contract")
    print("check_ecosystem_contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

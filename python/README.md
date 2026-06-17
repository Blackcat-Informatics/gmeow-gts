<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# gts — Graph Transport Substrate

<p>
  <a href="https://pypi.org/project/gmeow-gts/"><img src="https://img.shields.io/pypi/v/gmeow-gts.svg?label=PyPI" alt="PyPI"></a>
  <a href="https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml"><img src="https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
</p>

A single-file, language-independent transport for an **RDF 1.2** graph
(statements *and* statement-level metadata) together with any
content-addressed binary the graph references.

A GTS file is a CBOR Sequence of one or more **segments**, each an append-only
log: a header followed by frames chained by BLAKE3 content-id. Composition is
`cat` — concatenating valid GTS files yields a valid GTS file whose fold is
the value-union of the segment graphs.

Published GTS artifacts use media type `application/vnd.blackcat.gts+cbor-seq`. The
`+cbor-seq` suffix follows RFC 8742 because the file is a CBOR Sequence, not a single CBOR
data item.

This package is the **reference engine** for the
[GTS specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md):
reader (fold, chain verification, opaque degradation, torn-append detection),
writer, COSE signing and encryption, N-Quads projection, and the frozen
language-neutral conformance corpus. GTS ships **four independent, interoperable
engines** (Rust, Python, Go, TypeScript) that all gate against one frozen,
byte-exact corpus — Python is the source of truth they are measured against.

GTS is ontology-independent. GTS is the primary distribution method for GMEOW, but GTS does
not depend on GMEOW. The package name is `gmeow-gts` for distribution continuity; the import
name, CLI, and file extension remain `gts` and `.gts`.

## Install

```bash
pip install gmeow-gts
```

The installed package name is `gmeow-gts`; the import name and CLI binary both
remain `gts`, and GTS files keep the `.gts` extension. Optional extras:
`pip install 'gmeow-gts[rdf]'` for `rdflib` interop and
`pip install 'gmeow-gts[db]'` for the DuckDB/Parquet relational exports.

## Library

Read a file (fold + verify the chain) and project to N-Quads, or build one with
the writer:

```python
from pathlib import Path

import gts

# Read + verify + fold, then project to N-Quads.
graph = gts.read(Path("package.gts").read_bytes())
print(gts.to_nquads(graph))

# Write a minimal graph.
w = gts.Writer(profile="dist")
w.add_terms([
    gts.Term(gts.TermKind.IRI, "https://example.org/Cat"),
    gts.Term(gts.TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
    gts.Term(gts.TermKind.LITERAL, "Cat", lang="en"),
])
w.add_quads([(0, 1, 2, None)])
Path("cat.gts").write_bytes(w.to_bytes())
```

Reading `cat.gts` back yields the expected statement:

```text
<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> "Cat"@en .
```

`gts.Term`, `gts.TermKind`, `gts.Writer`, `gts.read`, and `gts.to_nquads` are
the stable public surface; `gts.from_nquads` builds a graph from N-Quads text.

## Command line

`pip install gmeow-gts` installs a `gts` binary whose verbs match the other
engines (§14.1 of the spec — the engines are CLI-compatible by conformance test):

```text
gts info <file>...            per-segment composition ledger
gts fold <file>               fold to N-Quads on stdout
gts from-nq <in.nq> -o <out>  build a GTS from N-Quads (inverse of fold; '-' = stdin)
gts verify <file>... [--key KID:HEXPUB]   verify chains + COSE signatures
gts extract-key <file>        print the embedded transport/verification key
gts cat -o <out> <file>...    validating composer: refuse degenerate inputs, then concatenate
gts ls <file>...              list inline blobs: digest, size, declared media type
gts pack <dir> -o <out>       package files/directories into a files-profile archive
gts unpack <file> -C <dir>    extract a files-profile archive (refuses path traversal)
gts extract <file> <digest>   write a single content-addressed blob
gts compact <file>            rewrite a segment into the streamable layout
gts diff <file> <directory>   compare a files-profile archive to a directory by digest
gts to-sqlite <file> <out>    export the folded graph to a SQLite database
gts to-duckdb <file> <out>    export to a DuckDB database (needs the [db] extra)
gts to-parquet <file> <dir>   export to Parquet, one file per table (needs the [db] extra)
```

Exit codes: `0` clean · `1` diagnostics or input refused · `2` usage/IO error.

`cat` output is the raw byte concatenation — validation added, transformation
never. It refuses dirty inputs, contributes-nothing segments, and compositions
whose suppressions hide every folded quad.

## Signing, encryption, and key identity

GTS integrity is layered. Every frame carries an independent BLAKE3 self-hash
and names its predecessor, so the head id transitively commits to all history.
On top of that:

- **COSE_Sign1 signatures** (§9.2, RFC 9052) are optional and algorithm-agile;
  `gts verify --key KID:HEXPUB` checks them against a raw Ed25519 public key.
- **COSE_Encrypt0 with AES-256-GCM** (§9.3) encrypts payloads; a held-back key
  degrades a frame to an **opaque node** rather than failing the read.
- **`gts extract-key`** prints the embedded transport key's kid, OpenPGP
  fingerprint, **emojihash**, and armored public key. The emojihash is a
  human-readable visual fingerprint (the `visual-hashing` lineage, exposed as
  `gts.emojihash.emojihash`) so a person can eyeball that two files carry the
  same key. All four engines parse the embedded key to the identical
  fingerprint and emojihash.

## Relational export

The folded graph can be projected into queryable stores via `gts to-sqlite`,
`gts to-duckdb`, and `gts to-parquet`. SQLite export needs no extra; DuckDB and
Parquet require the database extra:

```bash
pip install 'gmeow-gts[db]'
gts to-duckdb package.gts package.duckdb
```

GTS itself is not a database or query engine — these exports hand the graph off
to a transform target for random-access query.

## Example: grounded agent memory

The `gts.examples.agent_memory` module shows how to build a tiny claim store
on top of GTS: every claim is a reified RDF 1.2 statement with confidence,
standpoint, source, and timestamp; revision is supersession, never deletion;
the file is always a valid, `gts verify`-able package.

```bash
pip install gmeow-gts
python -m gts.examples.agent_memory
```

```python
from gts.examples.agent_memory import Memory

mem = Memory("assistant.gts")
mem.store(
    "Patrick prefers explicit error handling over exceptions-as-flow",
    source="conversation 2026-06-10",
    confidence=0.8,
    according_to="claude-fable-5",
)
print([c.text for c in mem.recall("error handling")])
```

Agent memory is one application of GTS, not the frame for the format. The same append-only,
content-addressed substrate also supports archives, evidence chains, local-first graph
synchronization, ontology distribution, and graph database interchange.

## Verifying the build

Wheels and sdists for `gmeow-gts` are built in GitHub Actions and signed with GitHub
artifact attestations. After downloading a package from PyPI, verify it with:

```bash
gh attestation verify <path-to-wheel-or-sdist> --repo Blackcat-Informatics/gmeow-gts
```

An SPDX SBOM is also generated for each release and attached as a workflow artifact.

## Contributing & security

Issues and pull requests are welcome — see
[`CONTRIBUTING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CONTRIBUTING.md)
and the [`CODE_OF_CONDUCT.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CODE_OF_CONDUCT.md).
To report a vulnerability, follow
[`SECURITY.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/SECURITY.md)
(do not open a public issue). The normative wire-format specification lives in
[`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md).

## License

Triple-licensed: **MIT OR Apache-2.0 OR proprietary** — use under MIT or Apache-2.0
at your option; a proprietary license is also available (see
[`LICENSING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md)).
© Blackcat Informatics® Inc.

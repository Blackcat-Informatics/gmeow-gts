<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<p align="center">
  <a href="https://github.com/Blackcat-Informatics/gmeow-gts">
    <img src="./docs/gts-logo.svg" alt="GTS logo" width="128" height="128">
  </a>
</p>

<h1 align="center">GTS — Graph Transport Substrate</h1>

<p align="center">
  <em>A single-file, content-addressed, append-only transport for RDF 1.2 graphs and the binaries they reference.</em>
</p>

<p align="center">
  <strong>A whole graph in a single, verifiable file.</strong>
</p>

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml"><img src="https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/gmeow-gts"><img src="https://img.shields.io/crates/v/gmeow-gts.svg?label=crates.io" alt="crates.io"></a>
  <a href="https://pypi.org/project/gmeow-gts/"><img src="https://img.shields.io/pypi/v/gmeow-gts.svg?label=PyPI" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/@blackcatinformatics/gmeow-gts"><img src="https://img.shields.io/npm/v/@blackcatinformatics/gmeow-gts.svg?label=npm" alt="npm"></a>
  <a href="./LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
</p>

---

GTS encodes a graph as an **append-only log of CBOR frames**. The logical graph is the
*fold* (replay) of the log. Growth is an append; "deletion" is **suppression**, never a
physical removal; optimisation is a separate, explicitly lossy compaction. Concatenating
two valid GTS files (`cat`) yields a valid GTS file whose fold is the value-union of the
inputs — so memory grows by append, never by rewrite.

Its lodestar is **reader simplicity**: a conformant baseline reader — "the rdflib of GTS" —
is small enough to prototype in a weekend in any language with a CBOR library, and a
consumer can do ~90% of what they'd do with an RDF library *without parsing RDF text*. This
repository holds **four interoperable engines** (Rust, Python, Go, TypeScript) that all gate
against one frozen, byte-exact conformance corpus, plus the specification that defines them.

## Table of contents

- [Why GTS?](#why-gts)
- [Install](#install)
- [Quick start](#quick-start)
  - [Python](#python)
  - [Rust](#rust)
  - [Go](#go)
  - [TypeScript](#typescript)
- [Command-line interface](#command-line-interface)
- [The file format in one minute](#the-file-format-in-one-minute)
- [Conformance corpus](#conformance-corpus)
- [Repository layout](#repository-layout)
- [Building from source](#building-from-source)
- [Versioning & releases](#versioning--releases)
- [Specification & docs](#specification--docs)
- [Contributing](#contributing)
- [License](#license)

## Why GTS?

Four properties define the format ([full spec](./docs/GTS-SPEC.md)):

1. **CBOR all the way down** (RFC 8949). One IETF-standardised binary encoding with native
   byte strings (no base64 tax), deterministic encoding (clean content hashes), and CBOR
   Sequences — concatenated items with no enclosing length, so append is cheap. A reader needs
   only a CBOR library.
2. **A durable transform catalog.** Each frame's payload carries a *stackable* chain of
   codecs from an open, long-lived catalog (`identity`, `gzip`, `zstd`, `zstd-rsyncable`,
   `cose-encrypt`, …) — separating *structure durability* (CBOR + this spec, forever) from
   *density and confidentiality* (swappable codecs).
3. **Integrity by construction.** Every frame carries an independent **BLAKE3 self-hash** and
   names its predecessor — a git-style content-addressed chain. Verification is parallel, a
   damaged frame is independently detectable, and the head id transitively commits to all
   history. Signatures and encryption (COSE, RFC 9052) are optional, layered, and algorithm-agile.
4. **Recursive composition (matryoshka).** A payload, once its transforms are reversed, is just
   bytes — and a GTS file is just bytes. So a payload MAY itself be a complete signed GTS,
   wrapped in any transform, riding inside an encrypted field with its own header and chain.

**Non-goals.** GTS is explicitly *not* a database, query engine, reasoner, or mutation
protocol. Random-access query, deep traversal, and SPARQL are the job of a transform target
(`.ttl`, `.nq`, DuckDB, SQLite, …), not of GTS. It is a *good-enough, durable, self-describing
container* — the narrow waist through which graphs and their referenced data travel.

> **Why it exists.** GTS was built as portable, auditable, content-addressed **agent memory**:
> belief revision modelled as suppression frames rather than destructive edits, so a memory
> file is a signable, independently verifiable record that travels across sessions, models, and
> vendors. See [`gts.examples.agent_memory`](./python/src/gts/examples/agent_memory.py).

## Install

| Language | Package | Install |
|---|---|---|
| **Rust** | [`gmeow-gts`](https://crates.io/crates/gmeow-gts) (binary `gts`) | `cargo install gmeow-gts` |
| **Python** | [`gmeow-gts`](https://pypi.org/project/gmeow-gts/) (module `gts`) | `pip install gmeow-gts` |
| **Go** | `go.blackcatinformatics.ca/gts` | `go install go.blackcatinformatics.ca/gts/cmd/gts@latest` |
| **TypeScript** | [`@blackcatinformatics/gmeow-gts`](https://www.npmjs.com/package/@blackcatinformatics/gmeow-gts) | `npm i @blackcatinformatics/gmeow-gts` |

The distributed package is named `gmeow-gts` everywhere; the import name and CLI binary stay
`gts`, and GTS files keep the `.gts` extension.

## Quick start

Every engine exposes the same shape: **read** bytes into a `Graph`, verify the chain, fold to
a value, and project to N-Quads — plus a **writer** for producing files.

### Python

```python
import gts
from pathlib import Path

# Read + verify + fold, then project to N-Quads
graph = gts.read(Path("package.gts").read_bytes())
print(gts.to_nquads(graph))

# Write a minimal graph
w = gts.Writer(profile="dist")
w.add_terms([
    gts.Term(gts.TermKind.IRI, "https://example.org/Cat"),
    gts.Term(gts.TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
    gts.Term(gts.TermKind.LITERAL, "Cat", lang="en"),
])
w.add_quads([(0, 1, 2, None)])
Path("cat.gts").write_bytes(w.to_bytes())
```

`pip install 'gmeow-gts[rdf]'` adds optional `rdflib` interop.

### Rust

```toml
# Cargo.toml
[dependencies]
gmeow-gts = "0.1"
```

```rust
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("package.gts")?;
    let graph = gmeow_gts::reader::read(&bytes)?;
    println!("{}", gmeow_gts::nquads::to_nquads(&graph));
    Ok(())
}
```

The pure-Rust crate (no C toolchain, wasm-friendly) installs the `gts` binary via
`cargo install gmeow-gts`.

### Go

```go
package main

import (
    "fmt"
    "os"

    "go.blackcatinformatics.ca/gts/nquads"
    "go.blackcatinformatics.ca/gts/reader"
)

func main() {
    data, _ := os.ReadFile("package.gts")
    g := reader.Read(data, false, nil) // (bytes, allowSegments, expectedHead)
    fmt.Print(nquads.ToNQuads(g))
}
```

### TypeScript

```typescript
import { Read, toNQuads } from "@blackcatinformatics/gmeow-gts";
import { readFileSync } from "node:fs";

const graph = Read(readFileSync("package.gts"), false);
console.log(toNQuads(graph));
```

Requires Node.js ≥ 22.16.0; ships as ES modules with type declarations.

## Command-line interface

`cargo install gmeow-gts`, `pip install gmeow-gts`, `npm i -g @blackcatinformatics/gmeow-gts`,
or `go install …` each install a `gts` binary with the **same verb surface** (§14.1 of the
spec — the engines are CLI-compatible by conformance test):

```text
gts info <file>...            per-segment composition ledger
gts fold <file>               fold to N-Quads on stdout
gts from-nq <in.nq> -o <out>  build a GTS from N-Quads (inverse of fold; '-' = stdin)
gts verify <file>...          verify chains; exit 1 on any diagnostic
gts extract-key <file>        print the embedded transport/verification key
gts cat -o <out> <file>...    validating composer: refuse degenerate inputs, then concatenate
gts ls <file>...              list segment digests, sizes, and media types
gts pack <dir> -o <out>       package a directory into a GTS files profile
gts unpack <file> -C <dir>    extract a files profile (refuses path traversal)
gts extract <file> <digest>   write a single content-addressed blob
gts compact <file>            compact a streamable segment into a self-contained one
gts diff <file> <directory>   compare a files profile to a directory
```

Exit codes: `0` clean · `1` diagnostics or input refused · `2` usage/IO error.

`extract-key` and `from-nq` are currently Python-CLI extensions (signing/verification
is Python-only today); cross-engine parity for them is in progress.

`cat` is raw byte concatenation with validation *added*, transformation *never*: it refuses
dirty inputs, contributes-nothing segments, and compositions whose suppressions hide every
folded quad.

## The file format in one minute

A GTS file is a **CBOR Sequence** (`application/cbor-seq`) of one or more **segments**. Each
segment is a header map followed by frames; each frame is a `[digest, content]` pair where the
digest is the BLAKE3 hash of the content, and each frame names its predecessor — a
content-addressed chain whose head transitively commits to all history.

```text
┌─ GTS file (CBOR Sequence) ───────────────────────────────────────────┐
│  ┌─ segment 0 ─────────────┐   ┌─ segment 1 (appended via `cat`) ──┐  │
│  │ header  {gts,t,prev,…}  │   │ header                            │  │
│  │ frame ─ [id, payload]   │   │ frame ─ [id, payload]             │  │
│  │ frame ─ [id, payload] ──┼──▶│ frame ─ [id, payload]             │  │
│  └─────────────────────────┘   └───────────────────────────────────┘  │
│            fold(segment) = value-union of all segment graphs           │
└───────────────────────────────────────────────────────────────────────┘
```

Payloads carry a stackable codec chain; unknown codecs or held-back keys degrade a frame to an
**opaque node** rather than failing the read. The full normative format is in
[`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md).

## Conformance corpus

[`vectors/`](./vectors) holds the frozen, language-neutral conformance corpus — one
`<name>.gts` (canonical bytes) and one `<name>.expected.json` (oracle-folded expectation) per
case (minimal files, zstd/gzip frames, unknown-codec fallback, damaged frames, torn appends,
suppression, multi-segment unions, streamable compaction, …). **Every engine must fold
identical bytes to identical expectations** — that is what makes the four implementations
interchangeable.

The Python reference implementation (`gts.vectors`) is the single source of truth. Regenerate
the committed corpus and prove it's reproducible byte-for-byte:

```bash
cd python && uv run python scripts/gen_vectors.py
git diff --exit-code vectors        # no changes ⇒ reproducible
```

## Repository layout

```text
gmeow-gts/
├── rust/        # Rust crate `gmeow-gts` + `gts` binary (pure Rust, wasm-friendly)
├── python/      # Python package `gmeow-gts` (module `gts`) + reference corpus generator
├── go/          # Go module go.blackcatinformatics.ca/gts
├── ts/          # TypeScript/npm package @blackcatinformatics/gmeow-gts
├── vectors/     # Frozen conformance corpus (*.gts + *.expected.json)
├── docs/        # GTS-SPEC.md (normative) + gts-reference.md
└── .github/     # CI (all four engines) + per-language release workflows
```

## Building from source

Each implementation builds and tests independently from its own directory:

```bash
cd rust   && cargo test                              # unit + CLI + conformance
cd go     && go test ./...                            # unit + conformance
cd ts     && npm ci && npm test                       # compiles, runs against vectors/
cd python && uv sync --extra rdf && uv run pytest     # reference + conformance
```

Or use the [`justfile`](./justfile): `just test` (all engines), `just lint`, `just fmt`,
`just gen-vectors`, `just interop`, `just fuzz-rust` / `just fuzz-go`, `just audit`, `just wasm`.

Repo-wide hygiene (formatting, SPDX/REUSE headers, YAML/Markdown/shell, secrets) runs through
`pre-commit run --all-files`. CI runs all four engines on Linux, macOS, and Windows, plus a
[live cross-engine interop check](./scripts/interop.sh) (each engine reads every other's
output), reader [fuzzing](./.github/workflows/fuzz.yml), and per-ecosystem
[supply-chain scanning](./.github/workflows/security.yml). Changes are tracked in
[`CHANGELOG.md`](./CHANGELOG.md).

## Versioning & releases

Each engine publishes to its native registry from this repo via a tag-triggered workflow:

| Engine | Registry | Release tag | Workflow |
|---|---|---|---|
| Rust | crates.io | `rust-v*` | [`release-cargo.yaml`](./.github/workflows/release-cargo.yaml) |
| Python | PyPI (trusted publishing) | `py-v*` | [`release-pypi.yml`](./.github/workflows/release-pypi.yml) |
| Go | GitHub Releases (GoReleaser) | `go-v*` | [`release-go.yaml`](./.github/workflows/release-go.yaml) |
| TypeScript | npm (provenance) | `npm-v*` | [`release-npm.yaml`](./.github/workflows/release-npm.yaml) |

Each release workflow verifies the tag matches the manifest version before publishing. Python
wheels carry GitHub build-provenance attestations and an SPDX SBOM; verify a downloaded
artifact with `gh attestation verify <file> --repo Blackcat-Informatics/gmeow-gts`.

## Specification & docs

- [`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md) — the authoritative, normative wire-format specification.
- [`docs/gts-reference.md`](./docs/gts-reference.md) — Python reference-implementation guide.
- Per-engine READMEs live in [`rust/`](./rust/README.md), [`python/`](./python/README.md),
  [`go/`](./go/README.md), and [`ts/`](./ts/README.md).

GTS is part of the broader [GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology)
project — a reasoning-centric, OWL 2 DL super-vocabulary — but the format and these engines
stand entirely on their own.

## Contributing

Issues and pull requests are welcome. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the
workflow; before opening a PR, run the relevant engine's tests and `pre-commit run --all-files`.
Please also read the [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md). To report a vulnerability,
follow [`SECURITY.md`](./SECURITY.md) (do not open a public issue).

Contributions are accepted under the project's open licenses (Apache-2.0 OR MIT); see
[`LICENSING.md`](./LICENSING.md) and [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the terms.

## License

Triple-licensed: **MIT OR Apache-2.0 OR proprietary**. Use this software under the terms of
[MIT](./LICENSE-MIT) **or** [Apache-2.0](./LICENSE-APACHE), at your option. A separate
commercial/proprietary license is also available — see [`LICENSING.md`](./LICENSING.md).

Every source file carries an SPDX `MIT OR Apache-2.0` license header.

> Copyright © 2026 Blackcat Informatics® Inc.

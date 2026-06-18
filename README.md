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
inputs.

GTS is ontology-independent. **GTS is the primary distribution method for GMEOW, but GTS does
not depend on GMEOW.** A conformant reader does not need GMEOW vocabulary, OWL reasoning,
domain specific rules, or agent-memory conventions to parse, verify, fold, or transport a GTS
file.

The package family is `gmeow-gts`; the format is GTS. The package name is intentionally
distinctive across ecosystems, while the CLI, import surface, and file extension remain the
short `gts` and `.gts` forms where ecosystem rules permit.

This repository holds **four interoperable engines** (Rust, Python, Go, TypeScript) that all
gate against one frozen, byte-exact conformance corpus, plus the specification that defines
them.

## Table of contents

- [Why GTS?](#why-gts)
- [Use GTS without GMEOW](#use-gts-without-gmeow)
- [Narrow-waist architecture](#narrow-waist-architecture)
- [Applications](#applications)
- [Install](#install)
- [Quick start](#quick-start)
  - [Python](#python)
  - [Rust](#rust)
  - [Go](#go)
  - [TypeScript](#typescript)
- [Command-line interface](#command-line-interface)
- [Engine feature matrix](#engine-feature-matrix)
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
(`.ttl`, `.nq`, DuckDB, SQLite, …), not of GTS. It is a *durable, self-describing
interchange container* — the narrow waist through which graphs and their referenced data travel.

## Use GTS without GMEOW

GMEOW is a primary downstream consumer and reference profile family for GTS artifacts. The
dependency direction is one-way: GMEOW rides on GTS; GTS does not require GMEOW.

A baseline reader needs the GTS wire-format rules, the codec catalog, and RDF term/fold
semantics. It does not need a GMEOW ontology checkout, GMEOW-specific vocabulary, music-domain
profile knowledge, or agent-memory conventions. Domain profiles can add validation rules above
the transport layer, but they do not change the core parse, verification, or fold path.

## Narrow-waist architecture

```text
Applications and profiles
generic graphs | files | evidence | images | media packages | GMEOW | agent memory
|
v
GTS narrow waist
CBOR Sequence segments
deterministic-CBOR headers and frames
BLAKE3 id/prev chains
transform catalog
deterministic fold
opaque-node degradation
|
v
Storage and transport
filesystem | HTTP range | object storage | artifact registries | message buses
```

GTS is the small stable waist. Profiles and applications sit above it; storage and distribution
systems sit below it. See [`docs/positioning.md`](./docs/positioning.md) for the full framing.

## Applications

GTS supports several use cases without making any of them the project frame:

- **Dataset and ontology distribution:** publish a verifiable graph package with the binary
  assets it names.
- **GMEOW distribution:** ship GMEOW ontology packages and profiles as GTS artifacts.
- **Archives and file manifests:** package directory trees with graph-native metadata and
  content-addressed blobs.
- **Evidence and custody chains:** append observations, signatures, and sealed payloads without
  rewriting prior history.
- **Local-first graph synchronization:** concatenate independently produced segments and fold
  the value-union.
- **Agent memory:** model belief revision with suppression frames while preserving the original
  signed history. See Python
  [`gts.examples.agent_memory`](./python/src/gts/examples/agent_memory.py) and Rust
  [`gmeow_gts::examples::agent_memory`](./rust/src/examples/agent_memory.rs).
- **Graph database interchange:** hand the folded graph state to N-Quads, SQLite, DuckDB, Parquet,
  or other transform targets.

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
gmeow-gts = "0.2.0"

# Optional native RDF data-model adapter:
# gmeow-gts = { version = "0.2.0", default-features = false, features = ["rdf"] }
```

```rust
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("package.gts")?;
    // read is total: (data, allow_segments, expected_head) -> Graph (never errors;
    // undecodable frames degrade to opaque nodes surfaced as diagnostics).
    let graph = gmeow_gts::reader::read(&bytes, false, None);
    println!("{}", gmeow_gts::nquads::to_nquads(&graph));
    Ok(())
}
```

The pure-Rust crate (no C toolchain, wasm-friendly) installs the `gts` binary via
`cargo install gmeow-gts`. Rust default features remain empty. `--features rdf`
enables the optional `gmeow_gts::rdf` adapter backed by `oxrdf`'s RDF data-model
crate. `--features oxigraph-adapter` adds `gmeow_gts::oxigraph` helpers and
`Writer::from_store` for native `Graph -> Store` and `Store -> Writer` handoff using
Oxigraph's in-memory store. `--features policy-config` adds JSON `TrustPolicy`
file loading and `gts verify --policy <file>` for release/profile verification;
`--features policy-config-yaml` adds YAML policy files. None of these features
affect default transport users. `--features duckdb` enables the DuckDB/Parquet CLI
exports without adding a Rust dependency; those commands invoke the `duckdb` binary
on `PATH`.

For streaming projections, implement `gmeow_gts::reader::StreamingSink` and call
`gmeow_gts::reader::read_to_sink(&bytes, allow_segments, expected_head, sink)`.
The sink API emits segment-local term, quad, reifier, annotation, suppression,
blob, opaque, signature, diagnostic, segment-head, and streamable-layout events
while returning final diagnostics and segment heads. It adds no crate dependency.
For folded graph consumers, `Graph::into_quads()` and `IntoIterator for Graph`
consume raw quad-id rows without cloning the `Vec<Quad>`, while
`Graph::quad_terms()` lazily resolves ids to borrowed `Term` references.
Call `reader::read_with_options` or `read_to_sink_with_options` with
`ReadOptions::with_content_key` to decrypt `COSE_Encrypt0` payloads while preserving
the same total-read behavior: missing or wrong keys become opaque nodes with `MissingKey`
diagnostics.

Rust writers support transformed and encrypted frames through
`writer::FrameOptions`: apply `gzip`, `zstd`, or `zstd-rsyncable` transforms, attach
recipient metadata, pass explicit signature bytes, or add `Encrypt0Options` for
`COSE_Encrypt0` authoring. This uses the existing codec and COSE modules and keeps the
default dependency set unchanged.

Rust signing works with raw Ed25519 keys or with an unencrypted Ed25519
OpenPGP secret-key block. The OpenPGP helper keeps the same narrow parser used
by `extract-key`; it does not add a full OpenPGP dependency.

```rust
use ed25519_dalek::SigningKey;
use gmeow_gts::writer::Writer;

let seed = [0u8; 32];
let mut raw = Writer::new("evidence");
raw.sign_with(SigningKey::from_bytes(&seed), "did:example:raw-key");

let armored = std::fs::read_to_string("transport.sec.asc")?;
let mut openpgp = Writer::new("evidence");
// `None` uses the OpenPGP v4 fingerprint as the COSE key id.
openpgp.sign_with_openpgp_secret_key(&armored, None)?;
```

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

Runtime support policy: Python >=3.13, Node.js >=22.16.0, and Go 1.26.4 are intentional
manifest floors. Older runtimes are unsupported so the engines can share one current CI and
release matrix and use current standard-library/toolchain behavior without compatibility shims.

## Command-line interface

`cargo install gmeow-gts`, `pip install gmeow-gts`, `npm i -g @blackcatinformatics/gmeow-gts`,
or `go install ...` each install a `gts` binary. The common verb surface is the cross-engine
contract; Python also ships the explicit extensions listed after it. The full API/CLI parity
contract lives in [`docs/GTS-API-CLI-PARITY.md`](./docs/GTS-API-CLI-PARITY.md).

<!-- cli-common:start -->
```text
gts info <file>...            per-segment composition ledger
gts fold <file>               fold to N-Quads on stdout
gts verify <file>... [--key KID:HEXPUB]   verify chains + COSE signatures
gts verify-proof <proof.json>  verify detached MMR proof JSON without the GTS file
gts extract-key <file>        print the embedded transport/verification key
gts ls <file>...              list segment digests, sizes, and media types
gts extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
gts cat -o <out> <file>...    validating composer: refuse degenerate inputs, then concatenate
gts compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
gts pack <dir|file>... -o <out>   package files/directories into a GTS files profile
gts unpack <file> [-C <dir>] [--include-suppressed]   extract a files profile
gts diff <file> <directory>       compare a files profile to a directory
```
<!-- cli-common:end -->

Python/Rust extensions:

```text
gts from-nq <in.nq> [-o <out>]  build a GTS from N-Quads (inverse of fold; '-' = stdin)
gts to-sqlite <file> <out>      export the folded graph to a SQLite database
gts to-duckdb <file> <out>      export to DuckDB (Rust: --features duckdb)
gts to-parquet <file> <dir>     export to Parquet (Rust: --features duckdb)
```

Rust-only proof creation extension:

```text
gts prove <file> <frame-id>      emit detached JSON proof from an index.mmr root
```

Rust-only replication extensions:

```text
gts heads <file>                 emit JSON segment heads and aggregate comparison digest
gts segments <file>              emit JSON segment byte ranges and layout inventory
gts missing --from-head <head> <file>   emit JSON byte ranges needed after a peer head
gts resume --after <frame-id> <file>    emit bytes after a verified frame boundary
```

Python-only extensions:

<!-- cli-python-extensions:start -->
```text
```
<!-- cli-python-extensions:end -->

Exit codes: `0` clean · `1` diagnostics or input refused · `2` usage/IO error.

`verify --key` and `extract-key` are cross-engine (all four `gts` binaries parse the
embedded OpenPGP transport key to the same fingerprint and emojihash, and verify COSE
signatures identically). For example, `gts extract-key` prints a key's identity three ways —
the hex fingerprint for machines and an **emojihash** for humans to compare at a glance:

```console
$ gts extract-key signed.gts
kid:         93F32F9F1439F0FBA266331B6F4732092D747581
fingerprint: 93F3 2F9F 1439 F0FB A266 331B 6F47 3209 2D74 7581
emojihash:   🐷 🦆 🐵 🦋 🍎 🍐 🦊 🐸 🐟 🍒 🍎
-----BEGIN PGP PUBLIC KEY BLOCK-----
…
```

The emojihash (and OpenSSH-style randomart) are also published standalone as the
[`visual-hashing`](https://crates.io/crates/visual-hashing) crate, which this repo's Rust
engine depends on and re-exports as `gmeow_gts::emojihash`.

`from-nq` and the `to-*` relational exports are available in Python and Rust. Python
DuckDB/Parquet exports need `pip install 'gmeow-gts[db]'`; Rust SQLite export shells out to
`sqlite3` by default. Rust DuckDB/Parquet exports are behind the no-dependency Cargo
feature `duckdb` and shell out to the `duckdb` binary. Rust emits relational SQL rows
directly to the runtime tool instead of building a complete SQL script in memory; transformed
inline blobs are decoded only while writing the `blobs` row required by the stable schema.
The CLI parity matrix is checked in CI against the four implemented command dispatch surfaces.

`cat` is raw byte concatenation with validation *added*, transformation *never*: it refuses
dirty inputs, contributes-nothing segments, and compositions whose suppressions hide every
folded quad.

## Engine feature matrix

| Capability | Python | Rust | Go | TypeScript |
|---|---|---|---|---|
| Baseline read/fold/verify | yes | yes | yes | yes |
| Writer | yes | yes | yes | yes |
| Shared conformance corpus | yes | yes | yes | yes |
| COSE signing and verification | yes | yes | yes | yes |
| COSE Encrypt0 helpers | yes | yes | yes | yes |
| Files profile `pack`/`unpack`/`diff` | yes | yes | yes | yes |
| Streamable compaction CLI | yes | yes | yes | yes |
| `from-nq` inverse | yes | yes | no | no |
| Native RDF/store adapter | rdflib extra | `rdf` feature (`oxrdf` data model); `oxigraph-adapter` feature (Oxigraph store) | no | no |
| SQLite/DuckDB/Parquet exports | yes | SQLite default; DuckDB/Parquet with `duckdb` feature | no | no |
| Package registry | PyPI | crates.io | Go module | npm |

The frozen vector corpus remains the compatibility oracle. The matrix summarizes public package
surfaces; it is not a replacement for conformance tests. The command-level contract is maintained
in [`docs/GTS-API-CLI-PARITY.md`](./docs/GTS-API-CLI-PARITY.md).

## The file format in one minute

A GTS file is a **CBOR Sequence** (`application/cbor-seq`) of one or more **segments**.
Published GTS artifacts use `application/vnd.blackcat.gts+cbor-seq`; the `+cbor-seq` suffix
records that the file is a CBOR Sequence, not a single CBOR item. Each segment is a header map
followed by frames; each frame is a `[digest, content]` pair where the digest is the BLAKE3 hash
of the content, and each frame names its predecessor — a content-addressed chain whose head
transitively commits to all history.

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
[`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md), with testable tier and vector-claim rules in
[`docs/GTS-CONFORMANCE.md`](./docs/GTS-CONFORMANCE.md).

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

Conformance tiers, named vector subsets, expected-result fields, diagnostics, and read/verify
modes are defined in [`docs/GTS-CONFORMANCE.md`](./docs/GTS-CONFORMANCE.md).

Current CI-gated conformance status:

| Engine | Baseline Reader | Streaming / Prefix Evidence | Writer | Validating Tool | Profile-Aware Tool |
|---|---|---|---|---|---|
| Rust | `wire-core`, `total-reader`, `graph-fold`, `profile-layout` | `read_to_sink` API plus prefix-fold corpus gate | deterministic compact oracle `25b` | CLI verify diagnostics | files profile pack/unpack/diff in interop |
| Python | corpus oracle and regenerated expected JSON | prefix-fold Python tests | source generator and compact oracle `25b` | CLI verify diagnostics | files profile pack/unpack/diff in interop |
| Go | `wire-core`, `total-reader`, `graph-fold`, `profile-layout` | corpus read gate; fuzz seeded from vectors | writer and compact tests | CLI verify diagnostics | files profile pack/unpack/diff in interop |
| TypeScript | `wire-core`, `total-reader`, `graph-fold`, `profile-layout` | corpus read gate | writer and compact tests | CLI verify diagnostics | files profile pack/unpack/diff in interop |

## Repository layout

```text
gmeow-gts/
├── rust/        # Rust crate `gmeow-gts` + `gts` binary (pure Rust, wasm-friendly)
├── python/      # Python package `gmeow-gts` (module `gts`) + reference corpus generator
├── go/          # Go module go.blackcatinformatics.ca/gts
├── ts/          # TypeScript/npm package @blackcatinformatics/gmeow-gts
├── visual-hashing/ # Standalone `visual-hashing` crate (emojihash + randomart)
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
- [`docs/GTS-CONFORMANCE.md`](./docs/GTS-CONFORMANCE.md) — conformance tiers, vector subsets,
  manifest schema, diagnostics registry, and read/verify modes.
- [`docs/GTS-GOVERNANCE.md`](./docs/GTS-GOVERNANCE.md) — GIP process, registry policies,
  compatibility rules, and the v1.0 release-candidate path.
- [`docs/GTS-API-CLI-PARITY.md`](./docs/GTS-API-CLI-PARITY.md) — cross-language API shape, CLI
  parity matrix, intentional gaps, and drift guard.
- [`docs/GTS-ADVANCED-PRIMITIVES.md`](./docs/GTS-ADVANCED-PRIMITIVES.md) — streaming sink,
  index/MMR/proof, replication, range-fetch, and benchmark contract.
- [`docs/GTS-ECOSYSTEM-INTEGRATIONS.md`](./docs/GTS-ECOSYSTEM-INTEGRATIONS.md) — RDF, data,
  browser, service, and object-store integration contract.
- [`docs/GTS-SECURITY-POLICY.md`](./docs/GTS-SECURITY-POLICY.md) — trust/profile-policy
  separation, nested-GTS budgets, and v1 crypto deferrals.
- [`docs/positioning.md`](./docs/positioning.md) — the project framing, narrow-waist
  architecture, application families, and engine feature matrix.
- [`docs/gts-reference.md`](./docs/gts-reference.md) — Python reference-implementation guide.
- Per-engine READMEs live in [`rust/`](./rust/README.md), [`python/`](./python/README.md),
  [`go/`](./go/README.md), and [`ts/`](./ts/README.md).

GTS is the primary distribution method for
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology), but GTS does not depend on
GMEOW. The format and these engines stand on their own.

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

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<p align="center">
  <a href="https://github.com/Blackcat-Informatics/gmeow-gts">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/gmeow-gts/main/docs/gts-logo.svg" alt="GTS logo" width="128" height="128">
  </a>
</p>

# `gmeow-gts` — Rust GTS Engine

[![CI](https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml/badge.svg)](https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/gmeow-gts.svg)](https://crates.io/crates/gmeow-gts)
[![docs.rs](https://docs.rs/gmeow-gts/badge.svg)](https://docs.rs/gmeow-gts)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md)

> **A whole graph in a single, verifiable file.**

`gmeow-gts` is the Rust implementation of the **Graph Transport Substrate (GTS)** — a
single-file, language-independent transport for an **RDF 1.2** graph (statements *and*
statement-level metadata) together with any content-addressed binary the graph references.
It is one of four interoperable engines (Python, Rust, Go, TypeScript) that gate against the
same frozen, language-neutral conformance corpus.

This crate provides a library and a command-line tool for reading, writing, verifying,
composing, compacting, and projecting GTS files — with optional COSE signing and encryption.
It is designed for systems that need portable, auditable, content-addressed graph packages:
archives, evidence chains, local-first synchronization, dataset distribution, GMEOW packages,
and agent memory.

---

## Table of Contents

- [What is GTS?](#what-is-gts)
- [What this crate provides](#what-this-crate-provides)
- [Installation](#installation)
  - [As a command-line tool](#as-a-command-line-tool)
  - [As a library](#as-a-library)
- [Quick start](#quick-start)
- [Library API](#library-api)
- [Command-line reference](#command-line-reference)
- [The GTS file format](#the-gts-file-format)
- [Developer documentation](#developer-documentation)
- [Project and community](#project-and-community)
- [Contributing](#contributing)
- [Support](#support)
- [License and copyright](#license-and-copyright)

---

## What is GTS?

GTS is the **Graph Transport Substrate**: a CBOR-sequence, append-only, content-addressed
file format for moving RDF 1.2 graphs and their evidence around. A GTS file is composed of
one or more **segments**, each a header followed by frames chained by BLAKE3 content-id.
Key properties:

- **Append-only and composable.** Concatenating valid GTS files (`cat`) yields a valid GTS
  file whose fold is the value-union of the segment graphs.
- **Content-addressed.** Frames and external binaries are referenced by BLAKE3 digests.
- **Signable and verifiable.** Frames can carry detached COSE_Sign1 signatures, and segments
  carry provenance metadata.
- **Language-independent.** The same file can be read and written by the Python, Rust, Go, and
  TypeScript engines.

For the authoritative specification, see [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md).
For the reference-implementation guide, see [`docs/gts-reference.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/gts-reference.md).

GTS is ontology-independent. GTS is the primary distribution method for
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology), but GTS does not depend on
GMEOW. A reader does not need GMEOW vocabulary or tooling to parse, verify, fold, or transport
a GTS file.

---

## What this crate provides

- **`gmeow_gts::reader`** — read a GTS byte slice into a `Graph`, verify chains, detect
  torn appends, and handle opaque/degraded frames.
- **`gmeow_gts::writer`** — build frames and emit full GTS files.
- **`gmeow_gts::cose`** — COSE_Sign1 signing and verification of frame ids (§9.2), plus
  COSE_Encrypt0 AES-256-GCM payload encryption (§9.3).
- **`gmeow_gts::openpgp`** — parse an embedded OpenPGP transport key to its fingerprint.
- **`gmeow_gts::compact`** — compact a streamable GTS segment into a self-contained one.
- **`gmeow_gts::files`** — pack and unpack directory trees using the GTS files profile.
- **`gmeow_gts::nquads`** — project a folded graph to N-Quads.
- **`gmeow_gts::stream`** — stream-vocabulary constants and helpers.
- **`gmeow_gts::emojihash`** — re-export of the [`visual-hashing`](https://crates.io/crates/visual-hashing)
  crate's `emojihash` and `randomart` key fingerprints.
- **`gts` binary** — a CLI for all of the above.

The crate gates against the identical frozen conformance corpus used by the Python, Go, and
TypeScript engines; every engine must fold identical bytes to identical expectations.

---

## Installation

### As a command-line tool

```bash
cargo install gmeow-gts
```

The installed binary is named `gts` for ergonomics, even though the crate is `gmeow-gts`.

### As a library

Add to `Cargo.toml`:

```toml
[dependencies]
gmeow-gts = "0.1.3"
```

---

## Quick start

```bash
# Inspect a GTS file
gts info example.gts

# Fold it to N-Quads
gts fold example.gts > example.nq

# Verify chain integrity
gts verify example.gts

# Compose two valid files
gts cat -o combined.gts a.gts b.gts

# Package a directory
gts pack ./my-dir -o archive.gts

# Extract it elsewhere
gts unpack archive.gts -C ./restore
```

---

## Library API

Read a GTS file and project it to N-Quads. `reader::read` is **total**: it never returns an
error — a damaged or undecodable frame degrades to an opaque node and surfaces as a diagnostic
on the returned `Graph`, so it always yields a usable (possibly partial) fold. The arguments are
`(data, allow_segments, expected_head)`:

```rust
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read("example.gts")?;
    let graph = gmeow_gts::reader::read(&bytes, false, None);
    println!("{}", gmeow_gts::nquads::to_nquads(&graph));
    Ok(())
}
```

Write a minimal graph. A `Writer` is created with a profile (e.g. `"dist"`), then frames are
appended: a `terms` frame interns the RDF terms by append-order id, and a `quads` frame
references them by those ids (the fourth tuple slot is the graph name, `None` for the default
graph):

```rust
use gmeow_gts::model::{Term, TermKind};
use gmeow_gts::writer::Writer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/Cat".into()),
            datatype: None,
            lang: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("http://www.w3.org/2000/01/rdf-schema#label".into()),
            datatype: None,
            lang: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Literal,
            value: Some("Cat".into()),
            datatype: None,
            lang: Some("en".into()),
            reifier: None,
        },
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    std::fs::write("cat.gts", writer.to_bytes())?;
    Ok(())
}
```

For the full API, see [docs.rs/gmeow-gts](https://docs.rs/gmeow-gts).

---

## Command-line reference

```text
gts info <file>...              per-segment composition ledger
gts fold <file>                 fold to N-Quads on stdout
gts verify <file>... [--key KID:HEXPUB]
                                verify chains + COSE signatures; exit 1 on any
gts extract-key <file>          print the embedded transport/verification key:
                                kid, OpenPGP fingerprint, emojihash, armored key
gts ls <file>                   list inline blobs: digest, size, media type
gts extract <file> <digest>     write a single content-addressed blob
gts cat -o <out> <file>...      validating composer: refuse degenerate inputs,
                                then byte-concatenate
gts compact <file> -o <out> --streamable
                                compact into the streamable layout state
gts pack <dir|file>... -o <out> package files/directories into a files profile
gts unpack <file> [-C <dir>]    extract a files profile (refuses path traversal)
gts diff <file> <directory>     compare a files profile to a directory
```

Exit codes:

- `0` — success / clean
- `1` — diagnostics or input refused
- `2` — usage or IO error

`verify --key` and `extract-key` are cross-engine: all four `gts` binaries parse the embedded
OpenPGP transport key to the same fingerprint and emojihash, and verify COSE signatures
identically. The `from-nq` and relational `to-sqlite`/`to-duckdb`/`to-parquet` exports remain
Python-CLI extensions and are **not** part of this Rust binary.

`cat` output is raw byte concatenation: validation is added, transformation never. It
refuses dirty inputs, contributes-nothing segments, and compositions whose suppressions
hide every folded quad.

---

## The GTS file format

A GTS file is a **CBOR Sequence** (RFC 8742, `application/cbor-seq`) of one or more
**segments** — there are no framing bytes between items. Published GTS artifacts use
`application/vnd.blackcat.gts+cbor-seq`; the `+cbor-seq` suffix records that the file is a CBOR
Sequence, not a single CBOR item. Each segment is a **Header** CBOR map (optionally preceded by
the CBOR self-describe tag `55799`, the human-recognizable magic) followed by zero or more
**frames**, each itself a CBOR map. Every frame carries its own `"id"` — the BLAKE3 content-id of
its canonical contents — and names its predecessor's id in `"prev"`, so a segment is a git-style
content-addressed chain whose head transitively commits to all history.

The logical graph is the **fold** of the log: the deterministic replay of the frames into the
in-memory tables (terms, quads, reifier bindings, annotations, blobs, …). The fold is *not* a
hash — concatenating two valid GTS files (`gts cat`) yields a file whose fold is the
**value-union** of the inputs. "Deletion" is additive **suppression**, never physical removal.

Payloads carry a stackable codec chain; an unknown codec or a held-back key degrades a frame to
an **opaque node** rather than failing the read — the reader is total. External binaries are
referenced by content-id and may be omitted without invalidating the file.

For full details, read [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md).

---

## Developer documentation

- [GTS Specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) — the authoritative, normative wire-format specification.
- [GTS Reference Guide](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/gts-reference.md) — the Python reference-implementation guide.
- [`CONTRIBUTING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CONTRIBUTING.md) — development workflow.
- [`CODE_OF_CONDUCT.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CODE_OF_CONDUCT.md) — community standards.
- [`SECURITY.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/SECURITY.md) — vulnerability reporting.
- [`CHANGELOG.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CHANGELOG.md) — release history.

### Building and testing locally

```bash
cd rust
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The conformance tests compare this engine's output against the frozen corpus in
`vectors/`.

---

## Project and community

`gmeow-gts` is developed by [Blackcat Informatics® Inc.](https://blackcatinformatics.ca).
GMEOW is a downstream ontology and tooling suite that uses GTS as a distribution substrate.

Related packages and engines:

- Python: [`python`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/python) (PyPI: `gmeow-gts`)
- Go: [`go`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/go)
- TypeScript/npm: [`ts`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/ts) (`@blackcatinformatics/gmeow-gts`)

---

## Contributing

Contributions are welcome. Please read [`CONTRIBUTING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CONTRIBUTING.md)
for the development workflow and [`CODE_OF_CONDUCT.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CODE_OF_CONDUCT.md)
before opening a PR. To report a vulnerability, follow [`SECURITY.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/SECURITY.md)
(do not open a public issue).

All changes must pass `cargo test`, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings`.

---

## Support

- Open an issue: https://github.com/Blackcat-Informatics/gmeow-gts/issues
- Discussions: https://github.com/Blackcat-Informatics/gmeow-gts/discussions

---

## License and copyright

Copyright © 2026 Blackcat Informatics® Inc.

Triple-licensed: **MIT OR Apache-2.0 OR proprietary**. You may use this crate under
the terms of [MIT](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSE-MIT)
**or** [Apache-2.0](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSE-APACHE),
at your option. A separate commercial/proprietary license is also available — see
[`LICENSING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md).

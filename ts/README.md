<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<p align="center">
  <a href="https://github.com/Blackcat-Informatics/gmeow-gts">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/gmeow-gts/main/docs/gts-logo.svg" alt="GTS logo" width="128" height="128">
  </a>
</p>

# `@blackcatinformatics/gmeow-gts` — TypeScript GTS Engine

[![CI](https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml/badge.svg)](https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml)
[![npm version](https://img.shields.io/npm/v/@blackcatinformatics/gmeow-gts.svg?label=npm)](https://www.npmjs.com/package/@blackcatinformatics/gmeow-gts)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fgmeow--gts-181717.svg)](https://github.com/Blackcat-Informatics/gmeow-gts)

> **A whole graph in a single, verifiable file.**

`@blackcatinformatics/gmeow-gts` is the TypeScript/npm implementation of the **Graph
Transport Substrate (GTS)** — a single-file, language-independent transport for an
**RDF 1.2** graph (statements *and* statement-level metadata) together with any
content-addressed binary the graph references. It is one of four interoperable engines
(Rust, Python, Go, TypeScript) that all gate against the same frozen, language-neutral
conformance corpus — every engine folds identical bytes to identical expectations, so the
files this package writes are read byte-for-byte by the other three (and vice versa).

This package provides a library and a command-line tool for reading, writing, verifying,
composing, compacting, signing, and projecting GTS files. It is designed for systems that need
portable, auditable, content-addressed graph packages: archives, evidence chains, local-first
synchronization, dataset distribution, GMEOW packages, and agent memory.

---

## Table of Contents

- [What is GTS?](#what-is-gts)
- [What this package provides](#what-this-package-provides)
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
  file whose fold is the value-union of the segment graphs — memory grows by append, never
  by rewrite.
- **Content-addressed.** Every frame carries an independent BLAKE3 self-hash and names its
  predecessor, a git-style chain whose head transitively commits to all history; external
  binaries are referenced by the same digests.
- **Signable and encryptable.** COSE_Sign1 signatures (RFC 9052) and COSE_Encrypt0
  (AES-256-GCM) are optional, layered, and algorithm-agile.
- **Interoperable.** The same file is read and written by all four engines (Rust, Python,
  Go, TypeScript).

For the authoritative specification, see [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md).
For the reference-implementation guide, see [`docs/gts-reference.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/gts-reference.md).

GTS is ontology-independent. GTS is the primary distribution method for
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology), but GTS does not depend on
GMEOW. A reader does not need GMEOW vocabulary or tooling to parse, verify, fold, or transport
a GTS file.

---

## What this package provides

- **`Read`** — read a GTS byte buffer into a `Graph`, verify chains, detect torn appends,
  and handle opaque/degraded frames.
- **`Writer`** — build segments and full GTS files frame by frame; optionally COSE-sign
  every frame with `signWith`.
- **`toNQuads`** — project a folded `Graph` to N-Quads.
- **`pack` / `unpack` / `diff`** — pack and unpack directory trees using the GTS files profile.
- **`compactStreamable`** — compact a streamable GTS segment into a self-contained one.
- **`cose`** — COSE_Sign1 signing/verification and COSE_Encrypt0 (AES-256-GCM) helpers.
- **`emojihash`** — render a key/digest as a stable, eyeball-comparable emoji string.
- **`stream`**, **`codec`**, **`wire`** — stream-vocabulary constants, the codec catalog,
  and low-level CBOR helpers.
- **`gts` binary** — a CLI for reading, verifying, composing, compacting, and packing.

The package gates against the identical frozen conformance corpus used by the Rust, Python,
and Go engines; every engine must fold identical bytes to identical expectations.

---

## Installation

### As a command-line tool

```bash
npx @blackcatinformatics/gmeow-gts info example.gts
```

Or install globally:

```bash
npm install -g @blackcatinformatics/gmeow-gts
gts info example.gts
```

The installed binary is named `gts` for ergonomics, even though the package is
`@blackcatinformatics/gmeow-gts`.

### As a library

```bash
npm install @blackcatinformatics/gmeow-gts
```

Requires Node.js ≥ 22.16.0.

---

## Quick start

```bash
# Inspect a GTS file
npx @blackcatinformatics/gmeow-gts info example.gts

# Fold it to N-Quads
npx @blackcatinformatics/gmeow-gts fold example.gts > example.nq

# Verify chain integrity
npx @blackcatinformatics/gmeow-gts verify example.gts

# Compose two valid files
npx @blackcatinformatics/gmeow-gts cat -o combined.gts a.gts b.gts

# Package a directory
npx @blackcatinformatics/gmeow-gts pack ./my-dir -o archive.gts

# Extract it elsewhere
npx @blackcatinformatics/gmeow-gts unpack archive.gts -C ./restore
```

---

## Library API

Read a GTS file, verify its chain, fold it, and project to N-Quads. `Read(bytes,
allowSegments)` returns a `Graph`; pass `true` to fold multi-segment files, `false` to read a
single segment. `toNQuads` is a free function:

```typescript
import { Read, toNQuads } from "@blackcatinformatics/gmeow-gts";
import { readFileSync } from "node:fs";

const graph = Read(readFileSync("example.gts"), false);
console.log(toNQuads(graph));
```

Write a minimal graph. A `Writer` interns terms in append order — their indices are the
term-ids that quads reference. `Term` is a plain object literal (`{ kind, value, … }`), and a
`Quad` references terms by id (`{ s, p, o }`, with an optional `g` graph slot); `toBytes()`
returns the finished GTS file:

```typescript
import { Writer, TermKind } from "@blackcatinformatics/gmeow-gts";
import type { Term, Quad } from "@blackcatinformatics/gmeow-gts";
import { writeFileSync } from "node:fs";

const w = new Writer("generic");

// Term 0, 1, 2 (interned in append order).
w.addTerms([
  { kind: TermKind.Iri, value: "https://example.org/Cat" },
  { kind: TermKind.Iri, value: "http://www.w3.org/2000/01/rdf-schema#label" },
  { kind: TermKind.Literal, value: "Cat", lang: "en" },
]);

// One quad in the default graph: (term 0, term 1, term 2).
w.addQuads([{ s: 0, p: 1, o: 2 }]);

writeFileSync("cat.gts", w.toBytes());
```

The `Writer` also exposes `addMeta(...)` for file-level metadata and `signWith(key, kid)` to
COSE_Sign1-sign every subsequently appended frame. For the full API, explore the TypeScript
declarations in `dist/index.d.ts` after building, or browse the source under
[`src/`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/ts/src).

### Browser streaming API

Browser bundlers can import the browser-safe surface directly:

```typescript
import {
  foldStream,
  readStream,
  toNQuads,
  type BrowserFoldEvent,
} from "@blackcatinformatics/gmeow-gts/browser";

const response = await fetch("/artifacts/example.gts");
const events: BrowserFoldEvent[] = [];
const result = await foldStream(response.body!, {
  allowSegments: true,
  onEvent(event) {
    events.push(event);
  },
});

console.log(toNQuads(result.graph));
```

`foldStream(stream, options)` consumes a `ReadableStream<Uint8Array>` and emits progressive
term, quad, blob, signature, diagnostic, segment-head, and streamable-layout events as CBOR
items arrive. `readStream(stream, options)` is a convenience wrapper that returns only the
final `Graph`.

The browser export also accepts a `BrowserKeyProvider` for WebCrypto-backed COSE verification
and decryption:

```typescript
const graph = await readStream(response.body!, {
  keys: {
    verificationKey: (kid) => lookupEd25519PublicKey(kid),
    contentKey: (kid) => lookupAes256GcmContentKey(kid),
  },
});
```

The browser path is intentionally narrower than the Node root export. It does not expose the
CLI, filesystem `pack`/`unpack`/`diff` helpers, or other Node-only behavior. It may claim the
`GTS Streaming Reader` surface for `@blackcatinformatics/gmeow-gts/browser` when the release
claim names the corpus commit and browser streaming test harness used. The Node `Read(bytes,
...)` API remains a materializing reader, not the streaming surface.

---

## Command-line reference

```text
gts info <file>...                 per-segment composition ledger
gts fold <file>                    fold to N-Quads on stdout
gts verify <file>... [--key KID:HEXPUB]
                                   verify chains + COSE signatures; exit 1 on any diagnostic
gts extract-key <file>             print the embedded transport key: kid, OpenPGP
                                   fingerprint, emojihash, and armored public key
gts ls <file>                      list inline blobs: digest, size, declared media type
gts extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                                   extract one content-addressed blob by digest
gts cat -o <out> <file>...         validating composer: refuse degenerate inputs,
                                   then byte-concatenate
gts compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                                   rewrite into the streamable layout state
gts pack <dir|file>... -o <out>    package files/directories into a GTS files profile
gts unpack <file> [-C <dir>] [--include-suppressed]
                                   extract a files profile (refuses path traversal)
gts diff <file> <directory>        compare a files profile to a directory by digest
```

Exit codes:

- `0` — success / clean
- `1` — diagnostics or input refused
- `2` — usage or IO error

`verify --key` and `extract-key` are cross-engine: all four `gts` binaries parse the
embedded OpenPGP transport key to the same fingerprint and emojihash, and verify COSE
signatures identically. (The `from-nq` inverse and the `to-sqlite`/`to-duckdb`/`to-parquet`
relational exports are Python-CLI extensions and are not part of this engine's binary.)

`cat` output is raw byte concatenation: validation is added, transformation never. It
refuses dirty inputs, contributes-nothing segments, and compositions whose suppressions
hide every folded quad.

---

## The GTS file format

A GTS file is a **CBOR Sequence** (`application/cbor-seq`, RFC 8742) of one or more
**segments**. Published GTS artifacts use `application/vnd.blackcat.gts+cbor-seq`; the
`+cbor-seq` suffix records that the file is a CBOR Sequence, not a single CBOR item. Each
segment is a **header** data item (the chain genesis, magic `"gts": "GTS1"`, spec version,
profile, and codec catalog) followed by **frames**, each a CBOR map. Every frame carries its
own `"id"` — the BLAKE3-256 self-hash of its content — and a `"prev"` naming the previous
item's `"id"`, forming a git-style content-addressed chain whose head transitively commits to
all history.

The logical graph is the **fold**: a replay of the log that accumulates terms, quads, and
metadata. It is *not* a hash of the frames — it is the value-union of everything the log
asserts (with suppression frames applied as an additive display overlay). Concatenating two
valid GTS files yields one whose fold is the value-union of the inputs. Payloads carry a
stackable codec chain; a frame whose codec is unknown or whose key is held back degrades to
an **opaque node** rather than failing the read, and external binaries referenced by digest
can be absent without invalidating the file.

For full details, read [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md).

---

## Developer documentation

- [GTS Specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) — the authoritative, normative wire-format spec.
- [GTS Reference Guide](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/gts-reference.md) — the reference-implementation guide.
- [GTS Ecosystem Integration Contract](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-ECOSYSTEM-INTEGRATIONS.md) — browser, range-fetch, and WebCrypto integration status.
- [`CONTRIBUTING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CONTRIBUTING.md) — development workflow and PR checklist.
- [`CODE_OF_CONDUCT.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CODE_OF_CONDUCT.md) — community expectations.
- [`SECURITY.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/SECURITY.md) — how to report a vulnerability.

### Building and testing locally

```bash
cd ts
npm ci
npm run build
npm run lint
npm test
```

The package ships as ES modules with TypeScript declarations and requires Node.js
≥ 22.16.0. The conformance tests compare this engine's output against the frozen corpus in
`vectors/`.

---

## Project and community

`@blackcatinformatics/gmeow-gts` is developed by
[Blackcat Informatics® Inc.](https://blackcatinformatics.ca). GMEOW is a downstream ontology
and tooling suite that uses GTS as a distribution substrate.

Related packages and engines (all four interoperate against the same corpus):

- Rust: [`rust`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/rust) (crates.io: `gmeow-gts`)
- Python: [`python`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/python) (PyPI: `gmeow-gts`)
- Go: [`go`](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/go) (`go.blackcatinformatics.ca/gts`)

---

## Contributing

Contributions are welcome. Please read
[`CONTRIBUTING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CONTRIBUTING.md)
for the development workflow and the
[`CODE_OF_CONDUCT.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CODE_OF_CONDUCT.md).
To report a vulnerability, follow
[`SECURITY.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/SECURITY.md)
(do not open a public issue).

All changes must pass `npm run lint` and `npm test`.

---

## Support

- Open an issue: https://github.com/Blackcat-Informatics/gmeow-gts/issues
- Discussions: https://github.com/Blackcat-Informatics/gmeow-gts/discussions

---

## License and copyright

Copyright © 2026 Blackcat Informatics® Inc.

Triple-licensed: **MIT OR Apache-2.0 OR proprietary**. You may use this package under
the terms of [MIT](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSE-MIT)
**or** [Apache-2.0](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSE-APACHE),
at your option. A separate commercial/proprietary license is also available — see
[`LICENSING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md).

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# GTS — Go engine

[![Go Reference](https://pkg.go.dev/badge/go.blackcatinformatics.ca/gts.svg)](https://pkg.go.dev/go.blackcatinformatics.ca/gts)
[![CI](https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml/badge.svg)](https://github.com/Blackcat-Informatics/gmeow-gts/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md)

The Go implementation of the **Graph Transport Substrate (GTS)** — a single-file,
content-addressed, append-only transport for RDF 1.2 graphs and the binaries they
reference. This module ships the baseline reader, a files-profile writer, and the
`gts` command-line tool.

GTS encodes a graph as an **append-only log of CBOR frames**; the logical graph is the
*fold* (replay) of the log. Concatenating two valid GTS files (`cat`) yields a valid
GTS file whose fold is the value-union of the inputs — memory grows by append, never by
rewrite. Four properties define the format
([full spec](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md)):

1. **CBOR all the way down** (RFC 8949) — one IETF-standardised binary encoding; a reader
   needs only a CBOR library.
2. **A durable transform catalog** — each frame's payload carries a stackable codec chain
   (`identity`, `gzip`, `zstd`, `cose-encrypt`, …).
3. **Integrity by construction** — every frame carries an independent **BLAKE3 self-hash**
   and names its predecessor, a git-style content-addressed chain; the head id transitively
   commits to all history.
4. **Recursive composition (matryoshka)** — a reversed payload is just bytes, and a GTS file
   is just bytes, so a payload MAY itself be a complete signed GTS.

This is one of **four interoperable engines** (Rust, Python, Go, TypeScript) that all gate
against one frozen, byte-exact conformance corpus — every engine folds identical bytes to
identical N-Quads. See the [project README](https://github.com/Blackcat-Informatics/gmeow-gts#readme)
for the cross-engine picture.

## Install

```bash
go install go.blackcatinformatics.ca/gts/cmd/gts@latest
```

The module path is `go.blackcatinformatics.ca/gts`. Releases are tagged in the
`gmeow-gts` repository, e.g. `go-v0.1.0`.

## Library quick start

Read bytes into a `*model.Graph`, then project to N-Quads:

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

`reader.Read` never panics on bad input: unknown codecs or held-back keys degrade a frame
to an opaque node, and any problems surface on `g.Diagnostics` rather than as an error
return. Pass `allowSegments=true` to fold a multi-segment (concatenated) file, and a
non-nil `expectedHead` to assert the file's head digest.

Produce a minimal graph with the writer (a files-profile writer; `New("")` writes a plain
graph with no profile):

```go
package main

import (
    "os"

    "go.blackcatinformatics.ca/gts/model"
    "go.blackcatinformatics.ca/gts/writer"
)

func main() {
    w := writer.New("") // or writer.New("files") for the files profile
    w.AddTerms([]model.Term{
        {Kind: model.Iri, Value: "https://example.org/Cat"},
        {Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"},
        {Kind: model.Literal, Value: "Cat", Lang: "en"},
    })
    w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}}) // term-ids into the term table
    _ = os.WriteFile("cat.gts", w.ToBytes(), 0o644)
}
```

The writer also exposes `AddBlob`, `AddReifies`, `AddAnnot`, `AddMeta`, `AddSuppress`,
`AddIndex`, and `SignWith(priv ed25519.PrivateKey, kid string)` for COSE_Sign1 signing.

## Command-line interface

`go install …` installs a `gts` binary whose verbs match the cross-engine surface
(§14.1 of the spec — the engines are CLI-compatible by conformance test):

```text
gts info <file>...            per-segment composition ledger
gts fold <file>               fold to N-Quads on stdout
gts verify <file>... [--key KID:HEXPUB]   verify chains + COSE signatures
gts extract-key <file>        print the embedded transport key: kid, OpenPGP
                              fingerprint, emojihash, armored public key
gts ls <file>                 list inline blobs: digest, size, declared media type
gts extract <file> <digest>   write a single content-addressed blob
gts cat -o <out> <file>...    validating composer: refuse degenerate inputs, then concatenate
gts compact <file> -o <out> --streamable   rewrite into the streamable layout state
gts pack <dir|file>... -o <out>            package files/directories into a files profile
gts unpack <archive> [-C dir] extract a files profile (refuses path traversal)
gts diff <archive> <dir>      compare a files profile to a directory by digest
```

Exit codes: `0` clean · `1` diagnostics found or input refused · `2` usage/IO error.

`cat` is raw byte concatenation with validation *added*, transformation *never*: it
refuses dirty inputs, contributes-nothing segments, and compositions whose suppressions
hide every folded quad.

> The `from-nq` builder and the `to-sqlite` / `to-duckdb` / `to-parquet` relational
> exports are Python-CLI extensions and are **not** provided by the Go binary.

## Signing & encryption

The Go engine implements the optional COSE layer (RFC 9052) and OpenPGP transport-key
handling:

- **COSE_Sign1 signatures** (§9.2) — `writer.SignWith` signs frame ids with Ed25519;
  `gts verify --key KID:HEXPUB` (or `cose.VerifySignatures`) verifies them. The `--key`
  value is a `kid:hex-ed25519-public-key` pair.
- **COSE_Encrypt0** (§9.3) — `cose.Encrypt0` / `cose.Decrypt0` provide AES-256-GCM
  payload confidentiality; an undecryptable frame degrades to an opaque node.
- **Transport key** — `gts extract-key` parses the embedded OpenPGP transport key and
  prints its kid, fingerprint, **emojihash** (a stable visual fingerprint), and armored
  public block. `verify --key` and `extract-key` are cross-engine: all four `gts`
  binaries resolve the same fingerprint and emojihash and verify COSE signatures
  identically.

## Binary releases

Pre-built binaries for Linux, macOS, and Windows are published to GitHub Releases when a
`go-v*` tag is pushed. See the
[releases page](https://github.com/Blackcat-Informatics/gmeow-gts/releases).

## Build and test

```bash
cd go
go build ./...
go vet ./...
golangci-lint run ./...
go test ./...        # unit + conformance against ../vectors/
```

## Layout

- `cmd/gts` — `gts` CLI
- `reader` / `writer` — baseline GTS reader and files-profile writer
- `files` / `wire` / `compact` / `nquads` — format plumbing
- `model` / `stream` / `codec` — core data types and codecs
- `cose` / `openpgp` / `emojihash` — COSE_Sign1/Encrypt0 and transport-key handling

## Specification & docs

- [`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md)
  — the authoritative, normative wire-format specification.
- [`docs/gts-reference.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/gts-reference.md)
  — reference-implementation guide.
- API reference: [pkg.go.dev/go.blackcatinformatics.ca/gts](https://pkg.go.dev/go.blackcatinformatics.ca/gts).

## Contributing

Issues and pull requests are welcome. See
[`CONTRIBUTING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CONTRIBUTING.md)
for the workflow; before opening a PR, run `go test ./...` and `pre-commit run --all-files`.
Please also read the
[`CODE_OF_CONDUCT.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/CODE_OF_CONDUCT.md).
To report a vulnerability, follow
[`SECURITY.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/SECURITY.md)
(do not open a public issue).

## License

Triple-licensed: **MIT OR Apache-2.0 OR proprietary** — use under MIT or Apache-2.0
at your option; a proprietary license is also available (see
[`LICENSING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md)).
© Blackcat Informatics® Inc.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS — Go engine

The [GTS v2 specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) defines the current format. The Go implementation is one of **six interoperable full engines** (Rust, Python, Go, TypeScript, Smalltalk/Pharo, Kotlin/JVM) in the released family. The interoperability suite exercises all six `gts` binaries.

## Install

```bash
go install go.blackcatinformatics.ca/gts/cmd/gts@v1.0.0-rc.1
```

The module path is `go.blackcatinformatics.ca/gts`. Releases are tagged in the
`gmeow-gts` repository with the module subdirectory prefix, e.g. `go/v1.0.0`.

## Binary releases

Pre-built 1.0-line binaries for Linux, macOS, and Windows are published to GitHub Releases
when a `go/v*` tag is pushed. See the
[releases page](https://github.com/Blackcat-Informatics/gmeow-gts/releases).

## Build and test

```bash
cd go
go build ./...
go vet ./...
golangci-lint run ./...
go test ./...        # unit + conformance against ../vectors/
```

## Format and conformance

`gts from-nq` writes committed v2 RDF segments. `gts fold` reads supported v2 RDF
terms and emits N-Quads. `gts verify` checks v2 frame envelopes, links, decoded
payload integrity, offsets and locator claims, and each commit's count and root. It reports committed and
uncommitted ordinary frame counts for each segment. The wire reader resumes at
valid resync openers after framing loss and reports the skipped bytes.

RDF and archive semantics, index payload claims, parity checks and repair,
cross-segment lineage, and signature authentication require separate support.
The `--key` option is unavailable for v2 files.

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

## License

See the repository’s [licensing terms](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md) for the package and its documentation.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# GTS Go engine

The Go implementation of the Graph Transport Substrate (GTS) baseline reader and
files-profile writer.

## Install

```bash
go install go.blackcatinformatics.ca/gts/cmd/gts@latest
```

The module path is `go.blackcatinformatics.ca/gts`. Releases are tagged in the
`gmeow-gts` repository, e.g. `go-v0.1.0`.

## Binary releases

Pre-built binaries for Linux, macOS, and Windows are published to GitHub Releases
when a `go-v*` tag is pushed. See the
[releases page](https://github.com/Blackcat-Informatics/gmeow-gts/releases).

## Build and test

```bash
cd go
go build ./...
go vet ./...
golangci-lint run ./...
go test ./...
```

## Layout

- `cmd/gts` — `gts` CLI
- `reader` / `writer` — baseline GTS reader and files-profile writer
- `files` / `wire` / `compact` / `nquads` — format plumbing
- `model` / `stream` / `codec` — core data types and codecs

## License

Triple-licensed: **MIT OR Apache-2.0 OR proprietary** — use under MIT or Apache-2.0
at your option; a proprietary license is also available (see
[`LICENSING.md`](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/LICENSING.md)).
© Blackcat Informatics® Inc.

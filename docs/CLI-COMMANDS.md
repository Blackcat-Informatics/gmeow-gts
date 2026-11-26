<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS command availability

The table lists commands exposed by the six in-repository engines. See each
engine's documentation for supported format generations and options. The
[GTS v2 specification](GTS-SPEC.md) defines the current wire format and reader
requirements.

<!-- cli-parity-matrix:start -->
| verb | Python | Rust | Go | TypeScript | Smalltalk | Kotlin | status |
|---|---|---|---|---|---|---|---|
| `info` | yes | yes | yes | yes | yes | yes | common |
| `fold` | yes | yes | yes | yes | yes | yes | common |
| `verify` | yes | yes | yes | yes | yes | yes | common |
| `extract-key` | yes | yes | yes | yes | yes | yes | common |
| `ls` | yes | yes | yes | yes | yes | yes | common |
| `extract` | yes | yes | yes | yes | yes | yes | common |
| `cat` | yes | yes | yes | yes | yes | yes | common |
| `compact` | yes | yes | yes | yes | yes | yes | common |
| `pack` | yes | yes | yes | yes | yes | yes | common |
| `unpack` | yes | yes | yes | yes | yes | yes | common |
| `diff` | yes | yes | yes | yes | yes | yes | common |
| `from-nq` | yes | yes | yes | yes | yes | yes | common |
| `to-trig` | yes | yes | yes | no | no | no | RDF transform |
| `from-trig` | yes | yes | yes | no | no | no | RDF transform |
| `to-nt` | no | yes | yes | no | no | no | RDF text codec |
| `from-nt` | no | yes | yes | no | no | no | RDF text codec |
| `to-rdfxml` | no | yes | yes | no | no | no | RDF text codec |
| `from-rdfxml` | no | yes | yes | no | no | no | RDF text codec |
| `to-turtle` | no | yes | yes | no | no | no | RDF transform |
| `from-turtle` | no | yes | yes | no | no | no | RDF transform |
| `to-yaml-ld` | no | yes | no | no | no | no | RDF transform |
| `from-yaml-ld` | no | yes | no | no | no | no | RDF transform |
| `to-okf` | no | yes | no | no | no | no | OKF profile |
| `from-okf` | no | yes | no | no | no | no | OKF profile |
| `to-tar` | no | yes | no | no | no | no | tar bridge |
| `from-tar` | no | yes | no | no | no | no | tar bridge |
| `tar` | no | yes | no | no | no | no | tar bridge |
| `to-sqlite` | yes | yes | no | no | no | no | table export |
| `to-duckdb` | yes | yes | no | no | no | no | table export |
| `to-parquet` | yes | yes | no | no | no | no | table export |
| `prove` | no | yes | no | no | no | no | proof creation |
| `dump` | no | yes | no | no | no | no | inspection export |
| `verify-proof` | yes | yes | yes | yes | yes | yes | common |
| `heads` | yes | yes | yes | yes | yes | yes | common |
| `segments` | yes | yes | yes | yes | yes | yes | common |
| `missing` | yes | yes | yes | yes | yes | yes | common |
| `resume` | yes | yes | yes | yes | yes | yes | common |
| `repair` | no | yes | no | no | yes | no | v2 parity repair |
<!-- cli-parity-matrix:end -->

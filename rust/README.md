<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `gmeow-gts` — Rust GTS Engine

The [GTS v2 specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) defines the current format. This package predates v2.

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
gmeow-gts = "1.0.0-rc.1"
```

For RDF library integration, select the crate feature explicitly:

```toml
gmeow-gts = { version = "1.0.0-rc.1", default-features = false, features = ["rdf"] }
```

## Format and conformance

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

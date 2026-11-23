<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# `@blackcatinformatics/gmeow-gts` — TypeScript GTS Engine

The [GTS v2 specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) defines the current format. This package predates v2.

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

## Format and conformance

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

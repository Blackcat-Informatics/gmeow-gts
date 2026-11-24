<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Kotlin/JVM Engine

The [GTS v2 specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) defines the current format.

## Build and test

Run `gradle test` from the `kotlin/` directory with a local JDK and Gradle installation.

## Format and conformance

The `gts info` command detects GTS v2 from the first complete item and reports the
observed item boundaries. Its v2 report checks opener and frame grammar,
frame links, stored checks, decoded payload integrity for plain and Zstd
storage, and offsets and locator claims.

The `gts verify` command also checks commit counts and roots for registry
version 1 and reports each segment's committed prefix and uncommitted tail.
RDF projection, index payload claims, signature verification, and resync
recovery require separate v2 support.

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

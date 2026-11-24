<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS - Smalltalk/Pharo engine

The [GTS v2 specification](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md) defines the current format.

## Runtime

The development and CI runtime is pinned to:

- `ghcr.io/ba-st/pharo:v13.1.2`
- Pharo image `13.1`
- Pharo VM `10.3.9`

The Dockerfile also provisions `libzstd`, `libsodium`, and pinned GTS-owned native
shims for BLAKE3 and libsodium calls. The Smalltalk package binds BLAKE3, zstd,
and libsodium through Pharo's Unified FFI surface.

Build the local runtime image:

```bash
docker build -t gmeow-gts-smalltalk smalltalk
```

Run the SUnit conformance and unit tests:

```bash
docker run --rm -v "$PWD:/workspace" -w /workspace gmeow-gts-smalltalk \
  sh /workspace/smalltalk/scripts/run-tests.sh
```

The `.smalltalk.ston` file is the smalltalkCI entry point for GitHub Actions.

## Format and conformance

`gts info` detects v2 from the first complete item, reports item byte
ranges, and validates opener and frame envelope grammar. Frame integrity,
RDF folding, and recovery require additional v2 support.

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

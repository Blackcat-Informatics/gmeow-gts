<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# GTS - Smalltalk/Pharo engine

This directory contains the Pharo implementation of the Graph Transport Substrate.
It is currently a Phase 0 engine bootstrap: Tonel packages, a Metacello baseline,
deterministic-CBOR primitives, a native BLAKE3 FFI spike, and SUnit tests that prove
canonical encoding and hashing rules by reproducing the committed `01-minimal.gts`
vector byte-for-byte.

The parity target is Go-equal support: baseline/full reader, deterministic writer,
COSE Sign1/Encrypt0, files profile, MMR, CLI verbs, and `scripts/interop.sh`
participation. Until those gates are implemented, this engine is intentionally
not listed in the cross-engine byte-identity interop matrix.

## Runtime

The development and CI runtime is pinned to:

- `ghcr.io/ba-st/pharo:v13.1.2`
- Pharo image `13.1`
- Pharo VM `10.3.9`

The Dockerfile also provisions `libzstd`, `libsodium`, and a pinned build of the
official BLAKE3 C implementation with a tiny GTS-owned one-shot hash shim. The
Smalltalk package binds that shim through Pharo's Unified FFI surface.

Build the local runtime image:

```bash
docker build -t gmeow-gts-smalltalk smalltalk
```

Run the current SUnit tests:

```bash
docker run --rm -v "$PWD:/workspace" -w /workspace gmeow-gts-smalltalk \
  sh /workspace/smalltalk/scripts/run-tests.sh
```

The `.smalltalk.ston` file is the smalltalkCI entry point for GitHub Actions.

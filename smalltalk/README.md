<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# GTS - Smalltalk/Pharo engine

This directory contains the Pharo implementation of the Graph Transport Substrate.
It is a source engine delivered as Tonel packages with a Metacello baseline and a
pinned Docker CLI runtime. The implementation covers the current Go-equal/common
surface: CBOR Sequence reading, deterministic writing, top-level corpus summaries,
native BLAKE3/zstd/libsodium, COSE Sign1 verification/signing helpers, COSE Encrypt0
helpers, MMR proof verification, OpenPGP `extract-key`, `from-nq`, streamable
compaction, files-profile `pack`/`unpack`/`diff`, replication verbs, and the common
`gts` CLI verbs.

The Smalltalk engine participates in `scripts/interop.sh` with Rust, Python, Go, and
TypeScript. Rust-only extension verbs such as TriG, OKF, tar, dump, and relational
exports remain explicit future parity work.

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

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

The Dockerfile provisions `libzstd`, `libsodium`, and OpenSSL 3.5.1 or later.
The Smalltalk package uses Pharo Unified FFI for compression, hashing, and
native cryptography.

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

`gts info` detects v2 from the first complete item, reports item byte ranges,
validates opener and frame envelope grammar, and checks frame links,
stored and decoded payload integrity, and offsets and locator claims.
Recovery requires additional v2 support.

`gts verify` also checks v2 commit counts and Merkle range roots for registry
version 1. It reports each segment's committed prefix and uncommitted tail.
It validates index payloads and compares their frame count, head, type runs,
and range root against the observed prefix. It compares each pack-member
directory when all covered packs can be read. Index graph directories, fold
statistics and content digests require additional support.

Verify Ed25519 signatures with `--key KEY_ID:PUBLIC_KEY` and composite
ML-DSA-65 and Ed25519 signatures with `--key KEY_ID:mldsa65_ed25519:PUBLIC_KEY`.
Supply the GTS v2 key identifier and the raw public key in hexadecimal.
Invalid signatures fail verification. Signatures without a held key are
reported as unverified. Authentication coverage requires additional support.

`gts from-nq` writes a committed v2 segment. It preserves named graphs,
language direction, and nested object triples, and uses value references
for large literals.

`gts fold` projects registry version 1 v2 RDF into N-Quads. It reads inline
and value-referenced text, preserves segment-scoped blank nodes and nested
object triples, and projects reifier bindings and annotations as RDF quads.
It reports unreadable inputs and unsupported content frames as errors.
The fold projects typed-value term blocks, including referenced octets, to their canonical RDF literals.
The v2 command writes visible quads by default. Use `gts fold FILE --whole` to
include hidden quads. Suppression applies to each source line before the fold
deduplicates the projected quads.

Use the [v2 reader requirements](https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/v2/P7.md) and the [v2 conformance corpus](https://github.com/Blackcat-Informatics/gmeow-gts/tree/main/vectors/v2) when implementing the current format.

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
When the item walk encounters incomplete or undelimitable bytes, or the first
item fails the framing test, it scans for a resync item and resumes at the
first candidate whose offset and identifier match. The ledger reports the
gap and the recovered opener. A complete item also triggers a scan when it
fails item classification, violates opener, frame-envelope or closing-record
grammar, or has a broken frame link after checking its immediate successor.
The reader checks parity-frame anchors, digest tables, placement, and shard
digests. It distinguishes damaged shards from stale parity over changed
bytes whose items still pass verification. It reports any unprotected
tail of a segment that carries parity. It reconstructs missing shards,
rechecks the affected frames, and reports accepted and failed repairs.
Use `gts repair INPUT -o OUTPUT` to write
a new copy with accepted repairs. The command reports the parity frame offset
and the shard count or failure reason. It creates a copy only when at least
one span passes rechecking. The command returns status 1 when a repair fails
or the item walk stops before the end of the input. If framing is lost,
the reader looks for a usable parity frame or resync item and
continues at the first candidate that passes its checks.

Use `GtsV2Wire>>typedWireFindings` to consume generation, framing, opener,
envelope, link, stored check, payload, capacity, and parity findings in
their P2 record form. The report provides file, segment, and frame
findings at their P7 locations and produces the P7 file record when the
input has no segment.
Use `segmentRangeAt:` for a segment's input offset and length, and
`fileEnd` for the P2 file-end code, including stops and commit status.
Use `wireFoldStateAt:` for fold states determined by the wire checks. It
returns `nil` when the frame needs further content processing.
`wireLeftOut` counts exclusions determined so far, including opaque frames
and items, damaged regions, torn tails, and items under unreadable openers.
Use `openerFieldsAt:` for the opener, header identifier, registry version,
hash algorithm, and profile fields by segment. On resync openers the report
checks the preceding bytes named by `past` using the whole-read access rules
and a separate byte count. It counts inapplicable advisory marks.
Use `frameWireFieldsAt:` for each frame's position, byte range, type, link,
payload, and stored state. Use `anchoredFramesAt:` and `transparentFramesAt:`
for the anchored prefix count and transparent frame positions by segment.
For registry version 1, it includes commit and structural index findings
and exposes the committed frame count through `committedFramesAt:`.
It checks declared canonical order for quads, reifier bindings, and
annotations within a frame and across consecutive frames of its type.
The report checks the content record of each eligible `terms` frame. Valid
runs reserve their term IDs even when the frame cannot be folded.
Pass P6 held-key records and trusted key identifiers to
`typedWireFindingsHeld:trusted:`. The report exposes the ordered sets
through `verificationInputs`, signature records through `signaturesAt:`
by frame byte offset, and the authenticity code and authenticated frame
count through `authenticationAt:` by segment ordinal.

`gts verify` also checks v2 commit counts and Merkle range roots for registry
version 1. It reports each segment's committed prefix and uncommitted tail.
It checks commit roots against the observed ordinary prefix and starts a new
signature coverage run at a detached frame.
It validates index payloads and compares their frame count, head, type runs,
and range root against the observed prefix. It compares graph and pack-member
directories when their covered frames can be read. Graph directory comparison
also requires written term identities for multi-row annotation frames.
Index fold statistics and content digests require additional support.

Verify Ed25519 signatures with `--key KEY_ID:PUBLIC_KEY` and composite
ML-DSA-65 and Ed25519 signatures with `--key KEY_ID:mldsa65_ed25519:PUBLIC_KEY`.
Supply the GTS v2 key identifier and the raw public key in hexadecimal.
Invalid signatures fail verification. Signatures without a held key are
reported as unverified. Use `--trust KEY_ID` to trust a v2 key explicitly.
Use `--key` separately to supply its public bytes.

The verifier emits JSON objects alongside its text diagnostics. Signature
objects give the half-open ordinary-frame interval `coverage_first` to
`coverage_end` and the boolean `authenticated`. Segment objects give
`authenticated_frames` and the P2 `authenticity` code. A valid trusted
commit can authenticate earlier linked frames across a break. Payload integrity
is reported separately.

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

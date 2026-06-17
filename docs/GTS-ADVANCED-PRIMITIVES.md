<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Advanced Primitives Contract

This document collects the implementation path for streaming sinks, indexes, MMR/proofs,
range-fetch, replication, and memory benchmarks. The core wire format remains normative in
[`GTS-SPEC.md`](./GTS-SPEC.md); this contract states what the current packages actually support
and what is intentionally deferred from the v1 surface.

## Current V1 Support

| primitive | current support | claim boundary |
|---|---|---|
| Prefix-fold property | Every top-level corpus vector is tested at CBOR item boundaries. | This proves total prefix reads, not a streaming sink API. |
| Streamable layout | `gts compact --streamable` rewrites delivery order and appends an `index` footer; readers validate the claim and report accretive tails. | This is a Validating Tool/Profile Layout feature. |
| Index footer fields | Writers emit `count`, `head`, `off`, and `ti`; readers consume `count`/`head` for layout validation. | Full-reader random access from `off`/`ti` is not claimed yet. |
| Segment heads | `gts info` reports per-segment heads and layout state. | Replication verbs are not public yet. |
| Blob introspection | `gts ls` lists content-addressed blob digests, sizes, and media types. | Range fetch still needs a verified index or a boundary scan. |
| Memory benchmark helper | `scripts/bench_reader_memory.py` reports full-reader materialization, a frame-scan baseline, and a Rust `read_to_sink` streaming-fold row when Cargo is available. | The frame scan is not a Streaming Reader fold; the Rust row is the sink API evidence. |

The current Rust package may claim the `Streaming Reader` tier for its `read_to_sink` API.
Python, Go, and TypeScript SHOULD NOT claim the sink tier yet. No package should claim an
MMR/proof tier or replication CLI support until the deferred gates below land.

## Deferred Advanced CLI Verbs

These verbs are planned vocabulary, not current public commands. The guard script
[`scripts/check_advanced_contract.py`](../scripts/check_advanced_contract.py) fails if any of
these verbs appear in an engine dispatch surface or in the public CLI parity matrix before this
table is updated.

<!-- advanced-cli-deferred:start -->
| verb | status | next implementation gate |
|---|---|---|
| `prove` | deferred | Define stable proof JSON, MMR algorithm, and covered-frame verification vectors. |
| `verify-proof` | deferred | Verify the proof JSON against an index `mmr` root and covered frame id. |
| `heads` | deferred | Define machine-readable segment-head output and peer comparison semantics. |
| `segments` | deferred | Define segment inventory output with byte ranges, heads, profiles, and layout state. |
| `missing` | deferred | Define `--from-head` semantics and output ranges without assuming remote trust. |
| `resume` | deferred | Define `--after` boundary handling and prove the resumed bytes start on a frame boundary. |
<!-- advanced-cli-deferred:end -->

## Streaming Sink API

A package may claim `GTS Streaming Reader` only when it exposes a documented API that folds or
projects by consuming frames in order and sending rows to a sink without materializing the whole
`Graph`.

Minimum requirements:

- verify the header id and frame id/prev chain while streaming;
- retain or spill the term dictionary as needed, because term ids are segment-local;
- emit term, quad, reifier, annotation, suppression, blob, opaque, signature, diagnostic,
  segment-head, and streamable-layout events in frame order;
- record the same final diagnostics and segment head ids as the full reader for the same input;
- report memory behavior with `scripts/bench_reader_memory.py` or an equivalent benchmark.

The existing `streaming-property` subset remains valuable, but it is a prefix-totality property.
It is not by itself a streaming sink claim.

## Index, MMR, And Proof Tier

The optional `index` payload currently has four implemented pieces:

- `count`: number of covered frames;
- `head`: frame id of the last covered frame;
- `off`: byte offset of each covered frame from the start of its segment;
- `ti`: map from frame type to covered frame positions.

The following pieces are deferred:

- `dict`: term-dictionary locator used by text projections that need a dictionary pass;
- `mmr`: Merkle-Mountain-Range root over the covered frame ids;
- inclusion proof creation and verification;
- public proof CLI verbs.

Before enabling the MMR/proof tier, the repo needs:

- a stable MMR leaf and parent preimage definition;
- proof JSON fixtures;
- positive and negative cross-engine vectors;
- a clear rule for whether an index frame is covered by a later index only;
- proof verification tests independent of full file availability.

## Range-Fetch Rules

Range fetch is byte-accurate only after the caller has frame boundaries.

With a verified index `off` array, the start of frame `i` is:

```text
segment_start + off[i]
```

The end of frame `i` is the next known boundary:

```text
segment_start + off[i + 1]       # when i + 1 is still covered
index_frame_start                # for the last covered frame, after a boundary scan
```

The current index payload does not store frame lengths. Therefore a client MUST NOT infer the
last covered frame's exact byte range from `off` alone; it must know the index frame start from a
scan, container metadata, or a future length-bearing index extension.

Without an index, range fetch is still possible but requires a sequential CBOR boundary scan from
the segment start. HTTP `Range` requests are then safe only for ranges whose start and end were
derived from scanned item boundaries.

## Replication Workflow

Current tools can inspect heads but do not implement replication verbs:

```bash
gts info package.gts
gts ls package.gts
```

The planned workflow is:

```bash
gts heads local.gts
gts segments local.gts
gts missing --from-head <peer-head> local.gts
gts resume --after <frame-id> local.gts
```

Required semantics before those commands become public:

- `heads` reports segment heads in file order and an aggregate view suitable for peer comparison;
- `segments` reports each segment's byte range, profile, head, frame count, and layout state;
- `missing` compares a peer's known head against local segment/frame ancestry and returns exact
  byte ranges or an explicit "unknown; scan required" result;
- `resume` emits bytes only after proving the requested frame id exists and the output starts at a
  CBOR item boundary.

## Memory Benchmarks

Run the local helper against one or more GTS files:

```bash
cd python
uv run python ../scripts/bench_reader_memory.py ../vectors/25-streamable-source.gts
```

The helper emits three rows per file:

- `full-reader`: materializes a `Graph` with the current Python reader;
- `frame-scan`: decodes one CBOR item at a time and counts headers/frames without folding;
- `streaming-fold`: runs the Rust `read_to_sink` benchmark helper when Cargo is available and
  reports the Rust process high-water RSS (`VmHWM`) on Linux.

Future non-Rust streaming implementations should replace fallback rows with engine-specific sink
benchmarks that report peak memory by distinct terms, frames, triples, and blob sizes.

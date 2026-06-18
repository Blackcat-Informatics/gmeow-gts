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
| Index footer fields | Writers emit `count`, `head`, `off`, and `ti`; Rust writers can opt in to `mmr`, and Rust readers validate `mmr` roots when present. | Full-reader random access from `off`/`ti` is not claimed yet. |
| MMR proof JSON | All engines verify detached proof JSON against `vectors/proofs/`; Rust also exposes `Writer::add_index_with_mmr`, validates optional `index.mmr`, and implements `gts prove`. | Detached verification is cross-engine; proof creation from indexed GTS files remains Rust-only. |
| Replication inventory | All four CLIs expose `gts heads`, `gts segments`, `gts missing`, and `gts resume` for machine-readable head comparison and byte-range resume. | Shared v1 replication surface; `resume` starts only after a verified frame id at a scanned CBOR item boundary. |
| Blob introspection | `gts ls` lists content-addressed blob digests, sizes, and media types. | Range fetch still needs a verified index or a boundary scan. |
| Memory benchmark helper | `scripts/bench_reader_memory.py` reports full-reader materialization, a frame-scan baseline, and a Rust `read_to_sink` evented-fold row when Cargo is available. Go reports its full-reader and non-materializing streaming-sink allocation evidence with `go test ./reader -bench 'Benchmark(ReadFull\|ReadToSink)CorpusVector' -benchmem`. TypeScript exposes browser `foldStream` events, but memory reporting for browser runtimes remains release-report evidence rather than a shared script row. | The frame scan is not a Streaming Reader fold; Rust and TypeScript rows are event/progressive API evidence, while Go satisfies the current Streaming Reader tier. |

The current Go package may claim the `Streaming Reader` tier for
`reader.ReadToSink(ctx, io.Reader, reader.Options, sink)`. The current Rust package SHOULD NOT
claim that tier for `read_to_sink` yet because it accepts a byte slice and uses the segment
`Graph` path while emitting events. The current TypeScript browser package SHOULD NOT claim that
tier for `foldStream(stream, options)` or `readStream(stream, options)` yet because those APIs are
progressive Web Streams readers that still return materialized graph state. Rust remains the only
package that may claim MMR proof creation. All four packages may claim detached proof
verification for the fixture set in `vectors/proofs/` and the shared replication inventory verbs.
Python SHOULD NOT claim the sink or proof-creation tiers yet; Go and TypeScript SHOULD NOT claim
proof creation yet.

## Deferred Advanced CLI Verbs

The rows below, when present, are planned vocabulary rather than current public commands. The guard script
[`scripts/check_advanced_contract.py`](../scripts/check_advanced_contract.py) fails if any of
these verbs appear in an engine dispatch surface or in the public CLI parity matrix before this
table is updated. The table may be empty when every currently planned advanced CLI verb has been
promoted.

<!-- advanced-cli-deferred:start -->
| verb | status | next implementation gate |
|---|---|---|
<!-- advanced-cli-deferred:end -->

## Streaming Sink API

A package may claim `GTS Streaming Reader` only when it exposes a documented API that folds or
projects by consuming frames in order and emitting events to a sink without materializing the
whole `Graph`.

Minimum requirements:

- verify the header id and frame id/prev chain while streaming;
- retain or spill the term dictionary as needed, because term ids are segment-local;
- emit term, quad, reifier, annotation, suppression, blob, opaque, signature, diagnostic,
  segment-head, and streamable-layout events in frame order;
- record the same final diagnostics and segment head ids as the full reader for the same input;
- retain memory bounded by `O(distinct terms + maximum decoded frame size + validation
  sidecar state)`, not folded triples or blobs;
- report memory behavior with `scripts/bench_reader_memory.py` or an equivalent benchmark.

The existing `streaming-property` subset remains valuable, but it is a prefix-totality property.
It is not by itself a streaming sink claim.

## Index, MMR, And Proof Tier

The optional `index` payload currently has five implemented pieces:

- `count`: number of covered frames;
- `head`: frame id of the last covered frame;
- `off`: byte offset of each covered frame from the start of its segment;
- `ti`: map from frame type to covered frame positions.
- `mmr`: Rust-only Merkle-Mountain-Range root in indexed GTS files over the covered frame ids.

The following pieces remain deferred:

- `dict`: term-dictionary locator used by text projections that need a dictionary pass;
- cross-engine inclusion proof creation from indexed GTS files;
- `prove` CLI verbs outside Rust.

Before promoting MMR proof creation beyond Rust, the repo needs:

- indexed-file proof creation fixtures, including positive and negative behavior;
- `index.mmr` writer/reader implementation in Python, Go, and TypeScript against the stable
  preimages in `GTS-SPEC.md`;
- proof creation tests that prove generated detached JSON verifies independently of full file
  availability in each engine.

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

All engine CLIs implement the replication verbs:

```bash
gts heads local.gts
gts segments local.gts
gts missing --from-head <peer-head> local.gts
gts resume --after <frame-id> local.gts
```

The stable Rust JSON shapes are:

```text
gts-replication-heads-v1
gts-replication-segments-v1
gts-replication-missing-v1
```

Shared semantics:

- `heads` reports segment heads in file order and an aggregate view suitable for peer comparison;
- `segments` reports each segment's byte range, profile, head, frame count, and layout state;
- `missing` compares a peer's known head against local segment/frame ancestry and returns exact
  byte ranges or an explicit "unknown; scan required" result;
- `resume` emits bytes only after proving the requested frame id exists and the output starts at a
  CBOR item boundary.

## Memory Benchmarks

The release benchmark suite covers read, fold, write/from-N-Quads, files-profile pack/unpack,
and streaming-memory evidence across the engines that expose each surface:

```bash
just bench-release
```

The suite writes machine-readable JSON and a Markdown report under `dist/benchmarks/`. Use
[`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) as the v1 release-note
or paper-appendix template.

Run the local helper against one or more GTS files:

```bash
cd python
uv run python ../scripts/bench_reader_memory.py ../vectors/25-streamable-source.gts
```

The helper emits three rows per file:

- `full-reader`: materializes a `Graph` with the current Python reader;
- `frame-scan`: decodes one CBOR item at a time and counts headers/frames without folding;
- `streaming-fold`: runs the Rust `read_to_sink` evented-fold benchmark helper when Cargo is
  available and reports the Rust process high-water RSS (`VmHWM`) on Linux; this row is
  projection evidence and does not satisfy the current Rust Streaming Reader tier requirements.

Rust relational export regression fixtures cover the bounded row-emission path: the DB loader
streams SQL into `sqlite3`/`duckdb`, leaves lazy blob entries uncached in the folded graph, and
stops before `COMMIT` if a transformed blob cannot be decoded. The remaining schema constraint is
intentional: `blobs.bytes` exports must still decode each inline payload transiently for the row
being written.

Future non-Go streaming implementations should replace fallback rows with engine-specific
non-materializing sink benchmarks that report peak memory by distinct terms, maximum decoded
frame size, validation sidecar state, triples, and blob sizes.

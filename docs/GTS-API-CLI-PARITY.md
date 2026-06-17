<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS API And CLI Parity Contract

This document defines the cross-language surface that Rust, Python, Go, and TypeScript keep
compatible while the engines continue to expose native idioms. The wire format remains normative
in [`GTS-SPEC.md`](./GTS-SPEC.md), and corpus/tier rules remain normative in
[`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md). This contract owns the public API shape and CLI
parity matrix so feature gaps are explicit rather than inferred from package-specific docs.

## Language-Neutral API Shape

The stable waist is semantic, not syntactic. Each engine MAY use native names and containers, but
the following operations and folded fields are the compatibility target.

| operation | contract | current native surface |
|---|---|---|
| `read(input, options)` | Parse a byte buffer or path as a CBOR Sequence, verify the id/prev chain, fold every recoverable frame, and return a graph/result with diagnostics instead of panicking on malformed input. | Python `gts.read(data, keys=None, expected_head=None, allow_segments=True)`; Rust `reader::read(&bytes, allow_segments, expected_head)`; Go `reader.Read(data, allowSegments, expectedHead)`; TypeScript `Read(bytes, allowSegments, expectedHead?)`. |
| `verify(input, options)` | Apply strict transport checks over the same fold: chain/hash diagnostics, expected-head freshness when provided, streamable-layout checks when requested, and COSE signature status when keys are provided. | CLI `gts verify`; library verification is exposed through folded diagnostics plus each engine's COSE helpers. |
| `write(graph/events, options)` | Emit deterministic CBOR for hashed or signed bytes, compute each frame id from its content, and set `prev` to the previous frame id. | Python `Writer`; Rust `writer::Writer`; Go `writer.New`; TypeScript `Writer`. |
| `fold(input)` | Return the deterministic GTS value fold: terms, quads, reifiers, annotations, blobs, suppressions, opaque nodes, signatures, segment heads, profiles, and streamable layout state. | Same object returned by `read`. |
| `to_nquads(graph)` | Project the folded RDF dataset to sorted N-Quads text with the same value semantics across engines. | Python `to_nquads`; Rust `nquads::to_nquads`; Go `nquads.ToNQuads`; TypeScript `toNQuads`. |
| `from_nquads(input)` | Build a GTS file from N-Quads text using the shared writer semantics. | Python `from_nquads` and `gts from-nq` only. This is an intentional extension until other engines implement it. |
| graph iterators/accessors | Expose resolved access to terms, quads, reifier bindings, annotations, suppressions, blobs, opaque nodes, signatures, diagnostics, segment heads, profiles, metadata, and streamable state. | Native fields on `Graph` in all four engines, with helper lookups where idiomatic. |
| blobs | Preserve inline blob bytes by `blake3:<hex>` digest and retain declared blob metadata such as media type. Extraction MUST re-hash bytes before writing them. | `Graph.blobs`/`blob_meta`, `Graph.blobs`/`blob_meta`, `Graph.Blobs`/`BlobMeta`, `Graph.blobs`/`blobMeta`. |
| opaque nodes | Preserve undecodable or unsupported recoverable frames as graph-visible opaque nodes with a frame id, frame type, reason, and signature status. | `OpaqueNode` in every engine. |
| diagnostics | Preserve stable diagnostic `code` values and optional frame indexes; native detail text may differ. | `Diagnostic.code/detail/frame_index`, `Diagnostic { code, detail, frame_index }`, `Diagnostic{Code, Detail, FrameIndex}`, `Diagnostic.code/detail/frameIndex`. |
| streaming/full-reader options | Carry read mode, segment allowance, expected head, key provider, recursion/decode budgets, and streamable validation as options. Engines MAY stage these as separate helpers while preserving the same observable fold and diagnostics. | `allow_segments`/`allowSegments`, `expected_head`/`expectedHead`, COSE key callbacks, and CLI flags today; deeper recursion/MMR options are future Full Reader work. |

## Cross-Language Equality Targets

The conformance corpus compares the observable fields that make engines substitutable. New tests
and API additions should preserve these targets:

| target | equality rule |
|---|---|
| folded graph | Terms, quads, reifiers, annotations, suppressions, profile declarations, metadata, streamable state, and N-Quads projection match the expected JSON. |
| diagnostics | Diagnostic code order matches. Native detail text and native exception/warning wrappers are not frozen. |
| head id | Segment head ids match as lowercase hex. A single-segment file's last segment head is the file head for freshness checks. |
| opaque reasons | Opaque-node reason strings match after sorting, including `unknown-codec`, `missing-key`, `damaged`, and `unknown-frame-type`. |
| signature status | Per-frame signature status uses `valid`, `invalid`, or `unverified` with matching key ids when present. |
| blob digests | `blake3:<hex>` digest keys, declared media types, and decoded byte lengths match; extraction re-hashes bytes before writing. |

## Diagnostics And Native Error Mapping

Reader diagnostics are data, not thrown control flow, for permissive reads. Strict verification and
publication commands MAY convert any error or fatal diagnostic into a non-zero process exit or a
native error return.

| concept | Python | Rust | Go | TypeScript |
|---|---|---|---|---|
| diagnostic record | `gts.Diagnostic` dataclass | `model::Diagnostic` struct | `model.Diagnostic` struct | `Diagnostic` interface |
| code field | `code: str` | `code: String` | `Code string` | `code: string` |
| detail field | `detail: str` | `detail: String` | `Detail string` | `detail: string` |
| frame index | `frame_index: int \| None` | `frame_index: Option<usize>` | `FrameIndex *int` | `frameIndex?: number` |
| permissive read result | `Graph` with `diagnostics` | `Graph` with `diagnostics` | `*model.Graph` with `Diagnostics` | `Graph` with `diagnostics` |
| strict CLI failure | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal |
| usage or I/O failure | exit `2` | exit `2` | exit `2` | exit `2` |

The canonical diagnostic code registry is in
[`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md#6-diagnostics-registry).

## CLI Parity Matrix

`yes` means the verb is implemented by that engine's `gts` binary. `no` means the absence is an
intentional public gap until a parity issue lands. The matrix is checked by
[`scripts/check_cli_parity.py`](../scripts/check_cli_parity.py), which reads this table and the
actual dispatch surfaces.

<!-- cli-parity-matrix:start -->
| verb | Python | Rust | Go | TypeScript | status |
|---|---|---|---|---|---|
| `info` | yes | yes | yes | yes | common |
| `fold` | yes | yes | yes | yes | common |
| `verify` | yes | yes | yes | yes | common |
| `extract-key` | yes | yes | yes | yes | common |
| `ls` | yes | yes | yes | yes | common |
| `extract` | yes | yes | yes | yes | common |
| `cat` | yes | yes | yes | yes | common |
| `compact` | yes | yes | yes | yes | common |
| `pack` | yes | yes | yes | yes | common |
| `unpack` | yes | yes | yes | yes | common |
| `diff` | yes | yes | yes | yes | common |
| `from-nq` | yes | no | no | no | Python extension |
| `to-sqlite` | yes | no | no | no | Python extension |
| `to-duckdb` | yes | no | no | no | Python extension |
| `to-parquet` | yes | no | no | no | Python extension |
<!-- cli-parity-matrix:end -->

### Intentional Gaps

- `from-nq` is Python-only because the current inverse parser lives in the Python reference
  package. Other engines can still write GTS through their native writers.
- `to-sqlite`, `to-duckdb`, and `to-parquet` are Python-only relational exports. DuckDB and
  Parquet require the Python `[db]` extra.
- Future index/MMR proof, nested-GTS recursion, and encryption policy verbs are not part of the
  stable CLI surface yet. They should be added to this matrix before package-specific docs claim
  them.
- Advanced `prove`, `verify-proof`, `heads`, `segments`, `missing`, and `resume` verbs are
  deferred in [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) and guarded by
  `scripts/check_advanced_contract.py`.

## Drift Guard

Run the parity check locally with:

```bash
python scripts/check_cli_parity.py
```

The CI lint job runs the same command. The check fails when:

- an engine implements a CLI verb not represented in the matrix;
- the matrix marks a verb `yes` for an engine whose dispatch surface lacks it;
- the matrix marks a verb `no` for an engine that now implements it;
- the README common and Python-extension command blocks drift from the matrix.

When adding or removing a CLI verb, update the implementation, this matrix, the README command
blocks, and package-specific README text in the same change.

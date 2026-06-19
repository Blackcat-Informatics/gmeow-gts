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
| `read(input, options)` | Parse a byte buffer or path as a CBOR Sequence, verify the id/prev chain, fold every recoverable frame, and return a graph/result with diagnostics instead of panicking on malformed input. | Python `gts.read(data, keys=None, expected_head=None, allow_segments=True)`; Rust `reader::read(&bytes, allow_segments, expected_head)` or `reader::read_with_options` with `ReadOptions::with_content_key`; Go `reader.Read(data, allowSegments, expectedHead)`; TypeScript `Read(bytes, allowSegments, expectedHead?)`. |
| `verify(input, options)` | Apply strict transport checks over the same fold: chain/hash diagnostics, expected-head freshness when provided, streamable-layout checks when requested, and COSE signature status when keys are provided. | CLI `gts verify`; Python `gts.verify.verify_file`; Rust `gmeow_gts::verify::verify_file` plus folded diagnostics and lower-level COSE helpers in every engine. |
| `write(graph/events, options)` | Emit deterministic CBOR for hashed or signed bytes, compute each frame id from its content, and set `prev` to the previous frame id. | Python `Writer`; Rust `writer::Writer`; Go `writer.New`; TypeScript `Writer`. |
| `fold(input)` | Return the deterministic GTS value fold: terms, quads, reifiers, annotations, blobs, suppressions, opaque nodes, signatures, segment heads, profiles, and streamable layout state. | Same object returned by `read`. |
| `to_nquads(graph)` | Project the folded RDF dataset to sorted N-Quads text with the same value semantics across engines. | Python `to_nquads`; Rust `nquads::to_nquads`; Go `nquads.ToNQuads`; TypeScript `toNQuads`. |
| `from_nquads(input)` | Build a GTS file from N-Quads text using the shared writer semantics. | Python `from_nquads`; Rust `from_nquads::from_nquads`; Go `fromnquads.FromNQuads`; TypeScript `fromNQuads`; CLI `gts from-nq` in every engine. |
| `to_trig(graph)` / `from_trig(input)` | Project folded RDF to readable TriG graph blocks and rebuild GTS bytes from the supported TriG surface without changing N-Quads content. | Python `gts.trig.to_trig` / `from_trig`; Rust `trig::to_trig` / `from_trig::from_trig`; CLI `gts to-trig` and `gts from-trig` in Python and Rust. |
| graph iterators/accessors | Expose resolved access to terms, quads, reifier bindings, annotations, suppressions, blobs, opaque nodes, signatures, diagnostics, segment heads, profiles, metadata, and streamable state. | Native fields on `Graph` in all four engines, with helper lookups where idiomatic. |
| blobs | Preserve inline blob bytes by `blake3:<hex>` digest and retain declared blob metadata such as media type. Extraction MUST re-hash bytes before writing them. Implementations MAY keep transformed blob bytes lazy until access. | Python `Graph.blobs`/`blob_meta`; Rust `Graph.blobs` lazy `BlobEntry` plus `blob_entry`/`blob_bytes`/`decoded_blobs`; Go `Graph.Blobs`/`BlobMeta`; TypeScript `Graph.blobs`/`blobMeta`. |
| opaque nodes | Preserve undecodable or unsupported recoverable frames as graph-visible opaque nodes with a frame id, frame type, reason, and signature status. | `OpaqueNode` in every engine. |
| diagnostics | Preserve stable diagnostic `code` values and optional frame indexes; native detail text may differ. | `Diagnostic.code/detail/frame_index`, `Diagnostic { code, detail, frame_index }`, `Diagnostic{Code, Detail, FrameIndex}`, `Diagnostic.code/detail/frameIndex`. |
| streaming/full-reader options | Carry read mode, segment allowance, expected head, key provider, recursion/decode budgets, and streamable validation as options. Engines MAY stage these as separate helpers while preserving the same observable fold and diagnostics. | Python `keys`, Rust `ReadOptions`/`read_to_sink_with_options`, Go `reader.Options`/`reader.ReadToSink`, `allow_segments`/`allowSegments`, `expected_head`/`expectedHead`, and CLI flags today; deeper recursion/MMR options are future Full Reader work. |

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
| `from-nq` | yes | yes | yes | yes | common |
| `to-trig` | yes | yes | no | no | Python/Rust transform extension |
| `from-trig` | yes | yes | no | no | Python/Rust transform extension |
| `to-yaml-ld` | no | yes | no | no | Rust transform extension |
| `from-yaml-ld` | no | yes | no | no | Rust transform extension |
| `to-okf` | no | yes | no | no | Rust OKF profile extension |
| `from-okf` | no | yes | no | no | Rust OKF profile extension |
| `to-sqlite` | yes | yes | no | no | Python/Rust extension |
| `to-duckdb` | yes | yes | no | no | Python/Rust extension |
| `to-parquet` | yes | yes | no | no | Python/Rust extension |
| `prove` | no | yes | no | no | Rust proof creation extension |
| `dump` | no | yes | no | no | Rust inspection export extension |
| `verify-proof` | yes | yes | yes | yes | common |
| `heads` | yes | yes | yes | yes | common |
| `segments` | yes | yes | yes | yes | common |
| `missing` | yes | yes | yes | yes | common |
| `resume` | yes | yes | yes | yes | common |
<!-- cli-parity-matrix:end -->

### Intentional Gaps

- Rust `to-sqlite` requires `sqlite3` on `PATH`; Rust `to-duckdb` and `to-parquet` require
  the optional no-dependency `duckdb` Cargo feature plus `duckdb` on `PATH`. Python DuckDB
  and Parquet exports require the Python `[db]` extra. Rust streams SQL rows to the runtime
  tool instead of retaining all relational rows or a complete SQL script in memory; the stable
  `blobs.bytes` schema still requires transient blob decoding while each blob row is emitted.
- Go and TypeScript do not yet expose relational exports.
- `to-trig` and `from-trig` are Python/Rust transform extensions. They preserve the same
  folded RDF content as the N-Quads projection while using readable TriG graph blocks; Go and
  TypeScript parity can land later against the same round-trip expectations.
- `to-yaml-ld` and `from-yaml-ld` are Rust-only extension verbs behind
  `--features yaml-ld`. They are transform-only shims over folded graph tables,
  not a wire-format or canonical-catalog change; Python, Go, and TypeScript
  parity can land later with a shared corpus oracle addition if needed.
- `to-okf` and `from-okf` are Rust-only OKF profile verbs behind
  `--features okf`. They map an OKF Markdown bundle to GTS profile `okf`
  with manifest schema `gts-okf-v1`, content-addressed Markdown body blobs,
  queryable link edges, navigation `index.md` tolerance, and `_unmapped.nq`
  for out-of-profile RDF. The committed OKF corpus, including
  `vectors/okf/bigquery-join/`, is the required parity gate for any future
  Python, Go, or TypeScript implementation. Those engines must remain `no`
  here until they can import/export the `gts-okf-v1` directory contract and
  preserve the folded N-Quads expectations.
- `dump` is a Rust-only inspection export that writes a versioned directory tree with folded
  N-Quads, JSONL tables, unfolded frame views, blob indexes, and files-profile payloads. It is
  not a wire-format change; Python, Go, and TypeScript parity can implement the same
  `gts-dump-v1` directory contract later.
- All engines implement `verify-proof` for detached MMR proof JSON using the stable preimages and
  the positive/negative fixtures in `vectors/proofs/`. Rust additionally implements `prove` from
  files that carry a verified `index.mmr` root. Python, Go, and TypeScript should not expose
  `prove` until they can create file-backed proofs against the same fixture discipline.
- All engines implement the replication verbs with the same JSON schemas and resume boundary
  rules: `gts-replication-heads-v1`, `gts-replication-segments-v1`, and
  `gts-replication-missing-v1`.
- Future nested-GTS recursion and encryption policy verbs are not part of the stable CLI surface
  yet. They should be added to this matrix before package-specific docs claim them.
- Remaining advanced deferred verbs, if any, are tracked in
  [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) and guarded by
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

## Files Profile Command Contract

`pack`, `unpack`, and `diff` are common commands in all four engines. Their observable behavior
is part of the parity surface:

- `pack <dir|file>... -o out.gts` emits a single `files` segment with catalog terms/quads before
  inline blobs, stores each path once, and deduplicates identical content by digest.
- Stored archive paths are `/`-separated relative paths. Every engine refuses empty paths,
  absolute paths, Windows drive-relative paths, `..`, `.`, empty components, and backslash
  separators before reading or writing file bytes.
- Symlinks are not archived. `pack` and `diff` refuse symlink entries rather than following
  them; `unpack` refuses paths that escape the destination directory, including escapes through
  existing symlinks below that directory.
- `unpack` re-hashes inline blob bytes before writing. An unsuppressed `FileEntry` whose inline
  blob is absent is a refusal; suppressed blob digests are skipped by default and extracted only
  with `--include-suppressed`.
- `diff` compares the archive manifest to a directory by `files:digest` and returns sorted
  `added:`, `modified:`, and `removed:` lines. Exit `0` means no differences; exit `1` means
  either a difference or a refused input.

The live cross-engine guard is [`scripts/interop.sh`](../scripts/interop.sh): each engine packs
the same fixture, every engine folds and unpacks every package, and every engine diffs both the
matching tree and a changed tree against every package.

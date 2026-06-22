<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS API And CLI Parity Contract

This document defines the cross-language surface that Rust, Python, Go, TypeScript,
Smalltalk/Pharo, and Kotlin/JVM keep compatible while the engines continue to expose native idioms. The wire
format remains normative in [`GTS-SPEC.md`](./GTS-SPEC.md), and corpus/tier rules remain normative in
[`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md). This contract owns the public API shape and CLI
parity matrix so feature gaps are explicit rather than inferred from package-specific docs.

The Rust-backed C ABI and derived C-compatible wrappers are a separate interoperability layer.
They consume `libgts` through `rust/capi/include/gts.h`, expose ecosystem-native library APIs,
and do not add columns to the full-engine API/CLI parity tables below.

## Language-Neutral API Shape

The stable waist is semantic, not syntactic. Each engine MAY use native names and containers, but
the following operations and folded fields are the compatibility target.

| operation | contract | current native surface |
|---|---|---|
| `read(input, options)` | Parse a byte buffer or path as a CBOR Sequence, verify the id/prev chain, fold every recoverable frame, and return a graph/result with diagnostics instead of panicking on malformed input. | Python `gts.read(data, keys=None, expected_head=None, allow_segments=True)`; Rust `reader::read(&bytes, allow_segments, expected_head)` or `reader::read_with_options` with `ReadOptions::with_content_key`; Go `reader.Read(data, allowSegments, expectedHead)`; TypeScript `Read(bytes, allowSegments, expectedHead?)`; Smalltalk `GtsReader read:allowSegments:`; Kotlin `read(data, allowSegments)`. |
| `verify(input, options)` | Apply strict transport checks over the same fold: chain/hash diagnostics, expected-head freshness when provided, streamable-layout checks when requested, and COSE signature status when keys are provided. | CLI `gts verify`; Python `gts.verify.verify_file`; Rust `gmeow_gts::verify::verify_file` plus folded diagnostics and lower-level COSE helpers in every engine. |
| `write(graph/events, options)` | Emit deterministic CBOR for hashed or signed bytes, compute each frame id from its content, and set `prev` to the previous frame id. | Python `Writer`; Rust `writer::Writer`; Go `writer.New`; TypeScript `Writer`; Smalltalk `GtsWriter`; Kotlin `Writer`. |
| `fold(input)` | Return the deterministic GTS value fold: terms, quads, reifiers, annotations, blobs, suppressions, opaque nodes, signatures, segment heads, profiles, and streamable layout state. | Same object returned by `read`. |
| `to_nquads(graph)` | Project the folded RDF dataset to sorted N-Quads text with the same value semantics across engines. | Python `to_nquads`; Rust `nquads::to_nquads`; Go `nquads.ToNQuads`; TypeScript `toNQuads`; Smalltalk `GtsNQuads`; Kotlin `toNQuads`. |
| `from_nquads(input)` | Build a GTS file from N-Quads text using the shared writer semantics. | Python `from_nquads`; Rust `from_nquads::from_nquads`; Go `fromnquads.FromNQuads`; TypeScript `fromNQuads`; Smalltalk `GtsFromNQuads`; Kotlin `fromNQuads`; CLI `gts from-nq` in every engine. |
| `to_ntriples(graph)` / `from_ntriples(input)` | Project a default-graph RDF dataset to N-Triples and rebuild GTS bytes from N-Triples text using the shared RDF 1.2 parser/serializer. | Rust `rdf_codecs::to_ntriples` / `from_ntriples` behind `--features rdf-codecs`; CLI `gts to-nt` and `gts from-nt` in Rust. |
| `to_trig(graph)` / `from_trig(input)` | Project folded RDF to readable TriG graph blocks and rebuild GTS bytes from the supported TriG surface without changing N-Quads content. | Python `gts.trig.to_trig` / `from_trig`; Rust `trig::to_trig` / `from_trig::from_trig`; Rust `rdf_codecs::to_trig` / `from_trig` with `--features rdf-codecs`; CLI `gts to-trig` and `gts from-trig` in Python and Rust. |
| `to_turtle(graph)` / `from_turtle(input)` | Project a default-graph RDF dataset to Turtle and rebuild GTS bytes from Turtle text using the shared Turtle-family RDF 1.2 parser/serializer. | Rust `rdf_codecs::to_turtle` / `from_turtle` behind `--features rdf-codecs`; CLI `gts to-turtle` and `gts from-turtle` in Rust. |
| graph iterators/accessors | Expose resolved access to terms, quads, reifier bindings, annotations, suppressions, blobs, opaque nodes, signatures, diagnostics, segment heads, profiles, metadata, and streamable state. | Native fields on `Graph`/`GtsGraph` in all six engines, with helper lookups where idiomatic. |
| blobs | Preserve inline blob bytes by `blake3:<hex>` digest and retain declared blob metadata such as media type. Extraction MUST re-hash bytes before writing them. Implementations MAY keep transformed blob bytes lazy until access. | Python `Graph.blobs`/`blob_meta`; Rust `Graph.blobs` lazy `BlobEntry` plus `blob_entry`/`blob_bytes`/`decoded_blobs`; Go `Graph.Blobs`/`BlobMeta`; TypeScript `Graph.blobs`/`blobMeta`; Smalltalk `GtsGraph blobs`/`blobMeta`; Kotlin `Graph.blobs`/`blobMeta`. |
| opaque nodes | Preserve undecodable or unsupported recoverable frames as graph-visible opaque nodes with a frame id, frame type, reason, and signature status. | `OpaqueNode` in every engine. |
| diagnostics | Preserve stable diagnostic `code` values and optional frame indexes; native detail text may differ. | `Diagnostic.code/detail/frame_index`, `Diagnostic { code, detail, frame_index }`, `Diagnostic{Code, Detail, FrameIndex}`, `Diagnostic.code/detail/frameIndex`. |
| streaming/full-reader options | Carry read mode, segment allowance, expected head, key provider, recursion/decode budgets, and streamable validation as options. Engines MAY stage these as separate helpers while preserving the same observable fold and diagnostics. | Python `keys`, Rust `ReadOptions`/`read_to_sink_with_options`, Go `reader.Options`/`reader.ReadToSink`, TypeScript `allowSegments`, Smalltalk `allowSegments`, Kotlin `allowSegments`, and CLI flags today; deeper recursion/MMR options are future Full Reader work. |

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

| concept | Python | Rust | Go | TypeScript | Smalltalk | Kotlin |
|---|---|---|---|---|---|---|
| diagnostic record | `gts.Diagnostic` dataclass | `model::Diagnostic` struct | `model.Diagnostic` struct | `Diagnostic` interface | `GtsDiagnostic` object | `Diagnostic` data class |
| code field | `code: str` | `code: String` | `Code string` | `code: string` | `code` | `code: String` |
| detail field | `detail: str` | `detail: String` | `Detail string` | `detail: string` | `detail` | `detail: String` |
| frame index | `frame_index: int \| None` | `frame_index: Option<usize>` | `FrameIndex *int` | `frameIndex?: number` | `frameIndex` | `frameIndex: Int?` |
| permissive read result | `Graph` with `diagnostics` | `Graph` with `diagnostics` | `*model.Graph` with `Diagnostics` | `Graph` with `diagnostics` | `GtsGraph` with `diagnostics` | `Graph` with `diagnostics` |
| strict CLI failure | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal | exit `1` for diagnostics/refusal |
| usage or I/O failure | exit `2` | exit `2` | exit `2` | exit `2` | exit `2` | exit `2` |

The canonical diagnostic code registry is in
[`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md#6-diagnostics-registry).

## CLI Parity Matrix

`yes` means the verb is implemented by that engine's `gts` binary. `no` means the absence is an
intentional public gap until a parity issue lands. The matrix is checked by
[`scripts/check_cli_parity.py`](../scripts/check_cli_parity.py), which reads this table and the
actual dispatch surfaces.

<!-- cli-parity-matrix:start -->
| verb | Python | Rust | Go | TypeScript | Smalltalk | Kotlin | status |
|---|---|---|---|---|---|---|---|
| `info` | yes | yes | yes | yes | yes | yes | common |
| `fold` | yes | yes | yes | yes | yes | yes | common |
| `verify` | yes | yes | yes | yes | yes | yes | common |
| `extract-key` | yes | yes | yes | yes | yes | yes | common |
| `ls` | yes | yes | yes | yes | yes | yes | common |
| `extract` | yes | yes | yes | yes | yes | yes | common |
| `cat` | yes | yes | yes | yes | yes | yes | common |
| `compact` | yes | yes | yes | yes | yes | yes | common |
| `pack` | yes | yes | yes | yes | yes | yes | common |
| `unpack` | yes | yes | yes | yes | yes | yes | common |
| `diff` | yes | yes | yes | yes | yes | yes | common |
| `from-nq` | yes | yes | yes | yes | yes | yes | common |
| `to-trig` | yes | yes | no | no | no | no | Python/Rust transform extension |
| `from-trig` | yes | yes | no | no | no | no | Python/Rust transform extension |
| `to-nt` | no | yes | no | no | no | no | Rust RDF text codec extension |
| `from-nt` | no | yes | no | no | no | no | Rust RDF text codec extension |
| `to-turtle` | no | yes | no | no | no | no | Rust Turtle-family transform extension |
| `from-turtle` | no | yes | no | no | no | no | Rust Turtle-family transform extension |
| `to-yaml-ld` | no | yes | no | no | no | no | Rust transform extension |
| `from-yaml-ld` | no | yes | no | no | no | no | Rust transform extension |
| `to-okf` | no | yes | no | no | no | no | Rust OKF profile extension |
| `from-okf` | no | yes | no | no | no | no | Rust OKF profile extension |
| `to-tar` | no | yes | no | no | no | no | Rust tar bridge extension |
| `from-tar` | no | yes | no | no | no | no | Rust tar bridge extension |
| `tar` | no | yes | no | no | no | no | Rust tar-compatible extension |
| `to-sqlite` | yes | yes | no | no | no | no | Python/Rust extension |
| `to-duckdb` | yes | yes | no | no | no | no | Python/Rust extension |
| `to-parquet` | yes | yes | no | no | no | no | Python/Rust extension |
| `prove` | no | yes | no | no | no | no | Rust proof creation extension |
| `dump` | no | yes | no | no | no | no | Rust inspection export extension |
| `verify-proof` | yes | yes | yes | yes | yes | yes | common |
| `heads` | yes | yes | yes | yes | yes | yes | common |
| `segments` | yes | yes | yes | yes | yes | yes | common |
| `missing` | yes | yes | yes | yes | yes | yes | common |
| `resume` | yes | yes | yes | yes | yes | yes | common |
<!-- cli-parity-matrix:end -->

### Intentional Gaps

- Rust `to-sqlite` requires `sqlite3` on `PATH`; Rust `to-duckdb` and `to-parquet` require
  the optional no-dependency `duckdb` Cargo feature plus `duckdb` on `PATH`. Python DuckDB
  and Parquet exports require the Python `[db]` extra. Rust streams SQL rows to the runtime
  tool instead of retaining all relational rows or a complete SQL script in memory; the stable
  `blobs.bytes` schema still requires transient blob decoding while each blob row is emitted.
- Go, TypeScript, Smalltalk, and Kotlin do not yet expose relational exports.
- `to-trig` and `from-trig` are Python/Rust transform extensions. They preserve the same
  folded RDF content as the N-Quads projection while using readable TriG graph blocks; Go,
  TypeScript, Smalltalk, and Kotlin parity can land later against the same round-trip expectations.
- `to-nt` and `from-nt` are Rust-only RDF text codec extensions behind `--features rdf-codecs`.
  `to-nt` accepts only default-graph RDF projections; named-graph datasets should use `to-trig`.
  Python, Go, TypeScript, Smalltalk, and Kotlin parity can land later against the same
  parser/round-trip expectations.
- `to-turtle` and `from-turtle` are Rust-only Turtle-family transform extensions behind
  `--features rdf-codecs`. They use the same RDF 1.2 parser/serializer stack as the full TriG
  path. `to-turtle` accepts only default-graph RDF projections; named-graph datasets should use
  `to-trig`. Python, Go, TypeScript, Smalltalk, and Kotlin parity can land later against the same
  parser/round-trip expectations.
- `to-yaml-ld` and `from-yaml-ld` are Rust-only extension verbs behind
  `--features yaml-ld`. They are transform-only shims over folded graph tables,
  not a wire-format or canonical-catalog change; Python, Go, TypeScript, Smalltalk, and Kotlin
  parity can land later with a shared corpus oracle addition if needed.
- `to-okf` and `from-okf` are Rust-only OKF profile verbs behind
  `--features okf`. They map an OKF Markdown bundle to GTS profile `okf`
  with manifest schema `gts-okf-v1`, content-addressed Markdown body blobs,
  queryable link edges, navigation `index.md` tolerance, and `_unmapped.nq`
  for out-of-profile RDF. The committed OKF corpus, including
  `vectors/okf/bigquery-join/`, is the required parity gate for any future
  Python, Go, TypeScript, Smalltalk, or Kotlin implementation. Those engines must remain `no`
  here until they can import/export the `gts-okf-v1` directory contract and
  preserve the folded N-Quads expectations.
- `to-tar`, `from-tar`, and `tar` are Rust-only files-profile-v2 bridge verbs behind
  `--features tar`. They map tar streams to GTS files and back while preserving
  files-profile metadata, opt-in link/special-file records, gzip/zstd wrapping,
  unknown PAX records, and a tar-compatible `-c/-x/-t/-d` command surface. Python,
  Go, TypeScript, Smalltalk, and Kotlin parity should land later against the same safety policy and
  round-trip behavior. The required parity gate is the committed `vectors/tar/` corpus plus
  files-profile-v2 import/export behavior; those engines must remain `no` here until
  they can preserve the same manifest metadata, refusal policy, and tar round-trip
  expectations.
- `dump` is a Rust-only inspection export that writes a versioned directory tree with folded
  N-Quads, JSONL tables, unfolded frame views, blob indexes, and files-profile payloads. It is
  not a wire-format change; Python, Go, TypeScript, Smalltalk, and Kotlin parity can implement the same
  `gts-dump-v1` directory contract later.
- All engines implement `verify-proof` for detached MMR proof JSON using the stable preimages and
  the positive/negative fixtures in `vectors/proofs/`. Rust additionally implements `prove` from
  files that carry a verified `index.mmr` root. Python, Go, TypeScript, Smalltalk, and Kotlin should not expose
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

## C ABI Wrapper Surface

The C ABI wrapper family is intentionally narrower than a native full engine. Wrappers delegate
format semantics to the Rust engine and make the stable ABI convenient from C-compatible
ecosystems:

| Surface | Contract |
|---|---|
| ABI metadata | `gts_abi_version`, `gts_version`, build metadata JSON, and capability JSON identify the loaded `libgts` surface. |
| Read/fold | `gts_read_json` returns a stable JSON report for folded archive state. |
| Verify | `gts_verify_json` returns the Rust verifier report as JSON. |
| N-Quads | `gts_to_nquads` and `gts_from_nquads` bridge folded RDF content and GTS bytes. |
| Files profile | `gts_files_pack`, `gts_files_unpack`, and `gts_files_diff_json` expose files-profile helpers. |
| Ownership | Returned `gts_buffer` values are copied into ecosystem-native strings or byte arrays, then released with `gts_buffer_free`. |
| Errors | Non-OK `gts_status` returns are copied from `gts_error` handles into structured ecosystem errors, then released with `gts_error_free`. |

Current wrappers are C++, .NET, PHP, Lua, Swift, Ruby, R, and Julia. Each wrapper README owns
its local naming, loader behavior, threading notes, and smoke-test command. The wrapper smoke
tests prove ABI reachability and ownership behavior; they are not substitutes for the six
full-engine conformance corpus.

## Files Profile Command Contract

`pack`, `unpack`, and `diff` are common commands in all six engines. Their observable behavior
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

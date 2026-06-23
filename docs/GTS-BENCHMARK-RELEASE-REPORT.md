<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Benchmark Release Report Template

Use this template for v1 release notes, release-candidate review, and paper appendix evidence.
Generate the filled report with:

```bash
just bench-release
```

For a release-candidate run, use all engines, at least three iterations, and commit the generated
artifact to the release evidence bundle rather than this source tree:

```bash
python scripts/bench_release_suite.py \
  --engines rust,python,go,ts,smalltalk \
  --iterations 5 \
  --vectors vectors/01-minimal.gts,vectors/23-files-profile-tree.gts,vectors/25b-streamable-compacted.gts \
  --out-dir dist/benchmarks/v1.0-rc1 \
  --strict
```

The runner writes:

- `release-benchmark-report.json` for machine-readable evidence;
- `release-benchmark-report.md` for release notes or appendix text;
- deterministic write and archive fixtures used by the run;
- per-engine products used to measure write, pack, and unpack paths.

By default, the runner writes a complete report even when selected engines fail or are
unavailable. Use `--strict` for release-candidate gating once failed rows are expected to block
the candidate.

## Required Release Metadata

| Field | Value |
|---|---|
| Release candidate | |
| Generated report path | |
| Runner command line | |
| Repository commit | |
| GTS spec commit | |
| GTS spec blob | |
| Conformance corpus commit | |
| Corpus manifest SHA-256 | |
| Platform | |
| CPU / memory | |
| Runner versions | |

## Benchmark Inputs

| Kind | Path | Bytes | SHA-256 | Notes |
|---|---|---:|---|---|
| Conformance vector | | | | read/fold |
| Conformance vector | | | | read/fold |
| Write fixture | | | | `from-nq` input |
| Archive fixture | | | | `pack`/`unpack` input |

## CLI Timing Summary

Use medians for release-note claims. Keep failed or skipped rows in the report so unavailable
engines are visible rather than silently omitted.

| Engine | Operation | Input | Iterations | Median ms | Min ms | Max ms | Output evidence |
|---|---|---|---:|---:|---:|---:|---|
| Rust | read-info | | | | | | |
| Rust | fold | | | | | | |
| Rust | write-from-nq | | | | | | |
| Rust | pack | | | | | | |
| Rust | unpack | | | | | | |
| Python | read-info | | | | | | |
| Python | fold | | | | | | |
| Python | write-from-nq | | | | | | |
| Python | pack | | | | | | |
| Python | unpack | | | | | | |
| Go | read-info | | | | | | |
| Go | fold | | | | | | |
| Go | write-from-nq | | | | | | |
| Go | pack | | | | | | |
| Go | unpack | | | | | | |
| TypeScript | read-info | | | | | | |
| TypeScript | fold | | | | | | |
| TypeScript | write-from-nq | | | | | | |
| TypeScript | pack | | | | | | |
| TypeScript | unpack | | | | | | |
| Smalltalk | read-info | | | | | | |
| Smalltalk | fold | | | | | | |
| Smalltalk | write-from-nq | | | | | | |
| Smalltalk | pack | | | | | | |
| Smalltalk | unpack | | | | | | |

## Streaming Memory Summary

Streaming-memory evidence is not directly comparable with CLI wall time. Cite it separately and
name the method used for each engine.

| Engine | Method | Input | Elapsed | Peak memory / allocation evidence | Notes |
|---|---|---|---:|---:|---|
| Python | full-reader materialization | | | | |
| Rust | `read_to_sink_from_reader` streaming fold | | | | |
| Go | `go test ./reader -bench ... -benchmem` | | | | |
| TypeScript | browser `foldStreamToSink` harness | | | | |

## Release-Note Excerpt

Benchmarks for `<release>` were run on `<platform>` at repository commit `<repo_commit>`,
spec commit `<spec_commit>`, and conformance corpus commit `<corpus_commit>`. The median
read/fold/write/pack/unpack timings are listed in `<report path>`. Streaming-memory evidence is
reported separately because the Rust helper reports process high-water RSS, the Go benchmark
reports runtime allocation metrics, and browser TypeScript memory must be captured from the
browser harness used for the release candidate.

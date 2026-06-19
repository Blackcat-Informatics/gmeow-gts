<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS Dump Directory

`gts dump <archive.gts> --directory <out-dir>` expands a GTS archive into a
versioned inspection directory. The first implementation is Rust-only; the
layout is intentionally language-neutral so other engines can follow the same
contract later.

The dump is an exploration and diagnostic surface, not a new wire format. It
duplicates useful views of the archive while avoiding duplicate large payload
bytes by default.

## Layout

```text
out/
├── README.md
├── .gts-dump/
│   ├── manifest.json
│   ├── heads.json
│   └── segments.json
├── graph/
│   ├── README.md
│   ├── folded.nq
│   └── tables/
│       ├── terms.jsonl
│       ├── quads.jsonl
│       ├── reifiers.jsonl
│       ├── annotations.jsonl
│       ├── meta.jsonl
│       ├── blob-meta.jsonl
│       ├── suppressions.jsonl
│       ├── opaque.jsonl
│       ├── signatures.jsonl
│       └── diagnostics.jsonl
├── frames/
│   ├── README.md
│   ├── inventory.jsonl
│   └── segments/
│       └── 0000/
│           ├── header.json
│           ├── folded.nq
│           ├── frame-0001.nq
│           └── *.jsonl
├── blobs/
│   └── index.jsonl
└── files/
    ├── entries.jsonl
    └── tree/
```

Directories are omitted when there is no corresponding archive content. For
example, `files/` is present only when the archive contains a valid files
profile catalog.

## Graph Views

`graph/folded.nq` is the authoritative RDF text projection for the folded
archive. N-Quads is the default because it is deterministic, line-oriented, and
can represent named graphs. Turtle is not emitted by default because it cannot
represent the whole folded RDF dataset without policy choices; TriG is a better
future explicit format for users who want a more readable RDF dataset syntax.

`graph/tables/*.jsonl` exposes the same folded state as simple line-oriented
tables. These are meant for shell tools, spreadsheets, DuckDB, Python notebooks,
and users who do not want to understand RDF serialization before inspecting the
archive.

## Unfolded Frames

`frames/inventory.jsonl` records segment and frame byte ranges, frame ids, frame
types, and validity. Each `frames/segments/NNNN/` directory contains the
per-segment folded N-Quads and decoded frame-level JSONL rows. `frame-*.nq`
files are emitted when a frame has RDF contributions that can be projected as
N-Quads.

The unfolded frame view answers a different question than `graph/`: it shows
what the append log contributed in order, while `graph/` shows the final folded
state.

## Payload Policy

The default is single-copy payload materialization:

- files-profile payloads are written under `files/tree/`;
- `blobs/index.jsonl` always records digest, size, media type, suppression
  state, and materialized paths;
- `blobs/by-digest/` is written only for blob payloads that are not already
  materialized through `files/tree/`;
- suppressed payloads are indexed but not materialized unless
  `--include-suppressed` is passed;
- `--metadata-only` writes graph, frame, manifest, and index files without
  extracting payload bytes.

The dump does not copy the original `.gts` file by default. The source path,
size, and digest are recorded in `.gts-dump/manifest.json`.

## Future Import

The schema name in `.gts-dump/manifest.json` is `gts-dump-v1`. Future `undump`
support should treat the manifest and materialized payload map as the import
contract. The current Rust command is one-way: it prepares the directory shape
for round-trip editing without claiming import support yet.

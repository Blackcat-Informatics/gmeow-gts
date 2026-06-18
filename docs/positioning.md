<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS positioning and architecture

GTS is the **Graph Transport Substrate**: an ontology-independent transport for RDF 1.2
graphs and the content-addressed binary assets those graphs reference. It is not a database,
query engine, reasoner, ontology, or mutation protocol. It is the narrow waist that lets
graphs move as deterministic, append-only, verifiable binary artifacts.

## Identity policy

- **Format and specification:** GTS / Graph Transport Substrate.
- **Project URL:** <https://blackcatinformatics.ca/projects/gts>.
- **Project DOI:** <https://doi.org/10.67342/umcdg7675h/v1>.
- **Repository and package family:** `gmeow-gts`.
- **CLI, import surface, and file extension:** `gts` and `.gts` where ecosystem rules permit.
- **Downstream relationship:** GTS is the primary distribution method for GMEOW, but GTS does
  not depend on GMEOW.

The `gmeow-gts` package name is a distribution identity. The format itself is profile-neutral:
a conformant reader does not need GMEOW vocabulary, OWL reasoning, music-domain rules, or any
other domain ontology to parse, verify, fold, or transport a GTS file.

## Narrow-waist architecture

```text
Applications and profiles
generic graphs | files | evidence | images | media packages | GMEOW | agent memory
|
v
GTS narrow waist
CBOR Sequence segments
deterministic-CBOR headers and frames
BLAKE3 id/prev chains
transform catalog
deterministic fold
opaque-node degradation
|
v
Storage and transport
filesystem | HTTP range | object storage | artifact registries | message buses
```

GTS keeps the waist small. Domain profiles can add vocabulary and validation rules above the
waist, and deployments can choose storage or distribution mechanisms below it, but neither side
changes the core header/frame grammar, content-id chain, or fold semantics.

## What GTS is and is not

| GTS is | GTS is not |
|---|---|
| A single-file graph/binary transport | A mutable database |
| A CBOR Sequence container | A SPARQL engine |
| An append-only event log | A reasoner |
| A content-addressed archival package | A consensus or sync protocol |
| A verifiable dataset distribution primitive | A trust framework |
| A profile-neutral narrow waist | A domain ontology |

Transforms such as N-Quads, SQLite, DuckDB, Parquet, or profile-specific tools make GTS useful
inside existing systems. Those transforms are operating substrates; the GTS file remains the
portable transport artifact.

## Applications

GTS supports several application families without making any of them the project frame:

- **Dataset and ontology distribution:** publish a verifiable graph package with the binary
  assets it names.
- **GMEOW distribution:** ship GMEOW ontology packages and profiles as GTS artifacts. This is a
  major downstream use case, not a core dependency.
- **Archives and file manifests:** package directory trees with graph-native metadata and
  content-addressed blobs.
- **Evidence and custody chains:** append observations, signatures, and sealed payloads without
  rewriting prior history.
- **Local-first graph synchronization:** concatenate independently produced segments and fold
  the value-union.
- **Agent memory:** model belief revision with suppression frames while preserving the original
  signed history.
- **Graph database interchange:** hand the folded graph state to N-Quads, SQLite, DuckDB, Parquet,
  or other transform targets.

## Cross-engine feature matrix

| Capability | Python | Rust | Go | TypeScript |
|---|---|---|---|---|
| Baseline read/fold/verify | yes | yes | yes | yes |
| Writer | yes | yes | yes | yes |
| Shared conformance corpus | yes | yes | yes | yes |
| COSE signing and verification | yes | yes | yes | yes |
| COSE Encrypt0 helpers | yes | yes | yes | yes |
| Files profile `pack`/`unpack`/`diff` | yes | yes | yes | yes |
| Streamable compaction CLI | yes | yes | yes | yes |
| `from-nq` inverse | yes | yes | yes | yes |
| SQLite/DuckDB/Parquet exports | yes | yes | no | no |
| Package registry | PyPI | crates.io | Go module | npm |

This matrix is descriptive, not a conformance claim. The authoritative compatibility check is
the versioned vector corpus under `vectors/`, which every engine gates against. Command-level
parity and intentional gaps are maintained in
[`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md).
Advanced streaming, proof, and replication work is tiered separately in
[`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md).

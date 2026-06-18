<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<!-- gts-ecosystem-contract:v1 -->
# GTS Ecosystem Integration Contract

This document is the public contract for using GTS with RDF libraries, data
frames, browsers, services, and object stores. The core wire format remains
normative in [GTS-SPEC.md](./GTS-SPEC.md); this document records what the
current engines expose, what examples are supported, and what is explicitly
deferred.

## Status Matrix

| Ecosystem | Current integration path | Deferrals |
|---|---|---|
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` and `gmeow_gts::from_nquads::from_nquads(text)` remain the zero-extra-dependency bridge; `--features rdf` enables `gmeow_gts::rdf::{to_oxrdf_dataset, from_oxrdf_dataset}` for native `oxrdf::Dataset` interop without an embedded graph store; `--features oxigraph-adapter` enables `gmeow_gts::oxigraph::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}` and `Writer::from_store` using Oxigraph's in-memory store; `gmeow_gts::examples::agent_memory` demonstrates a downstream application shape without extra dependencies; `gts to-sqlite` exports the folded integer table model by default, while `to-duckdb` and `to-parquet` are behind the no-dependency Cargo feature `duckdb`. | Sophia and Rio adapters remain deferred until they can be optional features with round-trip tests and no mandatory database dependency. |
| Python RDF/data | `gts.from_rdflib()` and `gts.to_rdflib()` cover rdflib RDF 1.1 `Graph`/`Dataset` interop; `gts to-sqlite`, `to-duckdb`, and `to-parquet` cover relational/data-frame handoff. | RDF 1.2 quoted-triple export to rdflib is strict-by-default and lossy only when explicitly requested. |
| TypeScript browser | Current browser-safe handoff is `Uint8Array`: `fetch()`, optional HTTP `Range`, then `Read(bytes, allowSegments)`, `toNQuads`, or files helpers. | A package-level browser bundle, `ReadableStream` fold API, WebCrypto key provider, and progressive rendering API are deferred. |
| Go services | `reader.ReadFrom(ctx, io.Reader, reader.Options)` provides cancellation, byte limits, and ordinary `io.Reader` integration for HTTP bodies, object-store objects, and pipes; the Go CLI also exposes the shared replication inventory verbs. | True streaming fold and service-specific replication orchestration remain deferred to the advanced-primitives contract. |

## Python: rdflib And Data Frames

The Python package owns the richest ecosystem bridge because it already carries
optional extras for RDF and database targets:

```bash
pip install 'gmeow-gts[rdf,db]'
```

RDF 1.1 dataset round-trip:

```python
import gts
from rdflib import Dataset, Literal, URIRef
from rdflib.namespace import RDFS

ds = Dataset()
graph = ds.graph(URIRef("https://example.org/graph"))
graph.add((
    URIRef("https://example.org/Cat"),
    RDFS.label,
    Literal("Cat", lang="en"),
))

data = gts.from_rdflib(ds)
folded = gts.read(data)
assert sorted(gts.to_nquads(folded).splitlines()) == sorted(
    ds.serialize(format="nquads").splitlines()
)

back = gts.to_rdflib(folded)
```

RDF 1.2 limitation:

- rdflib's stable RDF 1.1 dataset parser does not faithfully represent GTS
  quoted-triple terms or `rdf:reifies <<( ... )>>` syntax.
- `gts.to_rdflib(graph)` raises `RDF12UnsupportedError` when the N-Quads
  projection contains quoted triples.
- `gts.to_rdflib(graph, allow_rdf12_lossy=True)` drops N-Quads lines containing
  quoted-triple syntax and parses the remaining RDF 1.1-compatible graph.

Relational/data-frame handoff is exposed by Python and Rust:

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

Python DuckDB and Parquet exports require `pip install 'gmeow-gts[db]'`. Rust uses `sqlite3`
for SQLite by default. Rust DuckDB/Parquet exports are available when built with
`--features duckdb`; they add no Rust crate dependencies and shell out to `duckdb` on `PATH`.

Performance expectation: these exports use the integer-id folded model. `terms`,
`quads`, `reifiers`, `annotations`, and `blobs` are bulk-loaded without resolving
IRIs during export; consumers join through the `terms` table. The Rust path writes rows
incrementally to `sqlite3`/`duckdb`, so it does not retain all SQL rows or a full load
script at once. The `blobs` table still preserves payload bytes, so transformed inline
blob payloads are decoded transiently when their row is emitted. SQLite is adequate
for small local inspection. DuckDB and Parquet are the preferred path for Pandas,
Polars, DuckDB SQL, and Arrow-style scans because they keep the dictionary
encoding and let the target engine choose projection/filter order.

## Rust: RDF Crates

Current Rust interop keeps the default crate explicit and low-dependency:

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let nquads = gmeow_gts::nquads::to_nquads(&graph);
```

Applications can feed `nquads` to Sophia, Rio, Oxigraph, or other RDF crates.
This is the stable bridge for v1 because the core crate should not force a graph
database or RDF toolkit into every transport user.

The inverse pure-graph path is also explicit:

```rust
let bytes = gmeow_gts::from_nquads::from_nquads(nquads.as_str())?;
```

For native Rust data-model interop, enable the optional `rdf` feature:

```toml
gmeow-gts = { version = "0.2.0", default-features = false, features = ["rdf"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let dataset = gmeow_gts::rdf::to_oxrdf_dataset(&graph)?;
let bytes = gmeow_gts::rdf::from_oxrdf_dataset(&dataset)?;
```

The `rdf` feature uses `oxrdf`, the RDF data-structure crate from the Oxigraph
ecosystem. It deliberately does not depend on the `oxigraph` store. `oxrdf`
keeps empty default features and a repo-compatible `MIT OR Apache-2.0` license,
which makes it the smallest practical native dataset/quad adapter for this
crate.

For native Oxigraph store interop, enable the heavier optional adapter:

```toml
gmeow-gts = { version = "0.2.0", default-features = false, features = ["oxigraph-adapter"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let package = gmeow_gts::oxigraph::graph_to_store_with_sidecar(graph)?;
let writer = gmeow_gts::writer::Writer::from_store(&package.store, "dist")?;
```

The `oxigraph-adapter` feature depends on `rdf` and `oxigraph` with Oxigraph's
default RocksDB feature disabled. The store projection is pure RDF; GTS-only state such
as blobs, suppressions, signatures, diagnostics, segment heads, and streamable-layout
metadata is returned in a sidecar. The adapter walks native quads and does not materialize
N-Quads text in the hot path.

Strict export is the default. GTS reifiers project to RDF 1.2 triple terms in
object position when `oxrdf` can represent them. If a GTS graph uses quoted
triples in positions `oxrdf` cannot represent, such as subject or graph-name
position, `to_oxrdf_dataset` raises `RdfAdapterError`. The explicit
`to_oxrdf_dataset_lossy` path drops only those unrepresentable rows and is
covered by feature-gated tests.

For application parity, the Rust crate includes a runnable grounded-memory
example:

```bash
cargo run --example agent_memory
```

`gmeow_gts::examples::agent_memory::Memory` appends claims, revises or suppresses
claims, records tool-call provenance, recalls claims with deterministic token
overlap, and produces packages accepted by `gts verify`. This is an application
example on top of GTS, not a prerequisite for core readers.

Rust data-frame handoff uses the same folded tables as the Python exports:

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

The Rust binary keeps these as runtime tool integrations rather than default crate dependencies:
`to-sqlite` invokes `sqlite3` in the default build, while `to-duckdb` and `to-parquet`
are enabled by the no-dependency `duckdb` Cargo feature and invoke the external `duckdb`
binary. The Rust loader streams row SQL to those tools and stages output replacement; it does not
build the complete row set or SQL script in memory.

Tracked deferral: additional native Rust RDF adapters should be added only as
optional features. A future Sophia or Rio adapter must include:

- GTS -> RDF and RDF -> GTS round-trip tests for IRIs, blank nodes, language
  literals, datatypes, named graphs, and RDF 1.2 reifier limitations.
- No default dependency on an embedded database.
- Clear behavior for quoted triples when the target crate cannot preserve them.

## TypeScript: Browser And Range Fetch

The TypeScript engine exposes byte-oriented APIs that are safe to call from
browser or service code once the caller supplies bytes:

```typescript
import { Read, toNQuads } from "@blackcatinformatics/gmeow-gts";

const response = await fetch("/artifacts/example.gts", {
  headers: { Range: "bytes=0-65535" },
});
const bytes = new Uint8Array(await response.arrayBuffer());
const graph = Read(bytes, false);
console.log(toNQuads(graph));
```

Range rule: callers may use HTTP `Range` only for byte spans that are known from
an index frame or from a sequential CBOR boundary scan. A range that cuts through
a CBOR item is a torn append and must be treated as an incomplete prefix.

Tracked deferrals:

- `ReadableStream<Uint8Array>` folding without full materialization.
- A browser conditional export with dependency choices audited for bundlers.
- WebCrypto-backed key-provider integration for COSE verification/decryption.
- Progressive rendering helpers that report graph/blob arrivals as UI events.

Until those exist, the npm package must not claim the `GTS Streaming Reader`
conformance tier.

## Go: Services And Object Stores

Go callers should use `reader.ReadFrom` at service boundaries:

```go
func handleGTS(w http.ResponseWriter, r *http.Request) {
    graph, err := reader.ReadFrom(r.Context(), r.Body, reader.Options{
        AllowSegments: true,
        MaxBytes:      64 << 20,
    })
    if err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }
    _, _ = io.WriteString(w, nquads.ToNQuads(graph))
}
```

The same API works for object-store SDK readers:

```go
obj, err := client.GetObject(ctx, bucket, key)
if err != nil {
    return nil, err
}
defer obj.Body.Close()

graph, err := reader.ReadFrom(ctx, obj.Body, reader.Options{
    AllowSegments: true,
    ExpectedHead:  expectedHead,
    MaxBytes:      512 << 20,
})
```

`ReadFrom` is intentionally a bounded full-reader wrapper. It gives Go services
idiomatic cancellation and resource limits today without claiming incremental
folding. The returned graph still carries reader diagnostics instead of turning
format diagnostics into Go errors.

## Replication And Service Boundaries

For current services:

- Use `gts heads` / `gts segments` in any engine to inventory segment heads and byte ranges.
- Use `gts ls` or folded `Graph.Blobs`/`BlobMeta` to inventory inline objects.
- Use the range rules above when serving byte ranges from HTTP or object stores.
- `gts missing` and `gts resume` provide the stable byte-range resume surface in every engine.
  Higher-level service-to-service protocols remain application code built on the
  [GTS-ADVANCED-PRIMITIVES.md](./GTS-ADVANCED-PRIMITIVES.md) JSON shapes and boundary rules.

## Contract Guard

`scripts/check_ecosystem_contract.py` verifies that this document keeps the
status matrix, per-ecosystem sections, deferral language, and public doc links.
It is a drift guard for integration promises, not a substitute for engine tests.

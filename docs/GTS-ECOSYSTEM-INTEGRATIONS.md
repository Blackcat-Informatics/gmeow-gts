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
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` and `gmeow_gts::from_nquads::from_nquads(text)` remain the zero-extra-dependency bridge; `--features rdf` enables `gmeow_gts::rdf::{to_oxrdf_dataset, from_oxrdf_dataset}` for native `oxrdf::Dataset` interop without an embedded graph store; `--features oxigraph-adapter` enables `gmeow_gts::oxigraph::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}` and `Writer::from_store` using Oxigraph's in-memory store; `--features sophia-adapter` enables `gmeow_gts::sophia::{to_sophia_dataset, from_sophia_dataset}` using Sophia's in-memory dataset and N-Quads parser/serializer; `gmeow_gts::examples::agent_memory` demonstrates a downstream application shape without extra dependencies; `gts to-sqlite` exports the folded integer table model by default, while `to-duckdb` and `to-parquet` are behind the no-dependency Cargo feature `duckdb`. | Rio remains deferred because the current `rio_api` crate is marked unmaintained upstream; the zero-dependency N-Quads bridge remains the Rio-compatible path. |
| Python RDF/data | `gts.from_rdflib()` and `gts.to_rdflib()` cover rdflib RDF 1.1 `Graph`/`Dataset` interop; `gts to-sqlite`, `to-duckdb`, and `to-parquet` cover relational/data-frame handoff. | RDF 1.2 quoted-triple export to rdflib is strict-by-default and lossy only when explicitly requested. |
| TypeScript browser | `@blackcatinformatics/gmeow-gts/browser` exposes `foldStream(ReadableStream<Uint8Array>, options)`, `readStream`, `toNQuads`, progressive fold events, and WebCrypto-backed COSE Sign1/Encrypt0 key-provider hooks. The package root also carries a browser condition that resolves to this narrower surface for bundlers. | This is a progressive Web Streams surface and does not satisfy the current non-materializing Streaming Reader tier. Node-only CLI and filesystem `pack`/`unpack`/`diff` helpers remain outside the browser export. Range fetch still needs a verified index or boundary scan. |
| Go services | `reader.ReadFrom(ctx, io.Reader, reader.Options)` provides graph-returning service integration, while `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` provides cancellation-aware, byte-limited streaming fold events for HTTP bodies, object-store objects, and pipes; the Go CLI also exposes the shared replication inventory verbs. | Service-specific replication orchestration remains application code built on the shared verbs. |
| OKF bundles | Rust `gts from-okf` and `gts to-okf` are available behind `--features okf`. They turn Markdown + YAML-frontmatter OKF bundles into GTS profile `okf` packages and project OKF-profile graphs back to bundle directories. | Broader OKF sample corpus and Python/Go/TypeScript parity are tracked as follow-up work. |

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
gmeow-gts = { version = "0.9.1", default-features = false, features = ["rdf"] }
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
gmeow-gts = { version = "0.9.1", default-features = false, features = ["oxigraph-adapter"] }
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

For native Sophia dataset interop, enable the optional Sophia adapter:

```toml
gmeow-gts = { version = "0.9.1", default-features = false, features = ["sophia-adapter"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let dataset = gmeow_gts::sophia::to_sophia_dataset(&graph)?;
let bytes = gmeow_gts::sophia::from_sophia_dataset(&dataset)?;
```

The `sophia-adapter` feature depends on Sophia's `sophia_inmem` dataset and
`sophia_turtle` N-Quads parser/serializer. It deliberately stays out of default
builds. The bridge follows the existing GTS N-Quads projection, so it preserves
IRIs, blank nodes, language literals, datatypes, named graphs, and RDF 1.2
`rdf:reifies`/quoted-triple terms that Sophia can parse and serialize. GTS-only
state such as blobs, signatures, suppressions, opaque frames, diagnostics, and
segment metadata is outside this pure RDF projection. Rio remains deferred
because `rio_api` 0.8.6 is marked unmaintained upstream; callers that need Rio
can continue to use the zero-dependency N-Quads text bridge.

Strict export is the default. GTS reifiers project to RDF 1.2 triple terms in
object position when `oxrdf` can represent them. If a GTS graph uses quoted
triples in positions `oxrdf` cannot represent, such as subject or graph-name
position, `to_oxrdf_dataset` raises `RdfAdapterError`; Sophia's parser likewise
rejects N-Quads shapes that RDF 1.2 concrete syntax cannot preserve. The
explicit `to_oxrdf_dataset_lossy` path drops only those unrepresentable rows and
is covered by feature-gated tests.

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
optional features. Rio remains deferred until a maintained Rio-compatible crate
or replacement path is selected. Any future adapter must include round-trip
tests for IRIs, blank nodes, language literals, datatypes, named graphs, and RDF
1.2 reifier limitations, must add no default dependency on an embedded database,
and must document quoted-triple behavior when the target crate cannot preserve
it.

## TypeScript: Browser And Range Fetch

The TypeScript package exposes a browser-specific entrypoint for Web Streams:

```typescript
import { foldStream, readStream, toNQuads } from "@blackcatinformatics/gmeow-gts/browser";

const response = await fetch("/artifacts/example.gts");
const result = await foldStream(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") renderQuad(event.quad);
    if (event.kind === "blob") renderBlob(event.digest, event.size);
  },
});

console.log(toNQuads(result.graph));
```

The browser path can also use platform WebCrypto for practical COSE verification and
decryption:

```typescript
const graph = await readStream(response.body!, {
  keys: {
    verificationKey: (kid) => lookupEd25519PublicKey(kid),
    contentKey: (kid) => lookupAes256GcmContentKey(kid),
  },
});
```

The browser export emits term, quad, reifier, annotation, suppression, blob, opaque,
signature, diagnostic, segment-head, and streamable-layout events in frame order. It is the
TypeScript package's browser-safe progressive stream surface and does not satisfy the current
non-materializing `GTS Streaming Reader` tier requirements. The root Node
`Read(bytes, allowSegments)` API remains materializing, and browser code must not rely on the
Node-only CLI/filesystem helpers.

Range rule: callers may use HTTP `Range` only for byte spans that are known from
an index frame or from a sequential CBOR boundary scan. A range that cuts through
a CBOR item is a torn append and must be treated as an incomplete prefix.

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
idiomatic cancellation and resource limits when the caller wants a materialized
`*model.Graph`. The returned graph still carries reader diagnostics instead of
turning format diagnostics into Go errors.

For streaming folds, callers can send segment-local fold events to a sink without
constructing the final union graph:

<!-- markdownlint-disable MD010 -->
```go
var sink reader.StreamingSink = reader.StreamingSinkFunc(func(event reader.StreamingEvent) error {
	if event.Kind == reader.StreamingEventQuad {
		// project or forward event.Quad here
	}
	return nil
})

result, err := reader.ReadToSink(ctx, obj.Body, reader.Options{
	AllowSegments: true,
	ExpectedHead:  expectedHead,
	MaxBytes:      512 << 20,
}, sink)
```
<!-- markdownlint-enable MD010 -->

`result.Diagnostics`, `result.SegmentHeads`, and `result.SegmentStreamable`
match the full reader for the same input and options.

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

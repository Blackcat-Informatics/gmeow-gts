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
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` projects folded data into RDF text that Sophia, Rio, Oxigraph, and other crates can parse today. | Native Sophia/Rio/Oxigraph adapters are deferred until they can be optional features with round-trip tests and no mandatory database dependency. |
| Python RDF/data | `gts.from_rdflib()` and `gts.to_rdflib()` cover rdflib RDF 1.1 `Graph`/`Dataset` interop; `gts to-sqlite`, `to-duckdb`, and `to-parquet` cover relational/data-frame handoff. | RDF 1.2 quoted-triple export to rdflib is strict-by-default and lossy only when explicitly requested. |
| TypeScript browser | Current browser-safe handoff is `Uint8Array`: `fetch()`, optional HTTP `Range`, then `Read(bytes, allowSegments)`, `toNQuads`, or files helpers. | A package-level browser bundle, `ReadableStream` fold API, WebCrypto key provider, and progressive rendering API are deferred. |
| Go services | `reader.ReadFrom(ctx, io.Reader, reader.Options)` provides cancellation, byte limits, and ordinary `io.Reader` integration for HTTP bodies, object-store objects, and pipes. | True streaming fold and service-to-service replication verbs remain deferred to the advanced-primitives contract. |

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

Relational/data-frame handoff:

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

Performance expectation: these exports use the integer-id folded model. `terms`,
`quads`, `reifiers`, `annotations`, and `blobs` are bulk-loaded without resolving
IRIs during export; consumers join through the `terms` table. SQLite is adequate
for small local inspection. DuckDB and Parquet are the preferred path for Pandas,
Polars, DuckDB SQL, and Arrow-style scans because they keep the dictionary
encoding and let the target engine choose projection/filter order.

## Rust: RDF Crates

Current Rust interop is explicit and low-dependency:

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let nquads = gmeow_gts::nquads::to_nquads(&graph);
```

Applications can feed `nquads` to Sophia, Rio, Oxigraph, or other RDF crates.
This is the stable bridge for v1 because the core crate should not force a graph
database or RDF toolkit into every transport user.

Tracked deferral: native Rust RDF adapters should be added only as optional
features. A future `sophia`/`rio` adapter must include:

- GTS -> RDF and RDF -> GTS round-trip tests for IRIs, blank nodes, language
  literals, datatypes, named graphs, and RDF 1.2 reifier limitations.
- No default dependency on Oxigraph or an embedded database.
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
obj, _ := client.GetObject(ctx, bucket, key)
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

- Use `gts info` or `reader.ReadFileSegments` to inventory segment heads.
- Use `gts ls` or folded `Graph.Blobs`/`BlobMeta` to inventory inline objects.
- Use the range rules above when serving byte ranges from HTTP or object stores.
- Treat proof/MMR and replication verbs as deferred until
  [GTS-ADVANCED-PRIMITIVES.md](./GTS-ADVANCED-PRIMITIVES.md) promotes stable
  `heads`, `segments`, `missing`, and `resume` semantics.

## Contract Guard

`scripts/check_ecosystem_contract.py` verifies that this document keeps the
status matrix, per-ecosystem sections, deferral language, and public doc links.
It is a drift guard for integration promises, not a substitute for engine tests.

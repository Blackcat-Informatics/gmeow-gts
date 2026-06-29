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
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` and `gmeow_gts::from_nquads::from_nquads(text)` remain the zero-extra-dependency bridge for external RDF crates; `--features rdf` enables `gmeow_gts::rdf::{to_rdf_dataset, from_rdf_dataset}` for dependency-free native `Dataset` interop without an embedded graph store; `--features native-store` enables `gmeow_gts::native_store::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}` and `Writer::from_store` using the deterministic native in-memory RDF store; `--features rdf-codecs` enables native N-Triples, Turtle, TriG, and RDF/XML text codecs; `gmeow_gts::examples::agent_memory` demonstrates a downstream application shape without extra dependencies; `gts to-sqlite` exports the folded integer table model by default, while `to-duckdb` and `to-parquet` are behind the no-dependency Cargo feature `duckdb`. | Rio remains deferred because the current `rio_api` crate is marked unmaintained upstream; external Sophia/Oxigraph/Rio interop uses the zero-dependency N-Quads text bridge rather than an in-crate adapter. |
| Python RDF/data | `gts.from_rdflib()` and `gts.to_rdflib()` cover rdflib RDF 1.1 `Graph`/`Dataset` interop; `gts to-sqlite`, `to-duckdb`, and `to-parquet` cover relational/data-frame handoff. | RDF 1.2 quoted-triple export to rdflib is strict-by-default and lossy only when explicitly requested. |
| TypeScript browser | `@blackcatinformatics/gmeow-gts/browser` exposes `foldStreamToSink(ReadableStream<Uint8Array>, options)` for the non-materializing Streaming Reader tier, plus graph-returning `foldStream`, `readStream`, `toNQuads`, progressive fold events, and WebCrypto-backed COSE Sign1/Encrypt0 key-provider hooks. The package root also carries a browser condition that resolves to this narrower surface for bundlers. | Node-only CLI and filesystem `pack`/`unpack`/`diff` helpers remain outside the browser export. Range fetch still needs a verified index or boundary scan. |
| Go services | `reader.ReadFrom(ctx, io.Reader, reader.Options)` provides graph-returning service integration, while `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` provides cancellation-aware, byte-limited streaming fold events for HTTP bodies, object-store objects, and pipes; the Go CLI also exposes the shared replication inventory verbs. | Service-specific replication orchestration remains application code built on the shared verbs. |
| C ABI wrappers | `rust/capi/` builds `libgts` and `rust/capi/include/gts.h` for C-compatible runtimes. C++, .NET, PHP, Lua, Swift, Ruby, R, and Julia wrappers expose ABI metadata, read/verify JSON reports, registry-driven RDF text conversion, files-profile helpers, and structured errors while copying native buffers into ecosystem-owned values. | These wrappers delegate to the Rust engine and are not independent parity engines or new CLI columns. Installable native `libgts` archives publish through the `capi-v*` GitHub Release lane; wrapper registry release automation remains separate from the current Rust/Python/Go/TypeScript engine release lanes. |
| Tar-compatible archives | Rust `gts from-tar`, `gts to-tar`, and `gts tar -c/-x/-t/-d` are available behind `--features tar`. They bridge `.tar`, `.tar.gz`, and `.tar.zst` streams to files-profile-v2 GTS archives with digest-addressed file bodies, tar-equivalent metadata, unknown PAX preservation, and explicit extraction opt-ins. | Python/Go/TypeScript parity is intentionally deferred. Those engines should implement files-profile-v2 import/export and pass `vectors/tar/` before their CLIs claim `from-tar`, `to-tar`, or `tar`. |
| OKF bundles | Rust `gts from-okf` and `gts to-okf` are available behind `--features okf`. They turn Markdown + YAML-frontmatter OKF bundles into GTS profile `okf` packages and project OKF-profile graphs back to bundle directories. The committed corpus includes a BigQuery-style bundle under `vectors/okf/bigquery-join/`, including frontmatter-less navigation `index.md` pages matching Google's checked-in Knowledge Catalog samples. | Python/Go/TypeScript parity is intentionally deferred. Those engines should implement the same `gts-okf-v1` directory contract and pass the OKF corpus before their CLIs claim `from-okf` or `to-okf`. |

## C ABI Wrapper Contract

The C ABI compatibility policy lives in
[`rust/capi/README.md#compatibility-policy`](../rust/capi/README.md#compatibility-policy).
`GTS_ABI_VERSION` governs the native `gts.h`/`libgts` boundary and is separate
from package versions and JSON report schema versions. Wrapper packages must
reject unsupported ABI versions with a clear wrapper error, exception, or
install/configure failure rather than silently continuing with an unknown
native contract.

Files-profile path helpers use the ABI v1 NUL-terminated UTF-8 C-string path
contract. Wrapper docs must not present those helpers as full Windows
wide-character path coverage; future wide-character path functions should be
new additive C ABI symbols under the compatibility policy.

## Tar-Compatible Archive Bridge

The Rust tar bridge makes GTS usable as a signed, append-only, deduplicated
archive surface for users who already understand tar. `gts from-tar` imports
tar streams into files-profile-v2 GTS archives, `gts to-tar` exports those
archives back to tar, and `gts tar -c/-x/-t/-d` provides the familiar
create/extract/list/diff command shape. The bridge handles plain `.tar`,
`.tar.gz`, and `.tar.zst` streams, preserving tar-equivalent metadata and
unknown PAX records where the profile can represent them.

For large archives, the Rust import/create paths avoid resident memory scaling
with regular-file payload bytes on the direct GTS authoring paths:
`gts from-tar` decodes tar input as a stream, spools regular-file bodies while
collecting sorted metadata, and emits blob frames from bounded chunks; `gts tar
-cf out.gts ...` hashes and writes source file payloads in bounded chunks. The
folded `to-tar` path still exports from the in-memory `Graph` representation,
and `.tar.zst` output still uses the current zstd backend path that materializes
the encoded projection. Those are implementation boundaries, not format
requirements.

The canonical artifact should be the `.gts` file when verification matters:
frame ids, optional signatures, append-only revisions, suppressions, and
content-addressed blobs remain visible to GTS readers. Conventional `.tar`,
`.tar.gz`, and `.tar.zst` outputs are useful compatibility projections for
toolchains that do not speak GTS yet, but they should be treated as derived
exports when the signed GTS chain is the evidence record.

Artifact registries and object stores can carry the GTS archive directly using
`application/vnd.blackcat.gts+cbor-seq`. OCI or release-asset publishers can
ship the `.gts` artifact alongside generated tar projections: the registry gets
a single content-addressed archive for GTS-aware consumers, while existing tar
consumers keep a familiar download path. The same split works for OKF bundles:
the editable OKF directory remains the human authoring surface, `gts from-okf`
creates the semantic `okf` profile package, and the files-profile-v2 tar bridge
can package the directory bytes as a verifiable tarball-shaped distribution
artifact when consumers need ordinary archive tooling.

## OKF: Knowledge Catalog And BigQuery Bundles

OKF interop has two useful gates:

- The committed, hermetic gate is `vectors/okf/bigquery-join/`. It models
  BigQuery datasets, tables, table joins, extension frontmatter, Markdown links,
  and navigation `index.md` files without depending on Google credentials or
  upstream sample drift.
- The live ecosystem gate is
  <https://github.com/GoogleCloudPlatform/knowledge-catalog>. Its `okf/bundles/`
  samples are produced by the Knowledge Catalog OKF enrichment proof of concept,
  and its visualizer consumes the same Markdown + YAML-frontmatter directory
  surface.

The Rust command sequence for either gate is:

```bash
cargo run --features okf --bin gts -- from-okf okf-bundle/ -o bundle.gts
cargo run --bin gts -- verify bundle.gts
cargo run --features okf --bin gts -- to-okf bundle.gts --directory restored-okf/
```

`from-okf` imports concept documents with YAML frontmatter and treats
frontmatter-less `index.md` files as navigation pages. Those pages are not
concepts in the GTS profile, so they are not emitted by `to-okf`; consumers that
need static browse pages can regenerate them from the exported concept set.

The bridge positions OKF as a human-authoring front end for GMEOW knowledge:
people and agents edit Markdown, while GTS supplies append-only packaging,
content-addressed bodies, signatures, suppressions, and graph projections for
audit and machine use.

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
gmeow-gts = { version = "0.9.11", default-features = false, features = ["rdf"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let dataset = gmeow_gts::rdf::to_rdf_dataset(&graph)?;
let bytes = gmeow_gts::rdf::from_rdf_dataset(&dataset)?;
```

The `rdf` feature uses GTS-native RDF dataset, quad, term, graph-name, literal,
and quoted-triple types. It deliberately does not depend on the `oxrdf` crate or
an external RDF store, so `--features rdf` remains suitable for
`wasm32-unknown-unknown` builds.

For native in-memory RDF store interop, enable the optional native store:

```toml
gmeow-gts = { version = "0.9.11", default-features = false, features = ["native-store"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let package = gmeow_gts::native_store::graph_to_store_with_sidecar(graph)?;
let writer = gmeow_gts::writer::Writer::from_store(&package.store, "dist")?;
```

The `native-store` feature depends only on `rdf`. The store projection is pure
RDF; GTS-only state such as blobs, suppressions, signatures, diagnostics,
segment heads, and streamable-layout metadata is returned in a sidecar. The
adapter walks native quads and does not materialize N-Quads text in the hot path.

For Sophia, Oxigraph, Rio, or other external RDF crates, keep the dependency at
the application boundary and exchange N-Quads text with GTS. The core Rust crate
does not publish an in-crate Sophia adapter, because Sophia's N-Quads stack pulls
UUID generation into the all-features dependency graph. The native `rdf`,
`native-store`, and `rdf-codecs` features cover the in-crate structured and text
interop paths while preserving `wasm32-unknown-unknown` builds.
CI also treats all-features wasm as a permanent Rust library contract:
`scripts/check_rust_wasm_dependency_audit.py` checks the
`wasm32-unknown-unknown --all-features` normal/build dependency tree and fails if
Oxigraph/OxRDF/OxTTL/OxRDFXML, Sophia crates, `uuid`, or `getrandom` 0.3 return.

Strict export is the default. GTS reifiers project to RDF 1.2 triple terms in
object position. If a GTS graph uses quoted triples in positions the native
dataset surface intentionally does not represent, such as subject or graph-name
position, `to_rdf_dataset` raises `RdfAdapterError`. The explicit
`to_rdf_dataset_lossy` path drops only those unrepresentable rows and is covered
by feature-gated tests.

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
import { foldStream, foldStreamToSink, readStream, toNQuads } from "@blackcatinformatics/gmeow-gts/browser";

const response = await fetch("/artifacts/example.gts");
const result = await foldStream(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") renderQuad(event.quad);
    if (event.kind === "blob") renderBlob(event.digest, event.size);
  },
});

console.log(toNQuads(result.graph));

await foldStreamToSink(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") projectRow(event.segmentIndex, event.quad);
  },
});
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

The browser export emits term, quad, reifier, annotation, suppression, blob, opaque, signature,
diagnostic, segment-head, and streamable-layout events in frame order. `foldStreamToSink` is the
TypeScript package's non-materializing `GTS Streaming Reader` surface; `foldStream` and
`readStream` remain graph-returning conveniences. The root Node `Read(bytes, allowSegments)` API
remains a materializing reader, and browser code must not rely on the Node-only CLI/filesystem
helpers.

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

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS: A Content-Addressed Append-Only Transport Substrate For RDF Graphs And Binary Artifacts

Draft paper narrative for the Graph Transport Substrate (GTS).

This document is informative research material. It does not define normative GTS behavior.
Normative requirements remain in [`GTS-SPEC.md`](./GTS-SPEC.md), with testable tier and vector
rules in [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md), trust/profile policy in
[`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md), and change control in
[`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md).

## Abstract

RDF datasets are routinely exchanged through text serializations, database exports, ad hoc
archives, and application-specific package formats. Those mechanisms are useful, but they do
not provide a small common substrate for append-only graph history, content-addressed binary
payloads, partial readability, and cross-language conformance. GTS addresses that gap by
encoding RDF 1.2 graph state and referenced binary assets as a CBOR Sequence of deterministic
segments and frames. Each segment is folded into a graph state, frames are linked by BLAKE3
content identifiers, and unsupported or inaccessible payloads degrade to opaque graph nodes
rather than disappearing. The current repository hosts six reference engines in Rust, Python,
Go, TypeScript, Smalltalk/Pharo, and Kotlin/JVM, plus a shared vector manifest and corpus used
to compare fold results and diagnostics. GTS is not a database, reasoner, ontology, or
consensus protocol; it is a narrow waist for durable, verifiable graph transport.

## 1. Introduction

Graph data now crosses local-first applications, provenance workflows, evidence packages,
archives, service boundaries, and AI memory systems. A transport artifact for those settings
has to move more than triples: it has to preserve binary payloads, append history without
rewriting older bytes, survive missing codecs or keys, and allow independent implementations to
agree on what the bytes mean.

GTS frames that problem as transport rather than storage. The durable artifact is a `.gts`
file served as `application/vnd.blackcat.gts+cbor-seq`: a CBOR Sequence whose logical dataset
is produced by a deterministic fold. Query systems, databases, object stores, caches, and
domain profiles sit around the artifact, not inside the core format.

The intended contributions of the work are:

1. A CBOR Sequence wire format for append-only graph and binary logs.
2. A deterministic fold model for RDF 1.2 terms, quads, reifiers, annotations, blobs,
   metadata, suppression, snapshots, and opaque frames.
3. A content-addressed id/prev chain with optional COSE signatures, optional encryption, and
   an opacity model for missing capabilities.
4. Multi-segment composition by byte concatenation plus streamable layout compaction for
   delivery-oriented artifacts.
5. A cross-language conformance corpus and reference implementations in Rust, Python, Go,
   TypeScript, Smalltalk/Pharo, and Kotlin/JVM.

## 2. Design Overview

GTS is designed as a narrow waist:

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

The core format does not commit to an ontology, database, query engine, mutable transaction
model, or trust framework. Domain profiles add vocabulary and validation above the waist.
Deployments choose storage and serving behavior below it. Neither side changes the core
header/frame grammar, content-id preimages, segment-boundary rules, or fold semantics.

The current package family is named `gmeow-gts`; the format is GTS. GMEOW is a primary
downstream consumer and distribution use case, but the dependency direction is one-way: a GTS
reader does not need GMEOW vocabulary, OWL reasoning, music-domain rules, or agent-memory
conventions to parse, verify, fold, or transport a GTS file.

## 3. Wire Format

A GTS file is a CBOR Sequence of one or more segments. A segment contains a deterministic CBOR
header followed by deterministic CBOR frames. The registered provisional media type used by
published artifacts is `application/vnd.blackcat.gts+cbor-seq`, and the file extension is
`.gts`.

At the narrative level, the file structure is:

```text
GTS file
  segment 0
    header: magic/version/profile/catalog/layout/metadata/id
    frame:  type + transform chain + public envelope + payload + prev + id + optional sig
    frame:  ...
  segment 1
    header
    frame
    ...
```

Each frame's content identifier is a BLAKE3-256 digest over deterministic bytes. The `prev`
field links the frame to the previous item in its segment. Because segment headers and frames
are CBOR Sequence items, independently valid segments can be concatenated without rewriting
their bytes. The resulting file folds as the ordered value-union of segment folds.

Payloads use a transform catalog. The baseline surface includes the mandatory structural path
needed for the core reader, while optional codecs and cryptographic transforms are capability
dependent. Unknown codecs, unsupported frame types, or unavailable keys are represented as
diagnostics and opaque graph nodes when the surrounding bytes remain recoverable.

The optional index frame can carry offset tables, frame-type indexes, and an MMR root. Current
support is intentionally scoped: detached MMR proof verification is cross-engine, Rust can
create proofs from indexed GTS files, and broader random-access/proof creation surfaces remain
tracked as advanced primitives rather than baseline reader requirements.

## 4. Fold Semantics

The fold is the deterministic replay of segment frames into an RDF dataset-shaped state.
Relevant state includes:

- RDF terms, including IRIs, literals, blank nodes, and quoted triples.
- Quads and statement-level annotations.
- Reifier bindings.
- Inline blob summaries by digest, media type, and size.
- Segment metadata, profiles, diagnostics, and segment heads.
- Suppression records and opaque nodes.

Term ids are segment-local. Cross-segment identity is by RDF term value, not by local integer
id, and blank-node labels are not merged across independently produced segments. Appending a
new segment therefore keeps existing bytes intact while adding another fold contribution.

Suppression is additive. It records display or validity policy over prior graph claims without
physically removing the older signed bytes. Snapshot compaction can rewrite a graph into a
smaller distribution artifact, but that rewrite is explicit and lossy relative to the full
append history.

The paper should treat the fold model as the central abstraction, but should not restate new
normative rules. Formal notation can summarize the spec model as:

```text
fold(file) = value_union(fold(segment_0), ..., fold(segment_n))
```

The exact grammar, duplicate behavior, suppression behavior, diagnostics, and conformance
expectations remain owned by the specification and conformance documents.

## 5. Integrity, Confidentiality, And Opacity

GTS separates four concerns:

- Frame integrity: each frame has its own BLAKE3 content id.
- History integrity: `prev` links commit a frame to its chain position.
- Origin or authorship: optional COSE signatures can bind signers to frame ids.
- Freshness or non-truncation: an external or in-band head commitment is needed to detect
  dropped trailing frames.

The first two are key-free format properties. The last two are profile or deployment choices.
This distinction is important for the research narrative: a valid signature proves a key signed
specific bytes, but trust in that key and truth of the RDF claims are deployment or profile
policy.

The opacity model is also part of the transport design. A reader without a codec or key can
still preserve position, frame type, public envelope, recipient identifiers, signatures, and
diagnostics. Content can be hidden, but the existence and chain position of the hidden content
remain observable. This makes degraded reads explicit and testable instead of silently dropping
information.

Current v1 crypto status should be described narrowly. COSE_Sign1 and single-recipient
COSE_Encrypt0 are implemented optional Full Reader capabilities. Multi-recipient
COSE_Encrypt envelopes and ECDH key-wrap are deferred outside v1 conformance until byte-level
fixtures, interop tests, and key-management policy exist.

## 6. Conformance And Implementation Status

The repository contains six engines:

| Engine | Package surface | Current role |
|---|---|---|
| Rust | `gmeow-gts`, binary `gts` | Reference package, evented projection API, Rust-only proof creation, CLI transforms. |
| Python | `gmeow-gts`, module `gts` | Reference corpus generator and Python package. |
| Go | `go.blackcatinformatics.ca/gts` | Go package and CLI with streaming sink evidence. |
| TypeScript | `@blackcatinformatics/gmeow-gts` | npm package, Node reader surface, and browser progressive stream/WebCrypto surface. |
| Smalltalk/Pharo | Tonel + Metacello source package, Docker `gts` runtime | Pharo engine for the common corpus, CLI, and interop surface. |
| Kotlin/JVM | Gradle source package and `gts` runtime | JVM engine for the common corpus, CLI, and Java-callable library surface. |

The shared compatibility oracle is the checked-in vector corpus under `vectors/` and the
portable manifest at `vectors/manifest.json`. Conformance claims name a tier, the corpus
revision, vector subsets, enabled optional capabilities, and the command or harness that
produced the evidence.

The relevant tiers for the paper narrative are:

- Baseline Reader: parse, verify, fold, report diagnostics, and degrade unsupported recoverable
  frames to opaque nodes.
- Streaming Reader: Baseline Reader behavior plus a sink/event API that avoids materializing
  the whole graph. In the current repository, Go claims this tier for `reader.ReadToSink`,
  Rust claims it for `read_to_sink_from_reader`, and the TypeScript browser export claims it
  for `foldStreamToSink`.
- Full Reader: Baseline Reader behavior plus claimed optional capabilities such as COSE,
  decryption, nested-GTS recursion, security policy, or index/MMR behavior.
- Writer and Validating Tool: deterministic output and stricter tool/profile checks where
  those claims are made.

Implementation status should be presented as a moving repository fact, not as a standards
claim. At the time of this draft, all six engines are described as gating against the shared
corpus for their public surfaces, while several capabilities remain deliberately outside the
baseline: database and Parquet exports are not present in every engine, non-Rust proof creation
is deferred, range-fetch helpers still depend on verified boundaries, object-store service
patterns are integration contracts rather than core format behavior, and multi-recipient
encryption is pinned only as deferred contract descriptors.

## 7. Evaluation Plan

The paper should report measurements only from reproducible release artifacts. The current
repository provides a benchmark runner and report template in
[`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md). A measured evidence
run for this draft is kept as generated output in
[`dist/benchmarks/paper-evidence/release-benchmark-report.md`](../dist/benchmarks/paper-evidence/release-benchmark-report.md)
rather than overwriting the benchmark template.

Evaluation should cover four claims:

1. Correctness and interoperability: each engine folds the same vector bytes to the same
   expected graph summaries, segment heads, diagnostics, and opaque reasons for its claimed
   tier.
2. Streaming behavior: item-boundary prefixes fold to valid intermediate states, and engines
   that claim Streaming Reader status provide sink/API evidence with bounded memory behavior.
   Current paper claims should cite Go for the tier claim and describe Rust/TypeScript as
   evented or progressive evidence only.
3. Integrity behavior: damaged frames, broken chains, torn appends, truncation anchors, and
   unsupported capabilities produce the expected recoverable or fatal diagnostics.
4. Practicality: GTS can project to operating substrates such as N-Quads, SQLite, DuckDB, and
   Parquet where the relevant engine exposes those transform targets, with failures and gaps
   reported rather than omitted.

Suggested tables for a publication appendix:

- corpus pass/fail by engine, tier, vector subset, and corpus revision;
- read, fold, write, pack, and unpack timings by engine;
- peak memory or allocation evidence for full-reader and streaming-reader paths;
- file-size comparisons across codec choices and streamable compaction;
- corrupted-input recovery behavior with and without offset indexes;
- range-fetch byte savings for progressive-delivery examples after boundaries are known.

## 8. Applications

GTS is intended to support multiple application families without making any one of them the
core identity:

- Dataset and ontology distribution: publish a verifiable graph package with the binary assets
  it names.
- GMEOW distribution: ship GMEOW ontology packages and profiles as GTS artifacts while keeping
  GTS independent of GMEOW.
- Archives and file manifests: package directory trees with graph-native metadata and
  content-addressed blobs.
- Evidence and custody chains: append observations, signatures, and sealed payloads without
  rewriting prior history.
- Local-first graph synchronization: concatenate independently produced segments and fold the
  value-union.
- Image and media packages: lead with catalog metadata and small manifestations, then carry
  larger blobs and provenance later in the same verifiable stream.
- Agent memory and belief revision: append observations, suppressions, and provenance as one
  application-level profile rather than as the format's identity.
- Graph database interchange: project folded graph state into N-Quads, SQLite, DuckDB,
  Parquet, or other systems when those transforms are available.

## 9. Limitations And Future Work

GTS is not a query language, reasoner, mutable database, consensus protocol, key-discovery
system, trust framework, or external blob availability guarantee. Application-level conflict
resolution remains above the core fold, and deployments remain responsible for trust anchors,
signer authorization, key rotation, revocation, and external head commitments.

Known limitations and current deferrals include:

- Truncation detection requires a head commitment such as a signed head, index root, release
  manifest, or external anchor.
- Confidential frames can still reveal existence, type, recipient identifiers, signatures, and
  chain position.
- Recovery past arbitrary byte corruption needs known offsets or external framing; a bare CBOR
  Sequence can lose synchronization after damaged bytes.
- Compression, decompression, and nested GTS recursion require explicit resource budgets.
- Multi-recipient COSE_Encrypt and ECDH key-wrap are outside v1 conformance.
- Cross-engine proof creation, deeper range-fetch helpers, and object-store/service workflows
  are advanced surfaces rather than core reader requirements.
- Optional-standard and domain-specific profiles need governance, test vectors, and clear
  compatibility notes before they can make strong claims.
- Release and publication claims need stamped corpus revisions rather than the checked-in
  manifest placeholder.

## 10. Related Work

GTS deliberately overlaps several mature areas, but it occupies a different point in the design
space: a single transport artifact that is append-only, content-addressed, RDF-shaped after
folding, binary-payload aware, partially readable, and covered by a cross-engine conformance
corpus.

**RDF serializations and graph exchange.** W3C RDF serializations such as
[RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/),
[TriG](https://www.w3.org/TR/rdf12-trig/), N-Triples/N-Quads, Turtle, and
[JSON-LD 1.1](https://www.w3.org/TR/json-ld11/) define interoperable ways to write RDF graphs
or datasets. HDT, the W3C Member Submission for
[Header-Dictionary-Triples](https://www.w3.org/submissions/2011/SUBM-HDT-20110330/), addresses
compact binary RDF publication and exchange. GTS differs by treating the RDF projection as the
fold of an append-only binary log that can also carry content-addressed blobs, transforms,
signatures, opacity diagnostics, and multi-segment history.

**Binary encodings, sequences, and packages.** GTS reuses
[CBOR](https://www.rfc-editor.org/info/rfc8949) for deterministic binary structure and
[CBOR Sequences](https://datatracker.ietf.org/doc/html/rfc8742) for self-delimiting item
streams. Archival and research-data packaging systems such as
[BagIt](https://datatracker.ietf.org/doc/rfc8493/) and
[RO-Crate](https://www.researchobject.org/specs/) focus on reliable file transfer and
metadata-rich research-object description. GTS borrows the package-and-manifest intuition, but
the package manifest is itself folded graph state and every segment/frame participates in the
same content-id chain.

**Content-addressed systems and append-only logs.** Git is commonly described by its own
documentation as a [content-addressable filesystem](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects),
and IPFS names content through [CIDs](https://docs.ipfs.tech/concepts/content-addressing/) that
derive from cryptographic hashes. Transparency systems such as
[Certificate Transparency](https://datatracker.ietf.org/doc/html/rfc6962) use append-only logs
and audit proofs for globally observable issuance events. GTS applies content addressing inside
a portable graph artifact: frame ids and `prev` links give local chain integrity, optional MMR
indexes support detached inclusion proofs, and deployment profiles decide whether to anchor
heads in an external transparency or release system.

**Event sourcing and local-first synchronization.** Event sourcing records state changes as a
sequence of events from which state can be rebuilt, as summarized in Martin Fowler's
[Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) pattern. Local-first work
argues for user-controlled, offline-capable data with synchronization as an enhancement rather
than a central dependency, notably in the Ink & Switch
[local-first software](https://www.inkandswitch.com/essay/local-first/) essay. GTS is not a
CRDT or application synchronization protocol, but its segment concatenation, prefix-fold
validity, and additive suppression model make it suitable as a transport artifact for systems
that need append-only histories and later projection into application-specific merge logic.

**Provenance, custody, and research evidence.** W3C
[PROV-O](https://www.w3.org/TR/prov-o/) provides an RDF/OWL vocabulary for representing and
interchanging provenance information. RO-Crate and BagIt provide established packaging patterns
for research objects and digital-preservation payloads. GTS can carry PROV-O, RO-Crate-like, or
domain-specific metadata as ordinary graph content, while separating byte integrity and
signature verification from deployment trust: a valid GTS chain proves byte continuity, not the
truth of the claims or authority of a signer.

**Payload security layers.** GTS uses COSE rather than inventing a signature or encryption
envelope: [RFC 9052](https://www.rfc-editor.org/info/rfc9052) defines signing, MAC, and
encryption structures for CBOR serialization. JSON ecosystems commonly use
[JWS](https://datatracker.ietf.org/doc/html/rfc7515) for integrity-protected JSON-based
payloads. GTS's distinction is the opacity invariant: encrypted or unsupported payloads can
remain graph-visible as opaque nodes with diagnostics, public envelopes, and chain position
rather than causing a total read failure or disappearing from the fold.

**Graph databases and projection targets.** SPARQL 1.1 defines the standard
[query language for RDF](https://www.w3.org/TR/sparql11-query/), while systems such as SQLite,
DuckDB, and Parquet provide durable or analytical tabular substrates. SQLite documents a stable
[single-file database format](https://www.sqlite.org/fileformat.html); DuckDB is an
[embeddable analytical database](https://duckdb.org/pdf/SIGMOD2019-demo-duckdb.pdf); and Apache
Parquet is a [column-oriented file format](https://parquet.apache.org/) for analytics. GTS does
not compete with these systems as a query engine. Instead, it defines a portable, verifiable
transport from which N-Quads, SQLite, DuckDB, Parquet, or native RDF stores can be regenerated.

## 11. Conclusion

GTS explores a small transport layer for graph-shaped artifacts: deterministic CBOR bytes,
append-only frames, content-addressed history, fold semantics, graceful opacity, and a
cross-language conformance corpus. Its value is in the boundary it draws. The core artifact is
portable and verifiable; richer databases, profiles, proof systems, object stores, and domain
workflows can attach above or below it without changing the format's narrow waist.

## Appendix Drafts

Future paper revisions can add:

- CDDL excerpts from the spec.
- Fold pseudocode aligned to the normative algorithm.
- A conformance-vector catalog summary.
- The media type registration excerpt.
- CLI examples for read, verify, fold, cat, compact, pack, unpack, and transform targets.
- A security checklist summarizing integrity, trust, opacity, and resource-bound assumptions.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Governance And Release Policy

This document defines the lightweight change-control process for GTS, the extension registry
policies, the compatibility contract, and the v1.0 release-candidate path. It complements the
wire-format specification in [`GTS-SPEC.md`](./GTS-SPEC.md), the conformance policy in
[`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md), the profile/security policy in
[`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md), and the repository security disclosure
policy in [`../SECURITY.md`](../SECURITY.md).

GTS is ontology-independent. GMEOW is a primary downstream consumer and distribution use case,
but GTS readers and writers do not require GMEOW vocabulary, tooling, or semantics.

## 1. Change Classes

GTS uses different governance paths for core format changes and profile or registry additions.
The goal is to keep the wire-format narrow waist stable while allowing profiles and deployment
patterns to evolve.

| change class | examples | required path |
|---|---|---|
| Core wire-format change | Header or frame grammar, deterministic CBOR rules, id/hash preimages, `prev` semantics, segment boundaries, transform catalog mechanics, core fold semantics. | GTS Improvement Proposal (GIP), spec PR, conformance-corpus impact review, and cross-engine implementation plan. |
| Reader/writer conformance change | Baseline Reader tiers, diagnostic behavior, vector expectations, writer determinism requirements. | GIP when behavior changes; docs/corpus PR for clarifications that preserve behavior. |
| Optional-standard profile | `files`, `evidence`, `opaque`, `bundle`, `stream`, or a new profile intended to be maintained by core GTS. | Profile proposal plus GIP if it adds standard obligations, security policy, or conformance tiers. |
| Domain-specific profile | GMEOW distribution profiles, `music-package`, `agent-memory`, and third-party application profiles. | Profile registration review; no core GIP unless the profile needs new core behavior. |
| Registry addition | New codec name, diagnostic code, frame type, profile name, or transform target. | Registry policy in Section 3. |
| Implementation or package change | Engine API, CLI ergonomics, packaging, documentation, examples. | Normal issue/PR flow unless it changes conformance behavior. |

## 2. GTS Improvement Proposals

A GTS Improvement Proposal (GIP) is required before a change can alter core GTS semantics or
the required behavior of conformant implementations.

### 2.1 When A GIP Is Required

Open a GIP for:

- any change to header or frame grammar;
- any change to hash preimages, signature preimages, or deterministic CBOR requirements;
- any change to segment composition, `prev` chain rules, or fold semantics;
- adding, removing, or reclassifying a Baseline Reader, Writer, Validating Tool, or Full Reader
  requirement;
- making an optional capability mandatory;
- adding an optional-standard profile maintained by core GTS;
- promoting a codec, frame type, or diagnostic into a core-required registry entry;
- changing compatibility policy for wire format, corpus, packages, or profiles.

A GIP is not required for editorial clarification, examples, domain-specific profile
registration, package metadata, or implementation refactoring that preserves observable
conformance behavior.

### 2.2 GIP Shape

A GIP SHOULD be opened as a GitHub issue or checked-in design document with:

- title and summary;
- change class and affected spec sections;
- motivation and non-goals;
- exact normative behavior change;
- compatibility impact for wire, corpus, packages, and profiles;
- conformance vectors to add or update;
- affected engines and implementation plan;
- security and privacy considerations;
- migration plan and release milestone;
- alternatives considered;
- decision status: `draft`, `proposed`, `accepted`, `implemented`, `rejected`, or `superseded`.

Accepted GIPs are implemented through normal pull requests. The PR that implements a GIP MUST
link the GIP and update the relevant spec, conformance corpus, docs, and engines.

## 3. Registry Governance

The canonical registry tables may live in the spec, conformance document, security policy, or
future `docs/registries/` files. This section owns the change policy for each registry.

| registry | examples | change policy |
|---|---|---|
| Frame types | `terms`, `quads`, `blob`, `snapshot`, `index`, extension frame types. | Specification required. New core or optional-standard frame types require a GIP, a spec update, vectors, and downgrade behavior for unknown readers. |
| Core diagnostics | `DamagedFrame`, `UnknownCodec`, `StreamableLayoutError`, `RecursionLimit`. | Specification/conformance required. Additions need a conformance-doc update and vectors when the diagnostic is observable in a required tier. Renames are breaking. |
| Codec names | `identity`, `gzip`, `zstd`, `cose-encrypt0`, future compression or encryption names. | Expert review or PR with implementation evidence. Baseline codecs require a GIP. Optional codecs require fallback/opaque behavior and interoperability notes. |
| Security-sensitive codecs | Encryption, signature, key-wrap, decompression, nested-container, or executable-transform codecs. | Expert review plus security considerations, resource-budget analysis, and vectors before conformance claims. Mandatory promotion requires a GIP. |
| Profiles | `files`, `stream`, `evidence`, `opaque`, third-party profiles. | First-come registration with review for domain-specific profiles. Optional-standard or core-maintained profiles require stronger review and may require a GIP. |
| Transform targets | N-Quads, Turtle, SQLite, DuckDB, Parquet, blob externalization layouts. | Documentation PR for target shape and round-trip expectations. Implemented CLI/API targets also update parity docs and tests. |

### 3.1 Reserved Namespaces

Bare names in the following sets are reserved for the GTS core registry:

- frame types already named by `GTS-SPEC.md`, plus future short lowercase frame names accepted
  through the frame-type policy;
- diagnostic codes without a profile or owner prefix, especially `PascalCase` codes in
  `GTS-CONFORMANCE.md`;
- codec names in the canonical codec table and future short names accepted through the codec
  policy;
- standard profile names: `generic`, `dist`, `evidence`, `opaque`, `bundle`, `files`,
  `stream`, `image`, `ai-package`, and any future core-maintained profile;
- GTS-owned IRIs under `https://w3id.org/gts/`;
- CLI transform target names used by this repository, such as `nquads`, `sqlite`, `duckdb`,
  `parquet`, and `turtle`.

Third-party registrations SHOULD use one of:

- a stable URI;
- a reverse-DNS name such as `org.example.profile`;
- an owner-prefixed token such as `example-profile` when the owner is obvious from the registry
  row.

Profile-specific diagnostics SHOULD use a documented profile namespace or prefix. A profile MUST
NOT claim that its diagnostics, frame types, or codec names are core GTS behavior unless they
have been accepted into the corresponding core registry.

## 4. Profile Governance

Core spec changes and profile additions have separate paths.

Domain-specific profiles can be registered without changing the wire format. A profile
definition MUST state that it does not change:

- header or frame grammar;
- segment-boundary detection;
- content-id, signature, or hash preimages;
- transform-catalog resolution;
- deterministic fold semantics.

A profile can define vocabulary, validation rules, trust policy, publication workflow, and
conformance vectors. A profile can require signatures, keys, codecs, or deployment trust for
profile-aware tools without making those features mandatory for Baseline Readers.

The profile registration template in `GTS-SPEC.md` remains the required content checklist for a
new profile. Optional-standard profile promotion additionally requires:

- a named owner and change controller;
- at least one implementation or executable validator;
- profile-specific conformance vectors;
- security and privacy review;
- compatibility policy for future profile revisions.

## 5. Compatibility Policy

GTS compatibility is separated into four layers. A release or proposal MUST name the layer it
affects.

| layer | compatibility rule |
|---|---|
| Wire-format compatibility | The header `"v"` field is the wire-format major version. Before v1.0, incompatible changes remain possible. After v1.0, a file that is valid GTS major version 1 must remain parseable and safely foldable by future major-version-1 readers, subject to declared capabilities and resource limits. Breaking wire changes require a new wire-format major version. |
| Corpus compatibility | The vector corpus is the compatibility oracle for claimed tiers. New vectors may be added to clarify behavior or cover regressions. Existing vectors may change only through a GIP or correction that explains the previous expectation was wrong. Release candidates SHOULD attach conformance reports naming the corpus commit. |
| Package compatibility | Rust, Python, Go, and TypeScript packages are release artifacts. They SHOULD keep user-facing APIs stable within their normal ecosystem semver rules. Package versions may differ from the document version and corpus version, but release notes MUST state the spec/corpus commit they implement. |
| Profile compatibility | Profiles own their vocabulary and validation compatibility. Domain-specific profiles may version independently, but a profile revision MUST preserve core GTS parse, verify, and fold semantics for baseline readers. Optional-standard profiles need compatibility notes in the registry. |

Compatibility claims SHOULD identify:

- implementation name and package version;
- wire-format major version;
- document version or spec commit;
- corpus commit and tier;
- enabled optional capabilities;
- profile versions or profile registry rows used.

## 6. Parser And Cryptographic Security Governance

The repository-level disclosure process is defined in [`../SECURITY.md`](../SECURITY.md). This
section defines how security-sensitive format and registry changes are governed.

Security-sensitive changes include:

- CBOR parsing, deterministic encoding, and segment-boundary detection;
- decompression, compression dictionaries, and decompression-bomb limits;
- encryption, signatures, key identifiers, key-wrap, and trust-policy behavior;
- `files` profile extraction or any command that writes to disk;
- nested GTS handling, recursion, or decoded-size limits;
- profile rules that change what a tool treats as trusted, publishable, or safe.

Any security-sensitive codec, transform, or profile proposal MUST document:

- threat model and non-goals;
- resource-budget impact;
- downgrade or opaque behavior when unsupported;
- failure diagnostics;
- vectors or tests for hostile inputs;
- disclosure and release coordination if the change fixes a vulnerability.

Confirmed vulnerabilities in parser, crypto, extraction, or release pipelines follow private
coordinated disclosure. Public GIPs should avoid exploit details until the fix/advisory is ready.

## 7. v1.0 Release Path

The v1.0 path is staged so the baseline specification can publish without waiting for every
future profile, transform, or research artifact.

| milestone | publication criteria |
|---|---|
| `v1.0-alpha1` | Standalone framing is in place, the media type decision is documented, the CDDL/hash-preimage appendices have a draft path, and GMEOW independence is explicit. Wire-format changes are still expected. |
| `v1.0-beta1` | Conformance manifest or equivalent corpus policy is stable, registry policies exist, core/profile split is clear, and fold semantics are formal enough for implementer review. Wire-format changes require explicit compatibility notes. |
| `v1.0-rc1` | No intentional wire-format changes remain, baseline vectors are frozen, registry policies and reserved namespaces are published, security model is clear, and implementer review has no open blocking findings. |
| `v1.0` | Format spec is published, conformance corpus is tagged, reference implementation packages are released, and release notes identify the spec/corpus commits. |

The concrete `v1.0-rc1` runbook is
[`GTS-V1-RC1-CHECKLIST.md`](./GTS-V1-RC1-CHECKLIST.md). It records the spec
commit, corpus revision, blocker review, conformance reports, package dry-runs,
release notes, tag workflows, and artifact-verification evidence required for a
release-candidate cut.

### 7.1 v1.0-rc1 Blockers

The following block `v1.0-rc1` publication:

- unresolved intentional changes to header/frame grammar, hash preimages, signature preimages,
  segment composition, transform resolution, or core fold semantics;
- missing or failing baseline conformance vectors for required reader/writer behavior;
- cross-engine failure for a behavior claimed by the v1 baseline tier;
- missing registry change policies for frame types, diagnostics, codecs, profiles, and transform
  targets;
- missing reserved namespace guidance;
- unresolved high- or critical-severity parser, crypto, extraction, or release-pipeline
  vulnerability;
- missing media type and HTTP/distribution guidance needed for immutable publication;
- unclear compatibility language for wire format, corpus, packages, or profiles.

### 7.2 v1.0-rc1 Non-Blockers

The following do not block `v1.0-rc1` when the baseline conformance tier is otherwise ready:

- every optional-standard or domain-specific profile being complete;
- every transform target being implemented in all engines;
- database, Parquet, browser, object-store, index/MMR, replication, range-fetch, or advanced proof
  tooling;
- every key-management mode or multi-recipient encryption envelope;
- package renaming, neutral package aliases, or standards-body submission;
- paper, benchmark-suite, and third-party implementation-guide publication;
- future profile registry entries.

Non-blockers SHOULD be tracked as issues, project items, or release-adjacent checklist rows, but
they MUST NOT delay baseline conformance publication unless they expose a blocker above.

## 8. Release-Adjacent Deliverables

These deliverables are useful for adoption and standards posture, but are not baseline spec
publication blockers:

| deliverable | tracking expectation | release relationship |
|---|---|---|
| Paper outline and publication draft | Track as a docs/research issue or project item. Reuse the standalone framing and conformance results. | Describes and motivates GTS; does not define normative behavior. |
| Benchmark suite | Track memory, read, fold, pack, unpack, and cross-engine interop benchmarks. Record hardware and corpus commit. | Supports performance claims; does not gate wire-format validity. |
| Third-party implementation guide | Track examples, profile template guidance, registry process, and minimal-reader walkthrough; current guide: [`GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md`](./GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md). | Helps implementers adopt the v1 spec; not required for `v1.0-rc1` if conformance docs are sufficient. |

Each v1 release-candidate checklist SHOULD state whether these deliverables are complete,
deferred, or assigned to follow-up issues.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS governance and release policy

Changes to the [GTS v2 specification](GTS-SPEC.md) follow the proposal process below.
The normative documents define the format. This policy governs how the project changes and publishes them.

## 1. Change classes

Core changes require a GTS Improvement Proposal (GIP). Editorial clarifications, implementation refactoring,
package metadata and domain-specific profiles that preserve core behavior can proceed by ordinary pull request.

## 2. GTS Improvement Proposals

Open a GIP for changes to grammar, deterministic encoding, hash or signature preimages, fold semantics,
reader and writer requirements, conformance claims, compatibility rules, or promotion of a capability
or profile into the standard. Include the conformance impact and cross-engine implementation plan.

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

## 3. Registry governance

[P2](v2/P2.md) assigns the format codes and algorithm identifiers. A proposal for a new assignment
identifies its table, meaning, effect on existing readers and expected test results. Changes to core
requirements need a GIP. Security-sensitive additions require expert review, resource analysis and
conformance inputs before the project publishes an implementation claim.

## 4. Profile governance

[P7 §6](v2/P7.md#6-the-profile-mechanism) defines how a profile adds checks to the core format.
Register a domain-specific profile by pull request. Identify its owner, change controller, vocabulary,
validation rules and compatibility policy. Core-maintained profiles require a GIP, an implementation
or executable validator, profile-specific cases, and security and privacy review.

## 5. Compatibility policy

Identify each affected layer in a proposal or release.

- **Wire format.** A breaking wire change requires a new generation. [P1](v2/P1.md) defines how a reader identifies the generation.
- **Corpus.** A change to existing expected results needs a GIP or a correction explaining why the expectation was wrong.
  Identify the [corpus revision](../vectors/v2/README.md) and the [P7 claim](v2/P7.md#5-conformance) in conformance reports.
- **Packages.** Use ecosystem versioning for public APIs. State the implemented specification and corpus revision in release notes.
- **Profiles.** A revision must preserve core parsing and fold semantics. Document profile-specific compatibility and give the revision with a profile claim.

### 5.1 Deprecation policy

A public API surface — an exported function, type, or CLI verb — is removed in three steps, never fewer:

1. **Announce.** The release that deprecates it names it in `CHANGELOG.md` under `### Deprecated`,
   states the replacement (or that there is none), and gives the earliest release that MAY remove
   it. Deprecating and removing in the same release is not permitted.
2. **Warn where it costs nothing.** Ecosystem-native deprecation markers SHOULD be applied —
   `#[deprecated]`, `DeprecationWarning`, `@deprecated`, `Deprecated:` — so consumers learn at
   build time rather than at upgrade time. A deprecated item keeps working unchanged.
3. **Remove.** Removal happens only in a release whose version signals it under that ecosystem's
   rules, and no earlier than the release named in step 1.

The minimum notice is **one minor release AND 90 days**, whichever is longer. Removing a public
API is a MAJOR change for the affected package.

Two carve-outs, both narrow:

- A **security fix** MAY shorten or skip the notice period when leaving the surface in place would
  keep users exploitable. The release notes MUST say so explicitly and cite the advisory.
- An API that has never appeared in a released package MAY be removed freely; the promise attaches
  to what was published, not to what was merged.

## 6. Parser And Cryptographic Security Governance

The repository-level disclosure process is defined in [`../SECURITY.md`](../SECURITY.md). This
section defines how security-sensitive format and registry changes are governed.

Security-sensitive changes include:

- CBOR parsing, deterministic encoding, and file and frame boundary detection;
- decompression, compression dictionaries, and decompression-bomb limits;
- encryption, signatures, key identifiers, key-wrap, and trust-policy behavior;
- `files` profile extraction or a command that writes to disk;
- nested GTS handling, recursion, or decoded-size limits;
- profile rules that change what a tool treats as trusted, publishable, or safe.

Any security-sensitive codec, transform, or profile proposal MUST document:

- threat model and non-goals;
- resource-budget impact;
- reader outcomes when a capability is unavailable;
- failure diagnostics;
- vectors or tests for hostile inputs;
- disclosure and release coordination if the change fixes a vulnerability.

Confirmed vulnerabilities in parser, crypto, extraction, or release pipelines follow private
coordinated disclosure. Public GIPs should avoid exploit details until the fix/advisory is ready.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# GTS format generation 2: draft specification

Graph Transport Substrate (GTS) carries graph statements and content-addressed binary data in an appendable sequence of
CBOR items. A reader folds the items into graph content and reports what it could verify, recover or read only in part.
The format defines a projection of the fold into quads and empty named graphs, and a canonical representation of the fold.

**Status.** This is the draft of format generation 2, document version 2.0. The reference engines implement
[generation 1](../GTS-SPEC.md). Generation 2 has its own [draft conformance corpus](../../vectors/v2/README.md).
Adoption follows the [GTS proposal process](../GTS-GOVERNANCE.md).

Start with P1 for the wire grammar, P7 §5 for the claims an implementation can make and P7-10 for how to compare two
readers. For RDF interoperability, P3 §10a defines the projection, the information it loses, and the conditions for writing
it in an RDF syntax. Report a discrepancy with the document version or commit, the part and requirement or section, and,
where possible, the smallest input and the two outcomes. [GTS issue tracker](https://github.com/Blackcat-Informatics/gmeow-gts/issues)

The draft is a set of sixteen normative documents and four informative documents. The normative documents:

| document | title |
|---|---|
| [P1](P1.md) | Model and wire |
| [P2](P2.md) | Registry |
| [P3](P3.md) | RDF content |
| [P4](P4.md) | Blobs |
| [P5](P5.md) | Durability |
| [P6](P6.md) | Signatures |
| [P7](P7.md) | Canonical form and reading |
| [P8](P8.md) | Sealing |
| [P9](P9.md) | Conversion from the first generation |
| [PF-dist](PF-dist.md) | Profile `dist` |
| [PF-evidence](PF-evidence.md) | Profile `evidence` |
| [PF-stream](PF-stream.md) | Profile `stream` |
| [PF-bundle](PF-bundle.md) | Profile `bundle` |
| [PF-files](PF-files.md) | Profile `files` |
| [PF-container](PF-container.md) | Profile `container` |
| [L1](L1.md) | Canonical labelling of blank nodes, cited by `P3` |

A profile adds checks to a core file, and sometimes a vocabulary and constants. It is not a second format, and it is not a
route for every fold. The profile `dist` is defined for the folds whose canonical file stays within its two size ceilings; a
fold outside them has no distribution file and is carried as a core file. The profile `evidence` shows that a trusted key
signed every commit; it does not show which key signed when the commit was written.

## Informative documents

The sixteen documents state the format: its rules, definitions, registries, worked examples and claims. Four documents beside
them state no rule of the format. Where one of them and a normative document differ, the normative document is right.

| document | holds |
|---|---|
| [GUIDE-applications](GUIDE-applications.md) | what an application checks before it relies on a file, and the contracts and patterns of use |
| [GUIDE-migration](GUIDE-migration.md) | the map from the features, commands and helpers of the first generation |
| [GUIDE-adoption](GUIDE-adoption.md) | the measurements, the evidence and the order of work that adoption needs |
| [FIXTURES](FIXTURES.md) | the fixture catalogue: the inputs, the reads and the expected typed results |

The main `README.md` of the repository describes the first generation. Its command verbs, its detached proof JSON, its
embedded OpenPGP transport key, its streaming reader, its `fold` and `from-nq` pair and its profile spellings are interfaces
of that generation. This draft defines other typed contracts for proofs (`P5`), signatures (`P6`), sealing (`P8`), the
accesses of a reader (`P7`) and the RDF projection (`P3`). `GUIDE-migration.md` maps each of them. In this generation the fold
of two concatenated files is not in general the union of their folds (`P1`, "Growing a file").

Proofs, rationale and measurements are kept outside the sixteen documents. Each such record names the document, the
requirement or claim it supports and the commit of the text it was made against, as `GUIDE-adoption.md` states.

The [corpus instructions](../../vectors/v2/README.md) describe the read inputs, the expected outcomes, their provenance
and how to obtain the large fixtures.

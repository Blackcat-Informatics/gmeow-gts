<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# GTS v2 specification

Graph Transport Substrate (GTS) carries graph statements and content-addressed binary data in an appendable sequence of
CBOR items. A reader folds the items into graph content and reports what it could verify, recover or read only in part.
The format defines a projection of the fold into quads and empty named graphs, and a canonical representation of the fold.

**Document version:** 2.0

**Status:** In review. [Implementer Draft 1](https://github.com/Blackcat-Informatics/gmeow-gts/releases/tag/spec-v2-draft-1)
identifies the published review snapshot. Cite a commit when reviewing revisions to that snapshot.
Publication of this text invites independent implementations. Changes follow the [GTS proposal process](./GTS-GOVERNANCE.md).

For a reader's introduction to content identity, partial reads, recovery, privacy and migration,
see [what changes in v2](../DIFFERENCES_IN_v2.md).

Start with P1 for the wire grammar, P7 §5 for the claims an implementation can make and P7-10 for how to compare two
readers. For RDF interoperability, P3 §10a defines the projection, the information it loses, and the conditions for writing
it in an RDF syntax. Report a discrepancy with the document version or commit, the part and requirement or section, and,
where possible, the smallest input and the two outcomes. [GTS issue tracker](https://github.com/Blackcat-Informatics/gmeow-gts/issues)

The specification consists of the following sixteen normative documents.

| document | title |
|---|---|
| [P1](v2/P1.md) | Model and wire |
| [P2](v2/P2.md) | Registry |
| [P3](v2/P3.md) | RDF content |
| [P4](v2/P4.md) | Blobs |
| [P5](v2/P5.md) | Durability |
| [P6](v2/P6.md) | Signatures |
| [P7](v2/P7.md) | Canonical form and reading |
| [P8](v2/P8.md) | Sealing |
| [P9](v2/P9.md) | Conversion from the first generation |
| [PF-dist](v2/PF-dist.md) | Profile `dist` |
| [PF-evidence](v2/PF-evidence.md) | Profile `evidence` |
| [PF-stream](v2/PF-stream.md) | Profile `stream` |
| [PF-bundle](v2/PF-bundle.md) | Profile `bundle` |
| [PF-files](v2/PF-files.md) | Profile `files` |
| [PF-container](v2/PF-container.md) | Profile `container` |
| [L1](v2/L1.md) | Canonical labelling of blank nodes, cited by `P3` |

A profile adds checks to a core file, and sometimes a vocabulary and constants. It is not a second format, and it is not a
route for every fold. The profile `dist` is defined for the folds whose canonical file stays within its two size ceilings; a
fold outside them has no distribution file and is carried as a core file. The profile `evidence` shows that a trusted key
signed every commit; it does not show which key signed when the commit was written.

## Informative documents

The sixteen documents state the format: its rules, definitions, registries, worked examples and claims. Four documents beside
them state no rule of the format. Where one of them and a normative document differ, the normative document is right.

| document | holds |
|---|---|
| [GUIDE-applications](v2/GUIDE-applications.md) | what an application checks before it relies on a file, and the contracts and patterns of use |
| [GUIDE-migration](v2/GUIDE-migration.md) | the map from the features, commands and helpers of the first generation |
| [GUIDE-adoption](v2/GUIDE-adoption.md) | the measurements, the evidence and the order of work that adoption needs |
| [FIXTURES](v2/FIXTURES.md) | the fixture catalogue: the inputs, the reads and the expected typed results |

Proofs, rationale and measurements are kept outside the sixteen documents. Each such record names the document, the
requirement or claim it supports and the commit of the text it was made against, as `GUIDE-adoption.md` states.

The [corpus instructions](../vectors/v2/README.md) describe the read inputs, the expected outcomes, their provenance
and how to obtain the large fixtures.

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# GTS format generation 2: draft specification

This directory holds a DRAFT of the specification of format generation `2`. It is not in force. The specification in force
is `docs/GTS-SPEC.md`, and the engines and the corpus of this repository implement the first generation only. Adoption of this
generation follows the proposal process of `docs/GTS-GOVERNANCE.md`.

The draft is a set of sixteen normative documents and four informative documents. The normative documents:

| document | title |
|---|---|
| `P1.md` | Model and wire |
| `P2.md` | Registry |
| `P3.md` | RDF content |
| `P4.md` | Blobs |
| `P5.md` | Durability |
| `P6.md` | Signatures |
| `P7.md` | Canonical form and reading |
| `P8.md` | Sealing |
| `P9.md` | Conversion from the first generation |
| `PF-dist.md` | Profile `dist` |
| `PF-evidence.md` | Profile `evidence` |
| `PF-stream.md` | Profile `stream` |
| `PF-bundle.md` | Profile `bundle` |
| `PF-files.md` | Profile `files` |
| `PF-container.md` | Profile `container` |
| `L1.md` | Canonical labelling of blank nodes, cited by `P3` |

A profile adds checks to a core file, and sometimes a vocabulary and constants. It is not a second format, and it is not a
route for every fold. The profile `dist` is defined for the folds whose canonical file stays within its two size ceilings; a
fold outside them has no distribution file and is carried as a core file. The profile `evidence` shows that a trusted key
signed every commit; it does not show which key signed when the commit was written.

## Informative documents

The sixteen documents state the format: its rules, definitions, registries, worked examples and claims. Four documents beside
them state no rule of the format. Where one of them and a normative document differ, the normative document is right.

| document | holds |
|---|---|
| `GUIDE-applications.md` | what an application checks before it relies on a file, and the contracts and patterns of use |
| `GUIDE-migration.md` | the map from the features, commands and helpers of the first generation |
| `GUIDE-adoption.md` | the measurements, the evidence and the order of work that adoption needs |
| `FIXTURES.md` | the fixture catalogue: the inputs, the reads and the expected typed results |

The main `README.md` of the repository describes the first generation. Its command verbs, its detached proof JSON, its
embedded OpenPGP transport key, its streaming reader, its `fold` and `from-nq` pair and its profile spellings are interfaces
of that generation. This draft defines other typed contracts for proofs (`P5`), signatures (`P6`), sealing (`P8`), the
accesses of a reader (`P7`) and the RDF projection (`P3`). `GUIDE-migration.md` maps each of them. In this generation the fold
of two concatenated files is not in general the union of their folds (`P1`, "Growing a file").

Proofs, rationale and measurements are kept outside the sixteen documents. Each such record names the document, the
requirement or claim it supports and the commit of the text it was made against, as `GUIDE-adoption.md` states.

No implementation of this generation exists in this repository. Its conformance corpus is `vectors/v2/`; the `README.md` of
that directory says what an entry records and which entries were recorded against an earlier text.

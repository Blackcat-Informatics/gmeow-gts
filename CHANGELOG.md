<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# Changelog

All notable changes to GTS are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The four engines are
released in **lockstep** under a single version (the Go module is tagged
`go-v<version>`); see [`scripts/check-versions.sh`](./scripts/check-versions.sh).

The wire format is a working draft (`GTS-SPEC.md` is at draft `v0.3`) and MAY
change before `1.0`.

## [Unreleased]

### Added

- `gts extract-key <file>` (Python CLI, #12): prints the embedded transport
  (verification) key for a signed GTS — `kid`, OpenPGP fingerprint, emojihash,
  and the armored public key.
- `gts from-nq <in.nq> -o <out.gts>` (Python CLI, #14): build a GTS from
  N-Quads — the inverse of `fold`. Native N-Quads(-star) parser (no rdflib);
  handles IRIs, blank nodes, language/datatyped literals, named graphs, and the
  RDF 1.2 reifying style. `-` reads stdin, so `rdflib → n-quads → gts` pipes. Also
  exported as `gts.from_nquads`.

### Fixed

- **Cross-engine `pack` byte parity (#5).** The TypeScript and Go engines encoded
  the files-profile `mode` as an octal string (`"644"`) instead of the decimal
  `xsd:integer` value (`"420"`) the Rust and Python engines use, and TypeScript
  emitted millisecond-precision `modified` timestamps (`…20.000Z`). All four
  engines now pack identical fixtures to byte-identical output. The `interop`
  check promotes byte-identity to a hard gate and folds every package against a
  single global reference, so writer drift (not just reader disagreement) fails CI.

## [0.1.3] — 2026-06-16

### Added

- Live cross-engine interoperability check (`scripts/interop.sh`): every engine
  packs a fixture and every other engine folds/unpacks it, asserting identical
  results — interop coverage beyond the Python-generated frozen corpus.
- Fuzzing for the readers: Rust `cargo-fuzz` target and Go native `FuzzRead`,
  seeded from the conformance corpus, run on a schedule and as a PR smoke test.
- Per-ecosystem supply-chain vulnerability scanning (`cargo audit`,
  `govulncheck`, `npm audit`, `pip-audit`) on lockfile changes, on main, and weekly.
- Cross-platform CI (Linux, macOS, Windows) for every engine's build and tests.
- `wasm32-unknown-unknown` build gate for the Rust library.
- Developer `justfile`, a lockstep version-sync check, and this changelog.
- Full release attestations across all four engines: SLSA build-provenance
  attestations and SPDX SBOMs for the crate, the npm tarball (in addition to
  npm's sigstore provenance), the Python distributions, and the Go binaries.
- npm releases use trusted publishing (OIDC); the Go release cross-builds and
  publishes attested binaries (replacing the Pro-only goreleaser monorepo setup).

## [0.1.2] — 2026-06-16

### Added

- Initial public import of GTS into the standalone `gmeow-gts` repository: four
  interoperable engines (Rust, Python, Go, TypeScript), the `GTS-SPEC.md`
  specification, and the frozen conformance corpus.
- Triple licensing: `MIT OR Apache-2.0 OR proprietary`.

[Unreleased]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.1.3...HEAD
[0.1.3]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.1.2...py-v0.1.3
[0.1.2]: https://github.com/Blackcat-Informatics/gmeow-gts/releases/tag/py-v0.1.2

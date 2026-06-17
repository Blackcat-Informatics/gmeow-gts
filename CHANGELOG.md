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

- **Cross-engine `extract-key` (#17): OpenPGP key inspection in all four engines.**
  - A minimal OpenPGP reader (de-armor → packet parse → raw Ed25519 key + v4
    SHA-1 fingerprint) in Rust (`openpgp.rs`, pure-Rust `sha1`), Go
    (`openpgp` package, `crypto/sha1`), and TypeScript (`openpgp.ts`,
    `@noble/hashes/sha1`) — narrow by design: only the unencrypted armored
    Ed25519 (algorithm 22) certificates GPG emits, mirroring the Python
    `gts.openpgp` reference. SHA-1 appears solely because RFC 4880 mandates it
    for v4 fingerprints; it is not a general-purpose hash here.
  - **`gts extract-key <file>`** in the Rust, Go, and TypeScript CLIs (Python
    already had it, #12): prints the embedded transport key's `kid`, OpenPGP
    fingerprint, emojihash, and armored public key. Gated by a frozen shared
    vector set (`vectors/openpgp/*.json` + `scripts/gen_openpgp_vectors.py`):
    every engine parses the same key to the same raw bytes / fingerprint /
    emojihash, and the CLI reproduces the Python-generated stdout byte-for-byte.
  - The Rust OpenPGP reader stays pure / wasm-friendly (the `wasm32` build is
    still green).
  - COSE_Encrypt0 (random-IV AES-GCM, #18) remains the last tracked follow-up.

- **Cross-engine crypto parity (#15): COSE_Sign1 + emojihash in all four engines.**
  - **COSE_Sign1 Ed25519** (`sign_id`/`verify_sig`, detached over the frame id,
    §9.2) in Rust (`ed25519-dalek`), Go (`crypto/ed25519`), and TypeScript
    (`@noble/ed25519`) — byte-compatible with the Python reference and gated by a
    frozen shared vector set (`vectors/cose/*.json` + `scripts/gen_cose_vectors.py`).
    Ed25519 is deterministic, so every engine reproduces the exact COSE bytes and
    verifies them.
  - **emojihash** (BLAKE3-XOF → 6-bit → 64-emoji visual hash) in Rust, Go, and
    TypeScript, byte-identical to Python, gated by `vectors/emojihash/*.json`.
  - **File-level signing**: each engine's `Writer` can COSE-sign every frame
    (`sign_with`/`SignWith`/`signWith`), and each engine verifies the signatures of
    a signed GTS against resolved keys (`cose.verify_signatures` /
    `VerifySignatures` / `verifySignatures`). Gated by a frozen signed-GTS vector
    (`vectors/signed/basic.json`): every engine reproduces the Python-signed file
    byte-for-byte and verifies it (valid / unverified / invalid).
  - **`gts verify --key KID:HEXPUB`** in all four CLIs: verify a signed file's
    COSE signatures against raw Ed25519 public keys (repeatable; exit 1 on any
    invalid signature).
  - All crypto stays pure / wasm-friendly (the Rust `wasm32` build is still green).
  - Cross-engine `extract-key` landed as #17 (above); COSE_Encrypt0
    (random-IV AES-GCM) remains tracked as #18.

- `gts extract-key <file>` (Python CLI, #12): prints the embedded transport
  (verification) key for a signed GTS — `kid`, OpenPGP fingerprint, emojihash,
  and the armored public key.
- `gts from-nq <in.nq> -o <out.gts>` (Python CLI, #14): build a GTS from
  N-Quads — the inverse of `fold`. Native N-Quads(-star) parser (no rdflib);
  handles IRIs, blank nodes, language/datatyped literals, named graphs, and the
  RDF 1.2 reifying style. `-` reads stdin, so `rdflib → n-quads → gts` pipes. Also
  exported as `gts.from_nquads`.
- Relational export (Python CLI, #13): `gts to-sqlite`, `gts to-duckdb`,
  `gts to-parquet` load a folded graph into the integer-id, dictionary-encoded
  five-table schema (`terms`/`quads`/`reifiers`/`annotations`/`blobs`). SQLite
  uses the standard library; DuckDB and Parquet need the new `[db]` extra
  (`pip install 'gmeow-gts[db]'`). Library API: `gts.db.to_sqlite/to_duckdb/to_parquet`.

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

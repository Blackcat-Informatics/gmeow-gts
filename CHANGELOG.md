<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# Changelog

All notable changes to GTS are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Primary engines
publish from this repo through per-ecosystem release tags; versions are normally
kept aligned for cross-engine releases, while narrow Rust-first releases may
bump the Rust crate independently. See
[`scripts/check-versions.sh`](./scripts/check-versions.sh).

The wire format is a working draft (`GTS-SPEC.md` document version `0.9-draft`)
and MAY change before `1.0`.

## [0.9.10] — 2026-06-29

### Added

- Rust can emit deterministic graph snapshots directly from folded graph state,
  including terms, quads, reifiers, annotations, blobs, and snapshot-level zstd
  options.
- Python and Rust writers support per-frame zstd compression levels for both
  `zstd` and `zstd-rsyncable` frames.

### Changed

- Rust zstd handling now uses the pure-Rust `structured-zstd` backend for both
  encoding and decoding.
- Release metadata is aligned at `0.9.10` for the Python and Rust publication
  lanes, with gated companion metadata kept in sync.

### Fixed

- Removed the fixed 16 MiB zstd decoded-size ceiling across Python, Rust, Go,
  TypeScript, browser TypeScript, Kotlin, and Smalltalk. Implementations now
  rely on streaming/backpressure, storage, platform allocation failure, and
  corrupt-data failures instead of rejecting solely because decoded output
  exceeds a codec-level byte cap.

## [0.9.9]

### Fixed

- Native Turtle/TriG parser now resolves `PN_LOCAL_ESC` escapes in prefixed-name
  local parts: a backslash-escaped delimiter (`\(`, `\)`, `\,`, `\.`, …) is part of
  the local name, not a statement/object delimiter, and expands to the literal
  character in the IRI (e.g. `dbr:Semantic_analysis_\(linguistics\)` →
  `http://dbpedia.org/resource/Semantic_analysis_(linguistics)`). An escaped trailing
  `\.` is kept rather than stripped as the statement terminator.

## [0.9.7]

### Added

- Full RDF 1.2 reification/annotation surface in the native Turtle/TriG parser:
  `<< s p o [~ id] >>` reifying triples (expanding to `reifier rdf:reifies <<( s p o )>>`),
  `{| … |}` annotation blocks and `~` reifiers in any sequence, `VERSION`/`@version`
  directives, optional predicate-object lists for self-asserting subjects, and
  `rdf:annotationNodeID` (blank-node reifier) in RDF/XML.

### Fixed

- Serialize a triple term in quad-object position: the self-referential reifier entry
  (a triple term keying its own components in the reifiers map) no longer trips the
  event source's `cycle while declaring term N` guard.

## [0.9.8]

### Fixed

- Turtle/TriG prefixed names admit internal dots (`repo:README.md`): a `.` is no longer
  a name delimiter; only a trailing dot terminates the statement.
- The N-Triples/N-Quads statement-layer fold HARD-FAILS on a conflicting `rdf:reifies`
  rebind (same reifier subject, different triple term) instead of silently
  last-write-winning (CONSTITUTION P7); an identical rebind stays idempotent.

## [Unreleased]

### Added

- Installable C ABI archive packaging for `libgts`, including pkg-config,
  CMake, checksum, SBOM, provenance, and immutable GitHub Release publication
  through the `capi-v*` tag lane.
- Publishable `gmeow-gts-capi` Rust source crate metadata and a bootstrap
  crates.io release workflow for the C ABI source package.
- Credential-free package dry-run CI for the C ABI wrapper family, covering
  local package/artifact validation without registry publication secrets.

### Changed

- Corrected the Go module release tag shape to `go/v<version>` so
  `go.blackcatinformatics.ca/gts` versions in the `go/` subdirectory are
  discoverable by the Go proxy and pkg.go.dev.
- Moved canonical `visual-hashing` publication to
  `Blackcat-Informatics/visual-hashing`; version `0.1.3` is published from that
  repository with crates.io Trusted Publishing provenance, and the old monorepo
  release workflow is retired.
- Replaced the Rust Oxigraph/OxRDF/Sophia adapter surfaces with native RDF
  dataset, text-codec, RDF/XML, ULID, and in-memory store implementations; CI
  now locks the `wasm32-unknown-unknown --all-features` library build and audits
  that dependency tree to keep the removed toolkit blockers out.

## [0.9.6]

### Added

- Native Turtle/TriG parsing of bare numeric (`xsd:integer`/`decimal`/`double`),
  boolean (`xsd:boolean`), and single- and triple-quoted string literals, with
  lexical forms preserved verbatim (`0.70`, `1.0E0` survive unchanged). This
  closes the gaps that previously forced an oxttl-backed codec fallback, so the
  hand-rolled native codecs fully replace `oxttl`/`oxrdfxml`/`oxrdfio` with no
  text-codec dependency on the Oxigraph family.

### Changed

- N-Quads/N-Triples language-tag validation now accepts long BCP-47 private-use
  subtags (e.g. `x-gmeow-norwegiannynorsk`): once the `x` singleton appears the
  8-char per-subtag cap is dropped for the remainder. This is the native
  equivalent of the prior oxttl `.lenient()` mode and lets GMEOW's long
  private-use language tags round-trip through every native text codec.

## [0.9.5] — 2026-06-22

Rust-only release. Python, Go, TypeScript, and wrapper package versions remain
at `0.9.4`.

### Added

- Rust RDF event source/sink API for folded GTS graphs, including declaration
  ordering for sinks that require term declarations before references.
- Rust RDF text codecs for N-Triples, Turtle, TriG, and RDF/XML behind the
  `rdf-codecs` feature.
- Rust XSD lexical validation annotations for parsed RDF literals.
- C ABI RDF format discovery and conversion helpers for the new Rust codec
  surface.

### Changed

- Folded the RDF event protocol back into the main `gmeow-gts` crate so the
  Rust release does not require publishing a second support crate.

## [0.9.4] — 2026-06-21

### Added

- Rust GTS terms now preserve RDF 1.2 language-tagged literal base direction
  (`"dir": "ltr" | "rtl"`) across the wire model, deterministic writer,
  multi-segment union, N-Quads/TriG projection, YAML-LD import/export, SQL
  exports, and dump-directory JSONL tables.
- Regression coverage for multiple distinct reifiers bound to the same asserted
  triple.
- Regression coverage documenting the blob contract: inline blob frames preserve
  bytes, while external blob records carry metadata and digest references only.

### Changed

- Bumped lockstep package metadata to `0.9.4` across release manifests,
  citation metadata, C API metadata, Kotlin metadata, and documentation snippets.
- Native `oxrdf` export now refuses RDF 1.2 directional literals rather than
  silently dropping base direction on an adapter surface that cannot carry it.

## [0.9.2] — 2026-06-19

### Added

- Rust-only YAML-LD-star/JSON-LD-star import/export behind the `yaml-ld`
  Cargo feature, including `gts to-yaml-ld` and `gts from-yaml-ld`. This is a
  transform-only feature with no wire-format, canonical-catalog, or shared
  corpus oracle change, so no GIP is required.
- Rust `gts dump --directory` inspection export for versioned directory trees
  containing folded N-Quads, JSONL tables, unfolded frame views, blob indexes,
  and files-profile payloads.
- TriG import/export support for readable graph-block interchange over the same
  folded RDF content.
- Rust OKF profile import/export, an OKF conformance corpus, and ecosystem
  positioning for Markdown/YAML-frontmatter knowledge bundles.
- Files-profile-v2 metadata support plus the Rust tar bridge: `from-tar`,
  `to-tar`, and tar-compatible `gts tar -c/-x/-t/-d`.
- Tar round-trip conformance fixtures covering ustar, PAX metadata, gzip/zstd,
  links, special nodes, and unsafe archive refusals.

### Changed

- Documented OKF and tar as intentional Rust-first extension surfaces with
  explicit future parity gates for Python, Go, and TypeScript.
- Strengthened ecosystem and parity drift guards for the new OKF and tar
  documentation surfaces.

### Security

- Hardened files-profile-v2 and tar extraction behavior around traversal,
  symlinks, hardlinks, special files, ownership, setuid/setgid bits, and unsafe
  output paths.

## [0.9.1] — 2026-06-19

### Added

- v1.0-rc1 release support docs: release checklist, third-party implementer guide,
  benchmark release report template, paper draft, and public release verification workflow.
- Go and TypeScript `from-nq` parity so all four engines can import N-Quads into GTS.

### Changed

- Refined README, specification, conformance, vector-manifest, streaming-reader, and
  benchmark documentation for clearer release-candidate evidence and implementer guidance.
- Improved language-specific quality gates and docs: Rust public API docs and `rustdoc` gate,
  Python inline docs and typing, Go docs plus `golangci-lint`, and TypeScript API docs plus
  stricter lint rules.
- Added project DOI and homepage metadata across package and citation surfaces.

### Security

- Hardened release publication with crates.io Trusted Publishing, GitHub SLSA provenance,
  SPDX SBOM attestations for release artifacts, immutable Go GitHub Releases, and release
  smoke verification.
- Documented the current release SLSA posture as Build Level 2 evidence and the signer
  workflow requirements for any future Build Level 3 claim.

## [0.9.0] — 2026-06-18

### Added

- Rust parity expansion: N-Quads import, SQLite/DuckDB/Parquet exports, lazy blob
  decoding, verification APIs, policy/nested reader APIs, agent-memory example,
  streaming sink reader, MMR proof support, replication inventory verbs, deterministic
  writer helpers, transform encryption APIs, and native RDF adapters for `oxrdf`,
  Oxigraph, and Sophia.
- Cross-engine service/browser parity: Go streaming reader sinks, replication verbs
  across Rust/Go/TypeScript/Python CLIs, TypeScript browser streaming/WebCrypto export,
  and cross-engine MMR proof verification.
- Repository contracts for API/CLI parity, advanced primitives, ecosystem integrations,
  conformance, security policy, vector manifest metadata, and downstream Rust ontology
  CI coverage.

### Changed

- Vector corpus and conformance documentation now stamp corpus revision metadata and
  sharpen total-reader, fold-semantics, media-type, CDDL/preimage, and publication
  guidance.
- Release documentation now records visual-hashing publish ordering and lockstep
  release expectations for the four GTS engines.

### Security

- Added security policy parity for Go and TypeScript, including nested-GTS/profile
  policy coverage, and documented deferred multi-recipient crypto behavior.

## [0.2.0] — 2026-06-17

### Added

- **`visual-hashing` standalone Rust crate (#16).** The emojihash utility
  (BLAKE3-XOF → 6-bit → 64-emoji visual hash) is GTS-independent, so it now lives
  in its own crate, `visual-hashing/`, ready to publish to crates.io
  independently of GTS. It also carries **randomart** (the OpenSSH "Drunken
  Bishop" ASCII-art fingerprint) — ported to Rust from the Python reference and
  gated by a new frozen corpus (`vectors/randomart/*.json` +
  `scripts/gen_randomart_vectors.py`), so the Rust art reproduces Python's
  byte-for-byte. The `gmeow-gts` crate now depends on `visual-hashing` and
  re-exports it as `gmeow_gts::emojihash`, so existing paths keep working. The
  crate's only dependency is `blake3`; it stays `wasm32`-friendly. (The
  standalone crate is published to crates.io as `visual-hashing 0.1.1`; publish
  it first whenever the Rust `gmeow-gts` crate bumps its compatible
  `visual-hashing` requirement, since `gmeow-gts` packaging resolves that
  dependency from crates.io rather than the in-repo path.)

- **Cross-engine COSE_Encrypt0 (#18): AES-256-GCM payload encryption in all four engines.**
  - `encrypt0` / `decrypt0` / `recipient_kid` (§9.3) in Rust (pure-Rust
    `aes-gcm`), Go (`crypto/aes` + `crypto/cipher`), and TypeScript
    (`@noble/ciphers`) — byte-compatible with the Python reference. The COSE
    structure is `16([protected{1:3}, unprotected{4:kid, 5:iv}, ciphertext])`
    with the Enc_structure (`["Encrypt0", protected, b""]`) bound as AAD.
  - Because AES-GCM uses a random 12-byte IV, the seal is not deterministic, so
    conformance is gated two ways: a **fixed-IV vector**
    (`vectors/encrypt0/basic.json` + `scripts/gen_encrypt0_vectors.py`) freezes
    the exact transform (every engine reproduces the sealed bytes and opens
    them), and each engine round-trips a **random-IV** seal. Verified live
    across engine boundaries (e.g. Go seals → Python/Rust open).
  - The Rust seal/open paths stay pure / wasm-friendly: `aes-gcm` is built with
    `getrandom` off, and the random-IV `encrypt0` convenience (the only RNG user)
    is gated off `wasm32`, so the `wasm32-unknown-unknown` build is still green.
  - This was the last tracked crypto-parity follow-up from #15; signing (#15),
    `extract-key` (#17), and now encryption (#18) all reach four-engine parity.

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
  - COSE_Encrypt0 landed as #18 (above) — four-engine crypto parity is complete.

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

[Unreleased]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.10...HEAD
[0.9.10]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.9...rust-v0.9.10
[0.9.9]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.8...rust-v0.9.9
[0.9.8]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.7...rust-v0.9.8
[0.9.7]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.6...rust-v0.9.7
[0.9.6]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.5...rust-v0.9.6
[0.9.5]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.4...rust-v0.9.5
[0.9.4]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.9.2...py-v0.9.4
[0.9.2]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.9.1...py-v0.9.2
[0.9.1]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.9.0...py-v0.9.1
[0.9.0]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.2.0...py-v0.9.0
[0.2.0]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.1.3...py-v0.2.0
[0.1.3]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.1.2...py-v0.1.3
[0.1.2]: https://github.com/Blackcat-Informatics/gmeow-gts/releases/tag/py-v0.1.2

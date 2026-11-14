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

The wire format is FROZEN for the version 1 line (`GTS-SPEC.md` document
version `1.0-rc1`). A file that is valid GTS wire-format major version 1 will
remain parseable and safely foldable by future major-version-1 readers; a
breaking wire change requires a new wire-format major version.

## [Unreleased]

### Fixed

- Frame signatures now honour the signing algorithm declared in the COSE
  protected header (§9.2: readers MUST honour the declared algorithm). Rust,
  Python, Go, TypeScript (including the browser WebCrypto path), Kotlin and
  Smalltalk dispatch on `alg`: EdDSA is verified as Ed25519, and any other
  declared algorithm is reported `unverified` with its `kid` kept, never
  `invalid` and never `valid`. Before this, every engine ran Ed25519
  unconditionally, so a signature over another algorithm was reported `invalid`
  (or, with a protected header declaring another algorithm over a genuine
  Ed25519 signature, `valid`). A COSE_Sign1 whose protected header does not meet the following grammar is
  malformed, and its signature is `invalid`; so is an EdDSA signature that is not
  64 bytes long once its key resolves:

  > The protected header is a byte string whose content is deterministically
  > encoded CBOR (definite lengths, shortest-form integers and lengths, map keys
  > in bytewise order of their encodings, no duplicate keys, no trailing bytes)
  > consisting of exactly one map. Every map key, at any depth, is an integer or a
  > text string. Every value, at any depth, is an integer, a byte string, a text
  > string, an array, a map, null, true or false. Every integer, key or value,
  > lies in the range -2^63 to 2^63-1 inclusive. Text strings are valid UTF-8.
  > Nesting is at most 8 levels deep, counting the header map itself as level 1.
  > The map contains the integer key 1, whose value is an integer or a text
  > string. A header that does not meet all of this is malformed and the signature
  > is `invalid`.

  Written bytes are unchanged: signing output and every vector that existed
  before this change are byte-identical. New vectors `vectors/signed/sigalg-*.json` pin the behaviour, including the grammar's boundaries (integer range, key types, UTF-8, nesting depth), with the private-use algorithm identifier -65537; each engine's tests check every one of them against its stated status.
- Other behaviour changes: Rust no longer reports a malformed COSE_Sign1 as
  `unverified`; Smalltalk now reports a missing `kid` as `invalid` where it
  reported `unverified`; the Python reader no longer raises on undecodable
  signature bytes, and Python no longer reports a `kid` for a malformed
  COSE_Sign1, as in every other engine; Rust's `cose::parse` is no longer
  public.
- Consequence for scripts: the command-line `verify` of every engine, when
  given a key, treats only `invalid` as a failure. A signature with an
  unsupported algorithm and a resolvable `kid` is now `unverified` and exits 0
  where it exited 1 before, as an unresolved `kid` already did. Callers that
  must reject unverified signatures should check for `unverified` explicitly.
- The Python reader no longer lets its CBOR library interpret tags. GTS gives no
  tag a meaning except `55799` around a Header (§3), but the library built sets,
  decimals, dates and similar values from other tags, so what the reader accepted
  depended on the library release. Every tag now decodes to an uninterpreted
  `CBORTag`, for items in the file, for transformed payloads and for COSE
  structures, identically under `cbor2` 6.1.5 and 6.2.1. A complete CBOR item that
  is not a frame, with or without a tag, is reported as a `DamagedFrame` (a
  frame that is not a map, or a self-hash mismatch for a map) and the fold goes on
  with the next item. Before this, with `cbor2` 6.2.1, a tag `258` item over
  something other than an array, or a tag `4` or `5` item with a part that is not an
  integer, made the reader report a torn append (`TornAppendError`) and stop
  folding, although the item is complete CBOR. With 6.1.5 the same bytes were
  reported as a damaged frame and the later frames still folded. This change
  concerns well-formed tagged or stray items. Other decode failures, such as
  excessive nesting or invalid UTF-8, are unchanged by it and are not yet reported
  alike by all engines.
  Tags `2` and `3` stay integers (a byte string content becomes the integer it
  carries), so a frame is hashed over the integer, as in the Rust, Go and TypeScript engines, and a
  bignum written in a non-preferred form is a self-hash mismatch there and here.
- `decrypt0` and `recipient_kid` report undecodable bytes and arrays of the wrong
  length as a malformed `COSE_Encrypt0` (`missing-key`, or no `kid`) instead of
  raising `CBORDecodeError` or `IndexError`, and a COSE structure under a tag such
  as `258` is no longer unpacked as a Python set whose order decided the outcome.
  Whether an engine accepts a COSE structure under a tag other than `18` or `16`
  differs between engines and is not yet pinned by a vector.
- New vectors `28i-stray-set-between-frames`, `28j-stray-set-last-item`,
  `28k-stray-decimal-between-frames`, `28l-stray-decimal-last-item`,
  `28m-stray-self-reference-in-set` and `28n-stray-self-reference-as-key` pin the
  outcome for a stray tagged item between frames and as the last item: a
  `DamagedFrame`, no `TornAppendError`, and the quad after the item still folds.
  `28o-suppression-over-damaged-terms` pins a suppression in a later segment that
  names the ids of a damaged `terms` frame, and `28p-tagged-value-in-frame-field`
  pins a tagged value inside a frame field.
  Written bytes are unchanged: every vector that existed before is byte-identical.
- `read` no longer raises `IndexError` when a suppression in a later segment names a
  term id that the damaged `terms` frame of its segment never defined; the id is
  kept as received and the damage is the only diagnostic, as in Rust, Go and
  TypeScript.
- The Kotlin and Smalltalk decoders now delimit floating-point items and other
  well-formed simple values and keep them uninterpreted. Before this, a complete item
  holding one, such as a tag `4` item with a half-precision float part, was reported
  as a torn append and the fold stopped; it is now a damaged frame and the fold goes
  on, as the specification requires.
- A `zstd` or `zstd-rsyncable` payload whose frame or block structure is invalid (not
  zstd, cut short, a bad block header), that names a dictionary, or that declares a
  window the reader will not decode, now makes that one frame an opaque node (reason
  `damaged`, diagnostic `DamagedFrame`) and the fold goes on past it, in Python, Rust and
  TypeScript (its Node and browser builds). Go already did; Kotlin and Smalltalk are
  expected to, by reading only, since they were not run. Reporting an intact frame that
  a reader merely cannot decode as `damaged` is a gap in the specification's opaque
  reasons (§7.6 names only `unknown-codec`, `missing-key` and `damaged`), not a rule.
  Before this, Python and Rust registered a blob frame under its declared digest without
  looking at its payload: Python then raised the zstd library's error, with a traceback,
  from `gts ls` and `gts extract`, and returned fewer bytes without any error for a
  payload cut short; Rust reported a clean error on read but folded no opaque node. Both
  now walk the zstd frame and block headers when they fold the blob frame (a few bytes
  per 128 KiB block, no decompression, and not repeated when the blob is read). Damage
  inside a block still surfaces on first read; Python's command line reports it as
  `gts: cannot decode blob` with status 1 instead of a traceback, and library callers
  that caught `zstandard.ZstdError` from `g.blobs[...]` now get `PayloadDecodeError`
  (a `ValueError`). The command-line tools of Go and TypeScript exit with status 1 when
  any `DamagedFrame` is reported, as before.
- TypeScript no longer decodes a zstd frame that declares a window above 2**27 bytes
  (128 MiB), in either build. Its zstd library returns wrong bytes, with no error, once
  a match reaches back about 256 MiB, which a window of 2**29 or more allows; 2**27 is
  the largest window every reference reader was measured to decode correctly. The limit
  is one named constant (`MAX_ZSTD_WINDOW_LOG`) in a module both builds share, and any
  structure the walk does not accept is refused by the walk itself, not left to the
  library, which accepts more than the format does. As a backstop for every other way
  that library can return wrong bytes (a dictionary without a stored id, a single-segment
  frame declaring a content size it does not have), a blob whose decoded bytes do not
  hash to its declared `pub.digest` is now a damaged frame in both builds, whether the
  digest is written as text or as the raw 32 bytes; before it was listed under the hash of
  the wrong bytes with no diagnostic. The declared text is compared as Python and Rust
  compare it: bare hex gets the `blake3:` prefix and is accepted, but upper-case hex is
  not the lowercase form the specification requires and now makes the blob damaged.
- This changes what the Python reader accepts. It now refuses any frame that declares a
  window above 2**27, whether or not its library would decode it: the library skips its
  own limit when the whole content fits the caller's buffer, so what it accepted depended
  on a buffer size and not on the frame. Hand-built frames that declare a larger window
  over a small known content size were decoded before and are now opaque; no real
  encoder is known to write them. Rust's walk now also refuses a window above 2**31, the
  most its decoder accepts, at fold instead of at first read; no other engine's limit
  changed.
- Not settled here, and not decided by the format: the engines still disagree on frames
  the format permits. A frame declaring a window of 2**28 is decoded by Rust, Go and
  Smalltalk and refused by Python, TypeScript and (by reading) Kotlin; 2**29 by Rust and
  Go only; 2**30 by Rust and Smalltalk only. A frame that names a dictionary id is
  refused at fold by Python, Rust, TypeScript and Go, and fails at decode in Kotlin and
  Smalltalk (by reading); a frame compressed with a dictionary that stores no id (a
  raw-content dictionary) carries id 0, so no header walk sees it: Python and Rust fold it
  and fail on first read, TypeScript catches it only through the blob digest, and Go marks
  it damaged at fold. Smalltalk fails any frame that carries no content size. Whether the
  specification should name one window limit and a dictionary parameter is open.
- The Smalltalk decoder now decodes a payload of concatenated zstd frames to its end.
  It sized its output from the first frame only, so every `zstd-rsyncable` payload over
  one unit, which the reference writers make of one frame per unit, was reported as a
  damaged frame.
- New vectors `22b-zstd-rsyncable-multi-frame` (a payload of three concatenated frames,
  decoded, blob digest matching), `22c-zstd-declared-window-2-27`, `28q-blob-payload-not-zstd`,
  `28r-blob-payload-truncated` and `28s-blob-payload-names-dictionary` (one opaque node
  each, with frames before and after it folded). Every vector that existed before is
  byte-identical.

### Changed

- The Rust engine's zstd implementation was updated (`structured-zstd` 0.0.48 to 0.3.0) and now produces different compressed bytes for some inputs, all of which decode identically.
- Because a frame id covers the stored, compressed payload, a head written through the Rust library with a zstd codec changes with the compressor version, so such a head must not be pinned or compared between versions; compare content instead.
- The updates to `getrandom`, `aes-gcm`, `quick-xml`, `ed25519-dalek` and `visual-hashing` have no effect on written bytes.
- The TypeScript engine now uses `@noble/ed25519` 3.2.0 and `@noble/hashes` 2.4.0
  (both major updates), and its imports follow the new module layout (`sha2.js` and
  `legacy.js` replace `sha512.js` and `sha1.js`). Written and signed bytes are
  unchanged: signatures, digests and OpenPGP fingerprints are identical, shown by a
  before-and-after comparison. Signature verification changes for degenerate
  signatures that only a holder of the signing key can produce; honest signatures are
  unaffected. The old library accepted one signature built on the identity point for
  every message under a key, and the new library rejects it. The new library verifies
  over the `R` bytes of the signature as received (ZIP-215), so a signature with a
  non-canonical `R` that verified before no longer does, and the converse. The
  verification policy itself is unchanged and remains a format-wide decision.
- The Python dependencies were updated, among them `cbor2` 6.1.5 to 6.2.1. Written bytes
  are unchanged: the corpus regenerates byte-identically under both releases. The
  update removes a crash: `cbor2` 6.1.5 ended the process with a segmentation fault
  on a self-referential tag used as a map key or set element, and 6.2.1 raises an
  error that can be caught. The reader no longer hands tags to the library to
  interpret (see Fixed), so neither release can crash on such input and both fold
  it the same way.

## [1.0.0-rc.1] — 2026-09-04

### Changed

- **BREAKING (reader contract).** A quoted triple TERM is now self-describing: a
  `terms` row may carry `"tt": [s, p, o]`, which is AUTHORITATIVE when present.
  `"rf"` remains a legal legacy encoding and resolves to the first binding of
  its reifier, so existing bytes read unchanged — but a pre-existing
  third-party reader is now non-conformant, because `rdf:reifies` is NOT
  functional in RDF 1.2 and the `reifies` frame is a MULTI-VALUED statement
  layer. Every row survives with its own graph slot; a reader may not choose
  among them, reorder them, or drop any. Only byte-identical rows collapse
  (§7.8).
- `ConflictingReifier` is narrowed to the one genuinely incoherent shape: a
  `"tt"`-less term whose reifier binds more than one distinct triple. It is
  reported once per offending term, drops no row, and that term still resolves
  to the first binding in file order. Conformance vector 12 changes from a
  negative to a positive case.
- **BREAKING (§14 relational projection).** The `terms` table of the
  `gts → {sqlite,duckdb,parquet}` export grows from 7 columns to 10, appending
  `tt_s`, `tt_p`, `tt_o`. Existing positional indices 0-6 are unchanged, but a
  reifier id no longer identifies a triple, so any consumer keying or joining
  on `reifier` and expecting one triple will silently drop rows.

### Added

- Normative position constraint: in a `reifies` row the reifier MUST be an IRI
  or blank node (`k:0|2`), never a literal or quoted triple. It is the SUBJECT
  of the `R rdf:reifies <<( S P O )>>` statement every row asserts, and RDF 1.2
  admits only those kinds there. Violations are diagnosed `PositionConstraint`
  and the row is dropped. This closes a gap where the shape folded and each
  engine improvised a different, sometimes invalid, projection.
- Conformance vectors `13b-reifier-position-constraint`,
  `13c-self-describing-triple-term` and `13e-literal-base-direction`. The last
  covers RDF 1.2 literal base direction, which was fully implemented across six
  engines but had no frozen coverage at all — base direction belongs to a
  literal's identity, so an implementation could carry it on one write path and
  silently conflate on another while passing every vector.

### Fixed

- Resolution of a triple term now terminates on every walk, with ONE normative
  strategy rather than a choice. A `reifies` row may name the very term that
  resolves through it, which is constructible with the ordinary writer and needs
  no `"tt"`. Before this, folding such a file aborted Go with
  `fatal error: stack overflow` — unrecoverable, and a violation of the reader
  no-panic contract `FuzzRead` defends — and raised `RecursionError` in Python.
  Both are parser denial-of-service. A self-reaching term now states no triple
  and renders as a fresh blank node; the row is retained and still projects. The
  degradation applies to the term ITSELF, not to a nested occurrence — the two
  render different graphs, so the level is normative too.
- Go no longer discards an authoritative `"tt"` when a legacy `"rf"` is also
  present. A `Graph → events → Graph` round trip demoted such a term, losing
  its own components and minting a `reifies` row the source never had.
- The Rust writer emits `"tt"` only for `k:3` terms, so authored bytes cannot
  read back as something other than what was written.

## [0.9.11] — 2026-06-29

### Changed

- `reifies` and `annot` rows now carry the same optional graph term-id as
  `quads`, restoring per-named-graph RDF 1.2 reification and annotation
  partitioning. `reifies` is now a row-array payload
  (`[rid, s, p, o, g?]`) rather than a reifier-id map.
- Release metadata is aligned at `0.9.11` for the coordinated schema update.

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

## [0.9.8]

### Fixed

- Turtle/TriG prefixed names admit internal dots (`repo:README.md`): a `.` is no longer
  a name delimiter; only a trailing dot terminates the statement.
- The N-Triples/N-Quads statement-layer fold HARD-FAILS on a conflicting `rdf:reifies`
  rebind (same reifier subject, different triple term) instead of silently
  last-write-winning (CONSTITUTION P7); an identical rebind stays idempotent.

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

## [0.9.6]

### Added

- Native Turtle/TriG parsing of bare numeric (`xsd:integer`/`decimal`/`double`),
  boolean (`xsd:boolean`), and single- and triple-quoted string literals, with
  lexical forms preserved verbatim (`0.70`, `1.0E0` survive unchanged). This
  closes the gaps that previously forced an oxttl-backed codec fallback, so the
  hand-rolled native codecs fully replace `oxttl`/`oxrdfxml`/`oxrdfio` with no
  text-codec dependency on the Oxigraph family.
- Installable C ABI archive packaging for `libgts`, including pkg-config,
  CMake, checksum, SBOM, provenance, and immutable GitHub Release publication
  through the `capi-v*` tag lane.
- Publishable `gmeow-gts-capi` Rust source crate metadata and a bootstrap
  crates.io release workflow for the C ABI source package.
- Credential-free package dry-run CI for the C ABI wrapper family, covering
  local package/artifact validation without registry publication secrets.

### Changed

- N-Quads/N-Triples language-tag validation now accepts long BCP-47 private-use
  subtags (e.g. `x-gmeow-norwegiannynorsk`): once the `x` singleton appears the
  8-char per-subtag cap is dropped for the remainder. This is the native
  equivalent of the prior oxttl `.lenient()` mode and lets GMEOW's long
  private-use language tags round-trip through every native text codec.
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

[1.0.0-rc.1]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.11...rust-v1.0.0-rc.1
[0.9.11]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.10...rust-v0.9.11
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

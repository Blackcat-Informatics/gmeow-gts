<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Conformance

This document defines how an implementation makes a testable conformance claim for the
Graph Transport Substrate (GTS). It is a companion to
[`GTS-SPEC.md`](./GTS-SPEC.md): the spec defines the wire format and behavior; this document
defines tiers, vector subsets, expected-result formats, diagnostics, and read modes used to
compare implementations.

## 1. Conformance Claims

A conformance claim MUST name:

- the implementation name and version;
- the conformance tier or tiers claimed (§3);
- the read mode or verify mode used (§7);
- the corpus revision, usually the repository commit containing `vectors/`;
- the vector subsets passed (§2);
- any optional capabilities enabled, such as COSE keys, encryption keys, profile validators,
  or nested-GTS recursion;
- the exact command or test harness used to produce the result.

The claim is only meaningful for the named tier and capability set. For example, a Baseline
Reader can pass `baseline-reader` without claiming COSE signature verification, decryption,
nested-GTS recursion, or profile policy enforcement.

## 2. Vector Subsets

The frozen corpus currently has one top-level `vectors/<id>.gts` byte file and one
`vectors/<id>.expected.json` expected fold per case. Additional JSON subcorpora cover COSE,
Encrypt0, OpenPGP key extraction, emojihash, and randomart. These named subsets are the units
used by tier claims:

| subset | vectors | purpose |
|---|---|---|
| `wire-core` | `01-minimal`, `02-zstd-frame`, `06-header-tampered` | Header/frame grammar, mandatory codecs, deterministic CBOR, and header hash behavior. |
| `total-reader` | `03-unknown-codec`, `04-damaged-frame`, `05-torn-append`, `17-pre-segment-hard-fail`, `19-profile-union-opacity`, `28-empty-file`, `28b-non-header-item`, `28c-unsupported-version`, `28d-unknown-frame-type`, `28e-forward-term-reference`, `28f-malformed-transform-shape` | Graceful degradation, diagnostics, opaque nodes, torn input, malformed/boundary behavior, unsupported headers, and extension-frame opacity. |
| `graph-fold` | `09-suppression`, `11-datatype-defaulting`, `12-conflicting-reifier`, `13-position-constraint`, `14-bnode-label`, `15-two-segment-union`, `15b-anon-bnode-union`, `16-composed-round-trip`, `18-cross-segment-suppression`, `22-inline-blob` | Core graph fold, value equality, annotations/reifiers, suppressions, blobs, and multi-segment union. |
| `profile-layout` | `20-language-tag-discipline`, `21-degenerate-composition`, `23-files-profile-tree`, `24-files-profile-dedup`, `25-streamable-source`, `25b-streamable-compacted`, `26-streamable-lie`, `27-streamable-tail` | Profile conventions, archive/files profile behavior, streamable layout, compaction, and publication-tool refusal cases. The live `scripts/interop.sh` guard adds cross-engine `files` pack/unpack/diff command evidence for this subset. |
| `streaming-property` | every top-level `vectors/*.gts`, tested at each CBOR item boundary | Prefix-fold totality and monotone fold growth for streaming readers. |
| `corpus-generator-determinism` | every top-level `vectors/*.gts` | Reference generator reproducibility for the frozen corpus, including intentionally damaged, torn, tampered, and malformed fixtures. This proves corpus-build repeatability, not public Writer conformance. |
| `writer-determinism` | valid top-level writer outputs, including `25b-streamable-compacted` as the streamable compaction byte oracle and `29-deterministic-writer` as the graph-authoring byte oracle | Reproducible public writer output, deterministic hashes, deterministic graph authoring, and deterministic compaction under fixed parameters. Negative corpus fixtures MUST NOT use this subset. |
| `crypto-cose` | `vectors/cose/*.json`, `vectors/signed/basic.json` | COSE Sign1 serialization, per-frame signatures, and signature verification behavior. |
| `crypto-encrypt` | `vectors/encrypt0/basic.json` | COSE Encrypt0 sealing/opening behavior for engines that implement encryption. |
| `crypto-deferred` | `vectors/crypto-deferred/*.json` | Deferred multi-recipient `COSE_Encrypt` and ECDH-ES+A256KW contract descriptors. These vectors prevent premature support claims; they are not v1 implementation vectors until byte-level fixtures and interop harnesses replace the placeholders. |
| `openpgp-transport-key` | `vectors/openpgp/*.json` | Embedded OpenPGP transport-key extraction and cross-engine fingerprint/emojihash agreement. |
| `human-hash` | `vectors/emojihash/*.json`, `vectors/randomart/*.json` | Human-facing digest rendering used by CLIs and release tooling. |
| `security-policy` | `vectors/security/*.json` | Profile trust-policy separation, pseudonymous opaque recipients, and nested-GTS recursion-limit negative cases. |
| `advanced-index-proof` | `vectors/proofs/*.json` plus implementation-created indexed files | Stable MMR preimages, detached inclusion-proof JSON verification, bad-proof rejection, and optional `index.mmr` reader diagnostics. |

A tier MAY require a subset plus extra mode-specific assertions. For example,
`profile-layout` contains files that permissive readers fold, while validating tools must also
refuse specific publish-class or verify-class violations.

## 3. Tiers

| tier | required subsets and checks | claim string |
|---|---|---|
| Baseline Reader | `wire-core`, `total-reader`, `graph-fold`, and `profile-layout` in permissive-read mode; expected graph JSON matches; diagnostics match; malformed inputs never panic or abort the process. | `GTS Baseline Reader, corpus <commit>` |
| Streaming Reader | Baseline Reader plus `streaming-property`; implementation exposes a non-materializing sink API that emits segment-local fold events while preserving final diagnostics and segment heads. Retained memory is expected to be bounded by `O(distinct terms + maximum decoded frame size + validation sidecar state)`, not folded triples or blobs. | `GTS Streaming Reader, corpus <commit>` |
| Full Reader | Baseline Reader plus implemented optional subsets, at minimum `crypto-cose` for signature verification if claiming signature support, `crypto-encrypt` if claiming decrypt support, `security-policy` when claiming nested-GTS recursion, and index/MMR behavior when present. | `GTS Full Reader (<capabilities>), corpus <commit>` |
| Writer | Emitted bytes are deterministic where the spec requires deterministic output, and writer-created files pass Baseline Reader expectations. Reproducible generation of intentionally invalid corpus fixtures is covered by `corpus-generator-determinism` and does not imply public Writer conformance. | `GTS Writer, corpus <commit>` |
| Validating Tool | Baseline Reader plus strict verify and publish-class verify modes (§7); `profile-layout` refusal vectors produce the required non-zero/refusal outcomes. | `GTS Validating Tool, corpus <commit>` |
| Profile-Aware Tool | Validating Tool plus the named profile validator; profile-specific diagnostics and warnings match the profile contract. | `GTS Profile-Aware Tool (<profile>), corpus <commit>` |

Within this repository, only the Go engine currently claims the Streaming Reader tier. Its
`reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` API reads from an `io.Reader` and emits
sink events without materializing the folded graph. The `go test ./reader -run
TestStreamingFoldCorpusEquivalence` harness checks corpus equivalence against the full Go reader
for final diagnostic codes, segment heads, profiles, metadata, streamable-layout state, and
segment-local fold event counts.

The Rust `read_to_sink` API provides event-emitting reader evidence, but does not satisfy the
current Streaming Reader tier requirements: it accepts a byte slice, decodes the item collection,
and uses the segment `Graph` path while emitting events. The TypeScript browser
`foldStream(stream, options)` and
`readStream(stream, options)` APIs are progressive Web Streams surfaces where `options.onEvent`
receives segment-local events as CBOR items arrive, but they still return materialized graph
state. Python currently provides prefix-fold and full-reader evidence only. Future Rust,
TypeScript, or Python claims need a non-materializing sink path plus memory evidence matching the
bound above.

A tool can claim multiple tiers. A command-line package that exposes `read`, `verify`,
`compact`, and `files` archive commands might claim Baseline Reader, Writer, Validating Tool,
and Profile-Aware Tool (`files`), while not claiming Full Reader if it cannot decrypt or
recurse into nested GTS blobs.
The cross-language API and command matrix for those public surfaces is maintained in
[`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md).
The advanced streaming sink, index/MMR/proof, replication, range-fetch, and benchmark deferrals
are maintained in [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md).
The trust/profile-policy, nested-GTS budget, and crypto-deferral contract is maintained in
[`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md).

## 4. Expected Graph Format

The current top-level corpus uses `vectors/<id>.expected.json`, generated by
`python/src/gts/vectors.py::expected_for`. Implementations MUST compare the same fields unless
the manifest explicitly narrows a vector:

```json
{
  "mode": "default",
  "diagnostics": ["UnknownCodec"],
  "terms": 3,
  "quads": 1,
  "segments": 1,
  "segment_heads": ["0123..."],
  "profiles": ["generic"],
  "streamable": [
    {"claimed": false, "covered": 0, "tail": 0}
  ],
  "opaque_reasons": ["unknown-codec"],
  "suppressions": 0,
  "blobs": {
    "blake3:...": {"size": 13, "mt": "text/html"}
  },
  "nquads": [
    "<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> \"Cat\"@en ."
  ]
}
```

Field semantics:

| field | meaning |
|---|---|
| `mode` | Read mode used by the vector. Current expected JSON values are `default` (permissive read) and `pre-segment`; manifest values use the explicit names in §7. |
| `diagnostics` | Ordered diagnostic code list emitted by the reader. Diagnostic details are not frozen in the current corpus. |
| `terms`, `quads`, `segments`, `suppressions` | Folded count summaries. |
| `segment_heads` | Hex segment head ids in file order. The last value is the file head for single-head verification. |
| `profiles` | Segment profile declarations folded from headers. |
| `streamable` | Per-segment layout state: claim flag, covered frame count, and accretive tail count. |
| `opaque_reasons` | Sorted opaque-node reason strings. |
| `blobs` | Inline blob digest to declared media type and decoded byte size. |
| `nquads` | Sorted RDF projection lines from the folded graph. Blank-node labels are expected to match the reference renderer unless the manifest declares isomorphism-only comparison. |

## 5. Vector Manifest Schema

The repository commits `vectors/manifest.json` as the portable manifest for the frozen corpus.
It makes the former file-pair convention explicit for top-level byte vectors and names the JSON
subcorpora used by optional crypto, human-hash, OpenPGP, signed, and security-policy checks.
The manifest uses this shape:

```json
{
  "schema": "https://blackcatinformatics.ca/gts/vector-manifest/v1",
  "manifest_version": 1,
  "corpus_revision": "git:<commit>",
  "generated_by": "gts.vectors",
  "vectors": [
    {
      "id": "03-unknown-codec",
      "title": "unknown codec degrades to opaque node",
      "input": {
        "path": "vectors/03-unknown-codec.gts",
        "media_type": "application/vnd.blackcat.gts+cbor-seq"
      },
      "mode": "permissive-read",
      "negative": true,
      "required_capabilities": ["cbor", "blake3", "identity"],
      "subsets": ["total-reader"],
      "tiers": ["baseline-reader"],
      "expected": {
        "graph": "vectors/03-unknown-codec.expected.json",
        "diagnostics": ["UnknownCodec"],
        "expected_head": "<hex-or-null>",
        "opaque_reasons": ["unknown-codec"]
      },
      "notes": "Reader must keep chain/fold total and surface the undecodable frame."
    }
  ]
}
```

The checked-in manifest uses
`"corpus_revision": "git:repository-commit-containing-manifest"` as a deliberate placeholder.
That placeholder avoids a self-referential commit hash in the file that contains the hash. It
is valid for repository validation, but it is not a release conformance identifier.

Release candidates and third-party conformance reports MUST replace the placeholder at report
time with an exact `git:` revision. The revision MUST be either a full 40-character commit id
that resolves in the repository or a local Git tag. Do not hand-edit the committed manifest for
this; generate a stamped release manifest artifact:

```bash
python scripts/check_vector_manifest.py \
  --release-manifest dist/vector-manifest.release.json
```

That command validates the corpus and writes a copy of the manifest whose `corpus_revision`
names the current `HEAD` commit. To stamp a release tag or an explicit commit instead, pass
`--corpus-revision git:<tag-or-full-commit>`. The plain
`python scripts/check_vector_manifest.py` command continues to validate the checked-in
placeholder manifest.

Required vector fields:

| field | requirement |
|---|---|
| `id` | Stable vector id; SHOULD match the file basename. |
| `input.path` | Path to the canonical input bytes or JSON fixture. |
| `mode` | One of `permissive-read`, `strict-verify`, `publish-verify`, `profile-verify`, `pre-segment`, or a profile-defined extension. |
| `negative` | `true` when the vector expects diagnostics, refusal, non-zero verify status, or a profile violation. |
| `required_capabilities` | Capability names needed to exercise the vector, such as `zstd`, `cose-sign1`, `encrypt0`, `cose-encrypt`, `ecdh-es+a256kw`, `openpgp`, `streamable-index`, or `files-profile`. |
| `subsets` | One or more subset names from §2. |
| `tiers` | Tier names from §3 that consume the vector. |
| `expected.graph` | Expected graph JSON path, or `null` for non-graph JSON fixtures. |
| `expected.diagnostics` | Expected diagnostic code list in reader emission order. |
| `expected.expected_head` | Expected final file or segment head hex when the vector asserts one; `null` when not asserted. |
| `notes` | Human explanation of the behavior being pinned. |

Optional vector fields include `expected.segment_heads`, `expected.exit_code`,
`expected.stderr_contains`, `expected.signature_status`, `expected.profile_findings`,
`compare.nquads` (`exact` or `bnode-isomorphism`), and `links` to spec sections.

## 6. Diagnostics Registry

Diagnostic codes are stable API. Implementations MAY add details, frame indexes, segment ids,
or profile-specific fields, but MUST NOT rename these codes when claiming the tier that owns
them.

Severity values:

- `fatal`: no complete graph can be folded for the requested mode or no later content may be
  safely interpreted.
- `error`: the reader/tool can usually return a partial fold, but strict verification fails.
- `warning`: permissive read succeeds and strict verify MAY succeed if the mode declares the
  condition non-fatal.
- `info`: machine-readable observation that does not make verification fail by itself.

| code | severity | applies to | reader behavior | recoverable? | opaque reason | required tier |
|---|---|---|---|---|---|---|
| `EmptyFile` | fatal | file structure | Return an empty graph/result and diagnostic. | no | none | Baseline Reader |
| `DamagedFrame` | error | header/frame hash, payload decode, malformed payload | Isolate the damaged item when possible, surface a diagnostic, and fold survivors when boundaries are known. | partial | `damaged` when represented as opaque | Baseline Reader |
| `BrokenChain` | error | id/prev chain | Surface the chain break; strict verify fails. | partial | none | Baseline Reader |
| `TornAppendError` | warning | trailing incomplete CBOR item | Ignore trailing incomplete bytes and fold the last complete prefix. | yes | none | Baseline Reader |
| `UnknownCodec` | warning | transform capability | Preserve the frame as opaque and continue folding known content. | yes | `unknown-codec` | Baseline Reader |
| `MissingKey` | warning | encrypted transform | Preserve the frame as opaque and continue folding known content. | yes | `missing-key` | Full Reader when decrypt support is claimed |
| `KeyWrapFailed` | warning | deferred multi-recipient encrypted transform | Preserve the frame as opaque when ECDH recipient metadata or AES-KW unwrap fails. | yes | `missing-key` | Future Full Reader when `cose-encrypt`/ECDH support is claimed |
| `ConflictingReifier` | error | graph fold | Keep the first binding in file order and ignore the conflicting binding. | yes | none | Baseline Reader |
| `PositionConstraint` | error | graph fold | Reject the offending row and continue folding other rows/frames. | yes | none | Baseline Reader |
| `ForwardReference` | error | term dictionary | Drop or ignore the invalid forward reference and continue folding safely. | yes | none | Baseline Reader |
| `SegmentBoundary` | fatal | pre-segment compatibility mode | Stop before misfolding a later segment as file-global ids. | no for that mode | none | Baseline Reader compatibility test |
| `IllTypedLiteral` | warning | RDF/XSD syntax import | Preserve the literal lexical form and datatype verbatim; expose a diagnostic and/or `gts:illTypedLiterals` metadata sidecar. | yes | none | RDF codec / Profile-Aware Tool |
| `TruncatedLog` | error | expected head / freshness | Fold observed bytes but fail verification against the requested head. | yes | none | Full Reader or Validating Tool |
| `StreamableLayoutError` | error | streamable layout claim | Fold bytes but make strict/profile verify fail for the layout claim. | yes | none | Validating Tool |
| `IndexMmrError` | error | optional index MMR root | Fold bytes but make strict verification fail for the index commitment. | yes | none | Full Reader when MMR/proof support is claimed |
| `RecursionLimit` | error | nested GTS recursion | Stop recursion and expose the nested content as unavailable/opaque. | yes | implementation-defined | Full Reader |
| `UnknownFrameType` | warning | extension frame | Preserve chain verification; either ignore or surface opaque/diagnostic until a profile handles it. | yes | `unknown-frame-type` if opaque | Profile-Aware Tool |

Profile validators MAY define additional profile-specific diagnostic codes, but they MUST use a
profile namespace or document the code in the profile specification.

## 7. Read And Verify Modes

| mode | purpose | behavior | test evidence |
|---|---|---|---|
| `permissive-read` | Library read/fold for consumers that want the best recoverable graph. | Never panic on malformed corpus inputs; return graph state plus diagnostics/opaque nodes; diagnostics do not prevent returning a result. | `wire-core`, `total-reader`, `graph-fold`, `profile-layout` as folded graph expectations. |
| `strict-verify` | Transport verifier for chain/hash/layout/signature checks requested by the caller. | Exit/fail on any error or fatal diagnostic; MAY permit documented warnings such as unsupported profiles if the mode declares them warnings. | CLI `verify` tests, `04`, `05`, `06`, `17`, `26`, signed/head tests. |
| `publish-verify` | Publication and rewrite gate for commands that create or distribute artifacts. | Refuse structurally valid but policy-invalid artifacts, such as empty-fold composition, suppress-everything composition, streamable lies, unsafe extraction, or non-reproducible compaction. | `21-degenerate-composition`, `22-inline-blob`, `25b-streamable-compacted`, `26-streamable-lie`. |
| `profile-verify` | Profile-aware validation above core wire-format validity. | Apply profile vocabulary, capability, trust, layout, and archive rules without redefining core GTS validity. | `19-profile-union-opacity`, `20-language-tag-discipline`, `23-files-profile-tree`, `24-files-profile-dedup`, `25`-`27`. |

Mode names are manifest values, not necessarily literal CLI subcommands. A CLI MAY expose
several modes through one command with flags; the test harness MUST record which mode was used.

## 8. Reporting

A conformance report SHOULD include:

- implementation name, version, commit, operating system, and architecture;
- exact corpus revision or tag used for the report, matching the stamped release manifest;
- tier claims and vector subsets;
- command lines or test names;
- pass/fail count by subset;
- any skipped vector with the missing capability named;
- diagnostics emitted for failed vectors;
- whether the corpus was regenerated and found reproducible.

Reports SHOULD be durable build artifacts for release candidates and SHOULD be attached to
release notes for v1.0 and later.

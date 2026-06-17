<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<!-- gts-security-policy:v1 -->
# GTS Security And Trust Policy

This document pins the v1 security contract that sits above the core GTS wire
format. The core reader verifies bytes, frame hashes, chains, and optional COSE
cryptographic validity. It does not decide whether a signer is authorized, a
claim is true, or a recipient identifier is privacy-preserving.

## Trust Separation

`Signature.status == "valid"` means the COSE signature verifies under a key the
caller resolved. It does not mean:

- the key is trusted by the deployment;
- the signer is authorized for the profile;
- the signed RDF claim is true.

Deployment trust is represented by `gts.policy.TrustPolicy`. A profile-aware
tool can require a trusted signer, while a baseline reader can still return the
recoverable graph plus signature status.

## Profile Enforcement

| Profile | Enforcement in v1 | Finding codes |
|---|---|---|
| `evidence` | Requires signed frames and a signed segment head in profile verification. Deployment trust is optional unless the caller supplies trusted signer ids. | `ProfileSignatureRequired`, `ProfileSignatureInvalid`, `ProfileSignatureUnverified`, `EvidenceHeadCommitmentRequired`, `ProfileSignerUntrusted` |
| `opaque` | Requires signed frames in profile verification. High-privacy recipient `kid` values must be pseudonymous: default pattern `anon:[0-9a-fA-F]{32,}`. | `ProfileSignatureRequired`, `OpaqueRecipientKidMissing`, `OpaqueRecipientKidPublic` |
| `bundle` | Nested GTS blobs are optional Full Reader behavior. Baseline readers treat them as ordinary blobs. Full Readers must enforce recursion and decoded-size budgets. | `RecursionLimit` |
| `files` / `stream` | Existing profile vocabulary and streamable-layout checks remain profile/tool policy, not core validity. | `ProfileVocabularyUndeclared`, `ProfileVocabularyUnused`, `StreamVocabularyWithoutLayout`, `StreamableLayoutError` |

## Nested GTS Budgets

Full Reader callers use `gts.read_nested(...)` to recurse into blobs whose
declared media type is `application/vnd.blackcat.gts+cbor-seq`. The result
exposes nested subgraphs by the containing blob digest. Recursion stops when
`max_depth` or `max_decoded_bytes` is exceeded and records `RecursionLimit`.

## Crypto Deferrals

| Capability | v1 tier decision |
|---|---|
| COSE_Sign1 / Ed25519 | Implemented optional Full Reader capability and profile-policy input. |
| COSE_Encrypt0 / AES-256-GCM | Implemented optional Full Reader capability for one direct recipient. |
| COSE_Encrypt multi-recipient envelopes | Deferred outside v1 conformance. No engine may claim it until vectors and interop tests land. |
| ECDH key-wrap / ECDH-ES+A256KW | Deferred outside v1 conformance. The spec examples remain informative until vectors and key-management policy exist. |
| Pseudonymous recipient-id policy | Implemented as profile policy for the `opaque` profile. |

## Vectors

The committed security-vector descriptors live in `vectors/security/`:

- `nested-recursion-limit.json` records the required negative `RecursionLimit`
  behavior for nested-GTS recursion.
- `profile-policy.json` records the trust/profile findings proving that
  cryptographic validity, deployment trust, and claim truth are separate.

Python unit tests instantiate these vectors directly. Cross-engine byte vectors
can be promoted into the top-level corpus once more engines expose nested
Full Reader and profile-policy APIs.

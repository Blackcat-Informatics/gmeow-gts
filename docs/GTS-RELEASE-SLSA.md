<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Release SLSA Posture

## Decision

Defer the reusable-workflow migration for the v1.0-rc1 release path. Current
release artifacts should be described as GitHub artifact-attested SLSA v1.0
Build Level 2 evidence, plus the registry, SBOM, immutable-release, and public
verification controls described below. Do not claim SLSA v1.0 Build Level 3 for
GTS release artifacts until the release lanes are moved to hardened reusable
workflows and representative artifacts verify against the intended signer
workflow identity.

This is a documentation decision, not a reduction in release hardening. The
current release workflows already provide strong public evidence for the
multi-language release model. Moving all lanes into reusable workflows is worth
doing only when the reusable workflows create a clearer trust boundary and
consumer verification can enforce that boundary.

## Basis

GitHub documents artifact attestations as SLSA v1.0 Build Level 2 evidence.
GitHub also documents reusable workflows as the route to stronger isolation for
SLSA v1.0 Build Level 3 alignment, because the build can be tied to known,
vetted build instructions.

The current GTS release lanes are first-party workflow files in this repository,
except for `visual-hashing`, which now publishes from its standalone repository:

| Release lane | Workflow | Current publication path |
|---|---|---|
| Rust `gmeow-gts` crate | `.github/workflows/release-cargo.yaml` | crates.io Trusted Publishing through GitHub Actions OIDC |
| Rust `gmeow-gts-capi` source crate | `.github/workflows/release-cargo-capi.yaml` | crates.io bootstrap token for first publish; Trusted Publishing follow-up required |
| Rust `visual-hashing` crate | `Blackcat-Informatics/visual-hashing:.github/workflows/release.yml` | crates.io Trusted Publishing through GitHub Actions OIDC |
| Python package | `.github/workflows/release-pypi.yml` | PyPI trusted publishing with package attestations |
| TypeScript package | `.github/workflows/release-npm.yaml` | npm trusted publishing and npm provenance |
| Go CLI assets | `.github/workflows/release-go.yaml` | Immutable GitHub Release assets |
| C ABI native assets | `.github/workflows/release-capi.yaml` | Immutable GitHub Release archives |

Refactoring those jobs into same-repository reusable workflows would centralize
release logic, but it would not by itself add enough governance separation to
justify changing every release lane immediately before v1.0-rc1. The stronger
upgrade is a protected, reviewed reusable-workflow boundary with verification
that requires the expected reusable workflow identity.

## Current Guarantees

Every release lane must keep these controls:

- tag-to-manifest version checks before publication;
- least-privilege GitHub Actions permissions for release jobs;
- pinned third-party actions;
- registry OIDC, registry-native provenance, or a documented first-publish token bootstrap where needed;
- GitHub build-provenance attestations for released artifacts;
- SPDX SBOM attestations for representative registry artifacts and Go archives;
- immutable Go and C ABI GitHub Releases for archives, checksums, and SBOM assets;
- public post-release verification through `just verify-release`.

The current evidence durability is:

| Surface | Durable artifact | Attestation evidence |
|---|---|---|
| Go | Immutable GitHub Release archives, `checksums.txt`, and `sbom-go-gts.spdx.json` | GitHub release attestation, SLSA provenance attestations, and SPDX SBOM attestations |
| C ABI | Immutable GitHub Release archives, `checksums.txt`, and `sbom-gmeow-gts-capi.spdx.json` | GitHub release attestation, SLSA provenance attestations, and SPDX SBOM attestations |
| crates.io `gmeow-gts` | Registry-hosted `.crate` package | GitHub SLSA provenance and SPDX SBOM attestations |
| crates.io `gmeow-gts-capi` | Registry-hosted `.crate` package | GitHub SLSA provenance and SPDX SBOM attestations; bootstrap token until Trusted Publishing follow-up lands |
| PyPI | Registry-hosted wheel and sdist | PyPI publish attestations plus GitHub SLSA provenance and SPDX SBOM attestations |
| npm | Registry-hosted tarball | npm provenance plus GitHub SLSA provenance and SPDX SBOM attestations |

## Future Build Level 3 Path

Raise the posture only when the release model can verify the stronger boundary:

1. Create reusable workflows for the build, package, SBOM, and attestation steps
   for each release lane, or a smaller set of shared release factories when the
   ecosystems can safely share implementation.
2. Protect those reusable workflows with repository rules, required review, and
   CODEOWNERS. Prefer an organization-managed workflow repository if the
   governance gain is meant to be stronger than this repository's normal branch
   protection.
3. Keep caller workflows small. Callers should pass only version, tag, and
   release-material inputs, then let the reusable workflow build, package,
   generate SBOMs, attest, and publish.
4. Grant both caller and reusable workflows only the permissions required for
   the lane, including `contents: read`, `id-token: write`, and
   `attestations: write` for attestation-generating jobs.
5. Preserve the existing registry OIDC/trusted-publishing paths, Go immutable
   release flow, SBOM generation, and `just verify-release` smoke verification.
6. Extend `scripts/verify_release.py` with optional signer policy inputs so a
   release can require `gh attestation verify` with
   `--signer-workflow <owner>/<repo>/.github/workflows/<workflow>.yml@<ref>` and,
   when applicable, `--signer-repo ...`.
7. Validate at least one representative artifact from each adopted release lane
   against the expected reusable workflow identity before claiming Build Level 3.

Until those steps are complete, release notes and checklists should state the
current posture as SLSA v1.0 Build Level 2 artifact attestations with registry
provenance, SBOM attestations, immutable Go releases, and public verifier
coverage.

## References

- [GitHub artifact attestations](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- [Using artifact attestations and reusable workflows to achieve SLSA v1 Build Level 3](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/increase-security-rating)

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS v1.0-rc1 Checklist And Artifact Bundle

This checklist turns the v1.0-rc1 release path in
[`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md) into a runnable release-candidate
record. Copy the unchecked sections into the release issue or keep a filled copy
as a release artifact. Do not edit the committed vector manifest to stamp a
release revision; generate the stamped artifact described below.

## 1. Candidate Record

| Field | Value |
|---|---|
| Release issue | |
| Release PR | |
| Candidate name | `v1.0-rc1` |
| Release package version | |
| Spec commit | |
| Corpus revision | |
| Vector manifest artifact | `dist/v1.0-rc1/vector-manifest.release.json` |
| Rust package/tag | `gmeow-gts` / `rust-v<version>` |
| Python package/tag | `gmeow-gts` / `py-v<version>` |
| Go module/tag | `go.blackcatinformatics.ca/gts` / `go/v<version>` |
| npm package/tag | `@blackcatinformatics/gmeow-gts` / `npm-v<version>` |
| Ruby package/tag | `gmeow-gts` / `ruby-v<version>` |
| Release notes commit | |
| Release manager | |
| Date | |

Before choosing a pre-release package version, verify the string is accepted by
Cargo, PyPI, npm, and the repository version guard. If ecosystem syntax differs
for the same release candidate, record the exact per-ecosystem versions here and
update the release guard before tagging. A tag/manifest mismatch is a release
blocker because the tag workflows verify manifest versions before publishing.

## 2. Blocker Classification

Classify every finding against the blocker and non-blocker lists in
[`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md#71-v10-rc1-blockers).

### 2.1 v1.0-rc1 Blockers

| Blocker class | Required evidence | Status |
|---|---|---|
| No intentional wire-format changes remain | Open GIP/spec issues reviewed; no accepted change alters header/frame grammar, hash preimages, signature preimages, segment composition, transform resolution, or core fold semantics. | |
| Baseline vectors are present and frozen | Vector manifest validates; corpus regeneration has no diff; stamped release manifest names the corpus revision. | |
| Cross-engine baseline behavior passes | Rust, Python, Go, TypeScript, Smalltalk/Pharo, Kotlin/JVM, and interop checks pass against the same corpus revision. | |
| C ABI wrapper smoke coverage passes | `rust/capi` plus C++, .NET, PHP, Lua, Swift, Ruby, R, and Julia wrapper smoke tests pass against the release candidate. | |
| Registry policy is published | Frame types, diagnostics, codecs, profiles, transform targets, and reserved namespaces are covered in governance/spec/conformance docs. | |
| Security model is clear | Security policy and crypto-deferral guards pass; no high or critical parser, crypto, extraction, or release-pipeline vulnerability remains open. | |
| Media type and distribution guidance is present | `application/vnd.blackcat.gts+cbor-seq`, HTTP/range behavior, immutable publication, and artifact verification guidance are present in spec/docs. | |
| Compatibility language is clear | Wire, corpus, package, and profile compatibility rules are present in governance and cited by release notes. | |
| Implementer review has no blocking findings | Review issues/comments are closed, deferred as non-blockers, or recorded below with owner and rationale. | |
| Quality-budget paydown is recorded | Release PR reduces at least one over-target hotspot toward `target_lines`, or records a deliberate exception with owner, rationale, and follow-up issue; no baseline increase is accepted without the quality-budget review label or architecture-review note. | |

### 2.2 Release-Adjacent Non-Blockers

These items are useful for adoption, but do not delay rc1 when baseline
conformance is ready unless they reveal one of the blockers above.

| Deliverable | Tracking issue | Status | Release note |
|---|---|---|---|
| Third-party implementation guide | `#104` | | |
| Benchmark suite and release report | `#105` | `just bench-release` and [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) | |
| GTS paper draft | `#106` | [`GTS-PAPER-DRAFT.md`](./GTS-PAPER-DRAFT.md) | Informative paper narrative; not normative spec text. |
| Optional-standard profile completion | | | |
| Database, Parquet, browser, object-store, range-fetch, MMR, replication, or advanced proof tooling | | | |
| Future key-management or multi-recipient encryption envelopes | | | |
| Neutral package aliases or standards-body submission | | | |

## 3. Local Environment Snapshot

Record the toolchain and repository state used for the candidate.

```bash
git status --short --branch
git rev-parse HEAD
git tag --points-at HEAD
rustc --version
cargo --version
python --version
uv --version
go version
node --version
npm --version
```

## 4. Artifact Bundle Setup

Create a local bundle directory before running candidate checks.

```bash
export RC=v1.0-rc1
export OUT="dist/${RC}"
rm -rf "${OUT}"
mkdir -p "${OUT}/reports" "${OUT}/packages" "${OUT}/sbom" "${OUT}/attestations"
git rev-parse HEAD > "${OUT}/spec-commit.txt"
git rev-parse HEAD > "${OUT}/corpus-revision.txt"
python scripts/check_vector_manifest.py \
  --release-manifest "${OUT}/vector-manifest.release.json"
```

If the corpus revision is a tag or an explicit commit rather than the current
`HEAD`, stamp it explicitly:

```bash
python scripts/check_vector_manifest.py \
  --corpus-revision git:<tag-or-full-commit> \
  --release-manifest "${OUT}/vector-manifest.release.json"
```

The bundle should contain:

- `spec-commit.txt` with the full commit containing the release spec.
- `corpus-revision.txt` with the corpus commit or tag.
- `vector-manifest.release.json` with the stamped `corpus_revision`.
- Per-engine test and conformance logs in `reports/`.
- Package dry-run outputs in `packages/`.
- Workflow SBOMs and attestation references after tag release.
- Release notes or a link to the release PR.

## 5. Guard And Drift Checks

Run these from the repository root.

```bash
bash scripts/check-versions.sh
python scripts/check_cli_parity.py
python scripts/check_api_parity.py
python scripts/check_advanced_contract.py
python scripts/check_ecosystem_contract.py
python scripts/check_security_contract.py
python scripts/check_crypto_deferrals.py
python scripts/check_quality_budget.py
python scripts/check_vector_manifest.py
python scripts/check_vector_manifest.py --self-test
```

Confirm the rc1 policy anchors are present:

```bash
rg -n "v1.0-rc1|v1.0-rc1 Blockers|v1.0-rc1 Non-Blockers" docs/GTS-GOVERNANCE.md
rg -n "Wire-format compatibility|Corpus compatibility|Package compatibility|Profile compatibility" docs/GTS-GOVERNANCE.md
rg -n "application/vnd.blackcat.gts\\+cbor-seq|HTTP range|immutable" README.md docs/GTS-SPEC.md
rg -n "corpus_revision|release manifest|conformance report" docs/GTS-CONFORMANCE.md vectors/manifest.json
```

## 6. Corpus And Conformance Checks

Regenerate and compare the committed corpus:

```bash
just check-vectors
git diff --exit-code vectors
```

Run the full engine suites and capture logs:

```bash
cargo test --manifest-path rust/Cargo.toml --locked 2>&1 \
  | tee "${OUT}/reports/rust-test.log"
(
  cd python
  uv sync --extra rdf
  uv run pytest --junitxml "../${OUT}/reports/python-pytest.xml"
)
(
  cd go
  CGO_ENABLED=0 go test -json ./... \
    | tee "../${OUT}/reports/go-test.jsonl"
)
(
  cd ts
  npm ci
  npm test 2>&1 | tee "../${OUT}/reports/ts-test.log"
)
bash scripts/interop.sh 2>&1 | tee "${OUT}/reports/interop.log"
```

Run the validating-tool and publication refusal paths through the CLI tests in
the engine suites above. The release notes should state which conformance tiers
are claimed and cite the exact corpus revision.

Use this report row for each implementation:

| Implementation | Version | OS/arch | Tier claim | Corpus revision | Command/log | Pass/fail/skips |
|---|---|---|---|---|---|---|
| Rust | | | | | | |
| Python | | | | | | |
| Go | | | | | | |
| TypeScript | | | | | | |

## 7. Security And Supply-Chain Checks

Run local supply-chain and repository-hygiene checks when the tools are
available. `just audit` runs the OSV dependency scan defined in the justfile;
pre-commit is a separate hygiene, lint, and secret-scan gate.
Release SLSA posture is recorded in
[`GTS-RELEASE-SLSA.md`](./GTS-RELEASE-SLSA.md): current GitHub artifact
attestations are treated as SLSA v1.0 Build Level 2 evidence. Do not claim SLSA
v1.0 Build Level 3 unless the release lanes have moved to hardened reusable
workflows and representative artifacts verify against the expected signer
workflow identity.

```bash
just audit
pipx run pre-commit run --all-files
```

Inspect the GitHub security workflows for the candidate commit:

```bash
gh run list --workflow security.yml --branch main --limit 5
gh run list --workflow codeql.yml --branch main --limit 5
gh run list --workflow fuzz.yml --branch main --limit 5
```

Record any vulnerability, CodeQL, fuzz, release-pipeline, or signing finding as
a blocker unless it is explicitly out of scope for baseline v1 conformance.
If a release intentionally adopts reusable workflows for Build Level 3
alignment, record the signer workflow policy and verification evidence here
before tagging:

| Release lane | Reusable workflow | Signer verification command | Status |
|---|---|---|---|
| Rust `gmeow-gts` | | | |
| Rust `visual-hashing` | | | |
| Python | | | |
| Go | | | |
| TypeScript | | | |

## 8. Package Dry-Runs

Do not tag until all dry-runs pass from a clean release branch or release PR
merge commit.

```bash
cargo package --manifest-path rust/Cargo.toml --locked
(
  cd python
  uv lock --check
  uv build --out-dir "../${OUT}/packages/python"
)
(
  cd ts
  npm ci
  npm run build
  npm pack --pack-destination "../${OUT}/packages/npm"
)
(
  cd go
  CGO_ENABLED=0 go build -trimpath -ldflags "-s -w" \
    -o "../${OUT}/packages/go/gts" ./cmd/gts
)
archive="$(bash rust/capi/scripts/package.sh)"
bash rust/capi/scripts/verify-archive.sh "${archive}"
GTS_PACKAGE_DRY_RUN_OUT="${OUT}/packages/wrappers" \
  bash scripts/package_dry_run_wrappers.sh
```

The wrapper dry-run covers the Rust C ABI package list, installable C ABI
archive verification, installed C++ archive consumption, Conan and vcpkg
package-manager consumer smoke tests, .NET local NuGet packing, Composer
validation, PHP Packagist package-root generation plus local path-repository
consumer smoke testing, LuaRocks lint/make/pack plus installed-rock smoke
execution, Swift root package dump/run validation, Ruby gem build/install, R
build/check, and Julia package tests.

For Go release parity, also dry-run the cross-build shape used by
`.github/workflows/release-go.yaml`:

```bash
VERSION="<version>"
mkdir -p "${OUT}/packages/go-cross"
(
  cd go
  for os in linux darwin windows; do
    for arch in amd64 arm64; do
      ext=""
      [ "${os}" = windows ] && ext=".exe"
      CGO_ENABLED=0 GOOS="${os}" GOARCH="${arch}" \
        go build -trimpath -ldflags "-s -w" -o "gts${ext}" ./cmd/gts
      base="gts_${VERSION}_${os}_${arch}"
      if [ "${os}" = windows ]; then
        zip -qj "../${OUT}/packages/go-cross/${base}.zip" "gts${ext}"
      else
        tar czf "../${OUT}/packages/go-cross/${base}.tar.gz" "gts${ext}"
      fi
      rm -f "gts${ext}"
    done
  done
)
sha256sum "${OUT}"/packages/go-cross/* > "${OUT}/packages/go-cross/checksums.txt"
```

## 9. Release Notes

The release PR must update `CHANGELOG.md`, `CITATION.cff`, package manifests,
lockfiles, and README/docs snippets when package versions change.

Release notes must include:

- candidate name and package version or per-ecosystem versions;
- spec commit;
- corpus revision and stamped manifest artifact name;
- conformance tier claims by implementation;
- package registry names and release tags;
- blocker review summary;
- quality-budget reduction summary, or a documented exception with owner and follow-up issue;
- release-adjacent non-blockers and follow-up issue links;
- SBOM and attestation verification instructions;
- known limitations and deferred capabilities.

Minimum release-note evidence:

```bash
bash scripts/check-versions.sh
rg -n "<version>|spec commit|corpus revision|conformance|SBOM|attestation" CHANGELOG.md README.md docs
```

## 10. Tag And Publish Sequence

After the release PR is merged, tag the exact merge commit. Push release tags
one at a time so each tag-triggered workflow receives its own event.

Before pushing Rust tags, confirm the crates.io Trusted Publisher entry for
`gmeow-gts` is active with owner/repo `Blackcat-Informatics/gmeow-gts`,
workflow `release-cargo.yaml`, and environment `(none)`. The normal Rust
release path uses GitHub Actions OIDC and does not require
`CARGO_REGISTRY_TOKEN`.

Before the first `gmeow-gts-capi` source-crate publish, confirm the
`CARGO_REGISTRY_TOKEN` bootstrap secret is available to
`release-cargo-capi.yaml`. This token is only for the initial crate bootstrap.
After the first version appears on crates.io, file and complete the follow-on
Trusted Publishing migration for `gmeow-gts-capi`, owner/repo
`Blackcat-Informatics/gmeow-gts`, workflow `release-cargo-capi.yaml`, and
environment `(none)` unless a protected release environment is added.

If the `gmeow-gts` Rust crate depends on a newer `visual-hashing` version,
publish that crate first from its standalone repository. Its crates.io Trusted
Publisher entry must use owner/repo `Blackcat-Informatics/visual-hashing`,
workflow `release.yml`, and environment `(none)`.

Before pushing RubyGems tags, confirm the RubyGems pending Trusted Publisher for
`gmeow-gts` uses owner/repo `Blackcat-Informatics/gmeow-gts`, workflow
`release-rubygems.yaml`, and environment `(none)` unless the release explicitly
adds a protected environment.

Before pushing Go tags, confirm repository-level immutable releases are enabled:

```bash
gh api repos/Blackcat-Informatics/gmeow-gts/immutable-releases
```

```bash
MERGE_COMMIT="<full-merge-commit>"
VERSION="<version>"
git tag "rust-v${VERSION}" "${MERGE_COMMIT}"
git tag "py-v${VERSION}" "${MERGE_COMMIT}"
git tag "go/v${VERSION}" "${MERGE_COMMIT}"
git tag "npm-v${VERSION}" "${MERGE_COMMIT}"
git tag "capi-v${VERSION}" "${MERGE_COMMIT}"
git tag "ruby-v${VERSION}" "${MERGE_COMMIT}"
git tag "${VERSION}" "${MERGE_COMMIT}" # Swift Package Manager / Swift Package Index
git push origin "rust-v${VERSION}"
git push origin "py-v${VERSION}"
git push origin "go/v${VERSION}"
git push origin "npm-v${VERSION}"
git push origin "capi-v${VERSION}"
git push origin "ruby-v${VERSION}"
git push origin "${VERSION}"
```

If `visual-hashing` changed, publish it before `rust-v*` tags that depend on the
new crate version:

```bash
VISUAL_HASHING_VERSION="<visual-hashing-version>"
gh repo clone Blackcat-Informatics/visual-hashing ../visual-hashing
cd ../visual-hashing
git tag "v${VISUAL_HASHING_VERSION}" "<visual-hashing-merge-commit>"
git push origin "v${VISUAL_HASHING_VERSION}"
cd -
```

Monitor the release workflows:

```bash
gh run list --event push --limit 30
gh run list --workflow release-cargo.yaml --branch "rust-v${VERSION}" --limit 5
gh run list --workflow release-cargo-capi.yaml --branch "capi-v${VERSION}" --limit 5
gh run list --workflow release-pypi.yml --branch "py-v${VERSION}" --limit 5
gh run list --workflow release-go.yaml --branch "go/v${VERSION}" --limit 5
gh run list --workflow release-npm.yaml --branch "npm-v${VERSION}" --limit 5
gh run list --workflow release-capi.yaml --branch "capi-v${VERSION}" --limit 5
```

After the plain Swift semantic-version tag exists, validate the root package
and submit the repository URL with protocol and `.git` extension to Swift
Package Index:

```bash
diff -u rust/capi/include/gts.h swift/Sources/CGts/include/gts.h
swift package dump-package --package-path .
bash swift/scripts/smoke.sh
```

Submit:

```text
https://github.com/Blackcat-Informatics/gmeow-gts.git
```

If `visual-hashing` was released, monitor its workflow by tag:

```bash
gh run list --repo Blackcat-Informatics/visual-hashing --workflow release.yml --branch "v${VISUAL_HASHING_VERSION}" --limit 5
```

If a tag was pushed to the wrong commit or with the wrong version, stop and file
a release incident note before deleting or recreating it.

## 11. Published Artifact Verification

Verify the public registries and release artifacts after the workflows complete.
The maintainer smoke verifier performs the download, hash, registry provenance,
GitHub SLSA, SPDX SBOM, and immutable-release checks from public surfaces only:

```bash
VISUAL_HASHING_VERSION="<visual-hashing-version>"
just verify-release-dry-run "${VERSION}" "${VISUAL_HASHING_VERSION}"
just verify-release "${VERSION}" "${VISUAL_HASHING_VERSION}"
```

After C ABI wrapper packages are published, run the wrapper-aware verifier:

```bash
just verify-wrapper-release-dry-run "${VERSION}" "${VISUAL_HASHING_VERSION}"
just verify-wrapper-release "${VERSION}" "${VISUAL_HASHING_VERSION}"
```

The same verifier can be run from the GitHub Actions UI with the manual
`Verify published release` workflow. Enable `dry_run` before credentials or
registry propagation are ready, and enable `include_wrapper_packages` for the
wrapper pass. It uploads
`dist/release-verification/${VERSION}/release-verification-summary.md` and
the matching JSON report. The report keeps pass/warn/fail severity separate
from release status values such as `published`, `pending`,
`metadata-mismatch`, and `missing`, so propagation lag is not conflated with
bad metadata or absent artifacts.
`release-verification-summary.json` as workflow artifacts. Do not pass
`--allow-legacy-release-gaps` for new releases; that override is only for
auditing releases that predate the SBOM and immutable-release hardening.

Record quick registry state before or after the smoke verifier:

```bash
cargo search gmeow-gts --limit 1
cargo search gmeow-gts-capi --limit 1
python -m pip index versions gmeow-gts
npm view @blackcatinformatics/gmeow-gts version
dotnet nuget search Gmeow.Gts --source https://api.nuget.org/v3/index.json
composer show blackcatinformatics/gmeow-gts --available
curl -fsSL https://luarocks.org/manifest.json | python -m json.tool >/dev/null
gem info gmeow-gts --remote
curl -fsSL https://blackcat-informatics.r-universe.dev/src/contrib/PACKAGES
curl -fsSL https://raw.githubusercontent.com/JuliaRegistries/General/master/G/GmeowGTS/Package.toml
gh release view "go/v${VERSION}" \
  --json tagName,name,url,isDraft,isImmutable,isPrerelease,publishedAt
gh release view "capi-v${VERSION}" \
  --json tagName,name,url,isDraft,isImmutable,isPrerelease,publishedAt
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify "capi-v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
```

Consumer-facing verification commands, all of which are covered by the
maintainer smoke verifier, are:

```bash
pypi-attestations verify pypi \
  --repository https://github.com/Blackcat-Informatics/gmeow-gts \
  "https://files.pythonhosted.org/.../gmeow_gts-${VERSION}-py3-none-any.whl"
npm audit signatures
gh attestation verify <downloaded-artifact> --repo Blackcat-Informatics/gmeow-gts
gh attestation verify <downloaded-artifact> \
  --repo Blackcat-Informatics/gmeow-gts \
  --predicate-type https://spdx.dev/Document/v2.3
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify "capi-v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify-asset "go/v${VERSION}" <downloaded-go-asset> \
  --repo Blackcat-Informatics/gmeow-gts
gh release verify-asset "capi-v${VERSION}" <downloaded-capi-asset> \
  --repo Blackcat-Informatics/gmeow-gts
```

Release evidence durability:

| Surface | Durable artifact | Attestation evidence |
|---|---|---|
| Go | Immutable GitHub Release archives, `checksums.txt`, and `sbom-go-gts.spdx.json` | GitHub release attestation for the immutable release; SLSA provenance attestations for release assets; SPDX SBOM attestations for release archives |
| C ABI | Immutable GitHub Release archives, `checksums.txt`, and `sbom-gmeow-gts-capi.spdx.json` | GitHub release attestation for the immutable release; SLSA provenance attestations for release assets; SPDX SBOM attestations for release archives |
| crates.io `gmeow-gts` | Registry-hosted `.crate` package | SLSA provenance and SPDX SBOM attestations in GitHub's attestation store |
| crates.io `gmeow-gts-capi` | Registry-hosted `.crate` package | SLSA provenance and SPDX SBOM attestations in GitHub's attestation store; bootstrap token until Trusted Publishing follow-up lands |
| PyPI | Registry-hosted wheel/sdist | PyPI publish attestations plus GitHub SLSA provenance and SPDX SBOM attestations |
| npm | Registry-hosted tarball | npm provenance plus GitHub SLSA provenance and SPDX SBOM attestations |
| RubyGems | Registry-hosted `.gem` package | SLSA provenance and SPDX SBOM attestations in GitHub's attestation store |
| NuGet `Gmeow.Gts` | Registry-hosted `.nupkg` package | Registry metadata and package download check; source-only wrapper requiring host `libgts` |
| Packagist `blackcatinformatics/gmeow-gts` | VCS tag metadata from Packagist | Registry metadata and source reference check; source-only wrapper requiring host `libgts` |
| LuaRocks `gmeow-gts` | Registry-hosted rockspec/source rock | LuaRocks root manifest and rockspec download check; source-only wrapper requiring host `libgts` |
| Swift Package Index | Repository semantic-version tag and SPI package URL | Git tag check and canonical SPI URL record; source-only wrapper requiring host `libgts` |
| r-universe `gmeowgts` | Registry-hosted source package | PACKAGES index and source tarball download check; source package requiring host `libgts` |
| Julia General `GmeowGTS` | General registry package metadata | Registry identity and version check; source-only wrapper requiring host `libgts` |
| Conan/vcpkg `gmeow-gts` | Local first-party dry-runs until upstreamed | No public registry evidence until upstream recipes are accepted |

Download representative artifacts for verification:

```bash
mkdir -p \
  "${OUT}/packages/go-release" \
  "${OUT}/packages/npm" \
  "${OUT}/packages/python" \
  "${OUT}/packages/ruby" \
  "${OUT}/packages/rust" \
  "${OUT}/packages/wrappers"
gh release download "go/v${VERSION}" --dir "${OUT}/packages/go-release"

python -m pip download --no-deps --dest "${OUT}/packages/python" "gmeow-gts==${VERSION}"
npm pack "@blackcatinformatics/gmeow-gts@${VERSION}" \
  --pack-destination "${OUT}/packages/npm"
gem fetch gmeow-gts --version "${VERSION}" --clear-sources --source https://rubygems.org
mv "gmeow-gts-${VERSION}.gem" "${OUT}/packages/ruby/"
curl -L "https://crates.io/api/v1/crates/gmeow-gts/${VERSION}/download" \
  -o "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate"
curl -L "https://crates.io/api/v1/crates/gmeow-gts-capi/${VERSION}/download" \
  -o "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate"
curl -L "https://api.nuget.org/v3-flatcontainer/gmeow.gts/${VERSION}/gmeow.gts.${VERSION}.nupkg" \
  -o "${OUT}/packages/wrappers/Gmeow.Gts.${VERSION}.nupkg"
curl -L "https://luarocks.org/gmeow-gts-${VERSION}-1.rockspec" \
  -o "${OUT}/packages/wrappers/gmeow-gts-${VERSION}-1.rockspec"
curl -L "https://blackcat-informatics.r-universe.dev/src/contrib/gmeowgts_${VERSION}.tar.gz" \
  -o "${OUT}/packages/wrappers/gmeowgts_${VERSION}.tar.gz"
```

Verify default SLSA provenance on representative artifacts and Go release
manifests:

```bash
for artifact in "${OUT}"/packages/go-release/gts_"${VERSION}"_*; do
  gh attestation verify "$artifact" --repo Blackcat-Informatics/gmeow-gts
done
for artifact in \
  "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate" \
  "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate" \
  "${OUT}"/packages/npm/*.tgz \
  "${OUT}"/packages/python/* \
  "${OUT}"/packages/ruby/*.gem; do
  gh attestation verify "$artifact" --repo Blackcat-Informatics/gmeow-gts
done
gh attestation verify "${OUT}/packages/go-release/checksums.txt" \
  --repo Blackcat-Informatics/gmeow-gts
gh attestation verify "${OUT}/packages/go-release/sbom-go-gts.spdx.json" \
  --repo Blackcat-Informatics/gmeow-gts
```

These commands verify the current Build Level 2 artifact-attestation posture.
If the candidate adopts reusable workflows for a stronger posture, repeat the
representative artifact checks with the expected signer policy, for example:

```bash
gh attestation verify <downloaded-artifact> \
  --repo Blackcat-Informatics/gmeow-gts \
  --signer-workflow <owner>/<repo>/.github/workflows/<workflow>.yml@<ref>
```

Verify the immutable Go release attestation and each downloaded release asset:

```bash
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
for artifact in "${OUT}"/packages/go-release/*; do
  gh release verify-asset "go/v${VERSION}" "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts
done
```

Verify SPDX SBOM attestations for one representative artifact from each release
lane. The current SBOM generator emits SPDX 2.3, so the predicate type is
`https://spdx.dev/Document/v2.3`; if the emitted `spdxVersion` changes, update
the predicate version to match.

```bash
SBOM_PREDICATE="https://spdx.dev/Document/v2.3"
for artifact in "${OUT}"/packages/go-release/gts_"${VERSION}"_*; do
  gh attestation verify "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts \
    --predicate-type "${SBOM_PREDICATE}"
done
for artifact in \
  "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate" \
  "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate" \
  "${OUT}"/packages/npm/*.tgz \
  "${OUT}"/packages/python/*; do
  gh attestation verify "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts \
    --predicate-type "${SBOM_PREDICATE}"
done
```

Record final registry state:

| Surface | Expected version/tag | Evidence | Status |
|---|---|---|---|
| crates.io `gmeow-gts` | | | |
| crates.io `gmeow-gts-capi` | | | |
| PyPI `gmeow-gts` | | | |
| Go release `go.blackcatinformatics.ca/gts` | | | |
| npm `@blackcatinformatics/gmeow-gts` | | | |
| NuGet `Gmeow.Gts` | | | |
| Packagist `blackcatinformatics/gmeow-gts` | | | |
| LuaRocks `gmeow-gts` | | | |
| Swift Package Index `Blackcat-Informatics/gmeow-gts` | | | |
| RubyGems `gmeow-gts` | | | |
| r-universe `gmeowgts` | | | |
| Julia General `GmeowGTS` | | | |
| Conan/vcpkg `gmeow-gts` status | | | |
| SBOM attestations | | | |
| Build-provenance attestations | | | |

## 12. Final Decision

The release candidate is ready only when:

- every blocker row in Section 2.1 is resolved or explicitly disproven;
- every release-adjacent non-blocker is linked to an issue or marked complete;
- conformance reports name the same stamped corpus revision;
- release notes cite the spec commit, corpus revision, package versions, and
  known deferrals;
- package dry-runs and release workflows pass;
- public registries and artifact verification prove the released bits match the
  intended candidate.

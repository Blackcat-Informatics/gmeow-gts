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
| Go module/tag | `go.blackcatinformatics.ca/gts` / `go-v<version>` |
| npm package/tag | `@blackcatinformatics/gmeow-gts` / `npm-v<version>` |
| Release notes commit | |
| Release manager | |
| Date | |

Before choosing a pre-release package version, verify the string is accepted by
Cargo, PyPI, npm, and the repository lockstep guard. If ecosystem syntax differs
for the same release candidate, record the exact per-ecosystem versions here and
update the release guard before tagging. A version-string mismatch is a release
blocker because the tag workflows verify manifest versions before publishing.

## 2. Blocker Classification

Classify every finding against the blocker and non-blocker lists in
[`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md#71-v10-rc1-blockers).

### 2.1 v1.0-rc1 Blockers

| Blocker class | Required evidence | Status |
|---|---|---|
| No intentional wire-format changes remain | Open GIP/spec issues reviewed; no accepted change alters header/frame grammar, hash preimages, signature preimages, segment composition, transform resolution, or core fold semantics. | |
| Baseline vectors are present and frozen | Vector manifest validates; corpus regeneration has no diff; stamped release manifest names the corpus revision. | |
| Cross-engine baseline behavior passes | Rust, Python, Go, TypeScript, and interop checks pass against the same corpus revision. | |
| Registry policy is published | Frame types, diagnostics, codecs, profiles, transform targets, and reserved namespaces are covered in governance/spec/conformance docs. | |
| Security model is clear | Security policy and crypto-deferral guards pass; no high or critical parser, crypto, extraction, or release-pipeline vulnerability remains open. | |
| Media type and distribution guidance is present | `application/vnd.blackcat.gts+cbor-seq`, HTTP/range behavior, immutable publication, and artifact verification guidance are present in spec/docs. | |
| Compatibility language is clear | Wire, corpus, package, and profile compatibility rules are present in governance and cited by release notes. | |
| Implementer review has no blocking findings | Review issues/comments are closed, deferred as non-blockers, or recorded below with owner and rationale. | |

### 2.2 Release-Adjacent Non-Blockers

These items are useful for adoption, but do not delay rc1 when baseline
conformance is ready unless they reveal one of the blockers above.

| Deliverable | Tracking issue | Status | Release note |
|---|---|---|---|
| Third-party implementation guide | `#104` | | |
| Benchmark suite and release report | `#105` | | |
| GTS paper draft | `#106` | | |
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
python scripts/check_advanced_contract.py
python scripts/check_ecosystem_contract.py
python scripts/check_security_contract.py
python scripts/check_crypto_deferrals.py
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

Run local supply-chain checks when the tools are available:

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
```

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

```bash
MERGE_COMMIT="<full-merge-commit>"
VERSION="<version>"
git tag "rust-v${VERSION}" "${MERGE_COMMIT}"
git tag "py-v${VERSION}" "${MERGE_COMMIT}"
git tag "go-v${VERSION}" "${MERGE_COMMIT}"
git tag "npm-v${VERSION}" "${MERGE_COMMIT}"
git push origin "rust-v${VERSION}"
git push origin "py-v${VERSION}"
git push origin "go-v${VERSION}"
git push origin "npm-v${VERSION}"
```

Monitor the release workflows:

```bash
gh run list --event push --limit 30
gh run list --workflow release-cargo.yaml --branch "rust-v${VERSION}" --limit 5
gh run list --workflow release-pypi.yml --branch "py-v${VERSION}" --limit 5
gh run list --workflow release-go.yaml --branch "go-v${VERSION}" --limit 5
gh run list --workflow release-npm.yaml --branch "npm-v${VERSION}" --limit 5
```

If a tag was pushed to the wrong commit or with the wrong version, stop and file
a release incident note before deleting or recreating it.

## 11. Published Artifact Verification

Verify the public registries and release artifacts after the workflows complete:

```bash
cargo search gmeow-gts --limit 1
python -m pip index versions gmeow-gts
npm view @blackcatinformatics/gmeow-gts version
gh release view "go-v${VERSION}" --json tagName,name,url,isDraft,isPrerelease,publishedAt
```

Download and verify package provenance where supported:

```bash
gh release download "go-v${VERSION}" --dir "${OUT}/packages/go-release"
gh attestation verify "${OUT}"/packages/go-release/* \
  --repo Blackcat-Informatics/gmeow-gts
gh attestation verify <downloaded-python-wheel-or-sdist> \
  --repo Blackcat-Informatics/gmeow-gts
gh attestation verify <downloaded-rust-crate-or-npm-tarball> \
  --repo Blackcat-Informatics/gmeow-gts
```

Record final registry state:

| Surface | Expected version/tag | Evidence | Status |
|---|---|---|---|
| crates.io `gmeow-gts` | | | |
| PyPI `gmeow-gts` | | | |
| Go release `go.blackcatinformatics.ca/gts` | | | |
| npm `@blackcatinformatics/gmeow-gts` | | | |
| SBOM artifacts | | | |
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

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# GTS v1.0 Checklist

`docs/GTS-GOVERNANCE.md` §7 defines the `v1.0` milestone but had no runbook, so
the release-candidate runbook was the only one that existed. This file is
deliberately short: **the `v1.0` procedure is the `v1.0-rc1` procedure**, and
duplicating 600 lines would guarantee the two drift. Follow
[`GTS-V1-RC1-CHECKLIST.md`](./GTS-V1-RC1-CHECKLIST.md) end to end, then apply
the deltas below.

## 1. Promotion criteria

`v1.0` publishes when, per `GTS-GOVERNANCE.md:197`:

- the format spec is published,
- the conformance corpus is tagged,
- reference implementation packages are released,
- release notes identify the spec and corpus commits.

In addition, and specific to promoting a candidate:

| Criterion | Evidence |
|---|---|
| Every rc1 blocker in `GTS-V1-RC1-CHECKLIST.md` §2.1 stayed resolved | Re-run the §5 guard sweep and §6 conformance on the promotion commit |
| No wire-format change landed after the candidate | `git diff <rc-tag>..HEAD -- docs/GTS-SPEC.md vectors/` shows no normative or corpus change |
| Deferred lanes are published or explicitly re-deferred | See §3 |
| Implementer feedback from the candidate is closed or recorded | Release notes name each item and its disposition |

If a wire-format change *did* land after the candidate, it is a new candidate,
not a promotion. Cut `v1.0-rc2` rather than tagging `v1.0`.

## 2. Version strings collapse

The candidate needed four per-ecosystem spellings because a semver pre-release
is not expressible everywhere. **A final release has no pre-release part, so all
lanes carry the identical string** and `scripts/check-versions.sh` enforces
exact equality across every lane, including the R and LuaRocks branches that
relax to derived forms only when the family version contains a `-`.

Confirm before tagging:

```bash
bash scripts/check-versions.sh   # every lane must print the same version
```

| Lane | Candidate spelling | Final `1.0.0` |
|---|---|---|
| PyPI | `1.0.0rc1` | `1.0.0` |
| R | `0.9.11.9001` | `1.0.0` |
| LuaRocks | `1.0.0rc1-1` | `1.0.0-1` |
| RubyGems | `1.0.0.rc.1` | `1.0.0` |

## 3. Tag order

Push `ruby-v*` **first**, not sixth.

RubyGems publication depends on a *pending trusted publisher* registered on
rubygems.org, and there is no API to verify one exists — so it can only fail at
publish time. During `1.0.0-rc.1` it failed after six other registries had
already accepted an immutable version, which cannot be undone. Failing first
costs nothing; failing last strands the release.

The same reasoning applies to any lane whose authorisation lives outside this
repository. Order the tag sequence *least verifiable first*, then:

`ruby-v*` → `rust-v*` → `capi-v*` → `py-v*` → `go/v*` → `npm-v*` → `lua-v*` → bare version

`capi-v*` must still follow `rust-v*`; the C ABI crate depends on the published
core crate, enforced by `scripts/package_dry_run_wrappers.sh:287-308`.

## 4. Changelog and spec status

- `CHANGELOG.md` heading becomes `## [1.0.0] — <date>` with a `rust-v` compare
  link, mirrored into both `docs/i18n/*/CHANGELOG.md`.
- `docs/GTS-SPEC.md` status moves from `Release candidate` to `Standard`, and
  the document version from `1.0-rc1` to `1.0`.
- `docs/documentation-roster.json` `spec_document_version` follows.
- The wire-format freeze wording added for the candidate stays as-is; it already
  states the major-version-1 promise.

## 5. After publication

Run both verifiers against the live registries and keep the reports:

```bash
just verify-release "1.0.0" "<visual-hashing-version>"
just verify-wrapper-release "1.0.0" "<visual-hashing-version>"
```

The wrapper pass will report the manual lanes (NuGet, Packagist, r-universe,
Julia General) as missing until someone publishes them; those are
release-adjacent under `GTS-GOVERNANCE.md` §7.2 and do not block. Record them as
deferred rather than suppressing the check.

Submit the repository URL to Swift Package Index once the bare semantic-version
tag exists, per `GTS-V1-RC1-CHECKLIST.md` §10.

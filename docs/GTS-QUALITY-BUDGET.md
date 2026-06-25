<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# GTS Quality Budget

The repository quality budget is a lightweight regression gate for production-code
maintainability signals. It is intentionally baseline-driven: the current tree is allowed, and
new pull requests fail when they add counted risk, grow a ratcheted hotspot beyond its checked
limit, or raise the checked baseline without explicit review.

Run it directly:

```bash
python scripts/check_quality_budget.py
```

The checked baseline lives at [`quality/quality-budget-baseline.json`](../quality/quality-budget-baseline.json).
Regenerating it is a review action, not a routine cleanup. To intentionally accept a known
change, inspect the failure output first, regenerate the baseline in the same commit, and use one
of the explicit review paths below:

```bash
python scripts/check_quality_budget.py --write-baseline
```

Baseline increases are rejected in pull-request CI unless the PR has the
`quality-budget-baseline-increase` label or the regenerated baseline updates an architecture
review note:

```json
{
  "baseline_increase_review": {
    "reviewed_by": "architecture review <issue-or-pr>",
    "reason": "why this release accepts the temporary increase"
  }
}
```

Each new baseline increase must either carry the label for that PR or update the review note; a
stale note from an older exception does not approve later increases.

## Counted Signals

The gate scans production source roots for:

- file-size budgets for the largest reader, CLI, and browser surfaces;
- unchecked panic-like calls such as Rust `unwrap`/`expect`/`panic!`, Go `panic`, Swift
  `fatalError`, C-family `abort`/`assert`, and equivalent language-level unchecked failures;
- generic parser throws such as TypeScript `throw new Error(...)` and broad Python/PHP/.NET/Ruby
  generic exceptions;
- maintenance markers: `TODO`, `FIXME`, and `HACK`.

Failures include file and line output for counted source-level signals. File-size failures include
the current line count, the checked maximum, and the next ratchet target. Passing runs also report
how many tracked hotspots remain over target and the closest next ratchet opportunity.

## Exclusions

The gate is scoped to production code. It excludes tests, fixtures, examples, fuzz targets,
generated/build output, vendored code, `dist`, `target`, `node_modules`, worktrees, and cache
directories. It also skips test-style filenames such as `_test.go`, `.test.ts`, `.spec.ts`, and
`Test.kt`. Rust `#[cfg(test)] mod ...` blocks embedded in production files are blanked before
metric matching so unit-test assertions do not count as production panic-like calls.

The initial production roots are the implementation surfaces under `rust/src`, `rust/capi`,
`go`, `python/src`, `ts/src`, `kotlin/src/main`, `smalltalk/src`, and the C ABI wrapper package
source roots for C++, .NET, PHP, Lua, Ruby, R, Julia, and Swift.

## Ratchet Policy

The `max_lines` value is the enforced file-size limit for a hotspot. The `target_lines` value is
the release paydown target, not a permanent allowance. Release PRs must reduce at least one
over-target hotspot toward `target_lines`, or record a deliberate exception with an owner and
follow-up issue in the release record. Pattern-count budgets use `max_total` plus per-file
counts, so moving risk between files is still visible unless the baseline is intentionally
updated through the review path above.

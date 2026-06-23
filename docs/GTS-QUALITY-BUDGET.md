<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# GTS Quality Budget

The repository quality budget is a lightweight regression gate for production-code
maintainability signals. It is intentionally baseline-driven: the current tree is allowed, and
new pull requests fail only when they add new counted risk or grow a ratcheted hotspot beyond its
checked limit.

Run it directly:

```bash
python scripts/check_quality_budget.py
```

The checked baseline lives at [`quality/quality-budget-baseline.json`](../quality/quality-budget-baseline.json).
To intentionally accept a known change, inspect the failure output first and then regenerate the
baseline in the same commit:

```bash
python scripts/check_quality_budget.py --write-baseline
```

## Counted Signals

The gate scans production source roots for:

- file-size budgets for the largest reader, CLI, and browser surfaces;
- unchecked panic-like calls such as Rust `unwrap`/`expect`/`panic!`, Go `panic`, Swift
  `fatalError`, C-family `abort`/`assert`, and equivalent language-level unchecked failures;
- generic parser throws such as TypeScript `throw new Error(...)` and broad Python/PHP/.NET/Ruby
  generic exceptions;
- maintenance markers: `TODO`, `FIXME`, and `HACK`.

Failures include file and line output for counted source-level signals. File-size failures include
the current line count, the checked maximum, and the advisory ratchet target.

## Exclusions

The gate is scoped to production code. It excludes tests, fixtures, examples, fuzz targets,
generated/build output, vendored code, `dist`, `target`, `node_modules`, worktrees, and cache
directories. It also skips test-style filenames such as `_test.go`, `.test.ts`, `.spec.ts`, and
`Test.kt`.

The initial production roots are the implementation surfaces under `rust/src`, `rust/capi`,
`go`, `python/src`, `ts/src`, `kotlin/src/main`, `smalltalk/src`, and the C ABI wrapper package
source roots for C++, .NET, PHP, Lua, Ruby, R, Julia, and Swift.

## Ratchet Policy

The `max_lines` value is the enforced file-size limit for a hotspot. The `target_lines` value is
advisory and records the next reduction target when a file is naturally split or simplified.
Pattern-count budgets use `max_total` plus per-file counts, so moving risk between files is still
visible unless the baseline is intentionally updated.

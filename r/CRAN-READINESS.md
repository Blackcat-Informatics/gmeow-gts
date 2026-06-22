# CRAN Readiness Follow-Up

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

`gmeowgts` is being published to r-universe first. CRAN submission remains a
follow-up because the package is a source-only wrapper over the external
`libgts` shared library and does not bundle native GTS artifacts.

Before CRAN submission:

- Confirm the r-universe source build is green for
  `https://blackcat-informatics.r-universe.dev/gmeowgts`.
- Run `R CMD check --as-cran` from a clean source tarball with `GTS_LIB_DIR`
  pointing at the release `libgts`.
- Run win-builder and r-hub checks with documented `libgts` setup for each
  platform.
- Review CRAN external-library policy and request CRAN machine support where
  needed for Linux, macOS, and Windows binary checks.
- Decide whether CRAN needs a system package, macOS recipe, or Windows binary
  dependency route before submission.
- Keep tests offline and continue using synthetic test data only.
- Re-check `DESCRIPTION`, `LICENSE`, URLs, and `SystemRequirements` against
  current CRAN incoming checks.

Known blocker for CRAN:

- `libgts` is not yet a standard CRAN check-machine dependency. Until that is
  resolved, CRAN acceptance should not block r-universe publication.

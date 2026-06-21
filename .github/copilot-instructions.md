<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# GitHub Copilot Instructions

When suggesting code completions or generating templates in this repository, align with the
rules below:

1. This repo holds six interoperable GTS engines — `rust/`, `python/`, `go/`, `ts/`,
   `smalltalk/`, and `kotlin/` — plus the normative spec in `docs/GTS-SPEC.md` and the frozen conformance corpus in `vectors/`. See
   [README.md](../README.md) and [CONTRIBUTING.md](../CONTRIBUTING.md).
2. The corpus is the contract: all six engines MUST fold identical bytes to identical
   expectations. Do not edit files under `vectors/` by hand — they are generated from the
   Python reference (`gts.vectors`) via `python/scripts/gen_vectors.py`. A behaviour change
   must update the corpus and keep every engine green.
3. If you change one engine's observable behaviour, change the others to match; diverging from
   the spec or the other engines is a bug.
4. Every source file must carry an SPDX `MIT OR Apache-2.0` license header.
5. Match each engine's existing conventions and toolchain: `cargo fmt`/`clippy` (Rust),
   `gofmt`/`go vet`/`golangci-lint` (Go), ESLint/Prettier (TypeScript), `ruff`/`mypy`
   with `uv` (Python), and Gradle `test`/`detekt` (Kotlin). Keep the public API names
   consistent across engines.

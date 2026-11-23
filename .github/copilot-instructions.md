<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# GitHub Copilot Instructions

When suggesting code completions or generating templates in this repository, align with the
rules below:

1. The [GTS v2 specification](../docs/GTS-SPEC.md) and [v2 corpus](../vectors/v2/README.md)
   define the current format. The released engines predate v2.
2. Update affected conformance cases with every observable format change. See
   [CONTRIBUTING.md](../CONTRIBUTING.md) for the corpus and package-specific commands.
3. Check agreement between engines that claim the same format and capabilities.

4. Every eligible first-party source file must carry an SPDX `MIT OR Apache-2.0 OR MulanPSL-2.0` license header.
5. Match each engine's existing conventions and toolchain: `cargo fmt`/`clippy` (Rust),
   `gofmt`/`go vet`/`golangci-lint` (Go), ESLint/Prettier (TypeScript), `ruff`/`mypy`
   with `uv` (Python), and Gradle `test`/`detekt` (Kotlin). Keep the public API names
   consistent across engines.

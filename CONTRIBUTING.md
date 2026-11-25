<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
# Contributing to gmeow-gts

This repository publishes the [GTS v2 specification](docs/GTS-SPEC.md), its [conformance corpus](vectors/v2/README.md), and reference software. Issues and pull requests are welcome.

## Ways to contribute

- **Report a bug or request a feature** — open an issue with a minimal reproduction
  (ideally a `.gts` file or a failing conformance vector).
- **Fix a bug or add a feature** — open a pull request against `main`.
- **Improve the spec or docs** — corrections and clarifications to
  [`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md) and the per-engine guides are very welcome.

## Spec governance

Changes to the wire grammar, hash or signature preimages, fold, conformance claims and standard
profiles follow the [GTS Improvement Proposal process](docs/GTS-GOVERNANCE.md).

## The conformance corpus

For a v2 format change, update the affected normative part and its [conformance cases](vectors/v2/README.md).
Record input, capacity and expected typed result. Consult the [fixture catalogue](docs/v2/FIXTURES.md)
before adding a new case. The published engines use a separate corpus. For changes to those engines,
regenerate their fixtures with `just check-vectors` and check engine agreement on the affected behavior.

## Development

Each implementation builds and tests independently from its own directory:

```bash
cd rust   && cargo test                              # unit + CLI + conformance
cd go     && go test ./...                            # unit + conformance
cd ts     && npm ci && npm test                       # compiles, runs against vectors/
cd python && uv sync --extra rdf && uv run pytest     # reference + conformance
cd kotlin && gradle test                             # JVM unit + conformance
docker build -t gmeow-gts-smalltalk smalltalk && \
  docker run --rm -v "$PWD:/workspace" --entrypoint /bin/sh gmeow-gts-smalltalk -lc \
  'sh /workspace/smalltalk/scripts/run-tests.sh'      # Pharo bootstrap tests
```

## Before you open a pull request

- Run the relevant engine's test suite (above) and make sure it is green.
- Run repo-wide hygiene: `pre-commit run --all-files` (formatting, SPDX headers,
  YAML/Markdown/shell, secret scanning).
- Per-language gates: `cargo fmt --check` + `cargo clippy`, `go vet` + `golangci-lint`,
  `npm run lint`, `ruff check` + `mypy`.
- Every eligible first-party source file must carry an SPDX `MIT OR Apache-2.0 OR MulanPSL-2.0` license header.
- Keep changes focused; describe **what** changed and **why** in the PR description.

CI runs the Rust, Python, Go, TypeScript, Smalltalk/Pharo and Kotlin/JVM engine lanes and the repository lint checks.

## Licensing of contributions

Contributions intentionally submitted for inclusion in eligible first-party implementation
code, build and packaging tooling, GTS specifications, and frozen conformance test vectors
are accepted under **MIT OR Apache-2.0 OR MulanPSL-2.0**. Contributions to other documentation
retain that document's declared grant unless explicitly stated otherwise. The project CLA
separately governs permission for proprietary/commercial relicensing.

For context, contributions to **GMEOW tooling/code** elsewhere in the project (the
[`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology) repository) are
accepted under **AGPL-3.0-only** and, under the project CLA, under terms that permit Blackcat
Informatics® Inc. to relicense them under separate proprietary/commercial terms. gmeow-gts is
the deliberately permissive, dependency-light engine layer, so it carries the permissive
`MIT OR Apache-2.0 OR MulanPSL-2.0` terms rather than AGPL.

By submitting a contribution you agree to its applicable grant described above. For the
dual-licensing reservation to extend to your contribution, you agree to license it to
Blackcat Informatics® Inc. under terms that permit relicensing, including under proprietary
terms. A Contributor License Agreement (CLA) may be required before substantial
contributions are merged. See [`LICENSING.md`](./LICENSING.md) for the full licensing scheme.

## Releases

Package versions and the specification revision are independent. Keep installation snippets and
manifests in sync with `scripts/check-versions.sh`. For API deprecation, see
[governance §5.1](docs/GTS-GOVERNANCE.md#51-deprecation-policy).

## Conduct

Be respectful and constructive. Harassment and abuse are not tolerated.

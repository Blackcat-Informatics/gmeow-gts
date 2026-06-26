<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: CONTRIBUTING.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# 贡献 GTS

> [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) 的信息性中文翻译。英文文档仍然是治理、安全、发布、许可、贡献、行为义务、披露流程和可执行命令的权威来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。

感谢您有兴趣为 Graph Transport Substrate (GTS) 做出贡献。本文说明如何参与核心引擎、规范和生态系统工具的工作。

## 贡献方式

- **Report a bug or request a feature** — open an issue with a minimal reproduction
  (ideally a `.gts` file or a failing conformance vector).
- **Fix a bug or add a feature** — open a pull request against `main`.
- **Improve the spec or docs** — corrections and clarifications to
  [`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md) and the per-engine guides are very welcome.

## 规范治理

Core wire-format changes, baseline conformance changes, optional-standard profile promotion,
and registry additions follow the lightweight governance policy in
[`docs/GTS-GOVERNANCE.md`](./docs/GTS-GOVERNANCE.md). In short:

- changes to header/frame grammar, hash or signature preimages, transform resolution, segment
  composition, or fold semantics require a GTS Improvement Proposal (GIP);
- domain-specific profiles can be registered without changing core GTS, but they must not alter
  core parse, verify, or fold semantics;
- registry entries for codecs, frame types, diagnostics, transform targets, and profiles must
  follow the registry change policy and reserved namespace rules.

## 一致性语料库就是契约

The four parity engines are interchangeable only because they all fold the **same bytes** to the
**same expectations**. The frozen corpus lives in [`vectors/`](../../../vectors); the Python
reference implementation (`gts.vectors`) is its single source of truth.

- A change to format behaviour MUST update the corpus and keep all four engines green.
- Regenerate the committed corpus and prove it is reproducible byte-for-byte:

  ```bash
  cd python && uv run python scripts/gen_vectors.py
  git diff --exit-code vectors        # no changes ⇒ reproducible
  ```

- If you change one engine's observable behaviour, change the others to match (or open an
  issue first to discuss whether the spec itself should change).

## 开发

Each implementation builds and tests independently from its own directory:

```bash
cd rust   && cargo test                              # unit + CLI + conformance
cd go     && go test ./...                            # unit + conformance
cd ts     && npm ci && npm test                       # compiles, runs against vectors/
cd python && uv sync --extra rdf && uv run pytest     # reference + conformance
docker build -t gmeow-gts-smalltalk smalltalk && \
  docker run --rm -v "$PWD:/workspace" --entrypoint /bin/sh gmeow-gts-smalltalk -lc \
  'sh /workspace/smalltalk/scripts/run-tests.sh'      # Pharo bootstrap tests
```

## 开启拉取请求之前

- Run the relevant engine's test suite (above) and make sure it is green.
- Run repo-wide hygiene: `pre-commit run --all-files` (formatting, SPDX headers,
  YAML/Markdown/shell, secret scanning).
- Per-language gates: `cargo fmt --check` + `cargo clippy`, `go vet` + `golangci-lint`,
  `npm run lint`, `ruff check` + `mypy`.
- Every source file must carry an SPDX `MIT OR Apache-2.0` license header.
- Keep changes focused; describe **what** changed and **why** in the PR description.

CI runs all four parity engines, the Smalltalk/Pharo bootstrap, and a lint lane on every pull
request.

## 贡献许可

Contributions to **gmeow-gts** are accepted under **Apache-2.0 OR MIT** and, under the
project CLA, under terms that permit separate proprietary/commercial licensing.

For context, contributions to **GMEOW tooling/code** elsewhere in the project (the
[`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology) repository) are
accepted under **AGPL-3.0-only** and, under the project CLA, under terms that permit Blackcat
Informatics® Inc. to relicense them under separate proprietary/commercial terms. gmeow-gts is
the deliberately permissive, dependency-light engine layer, so it carries the permissive
`Apache-2.0 OR MIT` terms rather than AGPL.

By submitting a contribution you agree to license it under the terms above. For the
dual-licensing reservation to extend to your contribution, you agree to license it to
Blackcat Informatics® Inc. under terms that permit relicensing, including under proprietary
terms. A Contributor License Agreement (CLA) may be required before substantial
contributions are merged. See [`LICENSING.md`](./LICENSING.md) for the full licensing scheme.

## 行为准则

Be respectful and constructive. Harassment and abuse are not tolerated.

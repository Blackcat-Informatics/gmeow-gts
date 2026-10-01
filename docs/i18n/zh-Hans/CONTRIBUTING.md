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
- Every eligible first-party source file must carry an SPDX `MIT OR Apache-2.0 OR MulanPSL-2.0` license header.
- Keep changes focused; describe **what** changed and **why** in the PR description.

CI runs all four parity engines, the Smalltalk/Pharo bootstrap, and a lint lane on every pull
request.

## 贡献许可

有意提交以纳入本项目的、符合适用范围的第一方实现代码、构建与打包工具、GTS 规范和
冻结的一致性测试向量，采用 **MIT OR Apache-2.0 OR MulanPSL-2.0** 接收。其他文档的贡献
保留该文档声明的许可，除非另有明确规定。项目 CLA 单独规定专有或商业再许可的授权。

For context, contributions to **GMEOW tooling/code** elsewhere in the project (the
[`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology) repository) are
accepted under **AGPL-3.0-only** and, under the project CLA, under terms that permit Blackcat
Informatics® Inc. to relicense them under separate proprietary/commercial terms. gmeow-gts is
the deliberately permissive, dependency-light engine layer, so it carries the permissive
`MIT OR Apache-2.0 OR MulanPSL-2.0` terms rather than AGPL.

提交贡献即表示您同意上述适用于该贡献的许可。若要将双重许可保留权扩展至您的贡献，
您还须按允许再许可（包括专有许可）的条款向 Blackcat Informatics® Inc. 提供许可。
在合并重大贡献之前，可能需要签署贡献者许可协议（CLA）。完整许可方案见
[`LICENSING.md`](./LICENSING.md)。

## 发布

贡献者不负责发布，但发布流程中有两点会影响日常改动：

- **版本是被检查的，而非假定的。** `scripts/check-versions.sh` 将每个语言通道与 Rust crate 比对，
  并断言 `README.md`、`rust/README.md`、`docs/GTS-ECOSYSTEM-INTEGRATIONS.md`、
  `rust/capi/gts.pc.in`、vcpkg port 以及两个 `Cargo.lock` 中的具体版本字符串。若改动了正文中的
  版本片段而清单未同步，CI 会失败。
- **一致性语料是生成的。** 切勿手工编辑 `vectors/`。请修改 `python/src/gts/vectors.py` 并运行
  `just check-vectors`，它会重新生成语料，若提交的字节不同则失败。

发布流程本身：候选版见
[`docs/GTS-V1-RC1-CHECKLIST.md`](./docs/GTS-V1-RC1-CHECKLIST.md)，正式版见
[`docs/GTS-V1-CHECKLIST.md`](./docs/GTS-V1-CHECKLIST.md)；其背后的策略见
[`docs/GTS-GOVERNANCE.md`](./docs/GTS-GOVERNANCE.md)，若你的改动要移除公开 API，
另见 §5.1 的弃用策略。

## 行为准则

Be respectful and constructive. Harassment and abuse are not tolerated.

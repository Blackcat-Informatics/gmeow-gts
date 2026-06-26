<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-V1-RC1-CHECKLIST.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS v1.0-rc1 检查清单与制品包

> [`docs/GTS-V1-RC1-CHECKLIST.md`](../../../../docs/GTS-V1-RC1-CHECKLIST.md) 的信息性中文翻译。英文文档仍然是治理、安全、发布、许可、贡献、行为义务、披露流程和可执行命令的权威来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

此检查清单将 [`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md) 中的 v1.0-rc1 发布路径转换为可运行的候选发布版本记录。请将未勾选的部分复制到发布议题中，或保留一份填写的副本作为发布制品。请勿通过编辑已提交的向量清单来标记发布修订版本；请生成下文所述的标记制品。

## 1. 候选记录

| 字段 | 值 |
|---|---|
| 发布 Issue | |
| 发布 PR | |
| 候选版本名称 | `v1.0-rc1` |
| 发布包版本 | |
| 规范提交 | |
| 语料库修订版本 | |
| 向量清单产物 | `dist/v1.0-rc1/vector-manifest.release.json` |
| Rust 包/标签 | `gmeow-gts` / `rust-v<version>` |
| Python 包/标签 | `gmeow-gts` / `py-v<version>` |
| Go 模块/标签 | `go.blackcatinformatics.ca/gts` / `go/v<version>` |
| npm 包/标签 | `@blackcatinformatics/gmeow-gts` / `npm-v<version>` |
| Ruby 包/标签 | `gmeow-gts` / `ruby-v<version>` |
| 发布说明提交 | |
| 发布经理 | |
| 日期 | |

在选择 pre-release package version 之前，请验证该字符串是否被 Cargo、PyPI、npm 以及存储库 version guard 接受。如果同一个 release candidate 的生态系统语法存在差异，请在此记录确切的各生态系统版本，并在 tagging 之前更新 release guard。tag/manifest 不匹配是发布阻塞项 (release blocker)，因为 tag workflows 在发布前会验证 manifest 版本。

## 2. 阻碍因素分类

根据 [`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md#71-v10-rc1-blockers) 中的阻碍因素和非阻碍因素列表，对每一项发现进行分类。

### 2.1 v1.0-rc1 阻碍因素 (Blockers)

| 阻碍因素类别 | 所需证据 | 状态 |
|---|---|---|
| 不再存在有意的线缆格式更改 | 已审查开放的 GIP/规范问题；没有已接受的更改会改动标头/帧语法、哈希原像、签名原像、段组合、转换解析或核心折叠语义。 | |
| 基准向量已存在并冻结 | 向量清单验证通过；语料库 (corpus) 重新生成无差异；盖章的发布清单指明了语料库修订版本。 | |
| 跨引擎基准行为通过 | Rust、Python、Go、TypeScript、Smalltalk/Pharo、Kotlin/JVM 和互操作性检查均针对同一语料库修订版本通过。 | |
| C ABI 封装器冒烟覆盖通过 | `rust/capi` 以及 C++、.NET、PHP、Lua、Swift、Ruby、R 和 Julia 封装器冒烟测试均针对发布候选版本通过。 | |
| 注册表策略已发布 | 帧类型、诊断、编解码器、配置文件 (profile)、转换目标和保留命名空间均包含在治理/规范/一致性文档中。 | |
| 安全模型清晰 | 安全策略 (security policy) 和加密推迟 (crypto-deferral) 守卫通过；不再存在未解决的高危或严重解析器、加密、提取或发布流水线漏洞。 | |
| 媒体类型和分发指南已具备 | `application/vnd.blackcat.gts+cbor-seq`、HTTP/range 行为、不可变发布和制品验证指南均已包含在规范/文档中。 | |
| 兼容性表述清晰 | 线缆、语料库、包和配置文件 (profile) 兼容性规则已包含在治理文档中，并由发布说明引用。 | |
| 实现者审查无阻碍性发现 | 审查问题/评论已关闭、被推迟 (deferred) 为非阻碍因素，或在下方记录所有者和理由。 | |
| 质量预算支付已记录 | 发布 PR 减少了至少一个朝向 `target_lines` 的超标热点，或记录了包含所有者、理由和后续问题的特意异常；未经质量预算审查标签或架构审查注释，不接受任何基准增加。 | |
| 阻断类别 | 所需证据 | 状态 |
|---|---|---|
| 无剩余的预期线路格式 (wire-format) 更改 | 开放的 GIP/规范问题已审查；没有任何已接受的更改会改动头部/帧 (frame) 语法、哈希原像、签名原像、段 (segment) 构成、转换解析或核心折叠 (fold) 语义。 | |
| 基准向量已存在并冻结 | 向量清单验证通过；语料库重新生成无差异；加盖时间戳的发布清单指明了语料库修订版本。 | |
| 跨引擎基准行为通过 | Rust、Python、Go、TypeScript、Smalltalk/Pharo、Kotlin/JVM 以及互操作性检查针对同一语料库修订版本均通过。 | |
| C ABI 封装器冒烟测试覆盖通过 | `rust/capi` 以及 C++、.NET、PHP、Lua、Swift、Ruby、R 和 Julia 封装器冒烟测试针对候选发布版本均通过。 | |
| 注册表策略已发布 | 帧 (Frame) 类型、诊断、编解码器、配置文件 (profiles)、转换目标和保留命名空间均已涵盖在治理/规范/一致性文档中。 | |
| 安全模型清晰 | 安全策略 (Security policy) 和加密推迟防护通过；解析器、加密、提取或发布流水线中无剩余的开放高危或关键漏洞。 | |
| 媒体类型和分发指南已具备 | `application/vnd.blackcat.gts+cbor-seq`、HTTP/range 行为、不可变发布以及制品验证指南已包含在规范/文档中。 | |
| 兼容性描述清晰 | 线路、语料库、包和配置文件 (profile) 兼容性规则已包含在治理文档中，并由发布说明引用。 | |
| 实现者审查无阻断性发现 | 审查问题/评论已关闭、作为非阻断项推迟，或在下方记录负责人和理由。 | |
| 质量预算偿还已记录 | 发布 PR 至少减少了一个向 `target_lines` 迈进的超标热点，或者记录了一个包含负责人、理由和后续问题的特意例外；如果没有质量预算审查标签或架构审查注释，则不接受任何基准增加。 | |

### 2.2 与发布相关的非阻塞项

这些项目对采用很有用，但当基准一致性 (baseline conformance) 就绪时，不会延迟 rc1，除非它们揭示了上述阻塞项之一。

| 交付物 | 跟踪议题 | 状态 | 发布说明 |
|---|---|---|---|
| 第三方实现指南 | `#104` | | |
| 基准测试套件和发布报告 | `#105` | `just bench-release` 和 [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) | |
| GTS 论文草案 | `#106` | [`GTS-PAPER-DRAFT.md`](./GTS-PAPER-DRAFT.md) | 资料性论文叙述；非规范性规范文本。 |
| 可选标准配置文件完成 | | | |
| 数据库、Parquet、浏览器、对象存储、范围获取、MMR、复制或高级证明工具 | | | |
| 未来的密钥管理或多接收者加密信封 | | | |
| 中立包别名或标准组织提交 | | | |

## 3. 本地环境快照

记录用于该候选版本的工具链和仓库状态。

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

## 4. 产物包设置

在运行候选检查之前，创建一个本地包目录。

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

如果语料库修订版本是一个标签或明确的提交，而不是当前的
`HEAD`，请显式标记它：

```bash
python scripts/check_vector_manifest.py \
  --corpus-revision git:<tag-or-full-commit> \
  --release-manifest "${OUT}/vector-manifest.release.json"
```

该包应包含：

- 带有包含发布规范 (release spec) 完整提交 (commit) 的 `spec-commit.txt`。
- 带有语料库 (corpus) 提交 (commit) 或标签 (tag) 的 `corpus-revision.txt`。
- 带有已盖章 `corpus_revision` 的 `vector-manifest.release.json`。
- `reports/` 中的各引擎测试与一致性 (conformance) 日志。
- `packages/` 中的软件包空运行 (Package dry-run) 输出。
- 标签发布 (tag release) 后的工作流 SBOM 和证明 (attestation) 引用。
- 发布说明 (Release notes) 或发布 PR 的链接。

## 5. 守卫与偏移检查

请在仓库根目录下运行这些命令。

```bash
bash scripts/check-versions.sh
python scripts/check_cli_parity.py
python scripts/check_api_parity.py
python scripts/check_advanced_contract.py
python scripts/check_ecosystem_contract.py
python scripts/check_security_contract.py
python scripts/check_crypto_deferrals.py
python scripts/check_quality_budget.py
python scripts/check_vector_manifest.py
python scripts/check_vector_manifest.py --self-test
```

确认 rc1 策略锚点已存在：

```bash
rg -n "v1.0-rc1|v1.0-rc1 Blockers|v1.0-rc1 Non-Blockers" docs/GTS-GOVERNANCE.md
rg -n "Wire-format compatibility|Corpus compatibility|Package compatibility|Profile compatibility" docs/GTS-GOVERNANCE.md
rg -n "application/vnd.blackcat.gts\\+cbor-seq|HTTP range|immutable" README.md docs/GTS-SPEC.md
rg -n "corpus_revision|release manifest|conformance report" docs/GTS-CONFORMANCE.md vectors/manifest.json
```

## 6. 语料库与一致性检查

重新生成并比较已提交的语料库：

```bash
just check-vectors
git diff --exit-code vectors
```

运行完整的引擎套件并捕获日志：

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

在上述引擎套件中通过 CLI 测试运行 validating-tool 和发布拒绝路径。发布说明应该 (SHOULD) 说明声称的一致性层级，并引用确切的一致性语料库 (conformance corpus) 修订版本。

为每个实现使用此报告行：

| 实现 | 版本 | 操作系统/架构 | 层级声称 | 语料库修订版本 | 命令/日志 | 通过/失败/跳过 |
|---|---|---|---|---|---|---|
| Rust | | | | | | |
| Python | | | | | | |
| Go | | | | | | |
| TypeScript | | | | | | |

## 7. 安全与供应链检查

在工具可用时运行本地供应链和仓库卫生检查。`just audit` 运行 justfile 中定义的 OSV 依赖扫描；pre-commit 是一个独立的卫生、lint 和密钥扫描关卡。发布 (Release) SLSA 态势记录在 [`GTS-RELEASE-SLSA.md`](./GTS-RELEASE-SLSA.md) 中：当前 GitHub artifact 证明被视为 SLSA v1.0 Build Level 2 证据。除非发布通道已迁移到强化的可重用工作流，且代表性 artifact 针对预期的签名者工作流身份进行了验证，否则不得声称达到 SLSA v1.0 Build Level 3。

```bash
just audit
pipx run pre-commit run --all-files
```

检查候选提交的 GitHub security 工作流：

```bash
gh run list --workflow security.yml --branch main --limit 5
gh run list --workflow codeql.yml --branch main --limit 5
gh run list --workflow fuzz.yml --branch main --limit 5
```

除非明确不在基准 v1 一致性 (baseline v1 conformance) 范围内，否则请将任何漏洞、CodeQL、fuzz、发布流水线 (release-pipeline) 或签名发现项记录为阻碍项 (blocker)。
如果发布有意采用可重用工作流 (reusable workflows) 以实现构建级别 3 (Build Level 3) 对齐，请在打标签之前在此记录签名者工作流策略和验证证据：

| 发布通道 | 可重用工作流 | 签名者验证命令 | 状态 |
|---|---|---|---|
| Rust `gmeow-gts` | | | |
| Rust `visual-hashing` | | | |
| Python | | | |
| Go | | | |
| TypeScript | | | |

## 8. 软件包试运行 (Package Dry-Runs)

在来自干净的发布 (release) 分支或发布 PR 合并提交的所有试运行 (dry-runs) 通过之前，请勿打标签 (tag)。

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
archive="$(bash rust/capi/scripts/package.sh)"
bash rust/capi/scripts/verify-archive.sh "${archive}"
GTS_PACKAGE_DRY_RUN_OUT="${OUT}/packages/wrappers" \
  bash scripts/package_dry_run_wrappers.sh
```

wrapper 演练涵盖了 Rust C ABI 软件包列表、可安装 C ABI 归档验证、已安装 C++ 归档使用、Conan 和 vcpkg 软件包管理器消费者冒烟测试、.NET 本地 NuGet 打包、Composer 验证、PHP Packagist 软件包根生成以及本地路径仓库消费者冒烟测试、LuaRocks lint/make/pack 以及已安装 rock 的冒烟执行、Swift 根软件包 dump/run 验证、Ruby gem 构建/安装、R 构建/检查以及 Julia 软件包测试。

为了实现 Go 发布一致性 (release parity)，还需对 `.github/workflows/release-go.yaml` 使用的交叉构建形状 (cross-build shape) 进行演练：

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

## 9. 发布说明

发布 PR 必须 (MUST) 更新 `CHANGELOG.md`、`CITATION.cff`、包清单、锁定文件，以及当包版本发生变化时的 README/文档代码段。

发布说明必须 (MUST) 包括：

- 候选名称和包版本，或各生态系统的特定版本；
- 规范 (spec) 提交；
- 语料库 (corpus) 修订版和盖章清单 (manifest) 产物名称；
- 各实现的一致性层级 (conformance tier) 声明；
- 包注册表名称和发布标签；
- 阻碍因素 (blocker) 评审摘要；
- 质量预算 (quality-budget) 削减摘要，或带有所有者和后续议题链接的已记录异常；
- 与发布相关的非阻碍因素和后续议题链接；
- SBOM 和证明 (attestation) 验证说明；
- 已知限制和推迟 (deferred) 的功能。

最低发布说明证据：

```bash
bash scripts/check-versions.sh
rg -n "<version>|spec commit|corpus revision|conformance|SBOM|attestation" CHANGELOG.md README.md docs
```

## 10. 打标签和发布序列

在发布 PR 合并后，对准确的合并提交进行打标签。逐个推送发布标签，以便每个由标签触发的工作流都能接收到各自的事件。

在推送 Rust 标签之前，确认 `gmeow-gts` 的 crates.io Trusted Publisher 条目已激活，且所有者/仓库为 `Blackcat-Informatics/gmeow-gts`，工作流为 `release-cargo.yaml`，环境为 `(none)`。正常的 Rust 发布路径使用 GitHub Actions OIDC，不需要 `CARGO_REGISTRY_TOKEN`。
在首次 `gmeow-gts-capi` 源码 crate 发布之前，确认 `CARGO_REGISTRY_TOKEN` 引导密钥对 `release-cargo-capi.yaml` 可用。此令牌仅用于初始 crate 引导。在第一个版本出现在 crates.io 上之后，为 `gmeow-gts-capi` 提交并完成后续的 Trusted Publishing 迁移，包括所有者/仓库 `Blackcat-Informatics/gmeow-gts`、工作流 `release-cargo-capi.yaml` 和环境 `(none)`，除非添加了受保护的发布环境。

如果 `gmeow-gts` Rust crate 依赖于较新版本的 `visual-hashing`，请先从其独立存储库发布该 crate。其 crates.io Trusted Publisher 条目必须 (MUST) 使用所有者/仓库 `Blackcat-Informatics/visual-hashing`、工作流 `release.yml` 和环境 `(none)`。
在推送 RubyGems 标签之前，确认 `gmeow-gts` 的待处理 RubyGems 可信发布者 (Trusted Publisher) 使用了 owner/repo `Blackcat-Informatics/gmeow-gts`、workflow `release-rubygems.yaml` 以及 environment `(none)`，除非发布 (release) 明确添加了受保护环境。

在推送 Go 标签之前，确认已启用仓库级不可变发布 (immutable releases)：

```bash
gh api repos/Blackcat-Informatics/gmeow-gts/immutable-releases
```

```bash
MERGE_COMMIT="<full-merge-commit>"
VERSION="<version>"
git tag "rust-v${VERSION}" "${MERGE_COMMIT}"
git tag "py-v${VERSION}" "${MERGE_COMMIT}"
git tag "go/v${VERSION}" "${MERGE_COMMIT}"
git tag "npm-v${VERSION}" "${MERGE_COMMIT}"
git tag "capi-v${VERSION}" "${MERGE_COMMIT}"
git tag "ruby-v${VERSION}" "${MERGE_COMMIT}"
git tag "${VERSION}" "${MERGE_COMMIT}" # Swift Package Manager / Swift Package Index
git push origin "rust-v${VERSION}"
git push origin "py-v${VERSION}"
git push origin "go/v${VERSION}"
git push origin "npm-v${VERSION}"
git push origin "capi-v${VERSION}"
git push origin "ruby-v${VERSION}"
git push origin "${VERSION}"
```

如果 `visual-hashing` 发生了变化，请在发布依赖于新 crate 版本的 `rust-v*` 标签之前发布它：

```bash
VISUAL_HASHING_VERSION="<visual-hashing-version>"
gh repo clone Blackcat-Informatics/visual-hashing ../visual-hashing
cd ../visual-hashing
git tag "v${VISUAL_HASHING_VERSION}" "<visual-hashing-merge-commit>"
git push origin "v${VISUAL_HASHING_VERSION}"
cd -
```

监控发布工作流：

```bash
gh run list --event push --limit 30
gh run list --workflow release-cargo.yaml --branch "rust-v${VERSION}" --limit 5
gh run list --workflow release-cargo-capi.yaml --branch "capi-v${VERSION}" --limit 5
gh run list --workflow release-pypi.yml --branch "py-v${VERSION}" --limit 5
gh run list --workflow release-go.yaml --branch "go/v${VERSION}" --limit 5
gh run list --workflow release-npm.yaml --branch "npm-v${VERSION}" --limit 5
gh run list --workflow release-capi.yaml --branch "capi-v${VERSION}" --limit 5
```

在纯 Swift 语义化版本标签存在后，验证根包并将带有协议和 `.git` 扩展名的仓库 URL 提交到 Swift Package Index：

```bash
diff -u rust/capi/include/gts.h swift/Sources/CGts/include/gts.h
swift package dump-package --package-path .
bash swift/scripts/smoke.sh
```

提交：

```text
https://github.com/Blackcat-Informatics/gmeow-gts.git
```

如果 `visual-hashing` 已发布，请按标签监控其工作流：

```bash
gh run list --repo Blackcat-Informatics/visual-hashing --workflow release.yml --branch "v${VISUAL_HASHING_VERSION}" --limit 5
```

如果标签被推送到错误的提交或版本有误，请在删除或重新创建该标签之前停止并提交发布事故记录。

## 11. 已发布制品验证

在工作流完成后，验证公共注册表和发布制品。
维护者冒烟验证器仅从公共界面执行下载、哈希、注册表溯源、
GitHub SLSA、SPDX SBOM 和不可变发布 (immutable-release) 检查：

```bash
VISUAL_HASHING_VERSION="<visual-hashing-version>"
just verify-release-dry-run "${VERSION}" "${VISUAL_HASHING_VERSION}"
just verify-release "${VERSION}" "${VISUAL_HASHING_VERSION}"
```

在 C ABI 封装包发布后，运行封装感知验证器：

```bash
just verify-wrapper-release-dry-run "${VERSION}" "${VISUAL_HASHING_VERSION}"
just verify-wrapper-release "${VERSION}" "${VISUAL_HASHING_VERSION}"
```

相同的验证器可以通过手动 `Verify published release` 工作流从 GitHub Actions UI 运行。在凭据或注册表传播就绪之前启用 `dry_run`，并为封装器阶段启用 `include_wrapper_packages`。它会上传 `dist/release-verification/${VERSION}/release-verification-summary.md` 和匹配的 JSON 报告。该报告将通过/警告/失败严重程度与发布状态值（如 `published`、`pending`、`metadata-mismatch` 和 `missing`）分开，以便传播延迟不会与错误的元数据或缺失的制品混淆。`release-verification-summary.json` 作为工作流制品。不要为新发布传递 `--allow-legacy-release-gaps`；该覆盖仅用于审计早于 SBOM 和不可变发布硬化的发布。

### 安全与加密就绪情况

- [ ] [GTS-SEC-01] 安全策略已更新 RC-1 联系人。
- [ ] [GTS-SEC-02] 漏洞披露流程已通过演练 (dry-run) 测试。
- [ ] [GTS-SEC-03] BLAKE3 实现已针对官方测试向量进行验证。
- [ ] [GTS-SEC-04] Ed25519 (COSE) 实现已针对官方测试向量进行验证。
- [ ] [GTS-SEC-05] 随机数生成 (RNG) 使用平台最佳基元。
- [ ] [GTS-SEC-06] 所有加密 MAC/签名均使用恒定时间 (constant-time) 比较。

```bash
cargo search gmeow-gts --limit 1
cargo search gmeow-gts-capi --limit 1
python -m pip index versions gmeow-gts
npm view @blackcatinformatics/gmeow-gts version
dotnet nuget search Gmeow.Gts --source https://api.nuget.org/v3/index.json
composer show blackcatinformatics/gmeow-gts --available
curl -fsSL https://luarocks.org/manifest.json | python -m json.tool >/dev/null
gem info gmeow-gts --remote
curl -fsSL https://blackcat-informatics.r-universe.dev/src/contrib/PACKAGES
curl -fsSL https://raw.githubusercontent.com/JuliaRegistries/General/master/G/GmeowGTS/Package.toml
gh release view "go/v${VERSION}" \
  --json tagName,name,url,isDraft,isImmutable,isPrerelease,publishedAt
gh release view "capi-v${VERSION}" \
  --json tagName,name,url,isDraft,isImmutable,isPrerelease,publishedAt
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify "capi-v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
```

面向消费者的验证命令均由维护者冒烟验证器 (maintainer smoke verifier) 覆盖，如下：

```bash
pypi-attestations verify pypi \
  --repository https://github.com/Blackcat-Informatics/gmeow-gts \
  "https://files.pythonhosted.org/.../gmeow_gts-${VERSION}-py3-none-any.whl"
npm audit signatures
gh attestation verify <downloaded-artifact> --repo Blackcat-Informatics/gmeow-gts
gh attestation verify <downloaded-artifact> \
  --repo Blackcat-Informatics/gmeow-gts \
  --predicate-type https://spdx.dev/Document/v2.3
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify "capi-v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
gh release verify-asset "go/v${VERSION}" <downloaded-go-asset> \
  --repo Blackcat-Informatics/gmeow-gts
gh release verify-asset "capi-v${VERSION}" <downloaded-capi-asset> \
  --repo Blackcat-Informatics/gmeow-gts
```

### 发布证据持久性

| ID | 领域 | 要求 | 优先级 | 状态 | 证据 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| REL-DUR-01 | SLSA | $1$ 认证 (attestations) $2$ 跨注册表迁移 | P0 | 待定 | $3$ |
| REL-DUR-02 | Sigstore | $4$ 透明日志 $5$ 用于 $6$ | P0 | 待定 | $7$ |
| REL-DUR-03 | 来源 (Provenance) | $8$ 不可伪造的构建源 $9$ 适用于所有 $10$ 制品 | P0 | 待定 | $11$ |
| REL-DUR-04 | 身份 | $12$ 无密钥签名 $13$ 链接到 $14$ | P0 | 待定 | $15$ |
| REL-DUR-05 | 发现 | $16$ 元数据发现 $17$ 通过 $18$ | P1 | 待定 | $19$ |

| 发布界面 | 持久化产物 | 证明证据 |
|---|---|---|
| Go | 不可变的 GitHub Release 归档、`checksums.txt` 和 `sbom-go-gts.spdx.json` | 针对不可变发布的 GitHub release 证明；针对发布资产的 SLSA 来源证明；针对发布归档的 SPDX SBOM 证明 |
| C ABI | 不可变的 GitHub Release 归档、`checksums.txt` 和 `sbom-gmeow-gts-capi.spdx.json` | 针对不可变发布的 GitHub release 证明；针对发布资产的 SLSA 来源证明；针对发布归档的 SPDX SBOM 证明 |
| crates.io `gmeow-gts` | 注册表托管的 `.crate` 软件包 | GitHub 证明存储中的 SLSA 来源和 SPDX SBOM 证明 |
| crates.io `gmeow-gts-capi` | 注册表托管的 `.crate` 软件包 | GitHub 证明存储中的 SLSA 来源和 SPDX SBOM 证明；引导令牌，直至 Trusted Publishing 后续工作落地 |
| PyPI | 注册表托管的 wheel/sdist | PyPI 发布证明，以及 GitHub SLSA 来源和 SPDX SBOM 证明 |
| npm | 注册表托管的 tarball | npm 来源证明，以及 GitHub SLSA 来源和 SPDX SBOM 证明 |
| RubyGems | 注册表托管的 `.gem` 软件包 | GitHub 证明存储中的 SLSA 来源和 SPDX SBOM 证明 |
| NuGet `Gmeow.Gts` | 注册表托管的 `.nupkg` 软件包 | 注册表元数据和软件包下载检查；仅源码包装器，需要宿主 `libgts` |
| Packagist `blackcatinformatics/gmeow-gts` | 来自 Packagist 的 VCS 标签元数据 | 注册表元数据和源码引用检查；仅源码包装器，需要宿主 `libgts` |
| LuaRocks `gmeow-gts` | 注册表托管的 rockspec/源码 rock | LuaRocks 根清单和 rockspec 下载检查；仅源码包装器，需要宿主 `libgts` |
| Swift Package Index | 存储库语义化版本标签和 SPI 软件包 URL | Git 标签检查和规范 SPI URL 记录；仅源码包装器，需要宿主 `libgts` |
| r-universe `gmeowgts` | 注册表托管的源码包 | PACKAGES 索引和源码 tarball 下载检查；需要宿主 `libgts` 的源码包 |
| Julia General `GmeowGTS` | General 注册表软件包元数据 | 注册表身份和版本检查；仅源码包装器，需要宿主 `libgts` |
| Conan/vcpkg `gmeow-gts` | 本地第一方试运行，直到提交至上游 | 在上游配方被接受前，暂无公开注册表证据 |

下载代表性产物以进行验证：

```bash
mkdir -p \
  "${OUT}/packages/go-release" \
  "${OUT}/packages/npm" \
  "${OUT}/packages/python" \
  "${OUT}/packages/ruby" \
  "${OUT}/packages/rust" \
  "${OUT}/packages/wrappers"
gh release download "go/v${VERSION}" --dir "${OUT}/packages/go-release"

python -m pip download --no-deps --dest "${OUT}/packages/python" "gmeow-gts==${VERSION}"
npm pack "@blackcatinformatics/gmeow-gts@${VERSION}" \
  --pack-destination "${OUT}/packages/npm"
gem fetch gmeow-gts --version "${VERSION}" --clear-sources --source https://rubygems.org
mv "gmeow-gts-${VERSION}.gem" "${OUT}/packages/ruby/"
curl -L "https://crates.io/api/v1/crates/gmeow-gts/${VERSION}/download" \
  -o "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate"
curl -L "https://crates.io/api/v1/crates/gmeow-gts-capi/${VERSION}/download" \
  -o "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate"
curl -L "https://api.nuget.org/v3-flatcontainer/gmeow.gts/${VERSION}/gmeow.gts.${VERSION}.nupkg" \
  -o "${OUT}/packages/wrappers/Gmeow.Gts.${VERSION}.nupkg"
curl -L "https://luarocks.org/gmeow-gts-${VERSION}-1.rockspec" \
  -o "${OUT}/packages/wrappers/gmeow-gts-${VERSION}-1.rockspec"
curl -L "https://blackcat-informatics.r-universe.dev/src/contrib/gmeowgts_${VERSION}.tar.gz" \
  -o "${OUT}/packages/wrappers/gmeowgts_${VERSION}.tar.gz"
```

验证代表性制品和 Go 发布清单上的默认 SLSA provenance：

```bash
for artifact in "${OUT}"/packages/go-release/gts_"${VERSION}"_*; do
  gh attestation verify "$artifact" --repo Blackcat-Informatics/gmeow-gts
done
for artifact in \
  "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate" \
  "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate" \
  "${OUT}"/packages/npm/*.tgz \
  "${OUT}"/packages/python/* \
  "${OUT}"/packages/ruby/*.gem; do
  gh attestation verify "$artifact" --repo Blackcat-Informatics/gmeow-gts
done
gh attestation verify "${OUT}/packages/go-release/checksums.txt" \
  --repo Blackcat-Informatics/gmeow-gts
gh attestation verify "${OUT}/packages/go-release/sbom-go-gts.spdx.json" \
  --repo Blackcat-Informatics/gmeow-gts
```

这些命令验证当前的 Build Level 2 制品证明 (artifact-attestation) 态势。
如果候选版本采用可重用工作流 (reusable workflows) 以实现更强的态势，请使用预期的签名者策略重复代表性制品检查，例如：

```bash
gh attestation verify <downloaded-artifact> \
  --repo Blackcat-Informatics/gmeow-gts \
  --signer-workflow <owner>/<repo>/.github/workflows/<workflow>.yml@<ref>
```

验证不可变的 Go 发布证明以及每个下载的发布资产：

```bash
gh release verify "go/v${VERSION}" --repo Blackcat-Informatics/gmeow-gts
for artifact in "${OUT}"/packages/go-release/*; do
  gh release verify-asset "go/v${VERSION}" "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts
done
```

验证每个发布通道的一个代表性产物的 SPDX SBOM 认证。当前的 SBOM 生成器生成 SPDX 2.3，因此谓词类型为 `https://spdx.dev/Document/v2.3`; if the emitted `spdxVersion` 发生变化，请更新谓词版本以进行匹配。

```bash
SBOM_PREDICATE="https://spdx.dev/Document/v2.3"
for artifact in "${OUT}"/packages/go-release/gts_"${VERSION}"_*; do
  gh attestation verify "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts \
    --predicate-type "${SBOM_PREDICATE}"
done
for artifact in \
  "${OUT}/packages/rust/gmeow-gts-${VERSION}.crate" \
  "${OUT}/packages/rust/gmeow-gts-capi-${VERSION}.crate" \
  "${OUT}"/packages/npm/*.tgz \
  "${OUT}"/packages/python/*; do
  gh attestation verify "$artifact" \
    --repo Blackcat-Informatics/gmeow-gts \
    --predicate-type "${SBOM_PREDICATE}"
done
```

记录最终注册表状态：

| 发布面 | 预期版本/标签 | 证据 | 状态 |
|---|---|---|---|
| crates.io `gmeow-gts` | | | |
| crates.io `gmeow-gts-capi` | | | |
| PyPI `gmeow-gts` | | | |
| Go 发布 `go.blackcatinformatics.ca/gts` | | | |
| npm `@blackcatinformatics/gmeow-gts` | | | |
| NuGet `Gmeow.Gts` | | | |
| Packagist `blackcatinformatics/gmeow-gts` | | | |
| LuaRocks `gmeow-gts` | | | |
| Swift Package Index `Blackcat-Informatics/gmeow-gts` | | | |
| RubyGems `gmeow-gts` | | | |
| r-universe `gmeowgts` | | | |
| Julia General `GmeowGTS` | | | |
| Conan/vcpkg `gmeow-gts` 状态 | | | |
| SBOM 证明 | | | |
| Build-provenance 证明 | | | |

## 12. 最终决定

只有满足以下条件时，release candidate 才准备就绪：

- 第 2.1 节中的每个 blocker 行均已解决或被明确否定；
- 每个与发布 (release) 相关的非 blocker 项均已链接到 issue 或标记为已完成；
- 一致性报告 (conformance reports) 命名了相同的已盖戳 corpus revision；
- 发布说明 (release notes) 引用了 spec commit、corpus revision、package versions 以及已知的推迟 (deferred) 项；
- package dry-runs 和 release workflows 通过；
- 公共注册表 (public registries) 和 artifact verification 证明发布的 bits 与预期的 candidate 匹配。

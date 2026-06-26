<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-RELEASE-SLSA.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 发布 SLSA 态势

> [`docs/GTS-RELEASE-SLSA.md`](../../../../docs/GTS-RELEASE-SLSA.md) 的信息性中文翻译。英文文档仍然是治理、安全、发布、许可、贡献、行为义务、披露流程和可执行命令的权威来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

本文档定义了 Graph Transport Substrate (GTS) 项目的 **软件制品供应链级别 (SLSA)** 态势。它概述了该项目如何在多语言实现生态系统中实现并维护构建完整性、来源可追溯性以及供应链安全。

## 目标

1. **完整性 (Integrity)**：确保交付给用户的所有制品（crates、gems、wheels、npm 软件包等）均构建自受信任的 GTS 源代码。
2. **透明度 (Transparency)**：为每次发布提供可验证的来源信息。
3. **安全性 (Security)**：保护发布流水线，防止篡改、凭据盗窃和恶意依赖注入。
4. **SLSA L3 目标**：GTS 项目的所有主要分发渠道均以 **SLSA Level 3** (Build L3) 为目标。

## 决策

推迟 v1.0-rc1 发布路径的 reusable-workflow 迁移。当前的发布产物应被描述为 GitHub artifact-attested SLSA v1.0 Build Level 2 证据，以及下文所述的 registry、SBOM、immutable-release 和公共验证控制。在发布泳道移至加固的 reusable workflows 且代表性产物验证符合预期的 signer workflow identity 之前，不要声称 GTS 发布产物符合 SLSA v1.0 Build Level 3。

这是一个文档决策，而非发布加固的削减。当前的发布 workflows 已经为多语言发布模型提供了强有力的公共证据。只有当 reusable workflows 创建了更清晰的信任边界且消费者验证可以强制执行该边界时，将所有泳道移入 reusable workflows 才值得进行。
GitHub 将制品证明记录为 SLSA v1.0 Build Level 2 证据。
GitHub 还将可重用工作流记录为实现 SLSA v1.0 Build Level 3 对齐的更强隔离途径，因为构建可以与已知的、经过审核的构建指令绑定。

当前的 GTS 发布通道是本仓库中的第一方工作流文件，但 `visual-hashing` 除外，它现在从其独立的仓库发布：

| 发布通道 | 工作流 | 当前发布路径 |
|---|---|---|
| Rust `gmeow-gts` crate | `.github/workflows/release-cargo.yaml` | 通过 GitHub Actions OIDC 进行的 crates.io Trusted Publishing |
| Rust `gmeow-gts-capi` 源 crate | `.github/workflows/release-cargo-capi.yaml` | 用于首次发布的 crates.io bootstrap token；需要后续跟进 Trusted Publishing |
| Rust `visual-hashing` crate | `Blackcat-Informatics/visual-hashing:.github/workflows/release.yml` | 通过 GitHub Actions OIDC 进行的 crates.io Trusted Publishing |
| Python 软件包 | `.github/workflows/release-pypi.yml` | 带有软件包证明 (package attestations) 的 PyPI trusted publishing |
| TypeScript 软件包 | `.github/workflows/release-npm.yaml` | npm trusted publishing 和 npm provenance |
| Lua 软件包 | `.github/workflows/release-luarocks.yaml` | LuaRocks API-token 发布；无注册表原生 provenance |
| Ruby 软件包 | `.github/workflows/release-rubygems.yaml` | 通过 GitHub Actions OIDC 进行的 RubyGems Trusted Publishing |
| Go CLI 资产 | `.github/workflows/release-go.yaml` | 不可变的 GitHub Release 资产 |
| C ABI 原生资产 | `.github/workflows/release-capi.yaml` | 不可变的 GitHub Release 归档 |

将这些作业重构为同仓库的可重用工作流 (reusable workflows) 可以集中发布 (release) 逻辑，但其本身并不能提供足够的治理 (governance) 隔离，不足以证明在 v1.0-rc1 之前立即更改每个发布通道的合理性。更强大的升级是受保护且经过审查的可重用工作流边界，其验证要求具备预期的可重用工作流身份。

## 当前保证

每个发布 (release) 路径必须 (MUST) 保持以下控制措施：

- 发布前的标签到清单 (tag-to-manifest) 版本检查；
- 发布任务的最小权限 GitHub Actions 权限；
- 固定 (pinned) 第三方 Actions；
- 注册表 OIDC、注册表原生来源 (provenance)，或在需要时记录在案的首个发布令牌引导 (first-publish token bootstrap)；
- 已发布制品的 GitHub 构建来源 (build-provenance) 证明 (attestations)；
- 代表性注册表制品和 Go 归档文件的 SPDX SBOM 证明 (attestations)；
- 归档文件、校验和及 SBOM 资产的不可变 Go 和 C ABI GitHub Releases；
- 通过 `just verify-release` 进行公开的发布后验证，并在注册表上线前进行确定性的 `just verify-release-dry-run` 规划。
- 当发布 C ABI 封装包 (wrapper packages) 时，通过 `just verify-wrapper-release` 进行封装包发布后验证，包括注册表元数据链接检查以及已归档的 `published` / `pending` / `metadata-mismatch` / `missing` 状态报告。

当前证据持久性为：

| 发布面 | 持久工件 | 证明证据 |
|---|---|---|
| Go | 不可变的 GitHub Release 归档，`checksums.txt` 和 `sbom-go-gts.spdx.json` | GitHub release 证明、SLSA 来源证明和 SPDX SBOM 证明 |
| C ABI | 不可变的 GitHub Release 归档，`checksums.txt` 和 `sbom-gmeow-gts-capi.spdx.json` | GitHub release 证明、SLSA 来源证明和 SPDX SBOM 证明 |
| crates.io `gmeow-gts` | 注册表托管的 `.crate` 软件包 | GitHub SLSA 来源和 SPDX SBOM 证明 |
| crates.io `gmeow-gts-capi` | 注册表托管的 `.crate` 软件包 | GitHub SLSA 来源和 SPDX SBOM 证明；在 Trusted Publishing 后续落地之前的引导令牌 (bootstrap token) |
| PyPI | 注册表托管的 wheel 和 sdist | PyPI 发布证明，以及 GitHub SLSA 来源和 SPDX SBOM 证明 |
| npm | 注册表托管的 tarball | npm provenance，以及 GitHub SLSA 来源和 SPDX SBOM 证明 |
| RubyGems | 注册表托管的 `.gem` 软件包 | GitHub SLSA 来源和 SPDX SBOM 证明 |
| NuGet `Gmeow.Gts` | 注册表托管的 `.nupkg` 软件包 | 注册表元数据和软件包下载检查；在 NuGet 发布通道添加之前暂无项目证明 |
| Packagist `blackcatinformatics/gmeow-gts` | 来自 Packagist 的 VCS 标签元数据 | 注册表元数据和源码引用检查；软件包内容来自带标签的 package-root 提交 |
| LuaRocks `gmeow-gts` | 注册表托管的 rockspec/源码 rock | 注册表清单和 rockspec 下载检查；当前 LuaRocks 通道中暂无项目证明 |
| Swift Package Index | 仓库语义化版本标签和 SPI 软件包 URL | Git 标签检查和规范 SPI URL 记录；软件包源码为带标签的仓库 |
| r-universe `gmeowgts` | 注册表托管的 R 源码包 | PACKAGES 索引和源码 tarball 下载检查；在 R 发布通道添加之前暂无项目证明 |
| Julia General `GmeowGTS` | General 注册表软件包元数据 | 注册表身份和版本检查；软件包源码为带标签的仓库条目 |
| Conan/vcpkg | 本地第一方试运行 (dry-runs) | 在上游配方 (recipes) 被接受之前暂无公共注册表证据 |

## 未来的构建 3 级 (Build Level 3) 路径

仅当发布 (release) 模型可以验证更强的边界时，才提高态势：

1. 为每个发布 (release) 泳道创建用于构建、打包、SBOM 和认证 (attestation) 步骤的可重用工作流，或者在生态系统可以安全共享实现时，创建一组更精简的共享发布工厂。
2. 使用存储库规则、强制评审和 CODEOWNERS 保护这些可重用工作流。如果治理 (governance) 增益旨在强于此存储库的常规分支保护，则优先选择由组织管理的工作流存储库。
3. 保持调用者工作流简洁。调用者应该 (SHOULD) 仅传递 version、tag 和 release-material 输入，然后让可重用工作流进行构建、打包、生成 SBOM、认证 (attest) 并发布。
4. 仅授予调用者和可重用工作流该泳道所需的权限，包括用于生成认证 (attestation) 作业的 `contents: read`、`id-token: write` 和 `attestations: write`。
5. 保留现有的注册表 OIDC/受信任发布路径、Go 不可变发布 (release) 流、SBOM 生成以及 `just verify-release` / `just verify-wrapper-release` 冒烟验证。
6. 使用可选的签名者策略输入扩展 `scripts/verify_release.py`，以便发布 (release) 可以要求带有 `--signer-workflow <owner>/<repo>/.github/workflows/<workflow>.yml@<ref>` 的 `gh attestation verify`，并在适用时要求 `--signer-repo ...`。
7. 在声称达到构建 3 级 (Build Level 3) 之前，根据预期的可重用工作流身份，验证来自每个已采用发布 (release) 泳道的至少一个代表性产物 (artifact)。
在这些步骤完成之前，发布说明和清单应该 (SHOULD) 将当前态势说明为 SLSA v1.0 Build Level 2 制品证明 (artifact attestations)，并带有注册表来源 (registry provenance)、SBOM 证明 (attestations)、不可变的 Go 发布以及公共验证器覆盖。

## 参考

- [GitHub 构件证明](https://docs.github.com/en/actions/concepts/security/artifact-attestations)
- [使用构件证明和可复用工作流以实现 SLSA v1 Build Level 3](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/increase-security-rating)

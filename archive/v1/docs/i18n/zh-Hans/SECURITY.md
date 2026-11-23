<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: SECURITY.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 安全策略

> [`SECURITY.md`](../../../SECURITY.md) 的信息性中文翻译。英文文档仍然是治理、安全、发布、许可、贡献、行为义务、披露流程和可执行命令的权威来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。

GTS (Graph Transport Substrate 规范及其四个引擎 —— Rust, Python, Go 和 TypeScript) 由 Blackcat Informatics® Inc. 维护。我们非常重视安全报告，并要求私下报告漏洞，以便能够负责任地进行调查和修复。

## 受支持的版本

| 版本 | 受支持 |
| --- | --- |
| `0.1.x` | 是 |
| `< 0.1` | 否 |

## 报告安全漏洞

请勿针对安全漏洞开启公开的 GitHub issue。

而是应该：

- 发送电子邮件至 <security@blackcatinformatics.ca>
- 在邮件主题栏中包含 `SECURITY`
- 指明受影响的引擎（Rust / Python / Go / TypeScript）及版本
- 描述问题、影响以及受影响的版本
- 在可能的情况下提供重现步骤、概念验证 `.gts` 文件或补丁

本项目最相关的安全攻击面是**读取不受信任的 GTS 数据并运行 `gts` 工具**，例如：

- **解析 (Parsing)** — CBOR 解码和可堆叠转换/编解码器链（`gzip` / `zstd` 负载的解压缩，包括资源消耗/解压缩炸弹输入）以及不透明/降级帧 (frame) 处理。
- **完整性与加密 (Integrity & crypto)** — BLAKE3 链验证、COSE 签名/验证 (§9.2) 以及 COSE 加密 (§9.3)；导致接受被篡改或损坏日志的错误验证。
- **文件配置文件 (The files profile)** — `gts unpack`/`extract` 写入磁盘；路径遍历、符号链接或绝对路径转义（引擎预期将拒绝这些操作）。
- **供应链 (Supply chain)** — 在 crates.io、PyPI、npm 和 Go 模块代理上发布的包，以及生成这些包的发布 (release)/CI 工作流。

## 预期流程

- 在 48 小时内确认收到报告
- 在 7 天内进行初步分类
- 在验证后进行协调修复与披露

解决时间表取决于严重程度、可利用性以及发布约束，但我们的目标是尽可能快地处理已确认的问题。涉及 wire format 或共享行为的修复将在所有四个引擎和一致性语料库 (conformance corpus) 中进行协调。

## 负责任的披露流程

1. 私下报告问题。
2. 维护者验证并对报告进行分类。
3. 开发、审查并测试修复程序（跨受影响的引擎）。
4. 准备发布版本或安全公告。
5. 在用户有合理的时间进行更新后，进行公开披露。

## 报告者致谢

对于过去 12 个月内解决的每个漏洞报告，公开的发布说明或安全公告应该 (SHOULD) 为报告者致谢，除非报告者要求匿名或私下处理。如果多个报告者对一个已确认的问题做出了贡献，应为每一位希望获得公开认可的报告者致谢。

报告者致谢应该 (SHOULD) 使用报告者提供的名称、账号或组织。如果报告者要求匿名，发布说明和公告应该 (SHOULD) 说明该问题是私下报告的，而不提及他们的姓名。

目前没有在过去 12 个月内解决的公开披露的项目漏洞。

## 安全更新

- 关注代码仓库的发布 (release) 和安全公告 (advisory)
- 保持依赖项处于最新状态
- 当发布修复补丁时，更新至最新的受支持发布 (release)

## 联系方式

- Email: <security@blackcatinformatics.ca>
- 用于加密报告的 PGP 密钥：

```text
-----BEGIN PGP PUBLIC KEY BLOCK-----
Version: GnuPG v2

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
=fCTf
-----END PGP PUBLIC KEY BLOCK-----
```

对于非安全问题或错误报告，请使用常规的公开 issue 或讨论渠道。

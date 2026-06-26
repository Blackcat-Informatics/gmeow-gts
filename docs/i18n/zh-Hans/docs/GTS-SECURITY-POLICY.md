<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-SECURITY-POLICY.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 安全与信任策略 (GTS Security And Trust Policy)

> [`docs/GTS-SECURITY-POLICY.md`](../../../../docs/GTS-SECURITY-POLICY.md) 的信息性中文翻译。英文文档仍然是治理、安全、发布、许可、贡献、行为义务、披露流程和可执行命令的权威来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

本文档固定了位于核心 GTS 线格式之上的 v1 安全合约。核心读取器验证字节、帧哈希、链以及可选的 COSE 加密有效性。它不判定签名者是否获得授权、声明是否真实，或接收者标识符是否具有隐私保护性。

## 信任分离

`Signature.status == "valid"` 意味着 COSE 签名在调用者解析的密钥下验证成功。这并不意味着：

- 部署环境信任该密钥；
- 签名者已获得该配置文件的授权；
- 已签名的 RDF 断言为真。

部署信任由 Python 中的 `gts.policy.TrustPolicy`、Rust 中的 `gmeow_gts::policy::TrustPolicy`、Go 中的 `go.blackcatinformatics.ca/gts/policy.TrustPolicy` 以及 TypeScript 中的 `policy.TrustPolicy` 表示。高级文件验证在 Python 中公开为 `gts.verify.verify_file`，在 Rust 中公开为 `gmeow_gts::verify::verify_file`；所有公开配置文件策略的引擎都会将加密签名状态与部署信任分开报告。识别配置文件的工具可以要求受信任的签名者，而基准读取器仍可以返回可恢复的图以及签名状态。

需要基于文件的策略的 Rust 部署应启用 `--features policy-config`。该可选功能增加了 JSON 加载助手和 `gts verify --policy <file>`；`--features policy-config-yaml` 在此基础上增加了 YAML。默认的 Rust 构建保留了策略评估器，但不继承 serde 或 YAML 解析器依赖项。文件形态为：

```yaml
trusted_signers:
  - did:example:issuer
require_trusted_signer: true
pseudonymous_kid_pattern: "^anon:[0-9a-fA-F]{32,}$"
```

## Profile Enforcement (配置文件强制执行)

V1 一致性层级被刻意分开：

- Baseline Reader：解析并折叠 (fold) 可恢复的 GTS 数据、blob、签名、诊断和 segment (段) 元数据。它不递归进入嵌套的 GTS blob，也不授权签名者。
- Full Reader：包含 Baseline Reader 行为以及可选能力，例如签名验证、可解密性检查和有界嵌套 GTS 发现。
- Profile-Aware Tool：包含读取器 (reader) 输出以及部署/配置文件 (profile) 策略检查，例如受信任的签名者、证据头承诺、不透明接收者假名化、配置文件词汇表声明以及可流式处理布局 (streamable-layout) 声明。
| 配置文件 | v1 中的强制执行 | 发现代码 |
|---|---|---|
| `evidence` | 在配置文件验证中要求已签名的帧和已签名的段头。除非调用方提供了受信任的签名者 ID (trusted signer ids)，否则部署信任是可选的。 | `ProfileSignatureRequired`, `ProfileSignatureInvalid`, `ProfileSignatureUnverified`, `EvidenceHeadCommitmentRequired`, `ProfileSignerUntrusted` |
| `opaque` | 在配置文件验证中要求已签名的帧。高隐私接收者 `kid` 值必须 (MUST) 是假名的：默认模式为 `anon:[0-9a-fA-F]{32,}`。 | `ProfileSignatureRequired`, `OpaqueRecipientKidMissing`, `OpaqueRecipientKidPublic` |
| `bundle` | 嵌套的 GTS blob 是全量读取器 (Full Reader) 的可选行为。基线读取器 (Baseline readers) 将其视为普通 blob。全量读取器 (Full Readers) 必须 (MUST) 强制执行递归和解码大小预算。 | `RecursionLimit` |
| `files` / `stream` | 现有的配置文件词汇表和可流式处理布局 (streamable-layout) 检查仍属于配置文件/工具策略，而非核心有效性。 | `ProfileVocabularyUndeclared`, `ProfileVocabularyUnused`, `StreamVocabularyWithoutLayout`, `StreamableLayoutError` |

## 嵌套 GTS 预算

全量读取器 (Full Reader) 调用方在 Python 中使用 `gts.read_nested(...)`，在 Rust 中使用 `gmeow_gts::nested::read_nested(...)`，在 Go 中使用 `nested.ReadNested(...)`，或在 TypeScript 中使用 `nested.readNested(...)` 来递归进入其声明媒体类型为 `application/vnd.blackcat.gts+cbor-seq` 的 blob。其结果通过包含该 blob 的摘要公开嵌套子图。当超过 `max_depth` / `maxDepth` 或 `max_decoded_bytes` / `maxDecodedBytes` 时，递归停止并记录 `RecursionLimit`。

## 加密推迟 (Crypto Deferrals)

| 功能 (Capability) | v1 层级决策 (v1 tier decision) |
|---|---|
| COSE_Sign1 / Ed25519 | 已实现可选的全量读取器 (Full Reader) 功能和配置文件策略 (profile-policy) 输入。 |
| COSE_Encrypt0 / AES-256-GCM | 为单个直接接收者实现了可选的全量读取器 (Full Reader) 功能。 |
| COSE_Encrypt 多接收者信封 (multi-recipient envelopes) | 推迟 (Deferred) 至 v1 一致性范围之外。在字节级向量和互操作性测试落地之前，任何引擎不得 (No engine may) 声称支持该功能。描述符契约位于 `vectors/crypto-deferred/*.json`。 |
| ECDH 密钥封装 (key-wrap) / ECDH-ES+A256KW | 推迟 (Deferred) 至 v1 一致性范围之外。未来的支持将使用 `COSE_Encrypt` 以及 `A256GCM` 内容加密、`ECDH-ES+A256KW` 接收者密钥管理和 `A256KW` 内容密钥封装。 |
| 假名接收者 ID 策略 (Pseudonymous recipient-id policy) | 已作为 `opaque` 配置文件的配置文件策略 (profile policy) 实现。 |

推迟的 (Deferred) `cose-encrypt` 故障模式在任何引擎声称支持之前必须完成修复：

- 两个或多个接收者可以 (may) 解封相同的内容加密密钥；
- 没有匹配的持有接收者密钥将记录 `MissingKey` 并保持 `reason:"missing-key"` 的不透明性 (opacity)；
- 错误的密钥、畸形的 ECDH 接收者头部或 AES-KW 解封/认证失败将记录 `KeyWrapFailed` 并保持 `reason:"missing-key"` 的不透明性 (opacity)；
- 任何故障模式均不得 (no failure mode may) 泄露明文或将部署授权转换为加密有效性。

## Vectors

已提交的安全向量描述符位于 `vectors/security/`：

- `nested-recursion-limit.json` 记录了嵌套 GTS 递归所需的负面 `RecursionLimit` 行为。`nested-recursion-limit.gts.hex` 是 TypeScript 嵌套读取器 (nested-reader) 测试使用的晋级字节 fixture。
- `profile-policy.json` 记录了信任/配置文件 (trust/profile) 调查结果，证明了加密有效性、部署信任和声明真实性是分开的。
- `nested-duplicate-digest.gts.hex` 记录了用于证明共享嵌套内容仅计费一次的重复嵌套摘要预算 (nested-digest budget) fixture。

Python、Rust、Go 和 TypeScript 单元测试在每个引擎公开相关 API 的地方直接实例化这些向量 (vectors)。一旦更多引擎从清单 (manifest) 中使用相同的 fixture 格式，跨引擎字节向量就可以晋级到顶级语料库 (corpus) 中。

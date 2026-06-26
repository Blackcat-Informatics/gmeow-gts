<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-OKF.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS OKF 配置文件

> [`docs/GTS-OKF.md`](../../../../docs/GTS-OKF.md) 的信息性中文翻译。英文文档仍然是集成、高级功能、可选 profile、基准数据、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。


Rust `okf` 特性将 OKF Markdown 捆绑包映射为一个可验证的 GTS 软件包，并将 OKF 配置文件折叠图回投影到捆绑包目录。

```bash
cargo run --features okf --bin gts -- from-okf okf-bundle/ -o bundle.gts
cargo run --features okf --bin gts -- to-okf bundle.gts --directory restored-okf/
```

GTS 段 (segment) 配置文件为 `okf`。头部元数据携带一个模式为 `gts-okf-v1` 的 OKF 清单 (manifest)、用于铸造主体的基本 IRI、文档计数以及源路径。
## 词汇表

v1 词汇命名空间为：

```text
https://blackcatinformatics.ca/projects/gts/okf#
```

默认文档基准 IRI 为：

```text
https://blackcatinformatics.ca/projects/gts/okf/doc/
```
## 映射

| OKF 结构 | GTS 表示 |
|---|---|
| `foo/bar.md` | 一个 RDF 主体节点 |
| 主体 IRI | 存在时为 `resource:`，否则为 `base-iri + percent-encoded relative path` |
| bundle path | `okf:path` 字符串字面量 |
| `type:` | 必需的 `okf:type` 字符串字面量 |
| `title:` | `okf:title` 字符串字面量 |
| `description:` | `okf:description` 字符串字面量 |
| `resource:` | `okf:resource` IRI |
| `tags:` | 重复的 `okf:tag` 字符串字面量，排序后重新发出 |
| `timestamp:` | `okf:timestamp` `xsd:dateTime` 字面量 |
| 生产者扩展标量 (producer extension scalar) | `okf:<key>` 字符串、整数、小数或布尔字面量 |
| 生产者扩展对象/数组/空值 (producer extension object/array/null) | 带有数据类型 `okf:json` 的 `okf:<key>` JSON 字面量 |
| Markdown 正文 | 携带 `blake3:<hex>` 摘要的 `okf:body` 字面量，以及一个媒体类型为 `text/markdown` 的内联 Blob |
| 内联正文变体 | `okf:body` 字符串字面量，仅在带有 `--inline-body` 时被导出接受 |
| `[text](target.md)` | 指向目标主体的 `okf:links` 边，通过 `okf:linkText` 和 `okf:linkOccurrence` 实体化 |
| 无前置内容的 `index.md` | 导航页，在导入时被忽略，并在需要时由消费者重新生成 |

正文 Blob 对于重新序列化具有权威性。链接三元组是源自正文的查询表面；`to-okf` 不会根据它们重写 Markdown。
## 目录导出

`to-okf` 拒绝现有的目标目录。成功时，它会写入：

```text
out/
├── .gts-okf/
│   └── manifest.json
├── concept-a.md
├── nested/
│   └── concept-b.md
└── _unmapped.nq
```

`_unmapped.nq` 仅当图包含 OKF 配置文件 (profile) 之外的三元组、命名图或非 OKF 具现化/注释状态时出现。这些三元组会报告到 stderr 并保留在 sidecar 中，而不是被静默丢弃。
`.gts-okf/manifest.json` 使用架构 `gts-okf-v1`：

```json
{
  "schema": "gts-okf-v1",
  "base_iri": "https://blackcatinformatics.ca/projects/gts/okf/doc/",
  "doc_count": 2,
  "source_paths": ["concept-a.md", "nested/concept-b.md"],
  "unmapped_triples": 0
}
```

GTS 报头元数据携带相同的架构名称、基础 IRI、文档数量和源路径列表，用于包内部的可验证溯源。
## Knowledge Catalog 互操作性

Rust 导入器接受 Google Knowledge Catalog 概念验证示例中使用的 OKF v0.1 形式：

- 概念文档是带有 YAML frontmatter 的 UTF-8 Markdown 文件；
- `type:` 是唯一必填的 frontmatter 键；
- `title:`、`description:`、`resource:`、`tags:`、`timestamp:` 以及任意生成器扩展键都会在 OKF 图配置文件 (profile) 中保留；
- 概念文件之间的普通 Markdown 链接变为可查询的 `okf:links` 边；
- 不带 frontmatter 的 `index.md` 文件被视为导航页面，而非概念。

已提交的一致性语料库 (corpus) 固定装置 `vectors/okf/bigquery-join/` 是封闭的 BigQuery 风格互操作性入口。它包含表/字典概念、扩展 frontmatter、相对 Markdown 链接以及 `index.md` 导航页面。实时上游入口是 Knowledge Catalog 仓库的 `okf/bundles/` 目录：

```bash
cargo run --features okf --bin gts -- from-okf vectors/okf/bigquery-join -o /tmp/bq.gts
cargo run --bin gts -- verify /tmp/bq.gts
cargo run --features okf --bin gts -- to-okf /tmp/bq.gts --directory /tmp/bq-okf
```

当针对 <https://github.com/GoogleCloudPlatform/knowledge-catalog> 的检出版本进行测试时，请使用诸如 `okf/bundles/ga4/` 之类的 bundle 替换 `vectors/okf/bigquery-join`。
## 可验证的 OKF 演示器

在创作层，OKF “只是一个目录”。GTS 配置文件 (profile) 在该目录下方添加了一个仅追加 (append-only) 的验证层：

1. 人类或代理使用 Markdown 创作 OKF 包 (bundle)。
2. `gts from-okf` 将该包封装到配置文件 (profile) `okf` 中，将 Markdown 正文存储为内容寻址的 blob，并在包元数据中记录 `gts-okf-v1` 清单 (manifest)。
3. 需要签名托管的 Rust 应用程序使用 `Writer::sign_with` 或 `Writer::sign_with_openpgp_secret_key` 创作帧 (frame)；随后 `gts verify` 检查 ID 链和 COSE Sign1 观测值。
4. 修订版本会追加新的声明和抑制帧 (suppression frame)，而不是重写旧历史。该模式与 `gmeow_gts::examples::agent_memory` 相同：追加替换声明，追加 `gmeow:wasDerivedFrom` 出处 (provenance)，并抑制被取代的术语或 blob。
5. `gts to-okf` 将当前的 OKF 配置文件折叠 (fold) 投影回 Markdown，以供审核或发布。被抑制的历史记录将保留在 GTS 包中以供审计，除非稍后的压缩策略有意对其进行封存或重写。

这使得 OKF 成为一个 GMEOW 创作界面：贡献者可以使用 Markdown 和普通的存储库审查工具进行工作，而下游系统可以将同样的知识作为已签名、可抑制、可查询的 GTS 图状态来消费。
## 往返定律

前向 OKF 往返：

```text
okf-dir -> from-okf -> package.gts -> to-okf -> okf-dir'
```

还原后的 bundle 在排除排序后的 frontmatter 键、排序后的标签以及 YAML 规范化因素后，内容是相等的。Markdown 正文字节是逐字节一致的。

反向 GTS 往返：

```text
package.gts -> to-okf -> okf-dir -> from-okf -> package.gts'
```

对于 OKF 配置文件 (profile) 图，其折叠图 (folded graph) 投影在往返后是相等的。Content ID 可能会有所不同，因为导入器 (importer) 会生成一个确定性的新段 (segment)，而不是回放源字节。
## 拒绝情况

`from-okf` 拒绝：

- 非目录形式的 bundle 根路径；
- bundle 中的符号链接；
- 除导航 `index.md` 文件外，不含 YAML 前置内容 (frontmatter) 的 Markdown 文件；
- 非映射 (mapping) 形式的前置内容；
- 缺少必需 `type:` 的文档；
- 不安全的相对路径；
- 当传递了 `--strict-links` 时，存在悬空 Markdown 链接。

`to-okf` 拒绝：

- 已存在的输出目录；
- 缺少 `okf:path` 的 OKF 主体；
- 缺少 `okf:type` 的 OKF 文档；
- 缺失或无法解码的正文 blob (body blobs)；
- 内联 `okf:body` 字面量，除非传递了 `--inline-body`。
## 与其他目录表面的关系

`gts dump --directory` 为任意 GTS 存档写入检查树。
`gts to-okf --directory` 为使用 OKF 配置文件 (profile) 词汇表的图写入 OKF 创作表面。它们有意设计为独立的目录合约：
`gts-dump-v1` 用于存档检查，而 `gts-okf-v1` 用于 Markdown 捆绑包互换。

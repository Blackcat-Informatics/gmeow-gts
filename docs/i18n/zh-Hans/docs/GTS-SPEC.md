<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-SPEC.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS — Graph Transport Substrate — 规范

> [`docs/GTS-SPEC.md`](../../../GTS-SPEC.md) 的信息性中文翻译。英文文档仍然是协议规则、线格式、符合性要求、安全注意事项、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。


**文档版本：** 0.9-draft &nbsp;·&nbsp; **传输格式主版本：** 1 &nbsp;·&nbsp;
**日期：** 2026-06-18 &nbsp;·&nbsp; **编辑：** Patrick Audley, Blackcat Informatics® Inc. &nbsp;·&nbsp;
**本版本：** <https://github.com/Blackcat-Informatics/gmeow-gts/blob/main/docs/GTS-SPEC.md> &nbsp;·&nbsp;
**DOI：** <https://doi.org/10.67342/6pta6imnmw/v1>
## 摘要

GTS (Graph Transport Substrate) 是一种面向 RDF 1.2 数据集和内容寻址二进制有效负载的本体无关二进制容器和传输格式。GTS 文件是由一个或多个仅限追加的段 (segment) 组成的 CBOR Sequence。每个段由一个确定性 CBOR 头部以及随后的、通过 BLAKE3 内容标识符链接的确定性 CBOR 帧 (frame) 组成。逻辑数据集是通过对段序列进行确定性折叠 (fold) 获得的。GTS 支持部分可读性、不透明加密或未知编解码器的帧、仅追加抑制、可选签名和加密，以及通过共享向量语料库实现的跨语言一致性。
## 本文档状态

| 字段 | 值 |
|---|---|
| 状态 | 工作草案 |
| 文档版本 | 0.9-草案 |
| 有线格式主版本 | 1，编码在段标头 `"v"` 字段中 |
| 日期 | 2026-06-18 |
| 文档 DOI | <https://doi.org/10.67342/6pta6imnmw/v1> |
| 稳定性 | 在 v1.0 之前，有线格式更改仍有可能 |
| 变更控制 | Blackcat Informatics / [GTS 治理流程](./GTS-GOVERNANCE.md) |
| 一致性 | 由本文档和版本化矢量语料库 (§19) 定义 |
| 实现版本 | 软件包版本是独立的发布制品 |
| 语料库版本 | 语料库的版本控制独立于软件包发布 |

本规范在 [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts) 存储库中维护，同时还有六个互操作参考引擎（Rust、Python、Go、TypeScript、Smalltalk/Pharo、Kotlin/JVM），它们针对共享矢量语料库进行校验。请在那里报告勘误并提议更改。核心语义更改、注册表添加以及可选标准配置文件的晋升遵循 [GTS 治理流程](./GTS-GOVERNANCE.md)。

GTS 是本体无关的。GMEOW 是 GTS 的主要下游消费者和分发用例，但 GTS 读取器和写入器不需要 GMEOW 词汇、工具或语义。特定领域的配置文件，包括 GMEOW 和音乐包配置文件，分层在核心格式之上。
## 文档历史

本节记录了此规范文档的修订历史。软件包发布、软件包版本号以及各引擎的发布说明是独立的工件，并不受文档版本的暗示。

**v0.9-draft (2026-06-18) 中的变更：**

- 将发布元数据与当前的 v1.0-rc1 准备状态对齐，同时保持软件包版本独立于规范文档版本。
- 阐明了一致性范围、读取器 (reader)/写入器 (writer) 类别、流式读取器内存边界以及规范读取器诊断。
- 正式确定了图折叠 (graph fold)、多段 (multi-segment) 值并集、空白节点作用域、RDF 1.2 三元组项与 `rdf:reifies` 映射、位置约束以及重复/冲突行为。
- 增加了可流式处理布局 (streamable-layout) 规则、可选索引/MMR 证明原像、证明验证、未知扩展键行为、媒体类型与 HTTP 服务契约，以及持久性/安全性考量。
- 扩展了向量语料库 (vector-corpus) 和清单 (manifest) 引用，以便一致性声明可以命名语料库修订版本、子集、层级、模式以及带发布时间戳的清单工件。
- 将 RDF 1.2 底层固定为 2026 年 4 月 7 日的 W3C 候选推荐快照，并说明了 GTS 导入了哪些 RDF 语义。

**早期 v0.3 文档说明：**

- 多段 (Multi-segment) 文件（`cat`-追加组合，§3.1）；段作用域的术语 ID (§7.2)；逐段折叠 (fold) 和值并集语义 (§7.5)；跨段抑制 (§11)；配置文件 (profile) 并集和逐节语言标签规则 (§13)；组合工具要求 (§14.1)；一致性向量 15–21 (§19)。
- 布局状态和可流式处理 (streamable) 声明 (§3.3, §5)；带有分离帧 (frame) 签名的可流式处理压缩 (§10.1)；`stream` 词汇表 (§13.3)；`compact` 动词 (§14.1)；一致性向量 24–26 (§19)。
## 目录
- [1. 概述与非目标](#1-overview-and-non-goals)
- [2. 术语与一致性](#2-terminology-and-conformance)
  - [2.1 一致性范围](#21-conformance-scopes)
  - [2.2 读取器 (reader) 与写入器 (writer) 一致性级别](#22-reader-and-writer-conformance-classes)
  - [2.3 基准读取器 (reader) API 形态](#23-baseline-reader-api-shape)
  - [2.4 读取器 (reader) 诊断](#24-reader-diagnostics)
- [3. 文件结构](#3-file-structure)
  - [3.1 多段 (multi-segment) 文件（`cat`-追加组合）](#31-multi-segment-files-cat-append-composition)
  - [3.2 流式传输与渐进式增强](#32-streaming-and-progressive-enhancement)
  - [3.3 布局状态：增量式与可流式处理 (streamable)](#33-layout-states-accretive-and-streamable)
- [4. CBOR 惯例](#4-cbor-conventions)
- [5. 标头](#5-header)
- [6. 帧 (Frames)](#6-frames)
  - [6.1 负载解析](#61-payload-resolution)
  - [6.2 索引帧（可选）](#62-index-frame-optional)
- [7. 图数据模型与折叠 (fold)](#7-graph-data-model-and-fold)
  - [7.1 术语（`terms` 帧）](#71-terms-terms-frame)
  - [7.2 术语 ID 分配（规范性）](#72-term-id-assignment-normative)
  - [7.3 引用三元组与重申者（`reifies` 帧）](#73-quoted-triples-and-reifiers-reifies-frame)
  - [7.4 四元组与注释](#74-quads-and-annotations)
  - [7.5 折叠 (fold) 算法（规范性）](#75-fold-algorithm-normative)
  - [7.6 不透明节点](#76-opaque-nodes)
  - [7.7 流式折叠与受限内存](#77-streaming-fold-and-bounded-memory)
  - [7.8 重复与冲突（规范性）](#78-duplicates-and-conflicts-normative)
- [8. 转换目录](#8-transform-catalog)
  - [8.1 类别](#81-classes)
  - [8.2 堆叠](#82-stacking)
  - [8.3 能力模型与平滑降级](#83-capability-model-and-graceful-degradation)
  - [8.4 强制性核心集与持久性](#84-mandatory-core-set-and-durability)
  - [8.5 规范编解码器注册表 (v1)](#85-canonical-codec-registry-v1)
- [9. 完整性与机密性](#9-integrity-and-confidentiality)
  - [9.1 逐帧自哈希与内容 ID 链（强制性）](#91-per-frame-self-hash-and-content-id-chain-mandatory)
  - [9.2 签名（可选，算法敏捷型）](#92-signatures-optional-algorithm-agile)
  - [9.3 加密（可选）](#93-encryption-optional)
  - [9.4 不透明性不变式（规范性）](#94-the-opacity-invariant-normative)
- [10. 紧凑化](#10-compaction)
  - [10.1 可流式处理紧凑化（仅限排序）](#101-streamable-compaction-ordering-only)
- [11. 抑制（增量式“删除”）](#11-suppression-additive-deletion)
- [12. 二进制与内容寻址](#12-binary-and-content-addressing)
  - [12.1 嵌套 GTS（递归组合）](#121-nested-gts-recursive-composition)
- [13. 配置文件 (Profiles)](#13-profiles)
  - [13.1 语言标签规则（配置文件级规范）](#131-language-tag-discipline-profile-level-normative)
  - [13.2 `files` 配置文件（可选标准）](#132-the-files-profile-optional-standard)
  - [13.3 `stream` 词汇表（可选标准）](#133-the-stream-vocabulary-optional-standard)
  - [13.4 领域配置文件示例：`music-package`（资料性）](#134-domain-profile-example-music-package-informative)
- [14. 导出转换](#14-transforms-out)
  - [14.1 组合工具要求（对符合规范的工具具有规范性）](#141-composition-tooling-requirements-normative-for-conformant-tools)
  - [14.2 归档工具（`files` 配置文件）](#142-archive-tooling-files-profile)
- [15. 实际案例](#15-worked-examples)
  - [15.1 最小分发快照 (`dist`)](#151-minimal-distribution-snapshot-dist)
  - [15.2 证据：图像 + 签名累积 (`evidence`)](#152-evidence-image--signed-accrual-evidence)
  - [15.3 公证员：部分不透明帧 (`opaque`)](#153-notary-partially-opaque-frame-opaque)
  - [15.4 平滑降级（`image`，内容协商）](#154-graceful-degradation-image-content-negotiation)
  - [15.5 俄罗斯套娃：密封在帧内的完整签名 GTS (`bundle` / `opaque`)](#155-matryoshka-a-whole-signed-gts-sealed-inside-a-frame-bundle--opaque)
- [16. 媒体类型与 HTTP 服务契约](#16-media-type-and-http-serving-contract)
  - [16.1 媒体类型与文件扩展名（规范性）](#161-media-type-and-file-extension-normative)
  - [16.2 文件识别算法（规范性）](#162-file-identification-algorithm-normative)
  - [16.3 HTTP 服务语义（规范性）](#163-http-serving-semantics-normative)
  - [16.4 不可变性感知缓存（规范性）](#164-immutability-aware-caching-normative)
- [17. 版本控制与持久性保证](#17-versioning-and-durability-guarantees)
- [18. 安全注意事项](#18-security-considerations)
- [19. 一致性测试向量](#19-conformance-test-vectors)
- [20. IANA 注意事项](#20-iana-considerations)
- [21. 完整 CDDL 附录](#21-complete-cddl-appendix)
  - [21.1 序列语法](#211-sequence-grammar)
  - [21.2 可复制的 CDDL](#212-copyable-cddl)
- [22. 哈希、签名与扩展键原像](#22-hash-signature-and-extension-key-preimages)
  - [22.1 原像与主体表](#221-preimage-and-subject-table)
  - [22.2 未知扩展键行为](#222-unknown-extension-key-behavior)
- [23. 参考文献](#23-references)
## 1. 概述与非目标

GTS 将图编码为 **CBOR 帧 (frames) 的仅追加日志**。逻辑图是日志的*折叠 (fold)*（重放）。增长即追加；“删除”是**抑制 (suppression)**，绝非物理移除；优化是一个独立的、显式的**有损 (lossy)** 压缩过程，它将日志重写为快照。

四个属性定义了该格式：

1. **全链路 CBOR** (RFC 8949)。一种普及的、IETF 标准化的二进制编码，具有原生字节串（无 base64 开销）、确定性编码（干净的内容哈希）以及 CBOR 序列 (CBOR Sequences) —— 即没有外层长度限制的级联数据项，因此追加成本极低。读取器 (reader) 仅需要一个 CBOR 库。
2. **持久的转换目录。** 每个帧的有效载荷都携带一条从开放且长效的目录（`identity`、`base64`、`base85`、`gzip`、`zstd`、`lzma2`、`cose-encrypt`……）中提取的*可堆叠*编解码器链。该目录将*结构持久性*（CBOR + 本规范，永久有效）与*密度和机密性*（可插拔的编解码器）分离开来。
3. **构造即完整。** 每个帧都携带一个独立的 **BLAKE3 自哈希**（内容 ID），并指明其前序帧的 ID —— 这是一个 git 风格的内容寻址链。验证是**并行**的，损坏的帧是**可独立检测的**（在索引完整的情况下，幸存部分是可恢复的，参见 §9.1），且头部 ID 传递性地提交了所有历史记录。加密签名和加密 (COSE, RFC 9052) 是可选的、分层的，且具有算法灵活性。
4. **递归组合（俄罗斯套娃）。** 有效载荷在逆转其转换后仅是字节 —— 而 GTS 文件本身也仅是字节。因此，有效载荷可以 (MAY) 本身就是一个完整的 GTS，并封装在任何转换（压缩*或*加密）中。一个完整的已签名图可以承载在加密字段内，并拥有其独立的头部、链和签名 (§12.1)。

**非目标。** GTS 不定义查询语言、读取强制要求的索引格式、推理机或变更协议。随机访问查询、深度遍历和 SPARQL 是转换目标的工作，而非 GTS 的职责。

**资料性说明（动机）。** GTS 保持了基准读取器 (reader) 表面的精简：读取器仅需要 CBOR、BLAKE3、强制性编解码器以及折叠 (fold) 规则，而不需要 RDF 文本解析器。需要更丰富查询、索引或分析的工具会将折叠后的数据投射到运行基质上，如 N-Quads、SQLite、DuckDB 或 Parquet。
## 2. 术语与一致性

本文中的关键字 **必须 (MUST)**、**不得 (MUST NOT)**、**要求 (REQUIRED)**、**应当 (SHALL)**、**应该 (SHOULD)**、**可以 (MAY)** 和 **可选 (OPTIONAL)** 按照 BCP 14 (RFC 2119, RFC 8174) 中的描述进行解释。

- **Log** — GTS 文件中帧的有序列。
- **Frame** — 日志中的一个 CBOR 数据项 (§6)。
- **Fold** — 将日志确定性地回放到图状态的过程 (§7.5)。
- **Term** — 一个具有稳定整数 ID 的 RDF 项 (IRI、字面量、空白节点或引用三元组)。
- **Reifier** — 一个表示引用三元组的项，承载语句级元数据 (RDF 1.2)。
- **Capability** — 读取器解码有效负载必须 (MUST) 持有的内容：*编解码器库*或*密钥*。
- **Opaque node** — 读取器无法解码的帧在图中的表示 (§7.6)。
### 2.1 一致性范围 (Conformance scopes)

本规范区分以下一致性范围：

- **线路格式一致性 (Wire-format conformance)** 涵盖字节级 CBOR Sequence 结构、确定性 CBOR 编码、标头和帧 (frame) 语法、内容 ID (content-id) 原像以及段 (segment) 边界。
- **读取器一致性 (Reader conformance)** 涵盖解析、链验证、负载解析、折叠 (fold) 行为、诊断、不透明节点 (opaque-node) 处理以及资源限制行为。
- **写入器一致性 (Writer conformance)** 涵盖确定性输出、有效的标头和帧 (frame)、正确的内容标识符、编解码器 (codec) 声明以及签名/哈希原像。
- **工具一致性 (Tool conformance)** 涵盖比本地文件有效性更严格的命令行或库策略，例如验证组合、提取、发布或归档操作。
- **配置文件一致性 (Profile conformance)** 涵盖在核心格式之上分层的特定于配置文件 (profile) 的词汇、验证、功能和信任规则。
- **部署一致性 (Deployment conformance)** 涵盖服务和分发行为，例如媒体类型、缓存、范围请求以及跨 HTTP 或构件 (artifact) 托管的字节保留。

下文的一致性类别定义了读取器 (reader) 和写入器 (writer) 行为。工具、配置文件 (profile) 和部署要求在定义它们的章节中明确限定范围。

基准读取器/写入器 (reader/writer) 一致性独立于配置文件 (profile) 验证、CLI 动词、转换目标和 HTTP 部署行为。当本地有效的 GTS 文件声明了不支持的配置文件 (profile) 时，它仍然保持本地有效；读取器 (reader) 记录配置文件声明并根据其读取器类别对字节进行折叠 (fold)，而识别配置文件 (profile-aware) 的工具可以 (MAY) 在配置文件一致性范围内应用额外的检查。

配置文件 (Profiles)、工具和部署不得 (MUST NOT) 更改标头或帧 (frame) 语法、段 (segment) 边界检测、内容 ID (content-id) 或签名/哈希原像、转换目录 (transform-catalog) 解析或 §7 中的核心折叠 (fold) 语义。更严格的配置文件 (profile) 可以 (MAY) 仅将原本有效的构件 (artifact) 作为配置文件级验证失败而拒绝，而不是通过重新定义核心 GTS 有效性来拒绝。
### 2.2 读取器和写入器一致性类

- 一个 **Baseline Reader** 必须 (MUST)：解析 CBOR 序列；验证 id/prev 链 (§9.1)；折叠 `terms`、`quads`、`reifies`、`annot`、`blob`、`suppress`、`meta` 和 `snapshot` 帧；支持 `identity`、`gzip` 和 `zstd` 编解码器；并将任何无法解码的帧呈现为不透明节点 (§7.6)。它可以 (MAY) 忽略签名和加密。
- 一个 **Streaming Reader** 是一种 Baseline Reader，它一次处理一个帧并发送到接收端 (sink)，且 **不实例化整个图**：它仅维护术语字典（以及正在进行的链检查），加上最大已解码帧大小和验证旁车 (sidecar) 状态，其保留内存复杂度为 O(distinct terms + maximum decoded frame size + validation sidecar state)，而不是 O(triples + blobs) (§7.7)。当通过非实例化接收端实现时，`gts → duckdb`/`sqlite` 转换 (§14) 具有 Streaming Reader 的形态。
- 一个 **Full Reader** 还会额外验证 COSE 签名，解密其持有密钥的 COSE 加密帧，可以 (MAY) 递归进入嵌套的 GTS blob (§12.1)，并且可以 (MAY) 使用可选的索引帧 (§6.2) 进行并行验证和随机访问。
- 一个 **Writer** 必须 (MUST) 对任何经过哈希或签名的字节发出确定性 CBOR (§4)，并且必须 (MUST) 计算每个帧的 `"id"` 自哈希，并将 `"prev"` 设置为前一项的 `"id"`。
### 2.3 基准读取器 API 形态

基准读取器 (Baseline Reader) 应该 (SHOULD) 至少暴露：

```text
open(bytes|path)            -> Graph          # parse + verify chain + fold
Graph.quads()               -> iterator[(s,p,o,g)]   # term ids resolved to terms
Graph.term(id)              -> Term
Graph.annotations(reifier)  -> iterator[(prop, value)]
Graph.blob(digest)          -> bytes | OpaqueRef
Graph.opaque()              -> iterator[OpaqueNode]
Graph.to_nquads(out)        # §14
```

此 API 形态特意保持精简：它暴露了折叠表 (folded tables)、诊断信息和通用的投影路径，而无需 RDF 文本解析器、前缀解析器、查询引擎或推理机。跨语言 API 和 CLI 的对等合约 (parity contract) 维护在 [`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md) 中。
### 2.4 读取器诊断

读取器通过这些规范类别公开机器可观察的诊断信息（实现 可以 (MAY) 将其映射为错误返回或结构化警告）：

| 类别 | 含义 |
|---|---|
| `EmptyFile` | 空字节流或缺少段标头；返回空结果并带有致命诊断，而不是中止 (§3) |
| `TornAppendError` | 文件末尾 (EOF) 存在尾随的不完整 CBOR 项 (§3) |
| `DamagedFrame` | 自我 `"id"` 不匹配 / 无效帧哈希（内容损坏）；不透明 `reason:"damaged"` (§7.6) |
| `BrokenChain` | 帧哈希有效，但 `"prev"` ≠ 前一项的 `"id"`（插入 / 重新排序 / 拼接） (§9.1) |
| `TruncatedLog` | 存在头承诺 (head commitment)，但观察到的头不符 (§9, §18) |
| `UnknownCodec` | 转换指定了读取器缺少的编解码器；不透明 `reason:"unknown-codec"` |
| `MissingKey` | 读取器无法解密的 `encrypt` 编解码器；不透明 `reason:"missing-key"` |
| `KeyWrapFailed` | 延迟的多接收者密钥解封失败；不透明 `reason:"missing-key"` |
| `ConflictingReifier` | 具体化器 (reifier) 重新绑定到不同的三元组 (§7.8) |
| `PositionConstraint` | 术语出现在非法的主体、谓词、对象或图名称位置；拒绝/诊断违规行 (§7.4) |
| `ForwardReference` | 术语 ID (term-id) 引用指向了同一段中先前帧未引入的术语 (§7.2, §7.5) |
| `SegmentBoundary` | 兼容性读取器到达较后的段标头，在该处文件全局术语 ID 会发生错误折叠；停止并给出致命诊断 (§3.1, §19) |
| `IllTypedLiteral` | 已识别的 XSD 数据类型字面量具有无效的词法形式；逐字保留该字面量并公开诊断/元数据标志 (§7.1) |
| `RecursionLimit` | 嵌套 GTS 深度或解码大小预算超限 (§12.1, §18) |
| `StreamableLayoutError` | 段声称 `"layout": "streamable"` 但其覆盖区域违反了交付顺序，或者其索引页脚缺失或与其覆盖的帧矛盾 (§3.3) |
| `IndexMmrError` | 存在可选的 `index.mmr` 根，但与所覆盖的帧 ID 不匹配 (§6.2) |
| `UnknownFrameType` | 读取器/配置文件无法理解帧类型；保留链验证，并将其忽略或将其作为不透明内容公开，直到有配置文件处理它 (§7.8) |
## 3. 文件结构

一个 GTS 文件是一个 **CBOR 序列 (CBOR Sequence)** (RFC 8742)：项之间零分帧字节，每个项都是一个格式良好的 CBOR 数据项。每个段 (segment) 可以 (MAY) 以 CBOR 自描述标签 `55799` (`0xd9 0xd9 0xf7`) 开始，作为该段的魔数 (magic number)。如果存在，标签 `55799` 必须 (MUST) 标记段 **头 (Header)** 数据项；它不是一个单独的日志项，没有 `"id"`，也不参与 id/prev 链。GTS 文件不得 (MUST NOT) 将整个序列包装在外部 CBOR 项中。

```text
GTS-file = segment *segment
segment  = [self-describe-tag] header *frame
```

- 一个段的 **第一个** 数据项必须 (MUST) 是 **头 (Header)** (§5)。
- 段的每个后续数据项都是一个 **帧 (Frame)** (§6)，按日志顺序排列，直到下一个头 (Header)（其开始一个新段）或输入结束。
- **追加 (Append)** = 连接一个更多的帧（扩展最后一个段），或连接一个完整的后续段 (§3.1)。不存储长度前缀或计数，因此写入器 (writer) 永远不会重写之前的字节。
### 3.1 多段文件 (`cat` 追加组合)

一个 GTS 文件由一个或多个**段 (segments)**组成，每一段都是一个完整且自包含的 `header *frame` 日志。其定义属性为：**有效 GTS 文件的字节级拼接仍是一个有效的 GTS 文件** ——

```sh
cat music.gts >> core.gts        # core.gts is now a valid two-segment GTS
```

- **边界检测（规范性）。** 一个已消耗至少一个帧 (frame) 的读取器 (reader) 在遇到包含键 `"gts"` 且缺少键 `"t"` 的映射数据项时，必须 (MUST) 将其视为**新段**的标头 (Header)（可选的自描述标签 `55799` 可以 (MAY) 标记该标头；写入器 (writers) 应该 (SHOULD) 在每个段标头上发出该标签，以使边界易于被人类识别）。该标签附着在段标头上，因此独立有效的带标签段的字节拼接仍然是 CBOR Sequence 项的字节拼接，而不是嵌套的全文件封装器。任何其他非帧项仍属于格式错误的输入 (§17)。
- **独立完整性。** 每个段都有自己的起源（其标头 `"id"`）、自己的 id/prev 链、自己的签名以及自己可选的 `index`（索引仅覆盖其所属的段）。该文件的复合身份是**段首 id 的有序列表**。第三方段携带其自己的签名者；拼接不会重写任何内容（按照设计，`cat` 无法在不破坏其自身哈希的情况下重写较早段的标头）。
- **跨段身份。** 术语 ID (Term-ids) 是**段作用域的** (§7.2)；唯一的跨段身份是术语**值 (value)**（IRI、字面量、引用三元组结构）。空白节点标签是段本地的，且不得 (MUST NOT) 跨段合并（应用在顶层的 §12.1 嵌套 GTS 规则）。
- **配置文件并集。** 文件的有效配置文件/需求集是各段标头的 `"prof"` 值（以及段元数据中携带的任何配置文件需求）的并集。如果读取器 (reader) 缺少段所要求的功能，则会将该段的帧 (frames) 降级为不透明节点 (opaque nodes) (§7.6) —— “此数据需要 gmeow-music 配置文件”是一次标头读取，而非错误。
- **与嵌套的关系。** 嵌套 GTS (§12.1) 通过*包含*（一个密封的、可独立运输的子图）进行组合；段通过*拼接*（开放的、无需工具的聚合）进行组合。两者都产生并集折叠 (union fold)；当部件必须独立移动或密封时选择嵌套，当普通的 `cat` 必须工作时选择分段。
### 3.2 流式传输与渐进式增强

仅追加日志使得流式传输成为**格式的一项属性**，而非工具的功能。
三项事实构成了这一属性，符合规范的实现必须 (MUST) 保留这三者：

- **前缀折叠有效性（规范性）。** 任何在数据项边界结束的有效 GTS 文件的字节前缀，其本身也是一个有效的 GTS 文件，且读取器 (reader) 必须 (MUST) 将其折叠 (fold) 到与在完整文件中折叠这些相同项时完全一致的状态。传输中的实时流因此与带有断裂追加的文件（§3）*不可区分*：部分结尾项意味着“尚未到达”，且消费者可以 (MAY) 在字节落地时继续读取（`tail -f` 语义）——每一个中间折叠都是一个真实、可用的图状态，绝非解析一半的错误状态。
- **单调精化。** 追加的帧 (frame) 只会*增加*知识：quads 累加（§7.8 集合语义），具体化器绑定采用首胜制，因此已建立的渲染在其下永远不会改变，而抑制是一种加性显示覆盖（§11）——`suppress` 帧的到达会精化展示效果，而不会使之前的任何折叠失效。链校验同样是增量的：O(1) 状态（预期的 `"prev"`）在每帧到达时对其进行验证。
- **分块安全帧。** CBOR Sequence 项是自定界的，因此项边界对于中继和代理而言是安全的分块重组点，且恢复是基于内容寻址的：声明其已验证的最后一帧 `"id"` 的接收者可以从下一个字节开始恢复，除了该哈希值之外无需任何协商。

**渐进式增强。** 生成者应该 (SHOULD) 按最重要信息优先的顺序排列内容，使早期前缀尽可能有用：在段 (segment) 内，`terms`/`quads`（图）应位于笨重的 `blob` 帧之前，且小型或预览表现形式应位于大型形式之前；在整个文件中，段 (segment) 就是增强层——基础段（核心图 + 缩略图）后跟增强段（全分辨率 blob、计算投影），这使得接收者在每个段边界都能获得一个完整的、可验证的包，§3.1 的组合规则被应用为交付调度。
**检查点 (checkpoint) `index` 帧**（§6.2）定期发出，为流式消费者提供了中间截断锚点（`"head"`）、用于范围重新获取的随机访问偏移量，以及已到达内容的清单；索引始终只是加速器，而非依赖项（§3, §6.2）。

**清单即是图。** GTS 不需要目录结构，因为*描述*内容的帧可以先于*承载*内容的帧：生成者应该 (SHOULD) 在发出其承诺字节的 `blob` 帧之前，发出命名每个即将到来的表现形式的 quads——包括其内容摘要、媒体类型、大小、角色。早期前缀的折叠随后将交付调度作为普通知识包含在内：图中命名的但流尚未交付的每个摘要都是一个基于内容寻址的欠条 (IOU)，因此“在此停止”、“跳过”和“仅范围获取 RAW 文件”都是消费者在已有信息的基础上做出的决策，是针对可验证目录而非猜测做出的决策。（从未在此文件中到达的 blob 仅被视为外部 blob，§12——引用会优雅地降级为“字节存在于他处”。）

*交付调度示例*——作为渐进式流的照片；消费者可以在任何项边界停止，并获得其停止点之前所有内容的完整且经过验证的包：

```text
header                          profile, codec catalog
terms/quads                     the catalog: Work + every manifestation below,
                                each with digest, mt, size, role (the IOUs)
blob  image/webp        ~20 KB  thumbnail — first paint
blob  image/jxl         ~8 MB   full-resolution render
terms/quads                     scene description (what is IN the image)
blob  image/x-raw       ~80 MB  RAW sensor dump
meta/quads                      full camera metadata
terms/quads/annot               AI analysis as RDF, statement-level provenance
terms/quads/annot               opinions — standpoint-qualified claims
terms/quads                     processing-pipeline provenance
index                           footer: offsets, head anchor, MMR (§6.2)
```

普通查看者在缩略图后停止；档案保管员获取所有内容；编辑在仅阅读目录后通过摘要范围获取 RAW。相同的字节，相同的链，三个消费者。
读取器 (reader) 持续流式处理项，直到输入结束。尾部残缺字节（损坏的追加 (torn append)）必须 (MUST) 被检测并忽略，同时提供诊断信息：读取器尝试解码每个连续的 CBOR 项，如果解码器在文件末尾提示项不完整或出现非预期的 EOF，它必须 (MUST) 将尾部字节视为损坏的追加 (torn append)，忽略该不完整的项，并提供机器可观察的诊断信息（例如 `TornAppendError` 警告）。特别是，如果在写入 `index` 帧 (§6.2) 时发生崩溃，则尾部索引是损坏的：读取器 (reader) 必须 (MUST) 忽略它，并回退到更早的完整 `index` 或简单的**顺序扫描 (sequential scan)**，从而使每个幸存的帧 (frame) 都保持可恢复状态。可选索引是一种加速器，绝非依赖项。

上述所有属性对任何帧 (frame) 顺序均成立；生成器*选择*何种顺序是一个独立的、有名称的关注点：一个段 (segment) 处于两种**布局状态 (layout states)** 之一 —— **增量式 (accretive)**（追加排序）或**可流式处理布局 (streamable)**（交付排序）—— 下文 (§3.3) 将对其进行定义。
### 3.3 布局状态：增量式与可流式处理

一个 GTS 段（segment）始终有效且始终支持前缀折叠（§3.2），但它处于以下两种布局状态之一：

- **增量式（Accretive）** —— 针对追加进行了优化。帧（Frames）按到达顺序排列（实时捕获、代理内存增长、证据积累）。写入成本始终较低，且流在到达时即可使用，但重要性并未前置，且目录（catalog）可能滞后于它所描述的字节。这是默认状态；无需声明。
- **可流式处理（Streamable）** —— 按交付顺序排列。目录**预示**了有效载荷：在每个 `blob` 帧之前，都有一个**前置流式索引**（`stream` 词汇表中的普通 `terms`/`quads` 帧，§13.3 —— 每个承诺的 blob 对应一个 `stream:Manifestation`，携带摘要、媒体类型、大小、角色和预期顺序），blob 按重要性降序排列，最后由一个尾随偏移量 `index`（§6.2）作为随机访问页脚（footer）结束覆盖区域。

追加友好和流式优化是**同一内容的两种不同布局**（先例：mp4 `faststart`、zip 中央目录重写、LSM 压缩）。一次性写入器（one-pass writer）无法生成第二种状态，因此转换是一种显式的重写 —— 即**可流式处理压缩**（§10.1），公开为 `gts compact --streamable`（§14.1）。

**声明（规范性）。** 段通过可选的标头键 `"layout": "streamable"`（§5）声明其处于可流式处理状态。该声明是针对每个段的（每个段都有自己的标头，§3.1），且具有防篡改特性（标头自哈希涵盖了它）。从 §14.1 的意义上讲，可流式处理性是一种**“已声明与已计算”的声明** —— 拒绝且不信任：

- 已声明段的**覆盖区域**是由该段**最后一个完整的 `index` 帧**限定的前缀：即 `"count"` 帧，结束于其 `"id"` 等于索引中 `"head"` 的那个帧。页脚必须 (MUST) 紧跟在其覆盖的帧之后（`"count"` = 索引自身的帧位置 − 1）—— 否则帧可能会位于覆盖的前缀和页脚之间，既不被计为覆盖区域，也不被计为增量尾部。一个声明了该状态且没有完整 `index` 帧的段，或者其最后的索引未紧邻其覆盖的前缀，或者其 `"head"` 不等于帧 `"count"` 的 ID，则属于违规。
- 在覆盖区域内，每个内联 `blob` 帧必须 (MUST) 由一个通过 `stream:digest`（§13.3）描述其摘要的 `quads` 帧引导 —— 即目录先于有效载荷。在描述之前交付的覆盖 blob 属于违规。
- 遇到违规的读取器必须 (MUST) 提供一个 **`StreamableLayoutError`** 诊断（§2.3）；验证工具将其视为错误（§14.1）。该声明绝不会因字节的变化而失效。

**压缩后的追加是合法且可折叠的。** 最后一个 `index` 之后的帧仅仅是**未经预示的**：它们是段的**增量尾部**，不承担排序义务，也不会触发诊断。该段随后处于“在帧 *N* 之前可流式处理，之后为增量式”的状态 —— 工具应该 (SHOULD) 报告该边界（§14.1）。重新压缩以重新实现流式优化。同样，由 `cat` 追加的段除非其自身的标头进行了声明，否则不作任何声明。

**传输中的前缀。** 在尾随的 `index` 之前截断的可流式处理段的前缀，从构造上讲，具有声明但尚无页脚；流式消费者在输入可能仍在到达时，不得 (MUST NOT) 将缺失页脚视为虚假声明 —— 缺失页脚违规仅适用于**完整**文件。相比之下，目录先于有效载荷的规则是前缀稳定的：在任何前缀中观察到的违规都是整个文件的违规。
## 4. CBOR 约定

- 映射使用**短文本字符串键**（例如 `"t"`、`"d"`）以实现自描述和可视化调试；紧凑性是转换层的工作，而不是模式的工作。
- 任何**经过哈希或签名**的字节**必须 (MUST)**使用**确定性编码 (Deterministic Encoding)** (RFC 8949 §4.2)：最短形式整数、定长项，以及**按其编码形式逐字节排序**的映射键——明确遵循 RFC 8949 规则，而不是 (NOT) RFC 7049 的长度优先规范排序。（对于 GTS 自身使用的短文本键，两者是一致的，因为 CBOR 文本字符串的初始字节嵌入了其长度；规则在混合类型键上会产生分歧，因此实现在未检查其实现的排序方式之前，**不得 (MUST NOT)** 依赖 CBOR 库旧有的“规范”模式。）
- 所有 ID 均使用无符号整数。BLAKE3 摘要为 32 字节（256 位）字节字符串。
- 短语法片段以 **CDDL** (RFC 8610) 给出。完整的可复制 CDDL 附录见 §21，规范原像规则见 §22。

```cddl
term-id      = uint            ; append-order, frozen (§7.2)
digest       = bstr .size 32   ; BLAKE3-256
content-id   = digest          ; a frame's self-hash (§9.1)
digest-ref   = digest / tstr    ; raw digest or "blake3:<hex>" text (§21.2)
codec-id     = uint            ; index into the header codec catalog (§8)
```
## 5. Header

Header 是第一个数据项，也是链的创世项；它不是帧 (frame)（它没有 `"prev"`）。

```cddl
header = {
  "gts"  : "GTS1",                    ; magic / format id
  "v"    : uint,                      ; spec major version (1)
  "prof" : tstr,                      ; profile (§13); "generic" if unspecified
  "cat"  : { * codec-id => codec },   ; the transform catalog (§8)
  ? "layout": tstr,                   ; layout-state claim (§3.3); absent = accretive
  ? "dct": { * tstr => bstr },        ; named, UNCOMPRESSED dictionaries for dict-codecs
  ? "meta": any,                      ; free-form, non-normative metadata
  "id"   : content-id,                ; self-hash of the header content (the chain genesis)
}

codec = {
  "name" : tstr,                      ; "identity" | "gzip" | "zstd" | "lzma2" | "cose-encrypt" | ...
  "cls"  : "encode" / "compress" / "encrypt",
  ? "dct": tstr,                      ; references header "dct" key (dict codecs)
  ? "p"  : any,                       ; codec parameters (e.g. lzma2 level)
}
```

目录在**文件内是封闭的**（帧只能引用 Header 声明的 codec-id），但在**整个生态系统中是开放的**（新的编解码器可以通过名称注册）。Header 携带其自身的 `"id"`（其内容的自哈希），且没有 `"prev"` —— 它是创世项，第一帧的 `"prev"` 是 Header 的 `"id"`。Header 的 `"id"` 必须 (MUST) 等于 Header map 的确定性 CBOR 的 BLAKE3-256 哈希值，且**排除 `"id"` 键**；所有其他键（包括 `"meta"` 和未知的扩展键）都参与计算。§22 中的原像表 (preimage table) 是哈希和签名字节的唯一事实来源 (single source of truth)。可选的 `"layout"` 键声明了一个布局状态 (§3.3)：此版本定义的唯一值是 `"streamable"`，验证读取器 (reader) 必须 (MUST) 根据段 (segment) 的实际布局对其进行检查；读取器必须 (MUST) 忽略未知的 `"layout"` 值（前向兼容性 —— 未知状态不强制执行检查）。字典以**未压缩且带内 (in-band)** 的方式存储 —— 不存在外部字典依赖。编解码器的 `"dct"` 值必须 (MUST) 与 Header `"dct"` map 中的一个键匹配，且该编解码器必须 (MUST) 使用相应的字节串作为其压缩/编码字典。
## 6. 帧

所有帧共享一个信封：

```cddl
frame = {
  "t"   : frame-type,        ; discriminator
  ? "x" : [+ codec-id],      ; transform chain, applied in order on encode; default [identity]
  ? "pub": any,              ; CLEARTEXT public envelope (always readable; §9.4)
  ? "to": [+ recipient],     ; recipients, for encrypt-class chains
  ? "d" : bstr / any,        ; payload: bstr when "x" transforms it; structured CBOR otherwise
  "prev": content-id,        ; the PREVIOUS data item's "id" (chain link; §9.1)
  "id"  : content-id,        ; BLAKE3-256 self-hash of this frame's CONTENT (all keys but "id"/"sig")
    ? "sig": bstr,           ; COSE_Sign1 over "id" (§9.2)
}

frame-type = "terms" / "quads" / "reifies" / "annot" / "blob" / "suppress"
/ "snapshot" / "meta" / "index" / "opaque"

recipient = { "kid": tstr, ? "alg": tstr, * tstr => any }   ; key identifier; never the key
```

每个帧的 `"id"` 必须 (MUST) 等于其内容的确定性 CBOR 的 BLAKE3-256（除 `"id"` 和 `"sig"` 之外的每个键；未知的扩展键也参与其中）。每个帧的 `"prev"` 必须 (MUST) 等于前一个数据项的 `"id"`；**第一帧**的 `"prev"` 是标头的 `"id"`。
由于 `"prev"` 位于哈希内容中，每个 `"id"` 都会传递地提交到所有先前的帧 (§9.1)。§22 集中规定了完整的前向映像和主体规则。
### 6.1 负载解析

要获取帧的逻辑负载：

1. 如果 `"x"` 缺失，负载直接为 `"d"`（结构化 CBOR）——这等同于单次 `identity` 变换；解析结果仅为 `identity` 的链同样使 `"d"` 保持不变。
2. 如果 `"x"` 存在，`"d"` 必须 (MUST) 是一个字节串，且每个 codec-id 必须 (MUST) 通过标头 `"cat"` 进行解析；按从后到前的顺序应用每个编解码器的**逆向**操作。每一步都需要一种**能力 (capability)** (§8.3)。若缺少任何能力（未知的编解码器或缺失的密钥），请停止处理并将该帧视为**不透明 (opaque)** (§7.6)。
3. 完全解码后的字节是一个 CBOR 项；将其解码为特定类型的结构 (§7)。
### 6.2 索引帧 (可选)

写入器可以 (MAY) 追加一个 `index` 帧 —— 这是一个页脚，用于加速大文件处理，且不会提高简单读取器的门槛（基准读取器会忽略它）。由于日志是只增的，在更多帧之后可以 (MAY) 追加一个新的 `index`；**最后**一个 `index` 胜出。

```cddl
index-payload = {
  "count"  : uint,                        ; frames covered
  "head"   : content-id,                  ; "id" of the last covered frame (truncation anchor)
  ? "off"  : [+ uint],                    ; byte offset of each frame (random access; parallel verify)
  ? "ti"   : { * frame-type => [+ uint] },; frame indices by type
  ? "dict" : [+ uint],                    ; indices of "terms" frames (dictionary locator; §7.7)
  ? "mmr"  : content-id,                  ; Merkle-Mountain-Range root over frame ids (§9.1)
}
```

给定 `"off"`，全量读取器会将帧哈希验证分派到多个线程并寻址到任何帧；给定 `"dict"`，流式读取器仅加载字典 (§7.7)；给定 `"head"`/`"mmr"`，它可以检测截断并生成 O(log n) 的包含证明。**检查点**索引仅仅是定期发出而非仅作为页脚发出的 `index`；尽管为了加速首选最后一个完整的 `index`，但较早的 `index` 仍可以 (MAY) 作为恢复锚点。当前对 `off`/`ti`、`dict`、`mmr`、证明谓词、范围获取和复制工作流的包支持与延迟情况在 [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) 中跟踪。

`mmr` 根对索引覆盖的有序列帧 ID 使用了 Merkle-Mountain-Range。索引帧本身不被其自身的 `mmr` 覆盖；较晚的索引可以像任何其他较早的帧一样覆盖较早的索引帧。峰是通过为每个帧 ID 追加一个叶子并重复合并两个最右侧的高度匹配的峰，从左到右构建的。`count = 0` 的根是具有空峰列表的根原像。

所有 MMR 原像均使用确定性 CBOR (§4) 和 BLAKE3-256：

```text
leaf(index, frame_id) =
  BLAKE3-256(deterministic-CBOR(["gts-mmr-leaf-v1", index, frame_id]))

parent(parent_height, left_hash, right_hash) =
  BLAKE3-256(deterministic-CBOR(["gts-mmr-parent-v1",
                                 parent_height, left_hash, right_hash]))

root(count, peaks) =
  BLAKE3-256(deterministic-CBOR(["gts-mmr-root-v1",
                                 count,
                                 [[peak_height, peak_hash], ...]]))
```

分离的证明 JSON 对象具有以下稳定形状：

```json
{
  "schema": "gts-mmr-proof-v1",
  "hash": "blake3-256",
  "preimage": "gts-mmr-v1",
  "count": 4,
  "leaf_index": 2,
  "frame_id": "<32-byte frame id hex>",
  "root": "<32-byte mmr root hex>",
  "peak_index": 0,
  "peaks": [{"height": 2, "hash": "<32-byte peak hex>"}],
  "path": [
    {"side": "right", "parent_height": 1, "hash": "<32-byte sibling hex>"}
  ]
}
```

`verify-proof` 必须 (MUST) 拒绝证明，除非峰高度匹配 `count`，`leaf_index` 属于 `peak_index`，每个哈希字段均为 32 字节，路径重构了所选峰，且这些峰重构了 `root`。证明验证不需要原始的 `.gts` 文件。
## 7. 图数据模型与折叠

折叠（Fold）是将有序帧（frame）日志确定性地投影为按值索引的图状态。项 ID（Term id）是读取段（segment）时使用的本地压缩产物；公开的文件图由下文的值并集（value-union）规则定义。

**折叠状态模型（规范性）。** 读取器（reader）将每个段（segment）折叠为以下逻辑状态：

- `terms`：段本地项值的有序向量。
- `quads`：项值上的一组断言 RDF 四元组（quad）。
- `reifiers`：从具体化（reifier）项值到恰好一个引用三元组（quoted triple）值的部分映射。
- `annotations`：项值上 `(reifier, predicate, value)` 行的有序多重集。
- `blobs`：从 BLAKE3 摘要到内联字节或外部内容引用的内容寻址映射。
- `blob_meta`：每个 blob 摘要的浅层元数据映射，由 blob `"pub"` 映射构建。
- `meta`：浅层段元数据映射，加上文件级浅层合并（§7.5）。
- `suppressions`：显示/优先级指令的有序列表（§11）。
- `opaque`：在没有解码负载语义的情况下，有意或必然携带的帧（frame）有序列表（§7.6）。
- `signatures` 和 `diagnostics`：读取器关于帧（frame）的有序观测结果。它们是折叠 GTS 状态的一部分，但不属于 RDF 数据集。
- `segment_heads`、`segment_profiles`、`segment_meta` 以及段布局状态：为保留 cat-append 标识、配置文件（profile）要求、每段元数据和可流式处理布局（streamable-layout）声明所需的有序段账本。

**文件折叠（规范性）。** 文件折叠是其段（segment）折叠的有序值并集：各段按文件顺序处理；每个项 ID 首先在其所属段内解析；然后根据 §7.5 和 §7.8 中的规则合并四元组、具体化绑定、注释、blob 声明、元数据、抑制、不透明节点（opaque node）、签名、诊断和段账本。该并集不得 (MUST NOT) 跨段比较原始项 ID。跨段标识始终是值标识。

**RDF 基底（规范性）。** GTS 引入了日期为 2026 年 4 月 7 日的 RDF 1.2 概念与抽象数据模型候选推荐快照（Candidate Recommendation Snapshot），用于 IRI、空白节点、字面量、RDF 数据集、三元组项、版本标签 `"1.2"` 和 `rdf:reifies`（§23.1）。除非后续 GTS 主版本更新此引用，否则 GTS 将在第 1 个主版本中冻结该基底。基准读取器（Baseline Reader）不需要实现 RDF 解析器、查询语言、蕴涵政体（entailment regime）、规范化算法或 RDF 1.2 具体语法；它只需要本文定义的项、四元组、具体化、注释和数据集映射。RDF 语义蕴涵政体不属于核心 GTS 折叠的一部分，除非配置文件（profile）或投影明确在传输层之上应用它们。

**值相等性（规范性）。** 折叠按如下方式比较值：
| 值类型 | 相等规则 |
|---|---|
| IRI | CBOR 解码后精确的 Unicode 字符串相等。核心 GTS 不应用百分比、大小写、Unicode、base-IRI 或前缀规范化。 |
| Literal | 相同的词法字符串，默认化（§7.1）后相同的数据类型 IRI 值，存在时相同的语言标签字符串，以及存在时相同的 RDF 1.2 基础方向。不应用数据类型词法规范化；`"01"^^xsd:int` 与 `"1"^^xsd:int` 是不同的传输值。 |
| Language tag | 核心 GTS 中精确的字符串相等。配置文件 (Profiles) 和投影 (projections) 可以 (MAY) 应用语言范围匹配、首选 BCP 47 大小写或公有/私有标签转换（§13.1），但这不属于项标识。 |
| Datatype | 数据类型 IRI 值的相等，而非命名它的本地项 ID。 |
| Blank node | 相等性受限于空白节点作用域加上非空标签。来自不同段 (segments) 或嵌套 GTS 文件的空白节点永远不相等。缺失或为空 `"v"` 的空白节点是匿名的：其作用域内的每个项条目都是一个独立的空白节点。 |
| Quoted triple term | 引用三元组解析后的主语、谓语和宾语项值的相等。仅引用并不代表断言该三元组（§7.3）。 |
| Graph name | 图名称项值的相等。缺失的图插槽是默认图，且不与任何命名图相等。 |
| Blob | 规范化 BLAKE3 摘要（`blake3:<hex>` 或规范化为该形式的原始摘要字节）的相等；内联字节的相等性由摘要证明。 |
| Opaque node | 不透明节点出现的相等性是指其段标识加上帧 (frame) 内容 ID。显示层可以 (MAY) 合并完全重复的呈现方式，但折叠 (fold) 会保留出现顺序。 |
| Metadata | 映射键按精确字符串比较；值按确定性 CBOR 数据模型等价性比较。文件级视图是浅层的“后者胜出”合并，而每段 (per-segment) 的原始数据仍可被寻址（§7.5）。 |
| Suppression | 抑制目标首先在其所在的段 (segment) 中解析，然后按值应用于文件并集（§11）。重复的指令具有幂等的显示效果，但作为有序的折叠 (fold) 状态被保留。 |
### 7.1 项 (`terms` 帧)

载荷：项的**有序数组**。ID 按在当前段字典（或在移入外层段之前的 `snapshot` 字典）中的追加顺序分配。

```cddl
terms-payload = [+ term]
term = {
  "k"   : 0 / 1 / 2 / 3,   ; 0=IRI 1=literal 2=bnode 3=quoted-triple
  ? "v" : tstr,            ; IRI string | literal lexical form | bnode label
  ? "dt": term-id,         ; literal datatype IRI (a term)
  ? "l" : tstr,            ; literal language tag (BCP 47)
  ? "dir": "ltr" / "rtl",  ; RDF 1.2 base direction for language-tagged literals
  ? "rf": term-id,         ; quoted-triple: the reifier (§7.3) whose triple this term denotes
}
```

**字面量数据类型默认值（规范性）。** 对于 `k:1`（字面量）项：如果 `"l"`（语言标签）和 `"dir"` 存在且 `"dt"` 缺失，则数据类型为 `rdf:dirLangString`；如果 `"l"` 存在、`"dir"` 缺失且 `"dt"` 缺失，则数据类型为 `rdf:langString`；如果 `"l"` 和 `"dt"` 均缺失，则数据类型为 `xsd:string`。`"dir"` 的值必须 (MUST) 为 `"ltr"` 或 `"rtl"`，且若没有 `"l"` 则没有意义。

**空白节点标签（规范性）。** `k:2`（空白节点）项的非空 `"v"` 标签在当前空白节点作用域内是局部的：普通帧对应的段、`snapshot` 对应的快照字典，或递归组合（§12.1）对应的嵌套 GTS 文件。它不得 (MUST NOT) 被视为全局稳定的标识符，也不得 (MUST NOT) 与另一个段或嵌套 GTS 中的相同标签合并。如果 `"v"` 缺失或为空字符串，则该项是匿名的，并表示该作用域内该项条目的一个新空白节点。转换可以 (MAY) 在保留空白节点同构和作用域隔离的同时对空白节点进行重新标记。
### 7.2 Term-id 分配（规范性）

Term-id 是无符号整数，按**追加顺序、分段 (per segment)** 分配，从每个段标头的 `0` 开始，并在其所属段内**被冻结**：在折叠 (folding) 帧 *N* 时生成的词项 (term) 在该段的剩余部分中保留其 ID。位于位置 *N* 的 `quads`、`annot` 或 `reifies` 帧必须 (MUST) 仅引用在**同一段**的位置 `0..N-1` 处引入的 term-id（此类帧本身不引入词项）。这使得纯追加写入、单次遍历读取和拼接变得稳健：term-id 是**压缩产物，而非标识** —— 跨段标识仅取决于词项值 (§3.1)，正如 `snapshot` 的字典已经在 `0` 处重启一样 (§10)。将文件全局 ID 应用于多段文件的实现会导致静默的折叠错误 (misfold)；边界规则 (§3.1) 和向量 17 (§19) 的存在正是为了使此类故障能够显式地暴露。
### 7.3 引用三元组与转意项 (reifiers) (`reifies` 帧)

RDF 1.2 允许一个三元组作为另一个三元组的主语或宾语。GTS 将引用三元组保留在 id 域中：一个**转意项 (reifier)** 是一个普通的 IRI/bnode 项；一个 `reifies` 帧将其与其引用的三元组绑定。

```cddl
reifies-payload = { * term-id => [term-id, term-id, term-id] }  ; reifier => (s, p, o)
```

被用作节点的引用三元组是一个带有指向其转意项的 `"k": 3` 和 `"rf"` 的项。

**RDF 数据集映射（规范性）。** 一个折叠 (folded) 的 GTS 图按如下方式映射到 RDF 1.2 数据集：当 `G` 缺失时，每个 `quads` 行 `(S,P,O,G?)` 在默认图中主张 (assert) RDF 三元组 `(S,P,O)`；当 `G` 存在时，则在命名图 `G` 中主张。一个绑定 `R => (S,P,O)` 的 `reifies` 在默认图中主张三元组 `R rdf:reifies <<( S P O )>>`。一个 `k:3` 项表示该三元组项，通过其转意项 `R` 到达。每个 `annot` 行 `(R, P', V')` 在默认图中主张三元组 `R P' V'`。配置文件 (Profiles) 可以 (MAY) 为投影定义额外的图放置约定，但上述核心映射是互操作性的基准。

**引用不意味着主张（规范性）。** 引用一个三元组项（无论是通过转意项还是 `k:3` 项）并不 (NOT) 主张基础三元组 `(S P O)`。基础三元组仅当它也出现在 `quads` 帧中时才被主张。

**RDF 1.1 降级（资料性）。** RDF 1.1 没有引用三元组项。一个有损的 RDF 1.1 投影可以 (MAY) 将引用三元组项替换为其转意项资源，并发出普通转意风格 (reification-style) 的三元组，如 `R rdf:subject S`、`R rdf:predicate P` 和 `R rdf:object O`，或者携带 `R rdf:reifies` 作为消费者可理解的扩展谓词。此类投影不得 (MUST NOT) 仅仅因为 GTS 文件引用了 `(S P O)` 就对其进行主张，且只要存在三元组项，工具就应该 (SHOULD) 将投影标记为有损。
### 7.4 四元组与注解

```cddl
quads-payload = [+ [term-id, term-id, term-id, ? term-id]]  ; s, p, o, (g; default graph if absent)
annot-payload = [+ [term-id, term-id, term-id]]             ; reifier, predicate, value
```

语句级元数据（置信度、有效间隔、立场/视角、语态等）在 reifier 上表示为 `annot` 行。**有争议的断言并存**：一个 reifier 上的多个 `annot` 行，或针对一个 (s,p,o) 的多个 reifier，都会被全部保留 —— 没有哪个是特权的。
注解在折叠的 GTS 状态中是一个有序多重集：读取器 (readers) 必须 (MUST) 保留每个段 (segment) 内的行顺序，并按文件顺序连接段注解行。完全重复的注解行保留在 GTS 折叠 (fold) 中；RDF 数据集投影可以 (MAY) 合并生成的相同 RDF 三元组，因为 RDF 数据集是集合值的。

**位置约束（规范性）。** 在 `quads` 行中，谓词 `p` 必须 (MUST) 是一个 IRI (`k:0`)；主词 `s` 必须 (MUST) 是一个 IRI、空白节点或引用三元组 (`k:0|2|3`)；受词 `o` 可以 (MAY) 是任何项；而图名称 `g`（如果存在）必须 (MUST) 是一个 IRI 或空白节点 (`k:0|2`) —— 绝不能是字面量或引用三元组。`reifies` 三元组 `(S,P,O)` 遵守相同的主词/谓词/受词约束。在 `annot` 行中，谓词必须 (MUST) 是一个 IRI。
### 7.5 折叠算法 (规范性)

```text
result := empty file state
          (terms, quads, reifiers, annotations, blobs, blob_meta, meta,
           suppressions, opaque, signatures, diagnostics, segment ledger)
for segment in file order:                      # §3.1; single-segment files: one iteration
  verify each frame's id (self-hash) and prev-link within the segment;
  record sig status if "sig" present
  terms := []   graph := {}   reif := {}   annot := []
  blobs := {}   blob_meta := {}   meta := {}   suppressed := []   opaque := []
  diagnostics := []
  for frame in segment log order:
    P := resolve payload (§6.1); if undecodable -> add opaque node (§7.6); continue
    switch frame.t:
      "terms"    : append each term (assign next id); each "dt"/"rf" MUST name an
                   already-introduced term-id (no forward references)
      "quads"    : add each (s,p,o,g) value tuple to graph
      "reifies"  : bind reifier to (s,p,o), keeping the first non-conflicting binding (§7.8)
      "annot"    : append (reifier, predicate, value)
      "blob"     : if "d" present -> blobs[BLAKE3(decoded "d")] := bytes (inline);
                   else -> register external blob by "pub".digest;
                   shallow-merge "pub" into blob_meta[digest]
      "suppress" : append directive to `suppressed` (display contract; §11)
      "snapshot" : load a self-contained fold wholesale (§10)
      "meta"     : shallow-merge map into segment meta (later keys overwrite earlier)
      "opaque"   : add explicit opaque node
  union segment fold into result BY TERM VALUE     # ids resolve locally, never cross segments;
                                                   # bnodes keep their scope (§3.1, §12.1)
result
```

折叠是确定的：在每一个一致性读取器中，相同的完整日志都会产生相同的值状态。
在段内，`meta` 作为一个映射上的浅层并集进行累积 —— 后续帧的键会替换之前的键；值不会被连接。**跨段**时，每个段折叠后的 `meta` 会按段公开（以段头 ID 为键），并按文件顺序浅层合并到文件级视图中 —— 后续段的键胜出，但每个段的原始数据保持可寻址（第三方段的元数据绝不会被静默吸收）。
### 7.6 不透明节点

当帧的负载无法解码时——未知的编解码器，或者是读取器没有密钥的 `cose-encrypt` 编解码器——读取器不得 (MUST NOT) 丢弃它。它必须 (MUST) 向图中添加一个**不透明节点 (opaque node)**，携带所有仍处于明文状态的内容：

```cddl
opaque-node = {
  "id"      : content-id,      ; the frame's self-hash
  "type"    : frame-type,      ; declared "t"
  ? "pub"   : any,             ; the cleartext public envelope, if any
  ? "to"    : [+ recipient],   ; declared recipients
  "sigstat" : "none" / "valid" / "invalid" / "unverified",
  "reason"  : "unknown-codec" / "missing-key" / "damaged",
}
```

大多数不透明节点是由读取器在解码时生成的；写入器也可以 (MAY) 发出一个显式的 `opaque` 帧（例如脱敏占位符），其负载是上述结构，在这种情况下 `"sigstat"` 被省略（由读取器确定）。一个 `damaged` 帧（自哈希失败或缺失）也会被隔离并折叠为不透明节点 (§9.1)：读取器可以 (MAY) 将其明文字段作为**不可信的**诊断元数据呈现，但必须 (MUST) 将 `"sigstat"` 设置为 `invalid`/`unverified` 和 `"reason": "damaged"` — 这些字节是不可信的。该帧仍然参与 id/prev 链，因此不能被静默移除。
### 7.7 流式折叠与有界内存

图不需要为了被**转换**而进行**物化**。**流式读取器** (§2.1) 按顺序处理帧并发送至接收器，仅持有词条字典、当前解码的帧或 blob，以及运行中的 id/prev 和验证状态：

- `gts → duckdb`/`sqlite` (§14) 保持**整数 ID** 模型：将 `terms` 增量流式传输到 `terms` 表中，并将 `quads`/`reifies`/`annot` 增量流式传输到 ID 值表中，随帧到达进行批量插入。**不发生词条解析，也不进行图物化** —— 内存受字典、最大解码帧和验证边车状态的限制，而非受三元组或 blob 的限制。解析 ID 的关系连接是后续引擎的任务。
- `gts → ttl/nq` 必须解析 ID 才能输出文本。若字典超出内存，读取器将使用索引 `"dict"` 定位器 (§6.2) 先行加载（或内存映射，或转储到磁盘上的键值存储）`terms` 帧，然后再流式传输四元组。

即使是 O(非重复词条 + 最大解码帧大小 + 验证边车状态) 对于病态不规则的图（例如：抓取数百万个唯一 UUID IRI 的爬虫，或单个非常大的内联 blob）也可能超出内存。因此，流式读取器在达到内存限制时**可以 (MAY)** **将其内存中的字典刷新到临时的磁盘键值存储中**，以牺牲本地转储文件为代价换取 RAM；由于词条 ID 是追加顺序且已冻结的 (§7.2)，正确性不受影响。`gts → duckdb`/`sqlite` 转换自动获得这一特性 —— 目标表*就是*转储。

包级流式接收器声明边界和内存基准测试辅助工具维护在 [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) 中。

因此，数 GB 大小的日志在有界内存中转换为操作基质 —— “解析并物化”导致的 OOM 失败模式在结构上得以避免。
### 7.8 重复与冲突 (规范性)

所有重复与冲突行为均在此定义，以便帧 (frame) 处理器不会自行发明本地策略：

| 项目 | 折叠行为 |
|---|---|
| 重复词项 | 写入器 (writer) 应该 (SHOULD) 暂存 (intern) 重复词项，但每个词项条目仍会获得其自己的段局部 ID。根据 §7 比较相等的非空值在文件联合中是相同的值。匿名的空白节点 (`"v"` 缺失或为空) 在每个词项条目中都是全新的。 |
| 重复四元组 | 折叠后的图是一个集合：完全相同的 `(s,p,o,g)` 值行将合并为一个，且不产生诊断。 |
| 化身绑定 | 一个化身 (reifier) 应该 (SHOULD) 恰好具有一个 `reifies` 绑定。重复的相同绑定是无害的。冲突的绑定是数据质量错误：读取器 (reader) 会显现 `ConflictingReifier`，保留文件顺序中的第一个绑定，并忽略化身映射中冲突的绑定。 |
| 注解 | 注解行是一个有序多重集 (§7.4)。同一化身 (reifier) 上的多行可以并存；完全重复的行在 GTS 折叠中会被保留。RDF 数据集投影可能会合并生成的相同三元组。 |
| Blob 字节 | Blob 通过摘要进行寻址。重复相同的摘要/字节是幂等的；内容寻址视图为每个摘要存储一个字节值。验证提取会根据请求的摘要对内联字节重新进行哈希处理 (§14.1)。 |
| Blob 元数据 | `blob_meta[digest]` 是按文件顺序构建的浅层映射。对于同一摘要，较晚出现的元数据键将替换文件级视图中较早的键；较早的声明保留在原始帧 (frame) 中。 |
| 抑制 | 抑制指令是累加的。重复等效的指令具有幂等的显示效果，但仍存在于有序抑制列表中。不存在取消抑制帧 (unsuppress frame) (§11)。 |
| 元数据键 | 段 (segment) `meta` 是浅层“后者胜出”；文件 `meta` 是按文件顺序对段元数据进行的浅层“后者胜出”合并。每段元数据仍可寻址 (§7.5)。 |
| 畸形帧 | 在可以恢复的情况下，有效载荷无法解码或其处理器无法安全折叠的帧 (frame) 将成为带有诊断信息的不透明节点 (§7.6, §9.1)。当项目边界已知时，后续幸存的帧仍可折叠。 |
| 未知结构帧类型 | 基准读取器 (Baseline Reader) 不会为未知的帧类型分配图语义。它 必须 (MUST) 保留链验证，并 可以 (MAY) 显现不透明节点或诊断；感知配置文件 (profile) 的完整读取器 (Full Reader) 可以 (MAY) 解释该帧。 |
| 配置文件冲突 | 配置文件 (profile) 声明和配置文件要求在各段 (segment) 之间求并集 (§3.1, §13)。不支持的配置文件要求会将受影响段中不支持的有效载荷降级为不透明节点或配置文件诊断；它们本身不会使核心有线格式折叠失效。 |
## 8. 转换目录
### 8.1 类

每个目录项都声明了一个**类**：

| 类 | 示例 | 还原所需能力 |
|------------|----------------------------------|------------------------------|
| `encode`   | `identity`, `base64`, `base85`   | 无（纯函数）         |
| `compress` | `gzip`, `zstd`, `lzma2`          | 编解码器库              |
| `encrypt`  | `cose-encrypt0`, `cose-encrypt`  | **密钥**（每个接收者）    |
### 8.2 堆叠

`"x"` 在编码时按数组顺序应用，在解码时反序应用。示例：`[zstd, cose-encrypt]` 意味着*压缩，然后加密*；读取器 (reader) 先解密（如果带有密钥）然后解压。
### 8.3 能力模型与平滑降级

解码一条链需要其命名的**每一项**能力。缺失能力的处理是统一的，无论该能力是库 (`unknown-codec`) 还是密钥 (`missing-key`)：该帧都将成为一个不透明节点 (§7.6)。这一单一机制实现了**文件内内容协商** —— 一个逻辑对象可以 (MAY) 以不同编解码器/格式的多个帧形式出现（例如：一个读取器无法解码的高保真表示，*以及*一个其可以解码的广泛支持的后备表示），读取器将使用其持有相应能力的最佳帧。
### 8.4 强制核心集与持久性

基准读取器 (Baseline Reader) 必须 (MUST) 实现 `identity`、`gzip` 和 `zstd` —— 因此，符合标准的读取器的完整依赖集为 **CBOR + BLAKE3 + gzip + zstd**。旨在实现最大长期可用性的写入器 (Writers) 应该 (SHOULD) 仅限于核心集。面向密度的写入器 (Writers) 可以 (MAY) 使用带有带内字典的 `lzma2`。所有核心编解码器都是稳定且被广泛部署的原语。

**Rsyncable 编解码器。** `compress` 类编解码器可以 (MAY) 是 *rsyncable* 的：它定期同步（重置）其压缩状态，以便未压缩输入中的局部更改仅影响压缩输出的有限范围。这以少量的压缩率开销为代价，改进了增量传输工具（例如 `rsync`）和版本控制增量压缩（例如 Git packfiles）。本修订版中定义的唯一 rsyncable 编解码器是 `zstd-rsyncable` (§8.5)。
### 8.5 规范编解码器注册表 (v1)

目录条目通过文件内（§5）的整数 ID 进行引用，但每个条目的 `"name"` 必须 (MUST) 是此注册表中的规范标识符，以便写入器 (writer) 能够互操作：

| name            | cls        | baseline? | parameters                    |
|-----------------|------------|-----------|-------------------------------|
| `identity`      | `encode`   | 是       | 无                          |
| `gzip`          | `compress` | 是       | `level`?                      |
| `zstd`          | `compress` | 是       | `level`?, `window`?, `dct`?   |
| `zstd-rsyncable`| `compress` | 否        | `block_size`: uint (默认 65536) |
| `lzma2`         | `compress` | 否        | `level`?, `dct`?              |
| `base64url`     | `encode`   | 否        | 无 (未填充)               |
| `base85`        | `encode`   | 否        | 无                          |
| `cose-encrypt0` | `encrypt`  | 否        | `COSE_Encrypt0` (1 个接收者) |
| `cose-encrypt`  | `encrypt`  | 否        | `COSE_Encrypt` (n 个接收者) |

读取器 (reader) 必须 (MUST) 通过规范 `"name"` 而不是目录 ID 来匹配编解码器（ID 是文件局部的）。后续规范版本将通过规范名称注册新的编解码器；未知名称将降级为不透明节点 (§8.3)。
## 9. 完整性与机密性

GTS 将四个完整性关注点分离开来：

1. **帧完整性** — 每帧 BLAKE3 自哈希 `"id"` (§9.1)。
2. **历史完整性** — `"prev"` 内容 ID 链 (§9.1)。
3. **来源 / 署名** — 可选的 COSE 签名 (§9.2)。
4. **新鲜度 / 非截断性** — 头部承诺：对头部 `"id"` 的签名，或索引 `"mmr"`/`"head"` 根 (§9.1, §13)。

前两者是强制性的且无需密钥；后两者是可选的且由配置文件 (profile) 驱动。
### 9.1 帧自哈希与内容 ID 链（强制性）

每个帧的 `"id"` 是其自身内容（除 `"id"` 和 `"sig"` 以外的每个键）的 BLAKE3-256 哈希值，因此，帧是**内容寻址且可独立验证的**。每个帧的 `"prev"` 命名了前一帧的 `"id"`；由于 `"prev"` 是被哈希内容的一部分，该链是一个 Git 风格的内容寻址列表，其中**头部 ID 会传递性地提交所有历史记录**。

- **并行验证。** 每个 `"id"` 都是一个自包含字节范围的哈希；通过索引 `"off"` 表 (§6.2)，所有帧哈希都可以并发地重新计算，随后进行一个简单的 O(n) `"prev"` 相等性检查。没有累积依赖关系强制进行单线程读取。（唯一本质上是顺序的步骤是在裸 CBOR 序列中发现帧边界——索引消除了这种低廉的长度扫描工作。）
- **损坏隔离与恢复。** 损坏的帧将无法通过其自身的 `"id"` 验证，因此损坏是**可独立检测的**。然而，只有在已知后续帧的字节偏移量时（通过完整的 `index` `"off"` 表、检查点帧、外部帧封装或存储层），才能保证后续帧的恢复。在裸 CBOR 序列（无每帧长度）中，任意字节损坏都可能导致解码器去同步：具有偏移量的读取器会跳过坏帧并折叠 (fold) 幸存帧 (`reason: "damaged"`)，而没有偏移量的读取器可以 (MAY) 无法跨越损坏重新同步。`evidence` 写入器应该 (SHOULD) 发出周期性的检查点索引 (§13)，以确保恢复的稳健性。
- **篡改证据。** 任何插入、重排序或变动都会破坏 `"prev"` 链接或自哈希。**截断 (Truncation)**（丢弃尾部帧）仅能通过头部承诺来检测——例如对头部 `"id"`、索引 `"head"`/`"mmr"` 根 (§6.2) 或带外锚点的签名。不透明帧是链的一部分，因此机密帧无法在不被察觉的情况下被剥离。

帧 ID 之上的 **Merkle-Mountain-Range** (MMR) 根（可选，携带在索引中）是一个单一的全文件承诺，其自身可并行计算，并支持 O(log n) 包含证明——即在不发送日志的情况下证明某个帧存在于日志中。
### 9.2 签名（可选，算法敏捷）

一个帧 可以 (MAY) 携带 `"sig"`，即在帧的 `"id"` 之上的一个 `COSE_Sign1` (RFC 9052)。由于 `"id"` 是全部内容的自哈希——包括 `"pub"`、`"d"`（如果是加密的，则为密文）以及 `"prev"`（链位置）——因此在 `"id"` 之上的一个签名将公共声明与密封负载及链位置**绑定**在一起，而对头部 `"id"` 进行签名从而锚定了所有先前的历史记录 (§9.1)。签名算法在 COSE 头部中声明（例如 `EdDSA`/Ed25519，`ES256`）；读取器 必须 (MUST) 遵守声明的算法。`evidence` 和 `opaque` 配置文件 (§13) 要求 (REQUIRE) 签名。密钥发现和信任锚定（哪些密钥是真实的，哪些签名者是经过授权的）属于**配置文件/部署策略**，而非 GTS 核心：`sigstat: "valid"` 意味着签名在*已解析*密钥下在密码学上有效，并不意味着该密钥是受信任的。
### 9.3 加密（可选）

`encrypt` 类编解码器将有效负载包装为 `COSE_Encrypt`/`COSE_Encrypt0`。收件人以明文形式列在 `"to"` 中，且**仅通过密钥标识符 (key identifier)** 标识 —— 绝不包含密钥材料。多个收件人可以 (MAY) 共享一个密封的有效负载（每个收件人用自己的密钥解开内容加密密钥）。密钥托管、轮换和撤销由**发行者 (issuer)** 负责，且不属于讨论范围；使用已退役密钥加密的有效负载可以 (MAY) 永久变为不透明。

本草案的 v1 一致性表面为单个直接收件人实现并测试了 `COSE_Encrypt0`。多收件人 `COSE_Encrypt` 信封和 ECDH 密钥封装在拥有专门的向量、密钥管理策略和跨引擎互操作性测试之前，被推迟到 v1 之外；参见 [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md)。

被推迟的 `cose-encrypt` 合约被设定为未来的全功能读取器 (Full Reader) 能力，而非 v1 实现声明。声明支持该能力的未来实现必须 (MUST) 使用一个带有 CBOR 标签的 `COSE_Encrypt` 信封，其内容加密使用 `A256GCM`，且其收件人数组中的每个条目对应于帧的明文 `"to"` 列表中的一个已声明收件人。最初定义的密钥管理模式为 `ECDH-ES+A256KW`：每个收件人条目携带派生密钥加密密钥所需的信息，并使用 `A256KW` 解开同一个 256 位内容加密密钥。一个符合规范的未来读取器仅尝试其持有相应私钥材料的收件人。

推迟的故障模式包括：

- 持有的密钥均不匹配任何收件人 `kid`：发出 `MissingKey` 并将该帧保留为具有 `reason:"missing-key"` 的不透明节点；
- ECDH 收件人元数据格式错误、派生的密钥加密密钥无法解开内容密钥，或 AES-KW 身份验证失败：发出 `KeyWrapFailed` 并将该帧保留为具有 `reason:"missing-key"` 的不透明节点；
- 成功解开后内容身份验证失败属于加密有效负载损坏，绝对不得 (MUST NOT) 暴露明文。

`vectors/crypto-deferred/*.json` 中的描述符向量固定了双收件人形态以及密钥错误、密钥缺失和解开失败的不透明情况，直到字节级 `COSE_Encrypt` 向量替换这些占位符。
### 9.4 不透明性不变性 (规范性)

> 不透明性隐藏的是**内容** —— 绝非**存在**、**出处**或**位置**。

对于每一帧，`{"id", "prev", "t", "x", "to", "pub", "sig"}` 必须 (MUST) 保持为明文（转换链 `"x"` 是明文，以便读取器知道要逆转哪些编解码器）。因此，即使没有相关密钥，读取器仍然可以获知该帧*存在*、它是*什么类型*、它是*为谁*密封的、*谁*签署了它，以及它在链中处于*什么位置*。这就是使选择性披露安全的原因：持有者可以携带 —— 且验证器可以验证其位置 —— 双方都无法读取的数据。
## 10. 压缩 (Compaction)

压缩 (Compaction) 折叠 (folds) 日志并将其作为单个自包含的 `snapshot` 帧 (frame) 重新发出（重新内部化的字典、去重的四元组、丢弃的自循环，以及可选的实体化蕴含闭包）。`snapshot` 的负载是一个自包含的图折叠 (graph fold) —— 词项 (terms)、四元组 (quads)、重构器 (reifiers)、注解 (annotations)、内联二进制大对象 (inline blobs) 和元数据 (meta)：

```cddl
snapshot-payload = {
  "terms"    : terms-payload,
  ? "quads"  : quads-payload,
  ? "reifies": reifies-payload,
  ? "annot"  : annot-payload,
  ? "blobs"  : { * digest => bstr },   ; inline content-addressed blobs
  ? "meta"   : any,
}
```

读取器 (reader) 折叠 `snapshot` 的方式与其折叠等效的 `terms`/`quads`/`reifies`/`annot`/`blob` 帧 (frames) 序列完全一致；词项 ID (term-ids) 在快照自身的字典中从 `0` 重新开始。
压缩 (Compaction) **在定义上是有损的**：它丢弃了原始的逐帧签名和日志的时间堆叠。压缩器 (compactor)：

- 必须 (MUST) 将折叠的起源 (provenance)（源日志摘要、时间、代理）作为四元组记录在快照中，并且
- 应该 (SHOULD) 对快照发出一个新的签名。

随后是两类产物：**证据日志 (evidentiary log)**（仅追加、已签名、从不压缩）和**分发快照 (distribution snapshot)**（已压缩、稠密、有损 —— 理想的分发载体）。读取器 (reader) 可以通过配置文件 (profile) 和是否存在 `snapshot` 帧来辨别其持有的是哪一种。
### 10.1 可流式处理压缩（仅限排序）

可流式处理压缩 (Streamable compaction) 将增量段（或多段文件）转换为处于可流式处理布局状态 (§3.3) 的单个按交付顺序排列的段。与上述快照压缩不同，它是**对排序 (ORDERING) 的重新编写，且仅针对排序**：折叠图 (folded graph)、内联 blob 以及每个内容寻址的事实都将保留。在重写过程中，三种签名主体的行为有所不同，压缩器必须 (MUST) 遵守这三种行为：

- **内容签名**（主体 = 内容摘要：blob 的 BLAKE3、语句或声明哈希——“这是真的，由 Bob 签名”）是关于摘要的普通四元组/注解。它们是**压缩不变的 (compaction-invariant)**，并完整保留：它们所证明的内容均未改变。
- **帧签名**（针对帧 `"id"` 的 COSE_Sign1，该帧承诺了 `"prev"`，§9.2）变为**分离但未损坏**：它们永远针对原始帧 ID 进行验证。压缩器必须 (MUST) 在**压缩溯源**中携带每个源帧签名——每个签名一个 `stream:DetachedSignature` 节点，记录原始帧 ID (`stream:sourceFrame`) 和原始 COSE 字节 (`stream:cose`)，外加每个源段头 (§13.3) 一个 `stream:sourceHead`——以便每个签名仍然是*关于原始日志的可检查声明*。
- **排序承诺**（签名的头部，索引 `"mmr"` 根）是唯一的布局绑定证明。它们无法在重新排序后存续；压缩器重新发布排序承诺（新的带有 `"head"` 的尾部 `index`，§6.2），从而成为**新排序的唯一证明者**。压缩器可以 (MAY) 额外对新头部进行 COSE 签名。

压缩器必须 (MUST) 将重写本身作为溯源四元组记录在输出中——一个携带执行工具 (`stream:agent`)、时间 (`stream:timestamp`) 和源段头 (`stream:sourceHead`) 的 `stream:Compaction` 节点——§10 溯源必须 (MUST) 满足，其具体词汇由 §13.3 提供。

**要求原始第三方链证明的配置文件。** 对于 `evidence` 段，原始签名链*就是*该工件；压缩器必须 (MUST) 拒绝它——除非它在可流式处理重写内部**将原始日志逐字封存**为嵌套的 GTS blob (§12.1)（角色为 `"source"`，通过 `stream:sealedSource` 从溯源节点引用）。内部的原始字节、链和签名保持字节级完整且可独立验证；外部布局按交付顺序排列；一个内容摘要将它们绑定在一起。

**发布工具的拒绝情形 (§14.1)。** 压缩器必须 (MUST) 拒绝：未通过验证的输入（任何诊断信息）；以及其折叠携带了以帧寻址的抑制 (`kind: "frame"`，§11) 的输入——重写会分配新的帧 ID，因此帧摘要目标会静默悬空。以摘要寻址的 `blob` 抑制将原样保留（内容寻址与布局无关）；以 ID 寻址的抑制将按值保留 (§11)。
## 11. 抑制（增量式“删除”）

GTS 从不进行物理删除。为了撤回或隐藏之前的内容，写入器 (writer) 会追加一个引用被取代的子图或帧摘要的 `suppress` 帧。被抑制的字节仍然存在并保持哈希链接；抑制是一种**显示/优先级契约**，由使用者 (consumer) 解释，而非擦除。这保留了完整且防篡改的历史记录。

```cddl
suppress-payload = { "targets": [+ suppress-target], ? "reason": tstr, ? "by": term-id }
suppress-target =
    { "kind": "frame",   "id": digest } /                                ; a frame, by its "id"
    { "kind": "blob",    "digest": digest } /                            ; a content-addressed blob
    { "kind": "term",    "id": term-id } /                               ; a term + quads it appears in
    { "kind": "quad",    "q": [term-id, term-id, term-id, ? term-id] } / ; one specific quad
    { "kind": "reifier", "id": term-id }                                 ; a reifier + its annotations
```

抑制是**单调且增量的**：匹配的目标在默认解析中被隐藏（`term` 目标还会隐藏该术语出现的每一个四元组）；字节仍然存在并保持哈希链接，使用者可以 (MAY) 显式地展示被抑制的内容。v1 版本中没有取消抑制的操作——后续的帧可以添加进一步的抑制，并且后续相同的断言不会恢复已被抑制的目标。

**跨段抑制（规范性，§3.1）。** 通过摘要寻址的目标（`frame`、`blob`）是文件全局的：无论内容 ID (content-id) 位于何处，它都命名相同的字节，因此后续的段可以 (MAY) 通过摘要抑制之前段的帧或 blob。通过 ID 寻址的目标（`term`、`quad`、`reifier`）携带术语 ID (term-id)，这些 ID 是段本地的——它们首先在**抑制帧所属的段内解析为术语值**，然后抑制将**按值应用于整个联合折叠 (union fold)**：`quad` 目标会隐藏任何段中所有匹配的 `(s,p,o,g)` 值元组，而 `term` 目标则在全文件范围内隐藏该术语值（以及它出现的四元组）。这就是为什么追加的信念修正 (belief-revision) 段可以抑制早期段所作的陈述，而无需重写其中的任何字节——早期段的记录保持存在、已签名且哈希链接（在传输层级通过内容寻址）。
## 12. 二进制与内容寻址

```cddl
; a `blob` frame carries raw bytes in "d" (subject to "x"); its metadata lives in cleartext "pub":
blob-pub = { ? "mt": tstr, ? "rep": tstr, ? "digest": digest-ref }
; INLINE blob  -> "d" present; digest = BLAKE3(decoded "d").
; EXTERNAL blob -> "d" absent;  "pub".digest names bytes held elsewhere.
```

- `blob` 帧的字节通过其 **BLAKE3-256 摘要** 进行寻址——对于内联 blob，为解码后的 `"d"` 的 `BLAKE3`；对于外部 blob 则为 `"pub".digest`；图通过该摘要引用 blob。按照惯例，出现两次的相同字节仅存储一次。
- blob 可以 (MAY) 是 **内联** 的（字节存在，一个自包含包）或 **外部** 的（图中仅出现摘要；字节存在于别处）。
- 逻辑对象 可以 (MAY) 拥有 **多种表示形式**（通过 `"rep"`/`"mt"` 区分，例如，主版本和广泛支持的备选版本）——参见内容协商，§8.3。
- 转换为文本格式 (§14) 会将内联 blob 外部化到边车 (sidecar) 目录中。
### 12.1 嵌套 GTS（递归组合）

媒体类型为 `application/vnd.blackcat.gts+cbor-seq` 的 blob 本身是一个完整的 GTS 文件。
由于转换反转后的有效负载是不透明字节，**任何**帧有效负载可以 (MAY) 携带嵌套的 GTS，并包装在任何转换链中 —— `[zstd]`、`[cose-encrypt]` 或两者兼有。规范载体是一个 `blob`，其 `"pub".mt` 为 `application/vnd.blackcat.gts+cbor-seq`。

- **折叠语义。** 全量读取器可以 (MAY) 递归：解码 blob（受 §6.1 能力规则限制），然后将内部字节作为独立的 GTS 进行折叠，将其结果公开为父图通过该 blob 摘要引用的**子图**。基准读取器可以 (MAY) 将嵌套的 GTS 视为普通 blob（不进行递归）。
- **空白节点作用域。** 内部 GTS 具有独立的空白节点作用域。如果全量读取器在父折叠旁边公开内部折叠，它必须 (MUST) 对内部空白节点进行重新标记或划分作用域，以确保标签不会与父级或同级的嵌套 GTS 文件冲突。
- **独立的完整性。** 内部 GTS 拥有自己的标头、id/prev 链和签名。**外部**链证明嵌套的 blob 存在且在其位置上保持完整；**内部**链证明嵌套日志是完整的。这两项保证相互组合，但不相互依赖。
- **组合不透明性。** 如果通过 `encrypt` 类转换访问嵌套的 GTS，且读取器缺少密钥，则*整个子图* —— 包括其内部标头 —— 都是一个不透明节点 (§7.6)：持有者可以携带并证明其无法读取的整个密封图的位置。这就是俄罗斯套娃情况（“加密字段内的整个 GTS”）。
- **有界递归。** 读取器必须 (MUST) 强制执行最大嵌套深度和总解码大小预算 (§18)。

这种组合不需要新的帧类型：嵌套就是“碰巧是一个 GTS 的 blob”。
v1 全量读取器辅助工具和用于递归限制的负面安全向量在 [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md) 中跟踪。
## 13. 配置文件 (Profiles)

配置文件 (profile) 是基于该格式的一组命名约定，在段头 `"prof"` 字段中声明。配置文件可以定义词汇表预期、验证规则、信任策略、能力要求和发布工作流，但它们位于核心有线格式之上。

本规范使用的状态值如下：

- **core-required (核心强制要求)**：基准有线格式、读取器或写入器一致性的一部分。
- **optional-standard (可选标准)**：作为互操作基础设施在此处指定，但基准读取器或写入器不强制要求。
- **experimental (实验性)**：为早期互操作性而描述；细节可能会在不改变核心 GTS 的情况下发生变化。
- **domain-specific (领域特定)**：由应用程序或下游社区拥有，而非核心 GTS。

| 配置文件或系列 | 状态 | 配置文件层级的含义 | 核心影响 |
|---|---|---|---|
| `generic` | core-required (核心强制要求) 默认 | 没有任何额外配置文件验证的任何符合规范的日志。 | 无；这表示没有特定配置文件的要求。 |
| `dist` | optional-standard (可选标准) | 紧凑型分发 `snapshot`：词汇表、定义和物化闭包。 | 无。 |
| `evidence` | optional-standard (可选标准) | 仅追加保管链；配置文件验证器需要签名和头部承诺。 | 无；在核心 GTS 中签名仍然是可选的。 |
| `opaque` | optional-standard (可选标准) | 基于 `encrypt` 类帧、签名和匿名 `kid` 的选择性披露约定。 | 无；在核心 GTS 中加密仍然是可选的。 |
| `bundle` | optional-standard (可选标准) | 一个其 `blob` 本身就是 GTS 文件 (`mt: application/vnd.blackcat.gts+cbor-seq`) 的 GTS，使用 §12.1。 | 无。 |
| `files` | optional-standard (可选标准) | 在 §13.2 和 §14.2 中定义的便携式文件树归档配置文件。 | 无；基准读取器正常折叠其图。 |
| `stream` | optional-standard (可选标准) | §3.3 和 §10.1 使用的流式词汇表和发布布局支持。 | 无；布局检查是读取器/工具诊断，而非新的帧语法。 |
| `image` | experimental (实验性) | Blob 表示以及描述性元数据和分析帧。 | 无。 |
| `ai-package` | experimental (实验性) | 概念加上逻辑、观察、意见、驳斥的声明、嵌入和数据。 | 无。 |
| `music-package` | domain-specific (领域特定) | GMEOW 音乐传输约定；此处仅供参考，由下游配置文件指定。 | 无。 |
| GMEOW 分发配置文件 | domain-specific (领域特定) | 层叠在 GTS 分发制品之上的下游 GMEOW 包约定。 | 无。 |
| `agent-memory` | domain-specific (领域特定) | 用于记忆、信念修正、抑制和溯源的应用程序约定。 | 无。 |

配置文件约束的是约定，而非有线格式；`generic` 读取器会读取它能解析的所有配置文件声明。不实现特定命名配置文件的读取器仍会根据其读取器类进行解析、验证和折叠该文件，然后在诊断或元数据中报告该不支持的配置文件。在多段文件中，每一段都声明自己的配置文件；文件的有效要求集是这些配置文件的并集 (§3.1)。

`evidence` 配置文件在配置文件层级要求头部承诺 (§9，第 4 项)，并且写入器应该 (SHOULD) 至少每 1024 帧或 64 MiB (以先到者为准) 发出一个检查点 `index`，以便受损的日志能够稳健地恢复 (§9.1)。该要求并不会使签名、索引或证据配置文件支持成为基准 GTS 的强制要求。

**配置文件策略配置。** 识别配置文件的验证器可以 (MAY) 接受 GTS 字节之外的部署信任策略。v1 策略文档是 JSON 或 YAML 格式，包含以下字段：

```yaml
trusted_signers:
  - did:example:issuer
require_trusted_signer: true
pseudonymous_kid_pattern: "^anon:[0-9a-fA-F]{32,}$"
```

`trusted_signers` 列出了部署授权的签名者 `kid` 值。
`require_trusted_signer` 使得需要签名的配置文件在没有至少一个来自受信任签名者的有效签名时失败。`pseudonymous_kid_pattern` 控制 `opaque` 配置文件的隐私保护接收者 ID 形状。这些设置仅属于配置文件/部署策略：它们不会改变核心 GTS 解析、折叠、帧 ID、签名原像或基准读取器有效性。

**第三方配置文件注册模板。** 第三方配置文件定义应该 (SHOULD) 发布：
- 用于标头 `"prof"` 字段的稳定配置文件名称。
- 所有者、变更控制流程、联系人 URI 和规范 URI。
- 状态（`experimental`、`optional-standard` 或 `domain-specific`）以及预期的兼容性
  策略。
- 词汇表命名空间 IRI、术语形状以及任何特定于配置文件的验证规则。
- 要求的编解码器、密钥、签名算法、信任锚或部署假设。
- 失败分类法：对于配置文件感知型工具，哪些违规行为属于错误、警告或提供信息的
  诊断。
- 与段、`cat` 组合、抑制、紧凑化以及嵌套 GTS blob 的交互。
- 一致性向量，包括基准读取器对于不支持配置文件的行为。
- 安全和隐私注意事项。

配置文件定义 必须 (MUST) 声明其不会更改标头/帧语法、段边界检测、content-id 或签名/哈希原像、transform-catalog 解析或 §7 中的核心折叠语义。新的配置文件行为必须通过图词汇表、现有帧类型、转换功能、元数据或配置文件感知型验证规则来表达。

注册表变更策略、保留命名空间以及可选标准配置文件的晋升流程维护在 [`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md) 中。
### 13.1 语言标签规范 (配置文件级规范)

本小节定义的是配置文件/投影写入器规则，而非基准读取器 (Baseline Reader) 要求。

生产者的图有效载荷可以 (MAY) 携带**内部私用语言标签** (例如 GMEOW 的
`x-gmeow-*`)：`dist` 或 `ai-package` 段 (segment) 的有效载荷 *就是* 规范形式，且
规范形式保留其内部标签。每个**投影部分 (projection section)** —— 文档 blob、派生
视图、向下投影的表示，以及任何*为外部消费者*生成的内容 —— 必须 (MUST)
仅携带**公共 BCP 47 标签**；若生产者将私用标签泄露至投影部分，必须 (MUST)
在写入时失败而非警告 (向量 20)。边界是按 *角色* 而非按文件划分的：一个包可以合法地
在带有公共标签的文档部分旁边携带带有内部标签的规范有效载荷。(这反映了 GMEOW 生成器
框架的内部标签泄露门控；参考生产者在部分边界重用了其 `retag` 机制。)
### 13.2 `files` 配置文件（可选标准）

`files` 配置文件是一种可选标准的、按内容寻址的文件树归档。它是 GTS 对 tar 的
`c`/`x`/`d` 的回答：把目录打包成单段 GTS，稍后再解包，并且在不做逐字节比较的情况下
用 `diff` 与目录比较。以下规则是面向 `files` 写入器和验证器的配置文件级符合性要求；
基线读取器只折叠图，而不实现归档工具。

**命名空间。** 此配置文件在 `https://w3id.org/gts/files#`（前缀 `files`）拥有一小组由规范
定义的词汇。GTS 的独立性意味着解包器不得 (MUST NOT) 要求 GMEOW、schema.org 或任何其他
本体才能读取归档；该词汇由本规范定义，并作为字面 IRI 承载在图中。

| 术语 | IRI | 形状 |
|---|---|---|
| `FileEntry` | `https://w3id.org/gts/files#FileEntry` | 类。一个已归档条目。 |
| `path` | `https://w3id.org/gts/files#path` | 相对路径字符串，使用 `/` 分隔符，没有前导 `/`，没有 `..` 组件。 |
| `digest` | `https://w3id.org/gts/files#digest` | 文件字节的 `blake3:<hex>` 内容摘要。 |
| `size` | `https://w3id.org/gts/files#size` | 以 `xsd:integer` 表示的字节大小。 |
| `mode` | `https://w3id.org/gts/files#mode` | 以十进制 `xsd:integer` 表示的 POSIX 权限位（例如 `420` 表示 `0o644`）。不记录文件类型位。 |
| `modified` | `https://w3id.org/gts/files#modified` | 以 UTC 的 `xsd:dateTime` 表示的修改时间。 |
| `mediaType` | `https://w3id.org/gts/files#mediaType` | 声明的 IANA 媒体类型字符串。 |
| `type` | `https://w3id.org/gts/files#type` | v2 字符串枚举：`file`、`directory`、`symlink`、`hardlink`、`fifo`、`chardev`、`blockdev` 或 `socket`。缺失表示 `file`。 |
| `linkTarget` | `https://w3id.org/gts/files#linkTarget` | v2 原始符号链接目标字符串，或硬链接目标归档路径。 |
| `uid` / `gid` | `https://w3id.org/gts/files#uid`, `https://w3id.org/gts/files#gid` | 以 `xsd:integer` 表示的 v2 数字所有者 ID。 |
| `userName` / `groupName` | `https://w3id.org/gts/files#userName`, `https://w3id.org/gts/files#groupName` | 来自 tar/PAX 元数据的 v2 所有者名称。 |
| `devMajor` / `devMinor` | `https://w3id.org/gts/files#devMajor`, `https://w3id.org/gts/files#devMinor` | 用于 `chardev` 和 `blockdev` 的 v2 设备号。 |
| `xattr` | `https://w3id.org/gts/files#xattr` | 指向属性空白节点的 v2 链接。 |
| `xattrName` / `xattrValue` | `https://w3id.org/gts/files#xattrName`, `https://w3id.org/gts/files#xattrValue` | v2 扩展属性名和 base64 词法值。 |
| `paxRecord` | `https://w3id.org/gts/files#paxRecord` | 指向逐字 PAX 逃生舱空白节点的 v2 链接。 |
| `paxKey` / `paxValue` | `https://w3id.org/gts/files#paxKey`, `https://w3id.org/gts/files#paxValue` | v2 未知 PAX 键和值字符串，为无损 tar 往返而保留。 |

**配置文件版本。** v1 表面是上面的最小常规文件配置文件。v2 归档在头部元数据中声明
`profileVersion: 2`，并且应该 (SHOULD) 在折叠后的 `meta` 帧中也携带同一值，使感知配置
文件的工具可以在读取后检测它。读取器必须 (MUST) 将缺失的 `files:type` 视为 `file`，因此
v1 归档会继续在 v2 读取器下折叠和解包。写入器应该 (SHOULD) 只在调用方选择加入或存在
非 v1 元数据时发出 v2。

**Quad 形状。** v1 归档中的每个常规文件由一个空白节点 `FileEntry` 描述：

```text
_:entry a files:FileEntry ;
    files:path "relative/path.txt" ;
    files:digest "blake3:<hex>" ;
    files:size 1234 ;
    files:mode 33204 ;
    files:modified "2026-06-10T20:00:00Z"^^xsd:dateTime ;
    files:mediaType "text/plain" .
```

v2 对每一种条目类型使用相同的主体形状。常规文件携带 digest/size，并且可以携带所有权、
xattrs 或 PAX 行。目录携带 `files:type "directory"` 加路径和元数据，但没有 digest 或大小。
符号链接和硬链接携带 `files:linkTarget` 且没有 blob。特殊条目携带 `files:type`，并且对设备
条目携带 `files:devMajor`/`files:devMinor`：
```text
_:dir a files:FileEntry ;
    files:path "empty" ;
    files:type "directory" ;
    files:mode 493 ;
    files:modified "2026-06-10T20:00:00.123456789Z"^^xsd:dateTime .

_:link a files:FileEntry ;
    files:path "current" ;
    files:type "symlink" ;
    files:linkTarget "releases/current" .

_:file a files:FileEntry ;
    files:path "data/events.csv" ;
    files:type "file" ;
    files:digest "blake3:<hex>" ;
    files:size 1234 ;
    files:xattr _:x0 ;
    files:paxRecord _:p0 .
_:x0 files:xattrName "user.comment" ;
    files:xattrValue "dmVyaWZpZWQ=" .
_:p0 files:paxKey "SCHILY.dev" ;
    files:paxValue "opaque" .
```

**确定性 (Determinism)**。一个 `files` 归档对于相同的输入树必须 (MUST) 是字节级可重现的：

- 路径在发出之前按其 UTF-8 字节序列进行字典序排序。
- 存储的路径使用 `/` 分隔符，且必须 (MUST) 是非空的相对路径。写入器 (Writers)、解包器和差异工具在处理文件字节之前，必须 (MUST) 拒绝绝对路径、Windows 驱动器相对路径、`..` 组件、`.` 组件、空组件以及反斜杠分隔符。
- v1 修改时间被归一化为 UTC，并以 `xsd:dateTime` 的形式序列化，精度为秒。在 v1 发出之前，必须 (MUST) 截断秒的小数部分。v2 允许在源格式提供时使用规范的固定宽度小数秒。
- v1 仅记录普通文件、POSIX 模式和 mtime。v2 记录了无损来回转换所需的 tar 可移植性元数据：条目类型、链接目标、显式目录、uid/gid、所有者名称、设备号、xattrs 以及未知的 PAX 记录。除非作为 PAX 或 xattr 数据携带，否则 ACL 仍不属于 v2 核心。
- v1 `pack` 和 `diff` 必须 (MUST) 拒绝符号链接条目，而不是跟随它们。v2 读取器 (readers) 可以保留符号链接和特殊文件元数据，但默认情况下提取操作仍然保持“拒绝危险 (refuse-dangerous)”。`unpack` 必须 (MUST) 拒绝通过输出目录下现有符号链接进行的任何目标逃逸。
- v2 条目按路径排序，xattrs 按名称、然后是值排序，PAX 记录在发出之前按键、然后是值排序。

**内联和外部 blob**。普通文件的字节可以 (MAY) 作为内联 `blob` 帧携带（`"d"` 存在，摘要 = BLAKE3(解码后的 `"d")`)），或者作为外部 blob 携带（`"d"` 不存在，`pub.digest` 命名保存在他处的字节，§12）。按照惯例，出现在多个路径下的相同字节仅存储一次。标准 `gts pack` 命令仅发出内联 blob。实现可以 (MAY) 添加显式的外部 blob 模式，但它必须 (MUST) 是选择性加入 (opt-in) 的且经过文档说明。`gts unpack` 必须 (MUST) 拒绝未被抑制且内联 blob 缺失的 `FileEntry`；`gts diff` 可以 (MAY) 通过 `files:digest` 进行比较，而无需获取外部字节。

目录、符号链接、硬链接、fifo、设备节点和套接字不得 (MUST NOT) 携带 `files:digest` 或 `files:size`。硬链接的 `files:linkTarget` 是另一个归档路径；符号链接的 `files:linkTarget` 是原始链接有效载荷。

**抑制 (Suppression)**。针对 blob 的抑制 (§11) 在默认提取时隐藏匹配的文件字节。`gts unpack` 和 `gts extract` 默认跳过/拒绝被抑制的 blob，并在操作员有意保留历史记录时提供显式的 `--include-suppressed` 覆盖。

**与其他词汇表的关系**。该配置文件 (profile) 故意设计为自洽的，但术语通过引用通用表层词汇表进行对齐：`files:size` ↔ schema.org `contentSize`，`files:mediaType` ↔ schema.org `encodingFormat`，`files:modified` ↔ NFO `fileLastModified`，`files:path` ↔ NFO `fileName`。这些对齐存在于 GMEOW 的映射 DSL 中；文件配置文件 (files profile) 本身不依赖于它们。
### 13.3 `stream` 词汇（可选标准）

可流式处理布局状态 (§3.3) 和可流式处理 compaction (§10.1) 使用位于
`https://w3id.org/gts/stream#`（前缀 `stream`）的一小组可选标准词汇 — 这与 `files` 配置文件
(§13.2) 采用相同的独立性决策：流式传输照片归档不需要 GMEOW 或外部本体；这些术语在此
定义，并作为字面 IRI 承载在图中。该词汇有意与 `files#` 区分开来（两者可以组合：一个同时
可流式处理的 `files` 归档会把每个文件描述一次为 `files:FileEntry`，再描述一次为
`stream:Manifestation` — 配置文件检查 (§14.1) 与布局检查 (§3.3) 保持独立）。

**流式索引术语** — 每个承诺的 blob 一个 `stream:Manifestation`，在任何 `blob` 帧之前发出到
前导流式索引中 (§3.3)：

| 术语 | IRI | 形状 |
|---|---|---|
| `Manifestation` | `https://w3id.org/gts/stream#Manifestation` | 类。此段承诺交付的一个 blob。 |
| `digest` | `https://w3id.org/gts/stream#digest` | `blake3:<hex>` 内容摘要 — blob 偿还的 IOU。 |
| `mediaType` | `https://w3id.org/gts/stream#mediaType` | 声明的 IANA 媒体类型（镜像 blob 的 `pub.mt`）。 |
| `size` | `https://w3id.org/gts/stream#size` | 解码后 blob 的字节大小，作为 `xsd:integer`。 |
| `role` | `https://w3id.org/gts/stream#role` | 交付角色字符串：`"preview"` / `"primary"` / `"source"`；开放集合。 |
| `order` | `https://w3id.org/gts/stream#order` | 在该段 blob 中预期的交付位置，`xsd:integer`，从 0 开始。 |

**Compaction provenance 术语** — §10/§10.1 的 provenance MUST 对应的具体词汇：

| 术语 | IRI | 形状 |
|---|---|---|
| `Compaction` | `https://w3id.org/gts/stream#Compaction` | 类。一个重写事件（空白节点）。 |
| `agent` | `https://w3id.org/gts/stream#agent` | 执行操作的工具，一个字符串（例如 `"gts-compact"`）。 |
| `timestamp` | `https://w3id.org/gts/stream#timestamp` | 以 UTC 的 `xsd:dateTime` 表示的重写时间。 |
| `sourceHead` | `https://w3id.org/gts/stream#sourceHead` | 一个源段的 `blake3:<hex>` head id；每段重复一次。 |
| `sealedSource` | `https://w3id.org/gts/stream#sealedSource` | 保存逐字原始内容的嵌套 GTS blob 的 `blake3:<hex>` 摘要 (§10.1)。 |
| `DetachedSignature` | `https://w3id.org/gts/stream#DetachedSignature` | 类。一个被携带过来的帧签名（空白节点）。 |
| `sourceFrame` | `https://w3id.org/gts/stream#sourceFrame` | COSE 签名永远要验证的原始帧 `"id"`，即 `blake3:<hex>`。 |
| `cose` | `https://w3id.org/gts/stream#cose` | 原始 COSE_Sign1 字节，base64url（无填充）字面量。 |

**Quad 形状**（已 compact 段的流式索引，然后是 provenance）：

```text
_:m0 a stream:Manifestation ;
    stream:digest "blake3:<hex>" ;
    stream:mediaType "image/webp" ;
    stream:size 20480 ;
    stream:role "primary" ;
    stream:order 0 .
_:c a stream:Compaction ;
    stream:agent "gts-compact" ;
    stream:timestamp "2026-01-01T00:00:00Z"^^xsd:dateTime ;
    stream:sourceHead "blake3:<hex>" .
_:s0 a stream:DetachedSignature ;
    stream:sourceFrame "blake3:<hex>" ;
    stream:cose "<base64url>" .
```

**声明耦合（规范性）。** 在没有声明 `"layout": "streamable"` 的段中使用 `stream#` 术语是一个
**警告**，不是错误 (§14.1)：provenance quads 可以合法地在 `gts → nq → gts` 往返和追加后的
重新 accretion 中保留下来。错误类别保留给相反的腐化 — 字节与所声明布局相矛盾 (§3.3)。
### 13.4 领域配置文件示例：`music-package`（信息性）

本小节是领域特定配置文件的一个信息性示例。基线读取器、写入器或验证器不需要实现 GMEOW
词汇、音乐领域规则、记谱投影规则或 `music-package` 验证器，就可以符合核心 GTS。

`music-package` 配置文件可以定义为一个单段 GTS，用于承载相对于帧的音乐内容：一个
`MusicalWork`/`MusicalExpression`、其 `Voice`s 和 `MusicalSegment`s、`TuningSystem` 与
`MusicalTimeFrame` 参考帧、原子 `ToneEvent`s、`DegreeOfFreedom` 声明，以及按 standpoint
索引的分析声明。它是 GMEOW 音乐切片的规范传输形式，也是记谱投影的输入。

**命名空间。** 此配置文件复用 GMEOW 音乐词汇
(`https://blackcatinformatics.ca/gmeow/`)。`music-package` 不要求是 `dist` 配置文件：它可以只
承载音乐内容图加任何投影 blob，并且可以 (MAY) 依赖外部 `dist` 快照提供词汇定义。

**头部。** `music-package` 段声明 `"prof": "music-package"`。该配置文件可以对新声明保持
append-only；现有 triples 不会被删除，只会被语句层 provenance 取代 (§7.3)。

**示例 quad 形状。** 最小包可以包含：

```text
@prefix gmeow: <https://blackcatinformatics.ca/gmeow/> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .

:piece a gmeow:MusicalExpression ;
    gmeow:hasVoice :voice1 .

:voice1 a gmeow:Voice ;
    gmeow:voiceTuningFrame :tuning12EDO ;
    gmeow:voiceTimeFrame :timeGrid .

:tuning12EDO a gmeow:TuningSystem .
:timeGrid a gmeow:MusicalTimeFrame .

:event1 a gmeow:ToneEvent ;
    gmeow:segmentOf :voice1 ;
    gmeow:toneEventPitchValue :pitchC4 ;
    gmeow:segmentSpan :span1 .

:span1 a gmeow:MusicalTimeSpan ;
    gmeow:hasMusicalTimeFrame :timeGrid ;
    gmeow:timeStartNumerator 0 ;
    gmeow:timeStartDenominator 1 ;
    gmeow:timeDurationNumerator 1 ;
    gmeow:timeDurationDenominator 4 .
```

时间和音高都是**相对于帧**的：`toneEventPitchValue` 指向在事件所属声部调音帧下解释的
`PitchValue`，偏移量/时值则是在声部时间帧下解释的有理数值。

**投影。** `music-package` 可以包含 `blob` 帧，其字节是下投影表示（MusicXML、MEI、ABC、
LilyPond、Humdrum **kern、MIDI、Scala `.scl`、tablature、mensural、graphic notation）。
music-package 配置文件验证器可以要求每个投影都随附声明损失清单，列出所用的
`NotationProjectionProfile`、它可以表示的 `MusicalParameter`s，以及它造成的
`ProjectionLoss`es。该清单可以是 Turtle sidecar，也可以是嵌入的头部/注释，并被视为投影的
一部分，而不是规范图的一部分。

**Bundle 配置文件耦合。** 一个 `bundle` 配置文件 (§12.1)，当其 blobs 为 `music-package` 段时，
提供多乐章 / 多版本传输场景。每个嵌套段保留自己的配置文件声明；外层 bundle 不施加额外约定。

**验证。** 感知 music-package 的验证器可以检查投影 blob 引用的每个 `NotationSystem` 是否都
有对应的 `NotationProjectionProfile`，并且该配置文件说明了音乐切片中声明的每个
`MusicalParameter`（没有静默遗漏）。基线 `gts verify` 不要求实现此配置文件；它可以报告不支持
该配置文件，而不使线格式有效性失败。
## 14. 输出转换 (Transforms out)

转换将 GTS 转换为运行基质。每一个都是折叠表之上的薄垫层——不涉及 RDF 文本解析器。

- `gts → nquads` / `gts → turtle` — 序列化 `quads` + `reifies`/`annot`（后者作为 RDF 1.2 具象化）。内联 blob 被**外部化**到 `./blobs/<blake3>.bin`，并且图的摘要引用解析为这些路径。不透明帧序列化为其不透明节点描述。
- `gts → duckdb` / `gts → sqlite` — 批量加载四个表（`terms`、`quads`、`reifies`、`annot`）以及一个 `blobs` 表；创建适用于引擎的索引。由于 GTS 表已经符合关系形状，这几乎是一种机械化的加载。

每个转换都应该 (SHOULD) 可通过**往返等效性**进行验证：对于**完全可解码**的帧，`gts → nq → gts` 必须 (MUST) 产生相同的折叠图（不考虑空白节点标签和确定性 CBOR 重新编码）。不透明节点被排除在外——它们序列化为不透明节点描述，并作为普通四元组 (quads) 重新导入，而不是作为不透明帧。
### 14.1 组合工具要求 (对一致性工具具有规范性)

本节仅定义工具一致性。基线读取器 (Baseline Reader) 或写入器 (Writer) 无需附带这些 CLI 动词、转换目标、归档命令或发布策略即可实现核心一致性。感知配置文件的工具仅强制执行其声称支持的配置文件验证器；除非用户明确请求该配置文件的验证，否则不支持的配置文件将作为诊断或元数据呈现。

原始 `cat` 始终有效 (§3.1)；一致性的 **验证组合器** (`gts cat`) 和验证器 (`gts verify`) 增加了“拒绝-不信任”姿态：

- **`gts cat` 必须 (MUST) 拒绝退化输入**：即不是有效 GTS 的输入、折叠 (fold) 后产生零个三元组 (quads) 和零个数据块 (blobs) 的段 (segment) (这几乎总是连线错误，绝非真正的包)，或者在输出中仅包含抑制 (suppress-only) 的段会隐藏所有先前的帧 (frame)。发布级工具绝不信任病态状态是故意的。
- **`gts verify` 必须 (MUST) 检查支持配置文件的“声明与计算”要求**：段 (segment) 中的图如果使用了支持配置文件的词汇表但未声明该配置文件，则为 **错误**；已声明但未使用的支持配置文件则为 **警告**。工具读取的声明 (CLI 依赖报告，§13) 不得与它们描述的内容脱节。
- **`gts verify` 应该 (SHOULD) 按段报告**：头 ID (head id)、签名者集、配置文件 (profile)、术语/三元组计数、带原因的不透明节点 (opaque-node) 计数 —— 即文件的组合账本。
- **`gts verify` 必须 (MUST) 检查布局声明** (§3.3)：声称 `"layout": "streamable"` 的段其覆盖区域违反了交付顺序，或者其索引页脚缺失或与其覆盖的帧 (frames) 矛盾，则为 **错误** (`StreamableLayoutError`，§2.3)；在未声明的段中出现 `stream#` 词汇则为 **警告** (§13.3)。`gts info` 和 `gts verify` 应该 (SHOULD) 报告已声明段的可流式处理边界 —— “流式传输至第 *N* 帧，包含 *M* 个帧的增量尾部”。
- **`gts compact --streamable <in> -o <out>` 是布局重写** (§10.1)。它 必须 (MUST) 拒绝无法通过验证的输入、带有按帧寻址抑制的输入，以及在没有密封原始选项 (`--seal-original`，§10.1) 的情况下输入的 `evidence`；它 必须 (MUST) 发出处于规范可流式处理形状 (§3.3) 的单个已声明段，并带有压缩溯源和分离签名 (§13.3)，且对于相同的输入和参数，其输出 必须 (MUST) 是字节确定性的 (数据块按解码大小升序排列，大小相同时按摘要升序排列；重写时间戳是一个参数，而非当前系统时间)。
- **确定性图创作模式** 是折叠图 (folded graph) 的可重现构建写入器界面。它发出一个普通段，且在写入前 必须 (MUST) 重新映射本地术语 ID：术语按语义值排序 (IRI 字符串；字面量词法形式加有效数据类型 IRI 加语言标签；空白节点标签，匿名空白节点使用其输入出现顺序作为平局决胜因素；解析为其主体/谓词/对象值的引用三元组)。然后它按固定顺序发出可创作帧：`terms`、`quads`、`reifies`、`annot`、`blob`、`meta`、`suppress`。三元组 (Quads)、具体化绑定、注释、数据块 (blobs)、元数据键和抑制帧按重新映射后的确定性 CBOR 表示形式排序。该模式不重放读取器 (reader) 观察结果 (`opaque`、签名、诊断或段账本)；需要保留这些观察结果的发布工具必须使用特定于配置文件的重写 (例如可流式处理压缩) 或将原始字节作为证据密封。
- **数据块提取是验证，而非转换** (`gts ls`, `gts extract`)：数据块 (blobs) 按内容摘要寻址 (帧索引是物理偶然性，在 `cat` 下会发生偏移)；提取过程会根据请求的摘要对字节重新哈希；默认情况下拒绝按摘要抑制 (§11) 的数据块 (抑制是显示合同，而提取即显示)，除非有明确的覆盖选项；媒体类型标志是针对数据块声明的 `pub.mt` 的 **断言 (assertion)** —— 验证性发布工具拒绝不匹配的内容，而不是进行转码。
### 14.2 归档工具 (`files` 配置文件)

`files` 配置文件增加了验证发布命令。它们共享 §14.1 中“拒绝而非信任”的姿态：原始字节操作始终是有效的 GTS，但工具会拒绝病理性状态，而不是信任它们是故意的。稳定的 `pack` 命令默认发出 v1 普通文件配置文件。v2 元数据是 tar 桥接器和其他无损归档工具的可选创作界面。

- **`gts pack <dir|file>... -o out.gts`**
  生成一个标头声明为 `"prof": "files"` 的单段 GTS。每个参数都会被归档：文件按其基本名称添加；目录作为普通文件条目递归添加。此 v1 命令不包含空目录和非普通条目。生成的归档按顺序包含描述每个 `files:FileEntry` 的 `terms` 和 `quads`，随后是用于文件内容的内联 `blob` 帧。该命令 必须 (MUST) 拒绝：
  - 包含不安全存储路径的输入：绝对路径、驱动器相对路径、`..`、`.`、空组件或反斜杠分隔符；
  - 符号链接；
  - 不可读或在遍历过程中消失的输入。

- **v2 创作助手 / tar 桥接输入**
  声称支持文件配置文件 v2 的工具 可以 (MAY) 使用 §13.2 中的 v2 词汇发出显式目录、符号链接、硬链接、fifo、设备节点、套接字、所有权、xattrs 和 PAX 记录。它 必须 (MUST) 使用 `profileVersion: 2` 标记该段，保持条目按存储路径排序，保持 xattrs/PAX 记录排序，并通过在读取旧归档时省略 `files:type` 或将其默认设置为 `file` 来保持 v1 兼容性。

- **`gts unpack <archive> [-C dir]`**
  将归档中的每个 `files:FileEntry` 写入目标目录（默认为当前工作目录）。该命令 必须 (MUST)：
  - 拒绝写入目标目录之外（`..`、绝对路径或逃逸目录的符号链接）；
  - 创建显式 v2 目录，但拒绝符号链接、硬链接、fifo、设备节点和套接字提取，除非用户为这些类别提供了 `--allow-symlinks` 或 `--allow-special`；即使在选择加入后，符号链接目标仍局限于目标树内；
  - 对每个写入的普通文件重新计算哈希并验证其是否与 `files:digest` 匹配；
  - 恢复条目声明的修改时间和权限（受宿主操作系统限制）；
  - 除非用户提供 `--same-owner` 或等效的特权选择（如 `--numeric-owner`），否则绝不更改所有权；并且除非用户提供 `--preserve-setid`，否则绝不恢复 setuid/setgid/sticky 位；
  - 默认跳过摘要被抑制（§11）的条目，并带有显式的 `--include-suppressed` 覆盖。

- **`gts tar -c/-x/-t/-d`**
  兼容 tar 的 CLI 可以 (MAY) 使用熟悉的标志（`-cf`、`-czf`、`--zstd`、`-xf`、`-tf`、`-df` 和 `-C`）包装 `pack`、`unpack`、`diff`、`from-tar` 和 `to-tar`。包装器 必须 (MUST) 保持与 `unpack` 相同的安全策略：非变异的列表/差异操作可以检查链接和特殊文件元数据，但提取仍需要上述显式选择加入。工具 应该 (SHOULD) 根据归档扩展名选择 `.gts` 或 `.tar` 路径，并且在创建 tar 输出时 应该 (SHOULD) 从常见的 tar 后缀中推断 gzip/zstd 包装。声称支持大归档流式处理的工具 应该 (SHOULD) 说明其满足的确切边界：直接进行 `.gts` 创作可以在元数据排序时流式传输普通文件有效载荷帧，但折叠图投影和压缩后端可能仍需要有界临时存储或内存实例化。

- **`gts diff <archive> <dir>`**
  通过内容摘要将归档的 `files:FileEntry` 集与 `<dir>` 的当前状态进行比较。报告新增、删除和修改的路径。如果目录与归档完全匹配，则以 `0` 退出；如果任何路径不同或输入被拒绝，则以 `1` 退出。无需字节比较：内容寻址使该操作在目录上的复杂度为 O(read)。

**归档工作流比较。**
| 工作流 | 通常的目录 | GTS `files` 配置文件行为 |
|---|---|---|
| `tar` | 头部记录与文件字节交错；路径和元数据解释属于工具策略。 | v1 清单是针对常规文件的 RDF 四元组。v2 增加了等效于 tar 的条目种类、链接目标、所有权、设备节点、xattrs 和 PAX 转义记录，同时保持提取策略显式。 |
| `zip` | 中央目录支持随机访问，但其作为面向重写的页脚。 | GTS 保持仅追加；可选索引可加速访问，而不会使页脚成为归档标识。 |
| BagIt 样式的包 | 负载文件以及侧挂 (sidecar) 清单/校验和。 | 图原生清单和内容字节在同一个可验证的 CBOR Sequence 中传输；使用外部 blob 时仍保持内容寻址。 |

价值主张不在于压缩率。当大小占主导地位时，请使用压缩转换或外部传输。`files` 配置文件适用于图原生清单、摘要寻址去重、追加组合以及跨引擎的一致安全策略。
## 15. 完整示例 (Worked examples)

CBOR 以 **诊断表示法 (diagnostic notation)** (RFC 8949 §8) 显示。哈希/签名被省略为 `h'…'`。
### 15.1 最小分发快照 (`dist`)

```text
55799(                                   / self-describe magic /
  { "gts": "GTS1", "v": 1, "prof": "dist",
    "cat": { 0: {"name":"identity","cls":"encode"},
             4: {"name":"zstd","cls":"compress"} },
    "id": h'…header.id…' }
)
{ "t": "terms", "prev": h'…header.id…', "id": h'…terms.id…',
  "d": [ {"k":0,"v":"https://example.org/Cat"},          / id 0 /
         {"k":0,"v":"http://www.w3.org/2000/01/rdf-schema#label"},  / id 1 /
         {"k":1,"v":"Cat","l":"en"} ] }                  / id 2 /
{ "t": "quads", "prev": h'…terms.id…', "id": h'…', "x": [4],
  "d": h'…zstd([[0,1,2]])…' }                            / Cat rdfs:label "Cat"@en /
```

Term 2 是带有语言标签且没有 `"dt"` 的字面量，因此其数据类型为 `rdf:langString` (§7.1)。
### 15.2 证据：图像 + 已签名累积 (`evidence`)

```text
{ "t": "blob", "prev": h'…header.id…', "id": h'…',
  "pub": {"mt":"image/jp2"}, "d": h'…image bytes…',      / digest = blake3(d) /
  "sig": h'COSE_Sign1 by did:photographer' }
{ "t": "annot", "prev": h'…blob.id…', "id": h'…',
  "d": [[10,11,12]],                                     / reifier 10: capturedAt … /
  "sig": h'COSE_Sign1 by did:photographer' }
{ "t": "annot", "prev": h'…prev.id…', "id": h'…',        / later custody transfer, separate signer /
  "pub": {"event":"custody-transfer"},
  "d": [[13,11,14]], "sig": h'COSE_Sign1 by did:evidence-clerk' }
```

没有任何内容被重写；每一项累积都是哈希链接的并经过独立签名。
### 15.3 Notary：部分不透明帧 (`opaque`)

```text
{ "t": "annot", "prev": h'…prev.id…', "id": h'…',
  "pub": { "claim": "I hereby notarized this document.",
           "notary": "did:notary:jane", "ts": "2026-06-09T12:00:00Z" },
  "x": [4, 7],                                            / 7 = cose-encrypt /
  "to": [ {"kid":"anon:7f3a…","alg":"ECDH-ES+A256KW"} ],  / pseudonymous kid (opaque profile, §18) /
  "d": h'COSE_Encrypt(verified ID record + provenance)',
  "sig": h'COSE_Sign1 by did:notary:jane' }
```

任何人都可以验证公开公证及其签名；只有法庭密钥能解密加封记录；签名将两者绑定 (§9.2)。没有法庭密钥的读取器将其折叠为一个不透明节点，其中 `reason:"missing-key"`、`pub` 保持完整，`sigstat:"valid"`。
### 15.4 平滑降级 (`image`, 内容协商)

```text
{ "t": "blob", "prev": h'…', "id": h'…', "pub": {"mt":"image/vnd.djvu","rep":"master"}, "x":[9], "d": h'…' }
{ "t": "blob", "prev": h'…', "id": h'…', "pub": {"mt":"image/jpeg","rep":"fallback"}, "d": h'…' }
```

缺少编解码器 `9` (djvu) 的读取器将主节点折叠为不透明节点，并使用 JPEG 回退方案——两者均存在，且均已通过哈希链接。
### 15.5 Matryoshka：密封在帧 (`bundle` / `opaque`) 内的完整签名 GTS

```text
{ "t": "blob", "prev": h'…', "id": h'…',
  "pub": { "rep": "sealed-evidence-graph", "mt": "application/vnd.blackcat.gts+cbor-seq" },
  "x": [4, 7],                                            / zstd then cose-encrypt /
  "to": [ {"kid":"did:court:registry"} ],
  "d": h'COSE_Encrypt( zstd( <a complete, independently-signed GTS file> ) )' }
```

在没有法庭密钥的情况下，这会折叠 (fold) 为一个不透明节点 (opaque node) —— 一个持有者携带但无法读取的完整子图，然而其存在和位置由外部链证明。有了密钥，全量读取器 (Full Reader) 会进行递归 (§12.1)，并将内部 GTS —— 头部 (header)、链 (chain)、签名 (signatures) 等一切 —— 折叠 (fold) 为一个可验证子图。
## 16. 媒体类型与 HTTP 服务契约

GTS 文件是已发布的成果物。本节定义了部署一致性：即使本地存储的 GTS 文件从未通过 HTTP 提供服务，其也可以是线缆格式有效、符合读取器一致性 (reader-conformant) 且符合写入器一致性 (writer-conformant) 的。一致性部署必须 (MUST) 宣告媒体类型、支持范围请求，并设置尊重该格式不可变性的缓存标头。
### 16.1 媒体类型和文件扩展名 (规范性)

- **媒体类型：** `application/vnd.blackcat.gts+cbor-seq`（注册模板见 §20.1）。
  GTS 使用 `+cbor-seq` 结构化语法后缀，因为 GTS 文件是段头和帧的 CBOR 序列
  ([RFC 8742])，由段头和帧组成，而不是单个 CBOR 数据项。早期的
  临时拼写 `application/vnd.blackcat.gts+cbor` 已废弃；部署必须 (MUST) 发出
  `application/vnd.blackcat.gts+cbor-seq`。读取器可以 (MAY) 接受废弃拼写作为遗留
  别名，但不得 (MUST NOT) 在新写入的元数据中发出它。
- **文件扩展名：** `.gts`。
- **魔术字节：** 当第一段被标记时，位于第一段段头 (Header) 起始处的 CBOR 自描述标签 `55799` (`0xd9 0xd9 0xf7`)。读取器可以 (MAY) 在识别候选 GTS 文件时将这三个字节作为一个信号，但在将这些字节视为 GTS 之前必须 (MUST) 确认段头形状。

不识别 `application/vnd.blackcat.gts+cbor-seq` 的服务器应该 (SHOULD) 回退到
`application/octet-stream` 而不是错误的文本类型；客户端应该 (SHOULD) 在媒体类型
缺失或通用时检查第一个 CBOR 数据项。
### 16.2 文件识别算法 (规范性)

媒体类型元数据在可用时具有权威性。当读取器 (reader) 必须在没有可信元数据的情况下识别字节时，它必须 (MUST) 使用此算法：

1. 将 `.gts` 和 `application/octet-stream` 仅视为提示；两者都不能证明或反驳 GTS。
2. 如果前三个字节是 `0xd9 0xd9 0xf7`，则将第一个 CBOR 项解析为带标签项并解包标签 `55799`。否则从字节偏移量 `0` 开始解析第一个 CBOR 项。
3. 解包后的第一个项必须 (MUST) 是一个包含 `"gts": "GTS1"` 且缺少帧 (frame) 键 `"t"` 的 Header 映射。不匹配则不是 GTS 文件。
4. 肯定的识别结果仍然仅仅是一个识别结果。完整的有效性要求将整个观察到的字节流解析为 CBOR 序列 (§3)，应用段 (segment) 边界规则 (§3.1)，并根据所选一致性 (conformance) 类的要求验证 id、链 (chains)、配置文件 (profiles) 和功能 (capabilities)。
5. 实现不得 (MUST NOT) 要求全文件 CBOR 包装器、总项计数或长度前缀。独立有效的带标签段 (segments) 可以被串联，因此后来的 `55799` 标签识别的是后来的段 (segment) 标头，而不是嵌套的全文件对象。
### 16.3 HTTP 服务语义 (规范性)

GTS 包像任何其他不可变二进制发布版本一样进行服务，但有三个额外要求：

1. **`Accept-Ranges: bytes`** 必须 (MUST) 为每个 `.gts` 响应发送。该格式专为部分、流式消费而设计 (§3.2)：使用者可以在不下载整个文件的情况下折叠 (fold) 标头和帧 (frames) 的前缀。客户端从发现的 CBOR 项偏移量、索引或其他可信清单中选择字节范围；HTTP 范围支持本身并不验证或修复本地文件字节。
2. **边缘无转换。** 由于字节是内容寻址链，代理和服务器不得 (MUST NOT) 应用压缩、缩小或任何改变字节的转换。帧 (frames) 已经由写入器 (writer) 选择的编解码器压缩；在传输层重新压缩会破坏内容哈希和签名。
3. **CORS。** 公共词汇表/数据集包预期是跨域可读的。响应应该 (SHOULD) 为所服务的 `.gts` 源包含 `Access-Control-Allow-Origin: *`。
### 16.4 不可变性感知缓存 (规范性)

已发布的 GTS 发行版是不可变的；一个 GTS 包 URL 命名一个精确的字节序列。

- **版本化 URL**（`…/gmeow/1.2.3/gmeow.gts`、`…/packages/music/2026-06-18/music.gts` 或任何包含版本/日期/头部标识符的 URL）必须 (MUST) 使用以下内容提供服务：

  ```text
  Cache-Control: public, max-age=31536000, immutable
  ETag: "<last-segment-head>"
  ```

  天然的 ETag 是文件最后一段 (segment) 头部 ID 的十六进制（§3.1），因为它传递性地提交了文件的每一个字节。`immutable` 指令告诉缓存，在一年有效期内无需重新验证。
- **`latest` / conneg 别名**（解析为当前发行版且可能更改的 URL）不得 (MUST NOT) 缓存为单一变体：

  ```text
  Cache-Control: private, no-store
  Vary: Accept
  ```

  `Vary: Accept` 防止了当同一路径协商为 HTML、Turtle 或 GTS 包时发生的 conneg 缓存投毒。这与 Apache 生成器针对 slice IRI 处理的缓存投毒类别相同。

v0.2 中的配置文件 (profile) 选择仍保持为 URL 形式：每个包一个 URL。RFC 6906 / `Accept-Profile` 被记为可能的未来扩展，v0.2 一致性并不要求。
## 17. 版本控制与持久性保证

- 标头 `"v"` 是规范的主版本号。读取器 必须 (MUST) 拒绝其未实现的主版本，但 必须 (MUST) 仍验证 id/prev 链并枚举帧类型/ID。
- **段语义与旧版读取器。** 实现此修订版的读取器 必须 (MUST) 支持段边界 (§3.1)。不支持（即 §3.1 之前的实现）的读取器会将第二个 Header 视为非帧数据项：此类输入**对该读取器而言是格式错误的**，且它 必须 (MUST) 对文件的剩余部分给出致命诊断，而不是跳过该项 —— *静默地错误折叠（跨越边界应用文件全局术语 ID）是唯一被禁止的结果* (vector 17)。由于 `cat` 无法重写第一段的标头（自哈希已将其封存），多段文件无法在第一个标头中宣告自身；因此边界检测是结构性的，而硬失败规则正是保护生态系统中最旧读取器的手段。
- **结构持久性：** GTS 文件加上本规范在不需要引擎和外部字典的情况下永远是可解码的 —— CBOR 是 IETF 标准，且字典是带内的。
- **密度持久性：** 由编解码器目录管理；强制核心集 (`identity`/`gzip`/`zstd`) 保证了任何时代都可以解码的基准。
## 18. 安全注意事项 (Security considerations)

- id/prev 链提供完整性，而**非**机密性；使用 `encrypt` 类编解码器以实现机密性。
- **截断**（丢弃尾部帧）无法仅通过链本身检测；`evidence` 构件必须 (MUST) 锚定头部——即对头部 `"id"` 的签名，或索引 `"head"`/`"mmr"` 根 (§6.2)——以便验证器可以检测到被缩短的日志。
- 损坏帧*之后*帧的**恢复**仅在偏移量已知（完整的索引、检查点帧或外部帧封装）的情况下得到保证；裸 CBOR 序列 (Sequence) 在发生任意损坏时可能会失去同步 (§9.1)。GTS 未定义奇偶校验/纠删码——针对大批量丢失的持久性是存储层关注的问题。
- `"to"`/`kid` 值可能会泄露关系元数据（帧是为谁密封的）。因此，`opaque` 配置文件要求 (REQUIRES) 使用伪名 `kid`；其他高隐私配置文件应该 (SHOULD) 使用它们。使用按文档或成对的标识符——例如 `"kid": "anon:<BLAKE3(true-kid ∥ head-id)>"`——或密钥盲化 (key blinding)，使得同一个接收者在不同文件之间不可关联。
- 有效签名证明了签名者对帧字节的认可；它**不**断言声明的真实性（与证明语义一致——担保 ≠ 正确性）。
- 不透明帧 (Opaque frames) 不可读但**并非**不可见；请勿将秘密放置在 `"pub"`、`"to"` 或 `"meta"` 中。
- 快照压缩 (§10) 会破坏原始签名；`evidence` 构件不得 (MUST NOT) 进行快照压缩。流式处理压缩 (§10.1) 会分离帧签名而不是破坏它们，但重新排序的链仅由压缩器证明；`evidence` 构件不得 (MUST NOT) 进行流式处理压缩，除非对原始数据进行逐字密封 (§10.1)，并且使用者对压缩后文件*顺序*的信任是对压缩器的信任。
- 对攻击者提供的帧进行解压缩必须 (MUST) 有界（抵御 zip 炸弹 resistance）；读取器应该 (SHOULD) 限制解码大小。
- 嵌套 GTS (§12.1) 必须 (MUST) 有界：读取器必须 (MUST) 强制执行最大递归深度和跨所有嵌套级别的总解码大小预算（抵御俄罗斯套娃炸弹 resistance）。
- **段 (Segments) 是独立认证的，而非相互担保。** 级联不意味着背书：段 A 的签名者不证明关于段 B 的任何信息。验证器必须 (MUST) 报告每个段的签名者集合 (§14.1)，并且决定信任的使用者不得 (MUST NOT) 将文件级并集视为带有最强段的权威。按值的跨段抑制 (§11) 意味着不受信任的附加段可以从默认解析中**隐藏**早期内容——读取器应该 (SHOULD) 显现哪个段抑制了什么，高保证的使用者可以 (MAY) 仅从其信任签名者的段中解析抑制。
- 段边界处的断裂追加 (torn append) 看起来像断裂头部：适用 §3 断裂追加规则；先前的段保持完整折叠 (fold)。
## 19. 一致性测试向量

符合规范的实现必须 (MUST) 通过共享语料库。v1 要求至少包含这些向量（随参考实现一起发布），每个向量包含 GTS 字节以及预期的折叠图 (N-Quads) 和预期的诊断信息：

配套的 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) 定义了分层一致性声明、命名的向量子集、预期的 JSON 字段、向量清单架构 (vector manifest schema)、诊断注册表以及用于将此语料库转换为可比实现声明的读取/验证模式。
1. 最小有效文件（标头 + 一个 `terms` + 一个 `quads`）。
2. 一个经过 `zstd` 转换的 `quads` 帧。
3. 未知编解码器帧 → 不透明 `reason:"unknown-codec"`。
4. 带有错误自 `"id"` 的帧 → `DamagedFrame` 不透明。
5. EOF 处的撕裂追加 → `TornAppendError`，幸存部分完整。
6. 标头自哈希验证（正向和篡改）。
7. RDF 1.2 reifier + `annot` 往返 (`gts → nq → gts`)，包括无断言引用。
8. 嵌套的 GTS blob (`mt: application/vnd.blackcat.gts+cbor-seq`)，递归且折叠。
9. 针对术语 ID 和帧摘要的抑制。
10. 针对已签名的头 / 索引 `"mmr"` 根的截断检测。
11. 字面量数据类型默认值 (§7.1)：带有 `"l"` + `"dir"` 且没有 `"dt"` 的字面量 →
    `rdf:dirLangString`；带有 `"l"` 且没有 `"dt"` 的字面量 → `rdf:langString`；两者都没有 →
    `xsd:string`。
12. reifier 重新绑定到不同的三元组 → `ConflictingReifier`，保留第一个绑定 (§7.8)。
13. 位置约束违反，例如谓词位置的字面量 → 拒绝/诊断
    (§7.4)。
14. 空白节点标签局部性 (§7.1, §12.1)：外部和嵌套 GTS 中相同的空白节点 (bnode) 标签保持隔离（不合并）。
15. **双段并集 (§3.1)**：两个单段文件的 `cat` 折叠为两个图的值并集；术语 ID 在段局部解析（共享 IRI 统一；命名不同值的相同 ID 不会发生冲突）；跨段的相同空白节点标签保持隔离。
    *15b*: 无标签空白节点（缺失 **或为空** 的 `"v"`）是段内和跨段的截然不同的术语，并集序列化后的标签必须 (MUST) 保持它们的不同 —— 重新标记导致图中分隔的内容被合并是禁止的结果。
16. **组合往返 (§3.1, §14)**：一个由 `cat` 组合的文件在经过 `gts → nq → gts` 后仍具有相同的并集折叠。
17. **前段读取器硬故障 (§17, 负向)**：处于 pre-§3.1 模式的实现在被喂入双段文件时必须 (MUST) 在第二个标头处抛出致命诊断 —— 越过边界并使用文件全局术语 ID 折叠帧是此向量旨在捕获的禁止结果。
18. **跨段抑制 (§11)**：第二个段通过摘要抑制 (a) 较早段的帧，并通过值抑制 (b) 一个四元组；默认解析会隐藏两者；被抑制段的字节经验证是完整的；验证器报告哪个段抑制了什么内容 (§18)。
19. **配置文件并集 + 优雅的段不透明度 (§3.1)**：一个其第二段需要读取器未声明的能力的双段文件，将完整折叠第一段，而将第二段折叠为带有诊断中命名的配置文件的不透明节点。
20. **语言标签规范 (§13.1, 负向)**：生产者将私用语言标签发送到投影/文档部分时必须 (MUST) 在写入时失败；规范 `dist` 有效负载部分中的相同标签是被接受的。
21. **退化组合被拒绝 (§14.1, 负向)**：`gts cat` 拒绝空折叠段和抑制一切的组合；相同输入的原始字节 `cat` 仍然产生结构有效的文件（该工具的设计比格式更严格）。
22. **内联 blob (§12, §14.1)**：内联 blob 折叠为其 `blake3:<hex>` 摘要，并保留声明的元数据 (`pub.mt`)；通过摘要提取可重新验证字节；默认情况下，按摘要抑制的 blob 会被拒绝。
23. **前缀折叠流属性 (§3.2, 派生)**：不是一个向量，而是对本一致性语料库中每个向量的属性测试 —— 每个项边界前缀在折叠时均无错误，且在不断增长的前缀中，折叠表仅会扩展（在段计数不变的情况下，术语/四元组是列表前缀；基础（无空白节点）N-Quads 行在单段到多段表示切换过程中保持单调性）。
24. **可流式处理压缩 (§3.3, §10.1, §13.3)**：一个增量源（blob 交织在它们的目录之前，一个 COSE 签名的帧，无声明）及其压缩后的重写 —— 该重写声明了 `"layout": "streamable"`，以流式索引开头，按重要性从高到低排列 blob，以偏移量 `index` 页脚结束，并携带压缩来源信息（包括分离的源签名）；两个文件折叠为相同的内容图；压缩后的字节是**冻结**的，并充当跨引擎确定性预言机（相同输入 + 相同时间戳参数 ⇒ 在每个引擎中产生字节完全一致的输出）。
25. **撒谎的可流式声明 (§3.3, 负向)**：声明 `"layout": "streamable"` 的段在描述其摘要的四元组之前交付了所涵盖的 blob → `StreamableLayoutError`；验证工具必须 (MUST) 拒绝（以非零状态退出）。
26. **压缩后追加边界 (§3.3)**：在 `index` 页脚之后追加了帧的压缩段可以干净地折叠而无诊断，且工具报告“可流式处理至帧 *N*，增量尾部” —— 未预告的尾部是合法的。
27. **全量读取器敌意回归 (§2.4, 负向)**：空输入、第一个 CBOR 项不是标头、不支持的标头主版本、未知的结构帧类型、前向术语引用以及畸形的转换有效负载均在适用时返回结构化诊断/不透明节点。这些向量固定了“绝不对输入字节产生 Panic”的不变量，并使跨引擎诊断漂移在 CI 中可见。
28. **确定性图写入器 (§14.1)**：具有不同局部术语 ID 和行顺序的两个等效折叠图状态，通过确定性图创作生成字节完全一致的 GTS。冻结向量固定了 Python 和 Rust 生产者之间的术语重映射、行排序、blob 元数据保留、元数据输出以及抑制目标重映射。
## 20. IANA 注意事项

本节注册了一个媒体类型。它遵循 [RFC 6838] 的注册程序和 [RFC 9277] 的结构化语法后缀程序。在正式注册完成前，该类型位于供应商 (`vnd.`) 树中，并临时使用。
### 20.1 媒体类型注册：`application/vnd.blackcat.gts+cbor-seq`

- **类型名称：**`application`
- **子类型名称：**`vnd.blackcat.gts+cbor-seq`
- **必要参数：**无
- **可选参数：**无
- **编码考虑：**二进制。GTS 文件是一个 CBOR 序列 ([RFC 8742])，且不限于 7 位或 8 位文本；非 8 位清洁 (8-bit clean) 的传输必须 (MUST) 应用内容传输编码（例如 base64）。
- **安全考虑：**请参阅本规范的第 18 节。简而言之：内容 ID 链提供完整性但不提供机密性；如果没有头部承诺 (head commitment)，截断是不可检测的；解压缩和嵌套 GTS 递归必须 (MUST) 是有界的；且签名证明的是签署者对字节序列的认可，而非声明内容的真实性。
- **互操作性考虑：**`+cbor-seq` 结构化语法后缀 ([RFC 8742]) 表明有效载荷是一个 CBOR 序列，因此通用序列工具可以在应用 GTS 特定规则之前检查有序数据项。自描述标签 `55799` ([RFC 8949] §3.4.6) 可以 (MAY) 将每个段 (segment) 头部标记为幻数 (magic number)。一致性由共享一致性语料库 (§19) 定义。
- **发布的规范：**本文件 (GTS — Graph Transport Substrate — 规范)。
- **使用此媒体类型的应用程序：**内容寻址的 RDF 1.2 图传输和归档；签名的代理内存和溯源工件；有效载荷捆绑图及其引用的二进制文件的包分发。
- **分段标识符考虑：**无。
- **附加信息：**
  - **幻数：**`0xd9 0xd9 0xf7`（即 CBOR 自描述标签 `55799`），当其出现在文件开头时 (§16.1)。此前缀是可选的 (OPTIONAL)，因为第一个段 (segment) 头部可以 (MAY) 不带标签。
  - **文件扩展名：**`.gts`
  - **Macintosh 文件类型代码：**无
- **联系以获取进一步信息的个人及电子邮箱地址：** Patrick Audley <paudley@blackcatinformatics.ca>
- **预期用途：**常用 (COMMON)
- **使用限制：**无
- **作者 / 变更控制器：**Blackcat Informatics® Inc.
## 21. 完整 CDDL 附录

本附录是供实现者使用的可复制架构定义。本文档中较早出现的内联 CDDL 片段解释了局部上下文；本附录则将线缆级映射 (map) 形状汇总在了一处。
### 21.1 序列语法

GTS 文件是一个 **CBOR 序列**，而不是单个封装的 CBOR 项。CDDL 描述了该序列中的各个项；序列语法以英文和类 ABNF 符号定义：

```text
gts-file = 1*segment
segment  = [ self-describe-tag ] header *frame
```

`self-describe-tag` 是仅应用于 Header 项的 CBOR 标签 55799。它是一个线路级魔术提示，不是 Header 映射的成员，也不是 Header `"id"` 原像 (§22) 的一部分。
每个段 (segment) 都以一个 Header 开始，随后是零个或多个帧 (frame) 项，直到下一个 Header 或 EOF (§3.1)。
### 21.2 可复制的 CDDL

```cddl
; GTS v1 item grammar. The top-level file is a CBOR Sequence (§21.1).

gts-item = header-item / frame
header-item = header / self-described-header
self-described-header = #6.55799(header)

term-id = uint
frame-index = uint
codec-id = uint
digest = bstr .size 32
content-id = digest
blake3-uri = tstr                  ; "blake3:" + 64 lowercase hex characters
digest-ref = digest / blake3-uri
profile-name = tstr
layout-state = "streamable" / tstr
extension-key = tstr               ; any text key not defined by that map shape

header = {
  "gts": "GTS1",
  "v": 1,
  "prof": profile-name,
  "cat": { * codec-id => codec },
  ? "layout": layout-state,
  ? "dct": { * tstr => bstr },
  ? "meta": any,
  "id": content-id,
  * extension-key => any,
}

codec = {
  "name": tstr,
  "cls": "encode" / "compress" / "encrypt",
  ? "dct": tstr,
  ? "p": any,
  * extension-key => any,
}

frame = {
  "t": frame-type,
  ? "x": [+ codec-id],
  ? "pub": any,
  ? "to": [+ recipient],
  ? "d": frame-payload / bstr,
  "prev": content-id,
  "id": content-id,
  ? "sig": cose-sign1,
  * extension-key => any,
}

frame-type = "terms" / "quads" / "reifies" / "annot" / "blob" / "suppress"
/ "snapshot" / "meta" / "index" / "opaque"

recipient = {
  "kid": tstr,
  ? "alg": tstr,
  * extension-key => any,
}

cose-sign1 = bstr                  ; serialized COSE_Sign1, detached payload = frame "id"

frame-payload = terms-payload / quads-payload / reifies-payload / annot-payload
/ blob-payload / suppress-payload / snapshot-payload / meta-payload
/ index-payload / opaque-node

terms-payload = [+ term]
term = {
  "k": 0 / 1 / 2 / 3,              ; 0=IRI, 1=literal, 2=bnode, 3=quoted triple
  ? "v": tstr,
  ? "dt": term-id,
  ? "l": tstr,
  ? "dir": "ltr" / "rtl",          ; RDF 1.2 base direction for language-tagged literals
  ? "rf": term-id,
  * extension-key => any,
}

triple-row = [term-id, term-id, term-id]
quad-row = [term-id, term-id, term-id] / [term-id, term-id, term-id, term-id]

quads-payload = [+ quad-row]
reifies-payload = { * term-id => triple-row }
annot-payload = [+ triple-row]

blob-payload = bstr
blob-pub = {
  ? "mt": tstr,
  ? "rep": tstr,
  ? "digest": digest-ref,
  * extension-key => any,
}

suppress-payload = {
  "targets": [+ suppress-target],
  ? "reason": tstr,
  ? "by": term-id,
  * extension-key => any,
}

suppress-target = suppress-frame / suppress-blob / suppress-term
/ suppress-quad / suppress-reifier
suppress-frame = { "kind": "frame", "id": digest-ref, * extension-key => any }
suppress-blob = { "kind": "blob", "digest": digest-ref, * extension-key => any }
suppress-term = { "kind": "term", "id": term-id, * extension-key => any }
suppress-quad = { "kind": "quad", "q": quad-row, * extension-key => any }
suppress-reifier = { "kind": "reifier", "id": term-id, * extension-key => any }

snapshot-payload = {
  "terms": terms-payload,
  ? "quads": quads-payload,
  ? "reifies": reifies-payload,
  ? "annot": annot-payload,
  ? "blobs": { * digest-ref => bstr },
  ? "meta": any,
  * extension-key => any,
}

meta-payload = any

index-payload = {
  "count": uint,
  "head": content-id,
  ? "off": [+ uint],
  ? "ti": { * frame-type => [+ frame-index] },
  ? "dict": [+ frame-index],
  ? "mmr": content-id,
  * extension-key => any,
}

opaque-node = {
  "id": content-id,
  "type": frame-type,
  ? "pub": any,
  ? "to": [+ recipient],
  ? "sigstat": sig-status,
  "reason": opaque-reason,
  * extension-key => any,
}

sig-status = "none" / "valid" / "invalid" / "unverified"
opaque-reason = "unknown-codec" / "missing-key" / "damaged"
/ "unknown-frame-type"

diagnostic = {
  "code": diagnostic-code,
  "detail": tstr,
  ? "frame_index": frame-index,
  * extension-key => any,
}

diagnostic-code = "EmptyFile"
/ "TornAppendError" / "DamagedFrame" / "BrokenChain"
/ "TruncatedLog" / "UnknownCodec" / "MissingKey"
/ "KeyWrapFailed" / "ConflictingReifier" / "IllTypedLiteral"
/ "RecursionLimit" / "StreamableLayoutError" / "PositionConstraint"
/ "ForwardReference" / "SegmentBoundary" / "IndexMmrError"
/ "UnknownFrameType" / tstr

profile-status = "core-required" / "optional-standard" / "experimental"
/ "domain-specific"
profile-registration = {
  "name": profile-name,
  "status": profile-status,
  ? "owner": tstr,
  ? "spec": tstr,
  ? "namespace": [+ tstr],
  ? "requires": any,
  ? "validation": any,
  ? "security": any,
  * extension-key => any,
}
```
当 ``"x"`` 存在且非空时，帧 ``"d"`` 的值是一个携带已编码/压缩/加密有效负载的字节字符串。在反转转换链 (§6.1) 后，这些字节解码为上述帧类型特定的有效负载，``blob`` 除外，其解码后的有效负载为原始字节。当 ``"x"`` 缺失时，``"d"`` 直接携带帧类型特定的有效负载。

``blob-pub`` 是 blob 帧的 ``"pub"`` 映射的常规形状；帧包络将 ``"pub"`` 的类型保持为 ``any``，以便配置文件 (profile) 可以在不更改核心帧语法的情况下分层添加额外的公共元数据。``digest-ref`` 接受原始 32 字节摘要和参考引擎使用的 ``blake3:<hex>`` 文本形式。
## 22. 哈希、签名与扩展键原像

本节中的所有原像均使用 §4 中的确定性 CBOR 规则：定长 (definite lengths)、最短形式整数，以及按其编码后的 CBOR 形式进行逐字节排序的地图键 (map keys)。除非某行明确排除某个字段，否则地图中的每个键/值对均参与计算，包括未知的扩展键。
### 22.1 原像与主体表 (Preimage and subject table)

| 主体 | 被哈希或签名的字节 | 排除的字段 | 包含的扩展字段 | 验证器行为 |
|---|---|---|---|---|
| Header `"id"` | `BLAKE3-256(deterministic-CBOR(header-map without "id"))` | 仅限 `"id"`。可选的 CBOR 自描述标签 55799 位于 Header 映射之外，且不在原像中。 | 所有未知的 Header 键均参与。 | 在接受段 (segment) Header 之前重新计算；不匹配视为 Header 被篡改。 |
| 帧 (Frame) `"id"` | `BLAKE3-256(deterministic-CBOR(frame-map without "id" and "sig"))` | 仅限 `"id"` 和 `"sig"`。 | 所有未知的帧键均参与。 | 对每个帧重新计算；不匹配为 `DamagedFrame`。 |
| 帧 (Frame) `"prev"` 链接 | `"prev"` 值包含在帧 `"id"` 原像中。 | 除了帧 `"id"` 的排除项之外没有其他项。 | 未知的帧键不会改变 `"prev"` 语义，但仍包含在帧 `"id"` 原像中。 | 与同一段 (segment) 内前一项的 `"id"` 进行比较；不匹配为 `BrokenChain`。 |
| COSE 帧签名 | 基于帧 `"id"` 字节的分离式 (Detached) COSE_Sign1。COSE Sig_structure 为 `["Signature1", protected, h'', frame-id]`；COSE 有效负载字段为 `null`/detached。 | 由于 `"sig"` 被排除，签名不属于帧 `"id"` 原像的一部分。 | 扩展键通过改变帧 `"id"` 间接影响签名。 | 使用由 `kid` 解析的密钥进行验证；报告 `valid`、`invalid` 或 `unverified`。 |
| 内联 blob 摘要 | `BLAKE3-256(decoded blob bytes)`，在反转转换和解密（如果可用）之后。 | 帧包络字段不属于 blob 摘要的一部分。 | Blob 公共扩展键不影响 blob 摘要，但会影响包含该内容的帧 `"id"`。 | 当存在 `pub.digest` 时进行比较，并与命名该 blob 的图引用进行比较。 |
| 外部 blob 摘要 | `pub.digest` 命名存储在别处的字节；摘要主体是这些外部字节。 | 外部字节不在 GTS 帧中，因此只有摘要声明通过 `"pub"` 参与帧 `"id"`。 | 未知的公共元数据参与帧 `"id"`，而非外部 blob 摘要。 | 验证器仅在获取外部字节时才能进行检查。 |
| 索引 (Index) `"head"` | 最后一个覆盖帧的 content-id，其中 `"count"` 是索引有效负载覆盖的帧数。 | 不适用。 | 未知的索引有效负载键参与索引帧 `"id"`，不参与 `"head"` 主体。 | 将 `"head"` 与覆盖的帧 id 进行比较；不匹配会使索引/布局声明失效。 |
| 索引 (Index) `"mmr"` | 索引所覆盖的有序帧 id 之上的 Merkle-Mountain-Range 根，使用 §6.2 中的叶/父/根原像。 | 除非后续索引覆盖它，否则索引帧本身不被覆盖。 | 未知的索引有效负载键参与索引帧 `"id"`，不参与 MMR 根。 | 用作可选的全覆盖区域承诺和证明根；不匹配为 `IndexMmrError`。 |
| 分离式签名出处 | `stream:sourceFrame` 命名原始帧 `"id"`；`stream:cose` 携带原始 COSE_Sign1 字节。签名仍对原始帧 id 进行验证。 | 重写帧的新 `"id"` 不是旧签名的主体。 | 出处图扩展术语不会改变原始签名主体。 | 根据 `stream:sourceFrame` 验证携带的签名；不要将其视为对压缩帧的签名。 |
### 22.2 未知扩展键行为

扩展键是指该映射（map）的 CDDL 产生式中未定义的文本字符串映射键。定义的保留键，如 `"id"`、`"sig"`、`"prev"`、`"t"`、`"d"`、`"x"`、`"pub"` 和 `"to"`，不属于扩展键，且 配置文件 (profiles) 不得 (MUST NOT) 对其进行重新利用。

读取器 (Readers) 在重新计算 Header 和 帧 (frame) 预映射 (preimages) 时 必须 (MUST) 包含未知的扩展键。读取器 (reader) 不得 (MUST NOT) 仅因为 Header、帧 (frame)、编解码器 (codec)、接收者 (recipient)、术语 (term)、负载 (payload)、不透明节点 (opaque-node)、诊断 (diagnostic) 或 配置文件注册 (profile-registration) 映射中包含未知扩展键而拒绝该映射。除非受支持的 配置文件 (profile) 或扩展定义了它们，否则未知键没有核心 折叠 (fold) 语义。

重新发出（Re-emit）行为取决于具体操作：

- 字节保留操作，例如原始 `cat`、复制、镜像或分发，会自然地保留未知键，因为它们保留了原始字节。
- 解码并重新发出 Header 或 帧 (frame) 且声称保留相同逻辑项的工具，在重新计算 `"id"` 值之前，必须 (MUST) 逐字复制未知的扩展键。
- 无法保留未知扩展键的工具 必须 (MUST) 将该操作视为有损重构 (lossy re-authoring)，必须 (MUST) 重新计算受影响的 `"id"` 和 `"prev"` 值，并且 不得 (MUST NOT) 声称现有的帧签名仍附着在重写的帧上。
- 压缩器或其他重构工具 可以 (MAY) 仅将旧的帧签名保留为分离的出处 (detached provenance) (§10.1)，其中旧的帧 ID 仍作为显式的签名主体。

由于扩展键参与预映射，扩展作者可以在不更改核心 GTS 语法的情况下添加防篡改的元数据。他们无法更改头部/帧语法、哈希预映射、签名主体或 折叠 (fold) 语义 (§2.1, §13)。
## 23. 参考文献
### 23.1 规范性引用文件

- **[RFC 2119]** Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, March 1997.
- **[RFC 8174]** Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, May 2017.
- **[RFC 8949]** Bormann, C. and P. Hoffman, "Concise Binary Object Representation (CBOR)", STD 94, December 2020.
- **[RFC 8742]** Bormann, C., "Concise Binary Object Representation (CBOR) Sequences", February 2020.
- **[RFC 9052]** Schaad, J., "CBOR Object Signing and Encryption (COSE): Structures and Process", STD 96, August 2022.
- **[RFC 9053]** Schaad, J., "CBOR Object Signing and Encryption (COSE): Initial Algorithms", August 2022.
- **[RFC 9277]** Bormann, C. and M. Nottingham, "On the Use of Structured Suffixes in Media Types", June 2022.
- **[RFC 6838]** Freed, N., Klensin, J., and T. Hansen, "Media Type Specifications and Registration Procedures", BCP 13, January 2013.
- **[RFC 3339]** Klyne, G. and C. Newman, "Date and Time on the Internet: Timestamps", July 2002.
- **[BCP 47]** Phillips, A. and M. Davis, "Tags for Identifying Languages", September 2009.
- **[BLAKE3]** O'Connor, J., Aumasson, J-P., Neves, S., and Z. Wilcox-O'Hearn, "BLAKE3: one function, fast everywhere" (此处使用 256-bit 输出)。
- **[RDF 1.2]** W3C, "RDF 1.2 Concepts and Abstract Data Model", Candidate Recommendation Snapshot, 07 April 2026, <https://www.w3.org/TR/2026/CR-rdf12-concepts-20260407/> — 由 §7 导入的 RDF 术语、数据集模型、三元组项 (triple-term) 以及 `rdf:reifies` 底层 (substrate)。
### 23.2 资料性引用

- **[RFC 7049]** Bormann, C. 和 P. Hoffman, "Concise Binary Object Representation (CBOR)", 2013年10月 (已被 [RFC 8949] 废止；仅因其遗留的长度优先“规范”排序而被引用，§4)。
- **[RFC 8610]** Birkholz, H., Vigano, C., 和 C. Bormann, "Concise Data Definition Language (CDDL)", 2019年6月。
- **[RFC 9111]** Fielding, R., Nottingham, M., 和 J. Reschke, "HTTP Caching", 2022年6月 (§16.4 的缓存指令)。
- **[RFC 6906]** Wilde, E., "The 'profile' Link Relation Type", 2013年3月 (§16.4 中提到的 `Accept-Profile` 未来扩展)。

---

*GTS 旨在作为一种传输格式，而非本体或图存储。符合标准的实现会保留仅追加 (append-only)、内容寻址 (content-addressed) 的折叠 (fold)，以便可以从相同的字节重新生成独立的投影。*

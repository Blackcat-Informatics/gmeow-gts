<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 第三方实现者指南

> [`docs/GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md`](../../../../docs/GTS-THIRD-PARTY-IMPLEMENTER-GUIDE.md) 的信息性中文翻译。英文文档仍然是兼容性规则、一致性声明、对等矩阵、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

本指南适用于希望构建独立 GTS 读取器并提出可测试的基准读取器 (Baseline Reader) 一致性声明的实现者。本指南是非规范性的。传输格式仍由 [`GTS-SPEC.md`](./GTS-SPEC.md) 定义，一致性声明仍由 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) 定义。

请将本指南用作构建顺序和核对清单：

1. 实现能够解析、验证和折叠基准语料库的最小读取器。
2. 移植 vector-manifest 测试框架。
3. 发布一致性声明，其中指明确切的语料库修订版本和命令。

## 规范性锚点

请勿将这些规则作为独立要求复制到实现指南或 README 中。请链接到所属章节并据此进行实现。

| 主题 | 规范所有者 |
|---|---|
| 文件结构、段和 cat-append 组合 | [`GTS-SPEC.md` 第 3 和 3.1 节](./GTS-SPEC.md#3-file-structure) |
| CBOR 约定和确定性编码 | [`GTS-SPEC.md` 第 4 节](./GTS-SPEC.md#4-cbor-conventions) |
| 标头映射 | [`GTS-SPEC.md` 第 5 节](./GTS-SPEC.md#5-header) |
| 帧映射和有效负载解析 | [`GTS-SPEC.md` 第 6 和 6.1 节](./GTS-SPEC.md#6-frames) |
| 图模型和折叠算法 | [`GTS-SPEC.md` 第 7 节](./GTS-SPEC.md#7-graph-data-model-and-fold) |
| 不透明节点 | [`GTS-SPEC.md` 第 7.6 节](./GTS-SPEC.md#76-opaque-nodes) |
| 强制性编解码器 | [`GTS-SPEC.md` 第 8.4 节](./GTS-SPEC.md#84-mandatory-core-set-and-durability) |
| 帧链验证 | [`GTS-SPEC.md` 第 9.1 节](./GTS-SPEC.md#91-per-frame-self-hash-and-content-id-chain-mandatory) |
| 完整的 CDDL | [`GTS-SPEC.md` 第 21 节](./GTS-SPEC.md#21-complete-cddl-appendix) |
| 哈希、签名和扩展键原像 | [`GTS-SPEC.md` 第 22 节](./GTS-SPEC.md#22-hash-signature-and-extension-key-preimages) |
| 基准读取器层级 | [`GTS-CONFORMANCE.md` 第 3 节](./GTS-CONFORMANCE.md#3-tiers) |
| 向量清单架构 | [`GTS-CONFORMANCE.md` 第 5 节](./GTS-CONFORMANCE.md#5-vector-manifest-schema) |
| 诊断注册表 | [`GTS-CONFORMANCE.md` 第 6 节](./GTS-CONFORMANCE.md#6-diagnostics-registry) |
| 读取和验证模式 | [`GTS-CONFORMANCE.md` 第 7 节](./GTS-CONFORMANCE.md#7-read-and-verify-modes) |
| 第三方配置文件注册 | [`GTS-SPEC.md` 第 13 节](./GTS-SPEC.md#13-profiles) |

## 最低限度的基准读取器工作

基准读取器 (Baseline Reader) 是最小的有用的独立实现。它以宽容读取 (permissive-read) 模式读取 GTS 字节，验证内容 ID 链 (content-id chain) 直至足以展示诊断信息，折叠 (fold) 可恢复的图内容，并将未知或不受支持的内容保留为不透明节点 (opaque nodes)。

最低限度的工作：

- 解析 CBOR 序列 (CBOR Sequence)，而不是单个全文件 CBOR 对象。
- 检测段头 (segment headers) 和帧 (frames)。
- 当可选的 CBOR 自描述标签 `55799` 标记段头时，接受它。
- 从 CDDL 附录解码 Header 和 Frame 形状。
- 使用原像表 (preimage table) 重新计算 Header 和 Frame ID。
- 检查每个帧的 `prev` 链接，对比同一段中的前一项 ID。
- 实现强制转换栈：`identity`、`gzip` 和 `zstd`。
- 根据折叠算法折叠 (Fold) `terms`、`quads`、reifiers、注解 (annotations)、抑制 (suppressions)、blob、元数据 (metadata)、诊断 (diagnostics)、段账本 (segment ledgers)、签名 (signatures) 以及不透明节点。
- 针对畸形语料库输入返回诊断信息，而不是恐慌 (panicking)。
- 在可以恢复时，将无法解码、不受支持、未提供密钥的加密内容或损坏的内容保留为不透明节点。
- 将折叠后的输出与向量清单 (vector manifest) 指定的预期 JSON 字段进行比较。

基准读取器不需要实现：

- COSE 签名验证或加密支持。
- OpenPGP 密钥提取。
- 嵌套 GTS (Nested-GTS) 递归。
- MMR/索引证明验证。
- 流事件 (Stream events)。
- 写入器确定性 (Writer determinism)。
- 严格发布工具。
- 配置文件感知 (Profile-aware) 策略验证。
- 数据库、Parquet、对象存储或范围获取 (range-fetch) 辅助程序。

这些功能可以在以后添加，并在相应的流式读取器 (Streaming Reader)、全量读取器 (Full Reader)、写入器 (Writer)、验证工具 (Validating Tool) 或配置文件感知工具 (Profile-Aware Tool) 等级下进行声明。

## 建议的读取器流水线

具体的 API 取决于具体实现，但此流水线与一致性文档相匹配：

```text
bytes
  -> CBOR Sequence item iterator
  -> segment boundary detector
  -> Header validator
  -> Frame validator
  -> transform resolver
  -> frame-payload decoder
  -> fold accumulator
  -> Graph plus diagnostics, segment heads, opaque nodes, and metadata
```

伪代码：

```text
read_gts(bytes):
  items = parse_cbor_sequence(bytes)
  result = empty_graph()
  current_segment = none
  previous_id = none

  for item in items:
    if is_segment_header(item):
      current_segment = validate_header(item)
      previous_id = current_segment.id
      result.segments.append(current_segment.summary)
      continue

    frame = validate_frame_envelope(item, previous_id)
    previous_id = frame.id

    if frame.envelope_is_damaged:
      result.add_diagnostic("DamagedFrame")
      result.add_opaque(frame, reason="damaged")
      continue

    payload = resolve_transforms(frame)
    if payload.is_unsupported:
      result.add_diagnostic(payload.diagnostic)
      result.add_opaque(frame, reason=payload.opaque_reason)
      continue

    fold_payload(result, frame.type, payload)

  return result
```

重要的属性是完整性 (totality) 和可观测性 (observability)：格式错误或不受支持的语料库输入必须 (MUST) 返回带有诊断信息的结果，而不是中止进程。

## 使用 `vectors/manifest.core.json`

核心清单是便携式 Baseline Reader 一致性索引。它为每个矢量指定了输入文件、预期的图 JSON、所需功能、子集、层级、诊断和注释。聚合的 `vectors/manifest.json` 还包括可选的配置文件 (profile/配置文件)、transform、crypto、proof 和 human-hash 固定装置，这些对于完整的存储库检查很有用，但不是 Baseline Reader 的起点。

从核心清单中其 `tiers` 包含 `baseline-reader` 的矢量开始：

```bash
python - <<'PY'
import json
from pathlib import Path

manifest = json.loads(Path("vectors/manifest.core.json").read_text())
for vector in manifest["vectors"]:
    if "baseline-reader" in vector["tiers"]:
        expected = vector["expected"].get("graph")
        print(vector["id"], vector["input"]["path"], expected)
PY
```

对于每个选定的矢量：

1. 将 `input.path` 读取为字节。
2. 以 permissive-read 模式运行读取器 (reader/读取器)。
3. 当 `expected.graph` 不是 `null` 时加载它。
4. 比较清单命名的预期字段：counts、diagnostics、段 (segment) 头部、不透明原因 (opaque reasons)、blob 摘要、可流式处理 (streamable) 状态、配置文件 (profile/配置文件) 和 N-Quads。
5. 将 `negative: true` 视为“预期诊断/拒绝行为”，而不是“进程应该失败或 panic”。
6. 仅当 `required_capabilities` 命名了所声明层级之外的功能时，才记录跳过的矢量。

在使用清单作为发布或报告工件之前，先对其进行验证：

```bash
python scripts/check_vector_manifest.py
python scripts/check_vector_manifest.py --self-test
```

发布报告不应该 (SHOULD NOT) 引用检入的占位符 `git:repository-commit-containing-manifest` 作为语料库 (corpus)。为报告标记一个确切的修订版本：

```bash
python scripts/check_vector_manifest.py \
  --release-manifest dist/vector-manifest.release.json
```

## 预期 JSON 对比

当前的顶层语料库对比的是折叠图摘要，而非私有内部对象模型。实现可以使用其自身的数据结构，只要它能生成等效的字段。

至少对比：

- `diagnostics`：有序诊断代码列表。
- `terms`、`quads`、`segments` 和 `suppressions`：折叠计数摘要。
- `segment_heads`：按文件顺序排列的段 (segment) 头 ID。
- `profiles`：折叠配置文件 (profile) 声明。
- `streamable`：逐段布局状态。
- `opaque_reasons`：已排序的不透明原因。
- `blobs`：内联 blob 摘要、媒体类型和解码大小摘要。
- `nquads`：已排序的 RDF 投影行。

除非清单 (manifest) 将向量 (vector) 缩小为仅同构对比，否则空节点标签应与参考渲染器匹配。参见 [`GTS-CONFORMANCE.md` 第 4 节](./GTS-CONFORMANCE.md#4-expected-graph-format) 中的预期图格式。

## 诊断与不透明节点

诊断是一致性声明 (conformance claim) 公开行为的一部分。不要重命名您所声明层级 (tier) 所拥有的代码。

基准读取器 (Baseline Reader) 诊断包括格式错误或恶意输入行为，例如 `EmptyFile`、`DamagedFrame`、`BrokenChain`、`TornAppendError`、`UnknownCodec`、`ConflictingReifier`、`PositionConstraint`、`ForwardReference` 以及 `SegmentBoundary`。

不透明节点 (Opaque-node) 行为是保持读取器 (reader) 完备性 (total) 的关键：

- 未知编解码器 (codec)：使用 `reason:"unknown-codec"` 将帧 (frame) 保留为不透明节点。
- 缺少解密密钥：当存在加密支持但缺少密钥时，使用 `reason:"missing-key"` 将帧保留为不透明节点。
- 损坏的可恢复帧：当项边界已知时，将损坏的内容隔离为不透明。
- 未知结构帧类型：保留链验证，并忽略负载或将其作为不透明显现，直到受支持的配置文件 (profile) 对其进行处理。`UnknownFrameType` 是一致性注册表中的配置文件感知工具 (Profile-Aware Tool) 诊断，不属于基准读取器声明字符串的一部分。

不透明节点不是数据丢失。它是一种机器可读的陈述，表明读取器携带了其无法安全解码或解释的内容。

## 配置文件 (Profile) 注册基础

配置文件位于核心线路格式之上。领域配置文件可以 (MAY) 定义词汇表、验证规则、信任策略、发布工作流以及特定于配置文件的矢量，但不得 (MUST NOT) 更改：

- 头部或帧 (frame) 语法。
- 段 (segment) 边界检测。
- 内容 ID、签名或哈希原像。
- 转换目录解析。
- 确定性折叠 (fold) 语义。

基准读取器 (Baseline Reader) 应该 (SHOULD) 将配置文件声明和要求作为折叠元数据、诊断或不透明原因公开。声称符合基准读取器 (Baseline Reader) 一致性并不需要强制执行配置文件策略。

第三方配置文件应该 (SHOULD) 发布 [`GTS-SPEC.md` section 13](./GTS-SPEC.md#13-profiles) 中列出的配置文件注册字段，包括稳定的令牌或 URI、所有者/变更控制器、用途、所需词汇表、验证规则、失败分类、安全/隐私注意事项、版本控制策略和一致性矢量。

## 示例 Baseline 读取器声明

```text
Implementation: ExampleGTS Reader 0.1.0
Conformance tier: GTS Baseline Reader
Corpus revision: git:0123456789abcdef0123456789abcdef01234567
Read mode: permissive-read
Vector subsets passed: wire-core, total-reader, graph-fold
Capabilities enabled: cbor, blake3, identity, gzip, zstd
Command: example-gts-conformance --manifest vectors/manifest.core.json --tier baseline-reader
Skipped vectors: none for the claimed tier
Optional capabilities not claimed: signatures, encryption, nested GTS, MMR proofs, profile policy
```

如果您仅声明 Baseline 读取器行为的一个子集，请勿称其为 Baseline 读取器一致性。在所需的子集通过之前，请使用特定于实现的短语，例如“实验性读取器 (experimental reader)”。

## 常见陷阱

- 将文件视为单个封闭的 CBOR 对象，而不是 CBOR Sequence。
- 在包含 `id` 键的情况下对 Header 进行哈希。
- 在包含 `id` 或 `sig` 键的情况下对帧 (frame) 进行哈希。
- 丢弃未知的编解码器 (codecs)，而不是保留不透明节点 (opaque nodes)。
- 在负向向量 (negative vectors) 上使处理失败，而不是返回诊断信息 (diagnostics)。
- 在重新计算原像 (preimages) 时忽略未知的扩展键 (extension keys)。
- 将配置文件 (profile) 策略失败视为核心传输格式 (wire-format) 无效。
- 仅因为解析了签名就声称具有完全读取器 (Full Reader) 行为，即使缺少签名验证、密钥解析或信任策略行为。
- 仅比较 N-Quads，而忽略了诊断信息 (diagnostics)、不透明原因 (opaque reasons)、段首 (segment heads)、可流式处理状态 (streamable state)、配置文件 (profiles) 以及 blob 摘要 (blob summaries)。

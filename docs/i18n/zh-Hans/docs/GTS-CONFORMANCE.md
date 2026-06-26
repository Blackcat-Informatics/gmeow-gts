<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-CONFORMANCE.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 一致性

> [`docs/GTS-CONFORMANCE.md`](../../../../docs/GTS-CONFORMANCE.md) 的信息性中文翻译。英文文档仍然是兼容性规则、一致性声明、对等矩阵、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

本文档定义了实现如何针对 Graph Transport Substrate (GTS) 提交可测试的一致性声明。它是 [`GTS-SPEC.md`](./GTS-SPEC.md) 的配套文档：规范定义了线缆格式和行为；而本文档则定义了用于比较实现的层级、向量子集、预期结果格式、诊断信息以及读取模式。

## 1. 一致性声明 (Conformance Claims)

一致性声明必须 (MUST) 命名：

- 实现名称和版本；
- 所声明的一个或多个一致性层级 (§3)；
- 所使用的读取模式 (read mode) 或验证模式 (verify mode) (§7)；
- 语料库版本，通常是包含 `vectors/` 的仓库提交；
- 通过的向量子集 (§2)；
- 启用的任何可选功能，例如 COSE 密钥、加密密钥、配置文件 (profile) 验证器或嵌套 GTS 递归；
- 用于生成结果的确切命令或测试框架 (test harness)。

该声明仅对指定的层级和功能集有意义。例如，基准读取器 (Baseline Reader) 可以通过 `baseline-reader`，而无需声明支持 COSE 签名验证、解密、嵌套 GTS 递归或配置文件 (profile) 策略强制执行。

## 2. 向量子集

冻结语料库目前为每个用例包含一个顶层 `vectors/<id>.gts` 字节文件和一个 `vectors/<id>.expected.json` 预期折叠 (fold)。额外的 JSON 子语料库涵盖了 COSE、Encrypt0、OpenPGP key extraction、emojihash 和 randomart。这些命名的子集是 Tier 声明所使用的单元：

| 子集 | 向量 | 用途 |
|---|---|---|
| `wire-core` | `01-minimal`, `02-zstd-frame`, `06-header-tampered` | 帧头/帧语法，强制编解码器，确定性 CBOR，以及帧头哈希行为。 |
| `total-reader` | `03-unknown-codec`, `04-damaged-frame`, `05-torn-append`, `17-pre-segment-hard-fail`, `19-profile-union-opacity`, `28-empty-file`, `28b-non-header-item`, `28c-unsupported-version`, `28d-unknown-frame-type`, `28e-forward-term-reference`, `28f-malformed-transform-shape`, `28g-damaged-compressed-payload`, `28h-malformed-security-metadata` | 平稳退化，诊断，不透明节点，不完整输入，畸形/边界行为，不支持的帧头，受损的压缩负载，畸形的安全性元数据，以及扩展帧不透明度。 |
| `graph-fold` | `09-suppression`, `11-datatype-defaulting`, `12-conflicting-reifier`, `13-position-constraint`, `14-bnode-label`, `15-two-segment-union`, `15b-anon-bnode-union`, `16-composed-round-trip`, `18-cross-segment-suppression`, `22-inline-blob` | 核心图折叠，值相等性，注释/具体化器，抑制，二进制大对象 (blobs)，以及多段并集。 |
| `profile-layout` | `20-language-tag-discipline`, `21-degenerate-composition`, `23-files-profile-tree`, `24-files-profile-dedup`, `25-streamable-source`, `25b-streamable-compacted`, `26-streamable-lie`, `27-streamable-tail` | 配置文件惯例，归档/文件配置文件行为，可流式处理布局，紧凑化，以及发布工具拒绝案例。活跃的 `scripts/interop.sh` 守卫为此子集添加了跨引擎的 `files` pack/unpack/diff 命令证据。 |
| `okf-bundle` | `vectors/okf/*` Markdown 捆绑目录 | OKF 配置文件导入测试固定装置，折叠图预期，以及配置文件感知工具的未映射边车行为。 |
| `tar-archive` | `vectors/tar/*.tar`, `vectors/tar/*.tar.gz`, `vectors/tar/*.tar.zst` | Tar 导入/导出转换测试固定装置，包括正向归档投影和不安全归档拒绝案例。 |
| `resilience-negative` | `03-unknown-codec`, `04-damaged-frame`, `05-torn-append`, `06-header-tampered`, `17-pre-segment-hard-fail`, `19-profile-union-opacity`, `21-degenerate-composition`, `26-streamable-lie`, `28-empty-file`, `28b-non-header-item`, `28c-unsupported-version`, `28d-unknown-frame-type`, `28e-forward-term-reference`, `28f-malformed-transform-shape`, `28g-damaged-compressed-payload`, `28h-malformed-security-metadata` | 针对对抗性顶级输入的审计覆盖：截断的 CBOR，受损的帧，受损的压缩，错误的段边界，畸形的转换/配置文件/安全性元数据，空/非帧头输入，以及受限大小的拒绝/诊断行为。 |
| `streaming-property` | 每个顶级 `vectors/*.gts`，在每个 CBOR 项目边界进行测试 | 针对流式读取器的前缀折叠完整性和单调折叠增长。 |
| `corpus-generator-determinism` | 每个顶级 `vectors/*.gts` | 已冻结语料库的参考生成器可重现性，包括故意损坏、不完整、被篡改和畸形的测试固定装置。这证明了语料库构建的可重复性，而非公开写入器的一致性。 |
| `writer-determinism` | 有效的顶级写入器输出，包括作为可流式处理紧凑化字节预测器的 `25b-streamable-compacted` 和作为图创作字节预测器的 `29-deterministic-writer` | 公开写入器输出的可重现性，确定性哈希，确定性图创作，以及固定参数下的确定性紧凑化。负向语料库测试固定装置不得 (MUST NOT) 使用此子集。 |
| `crypto-cose` | `vectors/cose/*.json`, `vectors/signed/basic.json` | COSE Sign1 序列化，逐帧签名，以及签名验证行为。 |
| `crypto-encrypt` | `vectors/encrypt0/basic.json` | 实现加密的引擎的 COSE Encrypt0 密封/开启行为。 |
| `crypto-deferred` | `vectors/crypto-deferred/*.json` | 推迟的多接收者 `COSE_Encrypt` 和 ECDH-ES+A256KW 合约描述符。这些向量防止过早的支持声明；在字节级测试固定装置和互操作测试工具替换这些占位符之前，它们不是 v1 实现向量。 |
| `openpgp-transport-key` | `vectors/openpgp/*.json` | 嵌入式 OpenPGP 传输密钥提取以及跨引擎指纹/emojihash 一致性。 |
| `human-hash` | `vectors/emojihash/*.json`, `vectors/randomart/*.json` | CLI 和发布工具使用的面向人类的摘要渲染。 |
| `security-policy` | `vectors/security/*.json` | 配置文件信任策略分离，匿名不透明接收者，以及嵌套 GTS 递归限制负面案例。 |
| `advanced-index-proof` | `vectors/proofs/*.json` 加上实现创建的索引文件 | 稳定的 MMR 原像，分离的包含证明 JSON 验证，错误证明拒绝，以及可选的 `index.mmr` 读取器诊断。 |

一个层级可以 (MAY) 要求一个子集以及额外的特定于模式的断言。例如，`profile-layout` 包含宽容的读取器 (readers) 可以折叠 (fold) 为本地 GTS 字节的文件，而验证工具还必须 (MUST) 拒绝特定的 publish-class 或 verify-class 违规行为。

已提交的有范围清单 (manifests) 按一致性表面 (conformance surface) 对这些子集进行分组：
`vectors/manifest.core.json` 包含核心有线格式 (wire-format) 读取器 (reader)/写入器 (writer) 语料库，
`vectors/manifest.profiles.json` 包含配置文件 (profile) 和配置文件策略 (profile-policy) 固件 (fixtures)，
`vectors/manifest.transforms.json` 包含转换/工具固件，以及
`vectors/manifest.json` 仍然是用于仓库范围内检查的聚合清单。

`resilience-negative` 子集是一个审计覆盖层 (audit overlay)，而不是一个单独的层级。每个条目都是一个顶层 GTS 向量，被标记为 negative，保持在受限的已提交字节大小内，并具有一份说明诊断或拒绝结果的清单预期 (manifest expectation)。因为仓库的全引擎测试框架 (harnesses) 枚举顶层 `vectors/*.gts`，Python、Rust、Go、TypeScript、Kotlin 和 Smalltalk 消费相同的抗毁性负面 (resilience-negative) 字节文件，并将其与相同的 `*.expected.json` 结果进行比较。JSON 安全策略固件 (fixtures) 保留在 `security-policy` 中，用于感知配置文件 (profile-aware) 的信任策略 (trust-policy) 和嵌套 GTS (nested-GTS) 递归断言。

## 3. 层级

| 层级 | 要求的子集和检查 | 声明字符串 |
|---|---|---|
| 基础读取器 (Baseline Reader) | `wire-core`、`total-reader`、`graph-fold` 及其在宽容读取模式下的核心 `resilience-negative` 覆盖层；预期的图 JSON 匹配；诊断匹配；畸形输入绝不引发 panic 或中止进程。 | `GTS Baseline Reader, corpus <commit>` |
| 流式读取器 (Streaming Reader) | 基础读取器外加 `streaming-property`；实现暴露了一个非实例化汇 (sink) API，该 API 在保留最终诊断和段头的同时发送段 (segment) 本地折叠 (fold) 事件。保留内存预计受限于 `O(distinct terms + maximum decoded frame size + validation sidecar state)`，而非折叠的三元组或 blob。 | `GTS Streaming Reader, corpus <commit>` |
| 完整读取器 (Full Reader) | 基础读取器外加已实现的可选子集，在声明签名支持时至少包含用于签名验证的 `crypto-cose`，在声明解密支持时包含 `crypto-encrypt`，在声明嵌套 GTS 递归时包含 `security-policy`，以及存在时的索引/MMR 行为。 | `GTS Full Reader (<capabilities>), corpus <commit>` |
| 写入器 (Writer) | 在规范要求确定性输出的情况下，发出的字节是确定性的，且写入器创建的文件符合基础读取器的预期。故意失效的语料库固定装置 (corpus fixtures) 的可重现生成由 `corpus-generator-determinism` 涵盖，并不暗示公开写入器的一致性。 | `GTS Writer, corpus <commit>` |
| 验证工具 (Validating Tool) | 基础读取器外加严格验证和发布类验证模式 (§7)；`profile-layout` 拒绝向量会产生所需的非零/拒绝结果。 | `GTS Validating Tool, corpus <commit>` |
| 配置文件感知工具 (Profile-Aware Tool) | 验证工具外加具名配置文件验证器；配置文件特定的诊断和警告符合配置文件契约。 | `GTS Profile-Aware Tool (<profile>), corpus <commit>` |
| 转换工具 (Transform Tool) | 具名转换或归档操作根据其配置文件/工具契约对固定装置 (fixtures) 进行往返处理或拒绝，且不声明核心写入器的确定性。 | `GTS Transform Tool (<transform>), corpus <commit>` |

在此仓库中，Go、Rust 和 TypeScript 目前针对特定的汇 (sink) API 声明了流式读取器 (Streaming Reader) 层级。Go 使用 `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)`。Rust 使用 `gmeow_gts::reader::read_to_sink_from_reader(reader, ReadOptions, sink)`。TypeScript 使用浏览器导出项 `foldStreamToSink(stream, options)`。这些 API 从流/读取器输入中读取数据，并在不实例化折叠图并集、折叠三元组或 blob 有效负载表的情况下发送汇事件。它们的语料库网关根据完整读取器或段读取器预言机 (oracles) 检查最终诊断代码、段头、配置文件、元数据、可流式处理布局状态以及段本地折叠事件计数。

Rust 遗留的 `read_to_sink(&[u8], ...)` 仍然是为已经持有字节的调用者提供的兼容性包装器。TypeScript 的 `foldStream(stream, options)` 和 `readStream(stream, options)` 仍然是返回图的浏览器便利方法。Python 目前仅提供前缀折叠和完整读取器证据。未来对其他 API 的声明必须包含非实例化汇路径，以及符合上述限制的内存证据。

一个工具可以声明多个层级。一个公开了 `read`、`verify`、`compact` 和 `files` 归档命令的命令行包可能会声明基础读取器、写入器、验证工具、配置文件感知工具 (`files`) 和转换工具 (`tar`)，但如果它无法解密或递归进入嵌套 GTS blob，则不声明完整读取器。
这些公共表面的跨语言 API 和命令矩阵在 [`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md) 中维护。
高级流式汇、索引/MMR/证明、复制、范围获取和基准测试推迟在 [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) 中维护。
信任/配置文件策略、嵌套 GTS 预算和加密推迟契约在 [`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md) 中维护。

## 4. 预期图格式

当前顶层语料库使用由 `python/src/gts/vectors.py::expected_for` 生成的 `vectors/<id>.expected.json`。实现 必须 (MUST) 比较相同的字段，除非清单显式缩小了向量范围：

```json
{
  "mode": "default",
  "diagnostics": ["UnknownCodec"],
  "terms": 3,
  "quads": 1,
  "segments": 1,
  "segment_heads": ["0123..."],
  "profiles": ["generic"],
  "streamable": [
    {"claimed": false, "covered": 0, "tail": 0}
  ],
  "opaque_reasons": ["unknown-codec"],
  "suppressions": 0,
  "blobs": {
    "blake3:...": {"size": 13, "mt": "text/html"}
  },
  "nquads": [
    "<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> \"Cat\"@en ."
  ]
}
```

字段语义：

| 字段 | 含义 |
|---|---|
| `mode` | 向量使用的读取模式。当前预期的 JSON 值为 `default`（宽容读取）和 `pre-segment`；清单值使用 §7 中的显式名称。 |
| `diagnostics` | 由读取器 (reader) 发出的有序诊断代码列表。诊断详情在当前语料库中未冻结。 |
| `terms`, `quads`, `segments`, `suppressions` | 折叠计数摘要。 |
| `segment_heads` | 按文件顺序排列的十六进制段 (segment) 头标识符。最后一个值是用于单头验证的文件头。 |
| `profiles` | 从标头折叠的段 (segment) 配置文件 (profile) 声明。 |
| `streamable` | 逐段布局状态：声明标志、覆盖的帧 (frame) 计数和增量尾部计数。 |
| `opaque_reasons` | 排序后的不透明节点 (opaque node) 原因字符串。 |
| `blobs` | 内联 blob 摘要、声明的媒体类型以及解码后的字节大小。 |
| `nquads` | 来自折叠图的排序后的 RDF 投影行。除非清单声明仅进行同构比较，否则空白节点标签应与参考渲染器匹配。 |

## 5. 向量清单模式 (Vector Manifest Schema)

仓库为冻结语料库 (frozen corpus) 提交了便携式清单：

- `vectors/manifest.core.json`：用于基准读取器 (Baseline Reader)、流式读取器 (Streaming Reader) 和核心写入器 (Writer) 声明的核心线缆格式 (wire-format) 读取器/写入器向量。
- `vectors/manifest.profiles.json`：用于验证和配置文件识别工具的配置文件布局 (profile-layout)、OKF 捆绑包和安全策略固定装置 (fixtures)。
- `vectors/manifest.transforms.json`：tar 归档转换/工具固定装置 (fixtures)。
- `vectors/manifest.json`：仓库级检查和发布报告使用的聚合清单。

这些清单使之前的顶级字节向量文件对约定变得明确，并命名了可选加密、人类哈希 (human-hash)、OpenPGP、签名、配置文件和安全检查所使用的 JSON 子语料库。每个清单使用如下形状：

```json
{
  "schema": "https://blackcatinformatics.ca/gts/vector-manifest/v1",
  "manifest_version": 1,
  "manifest_scope": "core",
  "corpus_revision": "git:<commit>",
  "generated_by": "gts.vectors",
  "vectors": [
    {
      "id": "03-unknown-codec",
      "title": "unknown codec degrades to opaque node",
      "input": {
        "path": "vectors/03-unknown-codec.gts",
        "media_type": "application/vnd.blackcat.gts+cbor-seq"
      },
      "mode": "permissive-read",
      "negative": true,
      "required_capabilities": ["cbor", "blake3", "identity"],
      "subsets": ["total-reader"],
      "tiers": ["baseline-reader"],
      "expected": {
        "graph": "vectors/03-unknown-codec.expected.json",
        "diagnostics": ["UnknownCodec"],
        "expected_head": "<hex-or-null>",
        "opaque_reasons": ["unknown-codec"]
      },
      "notes": "Reader must keep chain/fold total and surface the undecodable frame."
    }
  ]
}
```

检入的清单使用 `"corpus_revision": "git:repository-commit-containing-manifest"` 作为刻意的占位符。
该占位符避免了在包含哈希的文件中出现自引用提交哈希。它对于仓库验证是有效的，但不是发布一致性标识符。

发布候选版和第三方一致性报告 必须 (MUST) 在报告时将占位符替换为准确的 `git:` 修订版本。该修订版本 必须 (MUST) 是在仓库中解析的完整 40 字符提交 ID 或本地 Git 标签。请勿为此手动编辑已提交的清单；请生成一个带盖印的发布清单制品 (artifact)：

```bash
python scripts/check_vector_manifest.py \
  --release-manifest dist/vector-manifest.release.json
```

该命令验证语料库并写入清单副本，其中的 `corpus_revision` 命名了当前的 `HEAD` 提交。若要改为盖印发布标签或明确的提交，请传递 `--corpus-revision git:<tag-or-full-commit>`。普通的 `python scripts/check_vector_manifest.py` 命令继续验证检入的占位符清单。`python scripts/check_vector_manifest.py --write` 从固定装置树重写所有已提交的清单。

要求的顶级清单字段包括 `schema`、`manifest_version`、`manifest_scope`、`corpus_revision`、`generated_by` 和 `vectors`。`manifest_scope` 是 `aggregate`、`core`、`profiles` 或 `transforms` 之一。

要求的向量字段：

| 字段 | 要求 |
|---|---|
| `id` | 稳定的向量 ID；应该 (SHOULD) 与文件基本名称匹配。 |
| `input.path` | 指向规范输入字节或 JSON 固件的路径。 |
| `mode` | `permissive-read`、`strict-verify`、`publish-verify`、`profile-verify`、`pre-segment` 之一，或者由配置文件 (profile) 定义的扩展。 |
| `negative` | 当向量预期出现诊断信息、拒绝、非零验证状态或配置文件违规时，为 `true`。 |
| `required_capabilities` | 运行该向量所需的能力名称，例如 `zstd`、`cose-sign1`、`encrypt0`、`cose-encrypt`、`ecdh-es+a256kw`、`openpgp`、`streamable-index` 或 `files-profile`。 |
| `subsets` | 来自 §2 的一个或多个子集名称。 |
| `tiers` | 来自 §3 的使用该向量的分层 (Tier) 名称。 |
| `expected.graph` | 预期的图 JSON 路径，对于非图 JSON 固件则为 `null`。 |
| `expected.diagnostics` | 按读取器 (reader) 发射顺序排列的预期诊断代码列表。 |
| `expected.expected_head` | 当向量进行断言时，预期的最终文件或段 (segment) 头部十六进制；未断言时为 `null`。 |
| `notes` | 对所固定行为的人类可读解释。 |

可选向量字段包括 `expected.segment_heads`、`expected.exit_code`、
`expected.stderr_contains`、`expected.signature_status`、`expected.profile_findings`、
`compare.nquads`（`exact` 或 `bnode-isomorphism`），以及指向规范章节的 `links`。

## 6. 诊断注册表

诊断代码是稳定的 API。实现可以 (MAY) 添加详细信息、帧索引、段 ID 或特定于配置文件的字段，但在声明拥有这些代码的层级时，不得 (MUST NOT) 重命名这些代码。

严重程度值：

- `fatal`：无法为请求的模式折叠出完整的图，或者以后无法安全地解释后续内容。
- `error`：读取器/工具通常可以返回部分折叠，但严格验证失败。
- `warning`：宽容读取成功，如果模式声明该条件为非致命，则严格验证可以 (MAY) 成功。
- `info`：机器可读的观察结果，其本身不会导致验证失败。

| 代码 | 严重程度 | 适用于 | 读取器行为 | 可恢复？ | 不透明原因 | 要求层级 |
|---|---|---|---|---|---|---|
| `EmptyFile` | fatal | 文件结构 | 返回空图/结果及诊断信息。 | no | none | 基准读取器 |
| `DamagedFrame` | error | 标头/帧哈希、有效载荷解码、格式错误的有效载荷 | 在可能的情况下隔离损坏的项，显示诊断信息，并在边界已知时折叠幸存部分。 | partial | 当表示为不透明时为 `damaged` | 基准读取器 |
| `BrokenChain` | error | id/prev 链 | 显示链断裂；严格验证失败。 | partial | none | 基准读取器 |
| `TornAppendError` | warning | 尾随的不完整 CBOR 项 | 忽略尾随的不完整字节，并折叠最后一个完整的完整前缀。 | yes | none | 基准读取器 |
| `UnknownCodec` | warning | 转换能力 | 将该帧保留为不透明并继续折叠已知内容。 | yes | `unknown-codec` | 基准读取器 |
| `MissingKey` | warning | 加密转换 | 将该帧保留为不透明并继续折叠已知内容。 | yes | `missing-key` | 当声称支持解密时的完整读取器 |
| `KeyWrapFailed` | warning | 延迟的多接收者加密转换 | 当 ECDH 接收者元数据或 AES-KW 解封失败时，将该帧保留为不透明。 | yes | `missing-key` | 当声称支持 `cose-encrypt`/ECDH 时的未来完整读取器 |
| `ConflictingReifier` | error | 图折叠 | 按文件顺序保留第一个绑定，并忽略冲突的绑定。 | yes | none | 基准读取器 |
| `PositionConstraint` | error | 图折叠 | 拒绝违规行并继续折叠其他行/帧。 | yes | none | 基准读取器 |
| `ForwardReference` | error | 术语字典 | 丢弃或忽略无效的前向引用，并继续安全地折叠。 | yes | none | 基准读取器 |
| `SegmentBoundary` | fatal | 前段兼容模式 | 在将后续段错误地折叠为文件全局 ID 之前停止。 | 在该模式下为 no | none | 基准读取器兼容性测试 |
| `IllTypedLiteral` | warning | RDF/XSD 语法导入 | 逐字保留字面量的词法形式和数据类型；公开诊断信息和/或 `gts:illTypedLiterals` 元数据边车。 | yes | none | RDF 编解码器 / 识别配置文件的工具 |
| `TruncatedLog` | error | 预期头部 / 新鲜度 | 折叠观察到的字节，但针对请求的头部验证失败。 | yes | none | 完整读取器或验证工具 |
| `StreamableLayoutError` | error | 可流式处理布局声明 | 折叠字节，但使布局声明的严格/配置文件验证失败。 | yes | none | 验证工具 |
| `IndexMmrError` | error | 可选索引 MMR 根 | 折叠字节，但使索引承诺的严格验证失败。 | yes | none | 当声称支持 MMR/证明时的完整读取器 |
| `RecursionLimit` | error | 嵌套 GTS 递归 | 停止递归，并将嵌套内容公开为不可用/不透明。 | yes | implementation-defined | 完整读取器 |
| `UnknownFrameType` | warning | 扩展帧 | 保留链验证；忽略或显示为不透明/诊断，直到有配置文件处理它。 | yes | 如果不透明则为 `unknown-frame-type` | 识别配置文件的工具 |

配置文件验证器可以 (MAY) 定义额外的特定于配置文件的诊断代码，但它们必须 (MUST) 使用配置文件命名空间或在配置文件规范中记录该代码。

## 7. 读取与验证模式

| 模式 | 目的 | 行为 | 测试凭据 |
|---|---|---|---|
| `permissive-read` | 供希望获得最佳可恢复图的消费者使用的库读取/折叠 (fold)。 | 绝不 (Never) 对格式错误的语料库输入产生 panic；返回图状态以及诊断/不透明节点 (opaque node)；诊断信息不会阻止结果的返回。 | `wire-core`、`total-reader` 和 `graph-fold` 作为核心折叠图预期；`profile-layout` 作为配置文件 (profile)/工具凭据。 |
| `strict-verify` | 用于调用者请求的链/哈希/布局/签名检查的传输验证器。 | 遇到任何错误或致命诊断时退出/失败；如果该模式将其声明为警告，则可以 (MAY) 允许文档化的警告（如不支持的配置文件 (profile)）。 | CLI `verify` 测试、`04`、`05`、`06`、`17`、`26`、已签名/头部测试。 |
| `publish-verify` | 用于创建或分发制品的命令的发布与重写闸门。 | 拒绝结构有效但策略无效的制品，例如空折叠 (empty-fold) 组合、抑制一切组合、流式谎言、不安全提取或不可重现的压缩。 | `21-degenerate-composition`、`22-inline-blob`、`25b-streamable-compacted`、`26-streamable-lie`。 |
| `profile-verify` | 在核心线缆格式 (wire-format) 有效性之上的感知配置文件验证。 | 应用配置文件词汇、能力、信任、布局和归档规则，而不重新定义核心 GTS 有效性。 | `19-profile-union-opacity`、`20-language-tag-discipline`、`23-files-profile-tree`、`24-files-profile-dedup`、`25`-`27`。 |

模式名称是清单 (manifest) 值，不一定是字面上的 CLI 子命令。CLI 可以 (MAY) 通过带有标志的一个命令公开多个模式；测试工具 必须 (MUST) 记录使用了哪种模式。

## 8. 报告

一致性报告应该 (SHOULD) 包含：

- 实现名称、版本、commit、操作系统和架构；
- 用于报告的准确语料库修订版或标签，与标记的发布清单 (release manifest) 匹配；
- 层级声明 (tier claims) 和向量子集；
- 命令行或测试名称；
- 按子集统计的通过/失败计数；
- 任何跳过的向量及缺失的功能名称；
- 为失败向量输出的诊断信息；
- 语料库是否已重新生成并被证明是可复现的。

报告应该 (SHOULD) 是发布候选版本 (release candidates) 的持久构建产物，并且对于 v1.0 及更高版本，应该 (SHOULD) 附在发布说明 (release notes) 中。

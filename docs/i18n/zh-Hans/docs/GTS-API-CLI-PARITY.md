<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-API-CLI-PARITY.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS API 与 CLI 对等协约

> [`docs/GTS-API-CLI-PARITY.md`](../../../docs/GTS-API-CLI-PARITY.md) 的信息性中文翻译。英文文档仍然是兼容性规则、一致性声明、对等矩阵、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

本文件定义了 Rust、Python、Go、TypeScript、Smalltalk/Pharo 以及 Kotlin/JVM 保持兼容的跨语言表面，同时各引擎继续暴露原生习语。传输格式在 [`GTS-SPEC.md`](./GTS-SPEC.md) 中保持规范性，语料库/层级规则在 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) 中保持规范性。本协约确立了公共 API 形态和 CLI 对等矩阵，使功能差距显性化，而非从特定软件包的文档中推断。

基于 Rust 的 C ABI 及衍生的 C 兼容包装器是一个独立的互操作层。它们使用 `libgts` 至 `rust/capi/include/gts.h`，暴露生态系统原生的库 API，且不会在下方的全引擎 API/CLI 对等表中增加列。

## 语言中立的 API 形态

稳定的核心（waist）是语义上的，而非语法上的。每个引擎可以 (MAY) 使用原生名称和容器，但以下操作和折叠 (folded) 字段是兼容性目标。
<!-- api-parity-shape:start -->
| operation | contract | current native surface |
|---|---|---|
| `read(input, options)` | Parse a byte buffer or path as a CBOR Sequence, verify the id/prev chain, fold every recoverable frame, and return a graph/result with diagnostics instead of panicking on malformed input. | Python `gts.read(data, keys=None, expected_head=None, allow_segments=True)`; Rust `reader::read(&bytes, allow_segments, expected_head)` or `reader::read_with_options` with `ReadOptions::with_content_key`; Go `reader.Read(data, allowSegments, expectedHead)`; TypeScript `Read(bytes, allowSegments, expectedHead?)`; Smalltalk `GtsReader read:allowSegments:`; Kotlin `read(data, allowSegments)`. |
| `verify(input, options)` | Apply strict transport checks over the same fold: chain/hash diagnostics, expected-head freshness when provided, streamable-layout checks when requested, and COSE signature status when keys are provided. | CLI `gts verify`; Python `gts.verify.verify_file`; Rust `gmeow_gts::verify::verify_file` plus folded diagnostics and lower-level COSE helpers in every engine. |
| `write(graph/events, options)` | Emit deterministic CBOR for hashed or signed bytes, compute each frame id from its content, and set `prev` to the previous frame id. | Python `Writer`; Rust `writer::Writer`; Go `writer.New`; TypeScript `Writer`; Smalltalk `GtsWriter`; Kotlin `Writer`. |
| `fold(input)` | Return the deterministic GTS value fold: terms, quads, reifiers, annotations, blobs, suppressions, opaque nodes, signatures, segment heads, profiles, and streamable layout state. | Same object returned by `read`. |
| `to_nquads(graph)` | Project the folded RDF dataset to sorted N-Quads text with the same value semantics across engines. | Python `to_nquads`; Rust `nquads::to_nquads`; Go `nquads.ToNQuads`; TypeScript `toNQuads`; Smalltalk `GtsNQuads`; Kotlin `toNQuads`. |
| `from_nquads(input)` | Build a GTS file from N-Quads text using the shared writer semantics. | Python `from_nquads`; Rust `from_nquads::from_nquads`; Go `fromnquads.FromNQuads`; TypeScript `fromNQuads`; Smalltalk `GtsFromNQuads`; Kotlin `fromNQuads`; CLI `gts from-nq` in every engine. |
| `to_ntriples(graph)` / `from_ntriples(input)` | Project a default-graph RDF dataset to N-Triples and rebuild GTS bytes from N-Triples text using the shared RDF 1.2 parser/serializer. | Rust `rdf_codecs::to_ntriples` / `from_ntriples` behind `--features rdf-codecs`; Go `rdfcodecs.ToNTriples` / `FromNTriples`; CLI `gts to-nt` and `gts from-nt` in Rust and Go. |
| `to_rdf_xml(graph)` / `from_rdf_xml(input)` | Project a default-graph RDF dataset to RDF/XML and rebuild GTS bytes from RDF/XML text, including RDF/XML namespace, parseType, collection, reification, annotation, and RDF 1.2 triple-term grammar. | Rust `rdf_codecs::to_rdf_xml` / `from_rdf_xml` behind `--features rdf-codecs`; Go `rdfcodecs.ToRDFXML` / `FromRDFXML`; CLI `gts to-rdfxml` and `gts from-rdfxml` in Rust and Go. |
| `to_trig(graph)` / `from_trig(input)` | Project folded RDF to readable TriG graph blocks and rebuild GTS bytes from the supported TriG surface without changing N-Quads content. | Python `gts.trig.to_trig` / `from_trig`; Rust `trig::to_trig` / `from_trig::from_trig`; Rust `rdf_codecs::to_trig` / `from_trig` with `--features rdf-codecs`; Go `rdfcodecs.ToTriG` / `FromTriG`; CLI `gts to-trig` and `gts from-trig` in Python, Rust, and Go. |
| `to_turtle(graph)` / `from_turtle(input)` | Project a default-graph RDF dataset to Turtle and rebuild GTS bytes from Turtle text using the shared Turtle-family RDF 1.2 parser/serializer. | Rust `rdf_codecs::to_turtle` / `from_turtle` behind `--features rdf-codecs`; Go `rdfcodecs.ToTurtle` / `FromTurtle`; CLI `gts to-turtle` and `gts from-turtle` in Rust and Go. |
| graph iterators/accessors | Expose resolved access to terms, quads, reifier bindings, annotations, suppressions, blobs, opaque nodes, signatures, diagnostics, segment heads, profiles, metadata, and streamable state. | Native fields on `Graph`/`GtsGraph` in all six engines, with helper lookups where idiomatic. |
| blobs | Preserve inline blob bytes by `blake3:<hex>` digest and retain declared blob metadata such as media type. Extraction MUST re-hash bytes before writing them. Implementations MAY keep transformed blob bytes lazy until access. | Python `Graph.blobs`/`blob_meta`; Rust `Graph.blobs` lazy `BlobEntry` plus `blob_entry`/`blob_bytes`/`decoded_blobs`; Go `Graph.Blobs`/`BlobMeta`; TypeScript `Graph.blobs`/`blobMeta`; Smalltalk `GtsGraph blobs`/`blobMeta`; Kotlin `Graph.blobs`/`blobMeta`. |
| opaque nodes | Preserve undecodable or unsupported recoverable frames as graph-visible opaque nodes with a frame id, frame type, reason, and signature status. | `OpaqueNode` in every engine. |
| diagnostics | Preserve stable diagnostic `code` values and optional frame indexes; native detail text may differ. | `Diagnostic.code/detail/frame_index`, `Diagnostic { code, detail, frame_index }`, `Diagnostic{Code, Detail, FrameIndex}`, `Diagnostic.code/detail/frameIndex`. |
| streaming/full-reader options | Carry read mode, segment allowance, expected head, key provider, recursion/decode budgets, and streamable validation as options. Engines MAY stage these as separate helpers while preserving the same observable fold and diagnostics. | Python `keys`, Rust `ReadOptions`/`read_to_sink_with_options`/`read_to_sink_from_reader`, Go `reader.Options`/`reader.ReadToSink`, TypeScript `allowSegments`/`foldStreamToSink`, Smalltalk `allowSegments`, Kotlin `allowSegments`, and CLI flags today; deeper recursion/MMR options are future Full Reader work. |
<!-- api-parity-shape:end -->
## 跨语言等价性目标

一致性语料库 (conformance corpus) 比较使引擎具有可替代性的可观测字段。新的测试和 API 增加内容应该 (SHOULD) 保留以下目标：

| target | equality rule |
|---|---|
| folded graph | 术语、四元组 (quads)、具体化器 (reifiers)、注解、抑制、配置文件 (profile) 声明、元数据、可流式处理 (streamable) 状态以及 N-Quads 投影与预期的 JSON 匹配。 |
| diagnostics | 诊断代码顺序匹配。原生详细文本和原生异常/警告包装器未冻结。 |
| head id | 段 (segment) 头部 ID 以小写十六进制匹配。单段文件的最后一段头部是用于新鲜度检查的文件头部。 |
| opaque reasons | 排序后，不透明节点 (opaque node) 原因字符串匹配，包括 `unknown-codec`、`missing-key`、`damaged` 和 `unknown-frame-type`。 |
| signature status | 每帧 (per-frame) 签名状态使用 `valid`、`invalid` 或 `unverified`，并在存在时匹配密钥 ID。 |
| blob digests | `blake3:<hex>` 摘要密钥、声明的媒体类型和解码后的字节长度匹配；提取操作在写入前对字节进行重新哈希处理。 |

## 诊断与原生错误映射

对于宽松读取，读取器 (reader) 诊断是数据，而非抛出的控制流。严格验证和发布命令可以 (MAY) 将任何错误或致命诊断转换为非零进程退出或原生错误返回。

| 概念 | Python | Rust | Go | TypeScript | Smalltalk | Kotlin |
|---|---|---|---|---|---|---|
| 诊断记录 | `gts.Diagnostic` dataclass | `model::Diagnostic` struct | `model.Diagnostic` struct | `Diagnostic` interface | `GtsDiagnostic` object | `Diagnostic` data class |
| 代码字段 | `code: str` | `code: String` | `Code string` | `code: string` | `code` | `code: String` |
| 详情字段 | `detail: str` | `detail: String` | `Detail string` | `detail: string` | `detail` | `detail: String` |
| 帧索引 | `frame_index: int \| None` | `frame_index: Option<usize>` | `FrameIndex *int` | `frameIndex?: number` | `frameIndex` | `frameIndex: Int?` |
| 宽松读取结果 | `Graph` 以及 `diagnostics` | `Graph` 以及 `diagnostics` | `*model.Graph` 以及 `Diagnostics` | `Graph` 以及 `diagnostics` | `GtsGraph` 以及 `diagnostics` | `Graph` 以及 `diagnostics` |
| 严格 CLI 失败 | 针对诊断/拒绝退出 `1` | 针对诊断/拒绝退出 `1` | 针对诊断/拒绝退出 `1` | 针对诊断/拒绝退出 `1` | 针对诊断/拒绝退出 `1` | 针对诊断/拒绝退出 `1` |
| 用法或 I/O 失败 | 退出 `2` | 退出 `2` | 退出 `2` | 退出 `2` | 退出 `2` | 退出 `2` |

规范诊断代码注册表位于 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md#6-diagnostics-registry)。

## CLI 一致性矩阵

`yes` 意味着该动词已由该引擎的 `gts` 二进制文件实现。`no` 意味着该缺失是在一致性 issue 落地之前有意留出的公开缺口。该矩阵由 [`scripts/check_cli_parity.py`](../scripts/check_cli_parity.py) 检查，它会读取此表格及实际的调度接口。

<!-- cli-parity-matrix:start -->
| verb | Python | Rust | Go | TypeScript | Smalltalk | Kotlin | status |
|---|---|---|---|---|---|---|---|
| `info` | yes | yes | yes | yes | yes | yes | common |
| `fold` | yes | yes | yes | yes | yes | yes | common |
| `verify` | yes | yes | yes | yes | yes | yes | common |
| `extract-key` | yes | yes | yes | yes | yes | yes | common |
| `ls` | yes | yes | yes | yes | yes | yes | common |
| `extract` | yes | yes | yes | yes | yes | yes | common |
| `cat` | yes | yes | yes | yes | yes | yes | common |
| `compact` | yes | yes | yes | yes | yes | yes | common |
| `pack` | yes | yes | yes | yes | yes | yes | common |
| `unpack` | yes | yes | yes | yes | yes | yes | common |
| `diff` | yes | yes | yes | yes | yes | yes | common |
| `from-nq` | yes | yes | yes | yes | yes | yes | common |
| `to-trig` | yes | yes | yes | no | no | no | Python/Rust/Go transform extension |
| `from-trig` | yes | yes | yes | no | no | no | Python/Rust/Go transform extension |
| `to-nt` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `from-nt` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `to-rdfxml` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `from-rdfxml` | no | yes | yes | no | no | no | Rust/Go RDF text codec extension |
| `to-turtle` | no | yes | yes | no | no | no | Rust/Go Turtle-family transform extension |
| `from-turtle` | no | yes | yes | no | no | no | Rust/Go Turtle-family transform extension |
| `to-yaml-ld` | no | yes | no | no | no | no | Rust transform extension |
| `from-yaml-ld` | no | yes | no | no | no | no | Rust transform extension |
| `to-okf` | no | yes | no | no | no | no | Rust OKF profile extension |
| `from-okf` | no | yes | no | no | no | no | Rust OKF profile extension |
| `to-tar` | no | yes | no | no | no | no | Rust tar bridge extension |
| `from-tar` | no | yes | no | no | no | no | Rust tar bridge extension |
| `tar` | no | yes | no | no | no | no | Rust tar-compatible extension |
| `to-sqlite` | yes | yes | no | no | no | no | Python/Rust extension |
| `to-duckdb` | yes | yes | no | no | no | no | Python/Rust extension |
| `to-parquet` | yes | yes | no | no | no | no | Python/Rust extension |
| `prove` | no | yes | no | no | no | no | Rust proof creation extension |
| `dump` | no | yes | no | no | no | no | Rust inspection export extension |
| `verify-proof` | yes | yes | yes | yes | yes | yes | common |
| `heads` | yes | yes | yes | yes | yes | yes | common |
| `segments` | yes | yes | yes | yes | yes | yes | common |
| `missing` | yes | yes | yes | yes | yes | yes | common |
| `resume` | yes | yes | yes | yes | yes | yes | common |
<!-- cli-parity-matrix:end -->
### 有意保留的差距

- Rust `to-sqlite` 在 `PATH` 上需要 `sqlite3`；Rust `to-duckdb` 和 `to-parquet` 需要可选的无依赖 `duckdb` Cargo 特性，外加 `PATH` 上的 `duckdb`。Python DuckDB 和 Parquet 导出需要 Python `[db]` extra。Rust 将 SQL 行流式传输到运行时工具，而不是在内存中保留所有关系行或完整的 SQL 脚本；稳定的 `blobs.bytes` 模式在发出每个 blob 行时仍需要瞬时 blob 解码。
- Go, TypeScript, Smalltalk, 和 Kotlin 尚未开放关系型导出。
- `to-trig` 和 `from-trig` 是 Python/Rust/Go 转换扩展。它们在使用可读的 TriG 图块时，保留与 N-Quads 投影相同的折叠 RDF 内容；TypeScript, Smalltalk, 和 Kotlin 的一致性可以稍后根据相同的往返预期实现。
- `to-nt` 和 `from-nt` 是 Rust/Go RDF 文本编解码器扩展。`to-nt` 仅接受默认图 RDF 投影；具名图数据集应该 (SHOULD) 使用 `to-trig`。Python, TypeScript, Smalltalk, 和 Kotlin 的一致性可以稍后根据相同的解析器/往返预期实现。
- `to-rdfxml` 和 `from-rdfxml` 是 Rust/Go RDF 文本编解码器扩展。它们通过事件合约涵盖 RDF/XML 解析和序列化，包括命名空间、`rdf:parseType`、集合、重构、注解和 RDF 1.2 三元组项表面。`to-rdfxml` 仅接受默认图 RDF 投影；具名图数据集应该 (SHOULD) 使用 `to-trig`。Python, TypeScript, Smalltalk, 和 Kotlin 的一致性可以稍后根据相同的 W3C RDF/XML 测试套件预期实现。
- `to-turtle` 和 `from-turtle` 是 Rust/Go Turtle 系列转换扩展。它们使用与完整 TriG 路径相同的 RDF 1.2 解析器/序列化器堆栈。`to-turtle` 仅接受默认图 RDF 投影；具名图数据集应该 (SHOULD) 使用 `to-trig`。Python, TypeScript, Smalltalk, 和 Kotlin 的一致性可以稍后根据相同的解析器/往返预期实现。
- `to-yaml-ld` 和 `from-yaml-ld` 是 `--features yaml-ld` 背后的仅限 Rust 的扩展谓词。它们是折叠图表上的仅转换垫片，而不是有线格式或规范目录的更改；如果需要，Python, Go, TypeScript, Smalltalk, 和 Kotlin 的一致性可以稍后通过添加共享语料库 oracle 来实现。
- `to-okf` 和 `from-okf` 是 `--features okf` 背后的仅限 Rust 的 OKF 配置文件谓词。它们将 OKF Markdown 包映射到 GTS 配置文件 `okf`，具有清单模式 `gts-okf-v1`、内容寻址的 Markdown 正文 blob、可查询的链接边缘、导航 `index.md` 容差以及用于配置文件外 RDF 的 `_unmapped.nq`。提交的 OKF 语料库（包括 `vectors/okf/bigquery-join/`）是任何未来 Python, Go, TypeScript, Smalltalk, 或 Kotlin 实现所需的一致性门槛。在能够导入/导出 `gts-okf-v1` 目录合约并保留折叠 N-Quads 预期之前，这些引擎在此处必须保持 `no`。
- `to-tar`, `from-tar`, 和 `tar` 是 `--features tar` 背后的仅限 Rust 的 files-profile-v2 桥接谓词。它们将 tar 流映射到 GTS 文件并映射回，同时保留 files-profile 元数据、选择性加入的链接/特殊文件记录、gzip/zstd 包装、未知的 PAX 记录以及与 tar 兼容的 `-c/-x/-t/-d` 命令表面。Python, Go, TypeScript, Smalltalk, 和 Kotlin 的一致性应该 (SHOULD) 稍后根据相同的安全策略和往返行为实现。要求的一致性门槛是提交的 `vectors/tar/` 语料库外加 files-profile-v2 导入/导出行为；在能够保留相同的清单元数据、拒绝策略和 tar 往返预期之前，这些引擎在此处必须保持 `no`。
- `dump` 是仅限 Rust 的审查导出，它写入一个版本化目录树，包含折叠的 N-Quads、JSONL 表、展开的帧视图、blob 索引和 files-profile 负载。这不属于有线格式更改；Python, Go, TypeScript, Smalltalk, 和 Kotlin 的一致性稍后可以实现相同的 `gts-dump-v1` 目录合约。
- 所有引擎都使用稳定的原像以及 `vectors/proofs/` 中的正向/反向测试用例来实现用于脱离的 MMR 证明 JSON 的 `verify-proof`。Rust 额外实现了从带有经验证的 `index.mmr` 根的文件中进行 `prove`。Python, Go, TypeScript, Smalltalk, 和 Kotlin 不应 (SHOULD NOT) 开放 `prove`，直到它们能够针对相同的测试用例规程创建基于文件的证明。
- 所有引擎都使用相同的 JSON 模式和续传边界规则实现复制谓词：`gts-replication-heads-v1`、`gts-replication-segments-v1` 和 `gts-replication-missing-v1`。
- 未来的嵌套 GTS 递归和加密策略谓词尚不属于稳定 CLI 表面的一部分。在包特定文档声明它们之前，应将它们添加到此矩阵中。
- 剩余的高级延迟谓词（如果有）在 [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) 中跟踪，并受 `scripts/check_advanced_contract.py` 保护。

## 偏移守卫

使用以下命令在本地运行平齐检查：

```bash
python scripts/check_api_parity.py
python scripts/check_cli_parity.py
```

CI lint 作业运行相同的命令。API 检查读取 [`api-parity.json`](./api-parity.json)、本文档、README 引擎特性矩阵，以及六个完整引擎的小型源码级冒烟证据。完整引擎包括 Rust、Python、Go、TypeScript、Smalltalk/Pharo 和 Kotlin/JVM。基于 C ABI 的封装 (wrappers) 保持独立声明，且`不得 (MUST NOT)` 成为完整引擎平齐列。

API 检查在以下情况下失败：

- 渲染的 API 形态表发生变化，但未进行匹配的声明更新；
- README 特性矩阵发生变化，但未进行匹配的声明更新；
- 声明的支持项缺乏源码证据，或者声明的公开导出/模块/文件消失；
- 延迟的声明未被明确记录为已延迟；
- 封装 (wrapper) 接口被添加到完整引擎声明中。

CLI 检查在以下情况下失败：

- 引擎实现了矩阵中未体现的 CLI verb；
- 矩阵将某个 verb 标记为 `yes`，但该引擎的调度接口中缺少该 verb；
- 矩阵将某个 verb 标记为 `no`，但该引擎现在已实现它；
- README 公共命令块和 Python 扩展命令块与矩阵发生偏移。

当添加 API 平齐声明时，请在同一变更中更新引擎实现/导出、`docs/api-parity.json` 中的源码证据、此 API 形态表或 README 特性矩阵，以及包级测试。当延迟引擎的平齐时，请将 README 单元格保持为 `no`，并在声明中添加延迟原因，而不是依赖于省略。

当添加或移除 CLI verb 时，请在同一变更中更新实现、此矩阵、README 命令块以及包特定的 README 文本。

## C ABI 包装器表面

C ABI 包装器系列有意设计得比原生完整引擎更窄。包装器将格式语义委托给 Rust 引擎，并使稳定 ABI 在兼容 C 的生态系统中更加便捷：

| 表面 | 合约 |
|---|---|
| ABI 元数据 | `gts_abi_version`、`gts_version`、构建元数据 JSON 和功能 JSON 标识已加载的 `libgts` 表面。 |
| 读取/折叠 | `gts_read_json` 为折叠 (fold) 后的存档状态返回稳定的 JSON 报告。 |
| 验证 | `gts_verify_json` 以 JSON 格式返回 Rust 验证器报告。 |
| RDF 文本格式 | `gts_formats_json`、`gts_to_format` 和 `gts_from_format` 为 N-Quads、N-Triples、Turtle、TriG、RDF/XML 和确定性 JSON-LD-star 配置文件 (profile) 公开注册表驱动的转换。`gts_to_nquads` 和 `gts_from_nquads` 仍作为兼容性助手。 |
| 文件配置文件 | `gts_files_pack`、`gts_files_unpack` 和 `gts_files_diff_json` 公开 files 配置文件 (profile) 助手。 |
| 所有权 | 返回的 `gts_buffer` 值被复制到生态系统原生的字符串或字节数组中，然后通过 `gts_buffer_free` 释放。 |
| 错误 | 非 OK 的 `gts_status` 返回值从 `gts_error` 句柄复制到结构化的生态系统错误中，然后通过 `gts_error_free` 释放。 |

当前的包装器包括 C++、.NET、PHP、Lua、Swift、Ruby、R 和 Julia。每个包装器的 README 拥有其本地命名、加载器行为、线程说明和冒烟测试命令。包装器冒烟测试证明了 ABI 的可达性和所有权行为；它们不能替代六个完整引擎的一致性语料库 (conformance corpus)。

## Files 配置文件命令契约

`pack`、`unpack` 和 `diff` 是所有六个引擎中的通用命令。它们的可观测行为是同等性表面的一部分：

- `pack <dir|file>... -o out.gts` 在内联 blob 之前发出包含目录项/四元组 (quads) 的单个 `files` 段，每个路径仅存储一次，并按摘要对相同内容进行去重。
- 存储的存档路径是以 `/` 分隔的相对路径。在读取或写入文件字节之前，每个引擎都会拒绝空路径、绝对路径、Windows 驱动器相对路径、`..`、`.`、空组件和反斜杠分隔符。
- 符号链接不被存档。`pack` 和 `diff` 拒绝符号链接条目，而不是跟随它们；`unpack` 拒绝逃逸目标目录的路径，包括通过该目录下现有符号链接进行的逃逸。
- `unpack` 在写入前对内联 blob 字节重新进行哈希计算。如果未禁用的 `FileEntry` 缺少其内联 blob，则视为拒绝；禁用的 blob 摘要默认会被跳过，且仅在使用 `--include-suppressed` 时提取。
- `diff` 通过 `files:digest` 将存档清单与目录进行比较，并返回排序后的 `added:`、`modified:` 和 `removed:` 行。退出码 `0` 表示没有差异；退出码 `1` 表示存在差异或输入被拒绝。

实时跨引擎保护机制为 [`scripts/interop.sh`](../scripts/interop.sh)：每个引擎打包相同的固定装置 (fixture)，每个引擎折叠 (fold) 并解包每个包，并且每个引擎都针对每个包，对匹配树和已更改树进行 diff 比较。

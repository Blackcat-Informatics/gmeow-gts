<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-ADVANCED-PRIMITIVES.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 高级原语契约

> [`docs/GTS-ADVANCED-PRIMITIVES.md`](../../../../docs/GTS-ADVANCED-PRIMITIVES.md) 的信息性中文翻译。英文文档仍然是集成、高级功能、可选 profile、基准数据、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。


本文档汇总了流式接收器、索引、MMR/证明、范围获取、复制以及内存基准的实现路径。核心线缆格式在 [`GTS-SPEC.md`](./GTS-SPEC.md) 中保持规范性；本契约陈述了当前软件包实际支持的内容，以及从 v1 表面有意推迟 (deferred) 的内容。
## 当前 V1 支持情况

| 原语 (primitive) | 当前支持 | 声明边界 (claim boundary) |
|---|---|---|
| 前缀折叠 (Prefix-fold) 特性 | 每个顶级语料库向量都在 CBOR 项目边界处进行了测试。 | 这证明了总前缀读取，而不是流式接收器 (streaming sink) API。 |
| 可流式处理布局 (Streamable layout) | `gts compact --streamable` 重写了交付顺序并追加了 `index` 页脚；读取器验证声明并报告增量尾部 (accretive tails)。 | 这是一个验证工具/配置文件布局 (Validating Tool/Profile Layout) 特性。 |
| 索引页脚字段 | 写入器发出 `count`、`head`、`off` 和 `ti`；Rust 写入器可以选择启用 `mmr`，并且 Rust 读取器在存在时验证 `mmr` 根。 | 尚未声称支持来自 `off`/`ti` 的全读取器随机访问。 |
| MMR 证明 JSON | 所有引擎都针对 `vectors/proofs/` 验证分离的证明 JSON；Rust 还公开了 `Writer::add_index_with_mmr`，验证可选的 `index.mmr`，并实现 `gts prove`。 | 分离验证是跨引擎的；从索引的 GTS 文件创建证明仍然仅限 Rust。 |
| 复制清单 (Replication inventory) | 所有四个 CLI 都公开了 `gts heads`、`gts segments`、`gts missing` 和 `gts resume`，用于机器可读的头部比较和字节范围恢复。 | 共享的 v1 复制表面；`resume` 仅在扫描的 CBOR 项目边界处的经过验证的帧 (frame) ID 之后开始。 |
| Blob 自省 | `gts ls` 列出了内容寻址的 blob 摘要、大小和媒体类型。 | 范围获取仍需要经过验证的索引或边界扫描。 |
| 内存基准助手 | `scripts/bench_reader_memory.py` 报告全读取器具体化、帧扫描基线、Rust `read_to_sink_from_reader` 和 TypeScript 浏览器 `foldStreamToSink` 行。Go 通过 `go test ./reader -bench 'Benchmark(ReadFull\|ReadToSink)CorpusVector' -benchmem` 报告其全读取器和非具体化流式接收器分配证据。 | 帧扫描不是流式读取器折叠 (Streaming Reader fold)；Rust、TypeScript 和 Go 行是其命名 API 的接收器内存证据。 |

当前的 Go 包可以为 `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` 声称 `Streaming Reader` 层级。Rust 包可以为 `read_to_sink_from_reader(reader, ReadOptions, sink)` 声称该层级。TypeScript 浏览器包可以为 `foldStreamToSink(stream, options)` 声称该层级。Rust 的 `read_to_sink(&[u8], ...)` 和 TypeScript 的 `foldStream(stream, options)`/`readStream(stream, options)` 仍然是兼容性或图形返回助手，而不是命名的声明表面 (claim surfaces)。Rust 仍然是唯一可以声称 MMR 证明创建的包。所有四个包都可以声称对 `vectors/proofs/` 中的固定装置集 (fixture set) 和共享复制清单谓词 (verbs) 进行分离证明验证。Python 尚不应该 (SHOULD NOT) 声称接收器或证明创建层级；Go 和 TypeScript 尚不应该 (SHOULD NOT) 声称证明创建。
## 推迟 (Deferred) 的高级 CLI 谓词 (Verbs)

下方的行（如果存在）是计划中的词汇，而非当前的公开命令。如果这些谓词 (verbs) 中的任何一个
在更新此表之前出现在引擎调度接口或公开 CLI 一致性矩阵中，守护脚本
[`scripts/check_advanced_contract.py`](../../../../scripts/check_advanced_contract.py) 将会失败。当所有当前计划的高级 CLI 谓词 (verb) 都已
提升时，此表可能为空。

<!-- advanced-cli-deferred:start -->
| verb | status | next implementation gate |
|---|---|---|
<!-- advanced-cli-deferred:end -->
## 流式接收器 API (Streaming Sink API)

只有当软件包公开一个已文档化的 API，且该 API 通过按顺序消费帧 (frame) 并向接收器 (sink) 发送事件（而不实例化整个 `Graph`）来进行折叠 (fold) 或投影时，该软件包才可以 (MAY) 声称符合 `GTS Streaming Reader`。

最低要求：

- 在流式处理时验证 header id 和 frame id/prev chain；
- 根据需要保留或溢出词条字典 (term dictionary)，因为 term ids 是段 (segment) 局部的；
- 按帧顺序发送 term, quad, reifier, annotation, suppression, blob, opaque, signature, diagnostic, segment-head 和可流式处理布局 (streamable-layout) 事件；
- 针对相同输入，记录与完整读取器 (reader) 相同的最终诊断和 segment head ids；
- 保留受 `O(distinct terms + maximum decoded frame size + validation sidecar state)` 限制的内存，而不是折叠的三元组或 blob；
- 使用 `scripts/bench_reader_memory.py` 或等效基准 (benchmark) 报告内存行为。

现有的 `streaming-property` 子集仍然具有价值，但它是一种前缀完整性属性。它本身并不是流式接收器 (streaming sink) 声明。
## 索引、MMR 和证明层级

可选的 `index` 有效负载目前有五个已实现的组成部分：

- `count`：所覆盖帧的数量；
- `head`：最后一个所覆盖帧的帧 ID；
- `off`：每个所覆盖帧从其段 (segment) 起始处开始的字节偏移量；
- `ti`：从帧类型到所覆盖帧位置的映射。
- `mmr`：索引 GTS 文件中，基于所覆盖帧 ID 的仅限 Rust 的 Merkle-Mountain-Range 根。

以下部分仍然被推迟 (deferred)：

- `dict`：用于需要词典传递的文本投影的术语词典定位器；
- 来自索引 GTS 文件的跨引擎包含证明创建；
- Rust 之外的 `prove` CLI 动词。

在将 MMR 证明创建推广到 Rust 之外前，仓库需要：

- 索引文件证明创建固定装置，包括正向和负向行为；
- 针对 `GTS-SPEC.md` 中稳定原像的 Python、Go 和 TypeScript 中的 `index.mmr` 写入器/读取器 (writer/reader) 实现；
- 证明创建测试，用以证明生成的独立 JSON 在每个引擎中可以独立于完整文件可用性进行验证。
## 范围获取 (Range-Fetch) 规则

只有在调用者拥有帧 (frame) 边界后，范围获取才是字节精确的。

利用经过验证的索引 `off` 数组，帧 `i` 的起始位置为：

```text
segment_start + off[i]
```

帧 `i` 的结束位置是下一个已知边界：

```text
segment_start + off[i + 1]       # when i + 1 is still covered
index_frame_start                # for the last covered frame, after a boundary scan
```

当前的索引负载不存储帧长度。因此，客户端不得 (MUST NOT) 仅根据 `off` 推断最后一个覆盖帧的精确字节范围；它必须通过扫描、容器元数据或未来的带长度信息的索引扩展来获知索引帧的起始位置。

在没有索引的情况下，范围获取仍然可行，但需要从段 (segment) 起始位置开始进行顺序 CBOR 边界扫描。只有当请求范围的起始和结束位置是源自已扫描的项目边界时，HTTP `Range` 请求才是安全的。
## 复制工作流

所有引擎 CLI 都实现了复制谓词：

```bash
gts heads local.gts
gts segments local.gts
gts missing --from-head <peer-head> local.gts
gts resume --after <frame-id> local.gts
```

稳定的 Rust JSON 形状为：

```text
gts-replication-heads-v1
gts-replication-segments-v1
gts-replication-missing-v1
```

共享语义：

- `heads` 按文件顺序报告段头，以及适用于对等节点比较的聚合视图；
- `segments` 报告每个段的字节范围、配置文件、头、帧数和布局状态；
- `missing` 将对等节点的已知头与本地段/帧谱系进行比较，并返回精确的字节范围或明确的“未知；需要扫描”结果；
- `resume` 仅在证明请求的帧 ID 存在且输出起始于 CBOR 项边界后才发射字节。
## 内存基准

发布基准套件涵盖了在暴露各表面的引擎上的读取、折叠 (fold)、写入/来自 N-Quads、文件配置文件打包/解包以及流式内存证据：

```bash
just bench-release
```

该套件在 `dist/benchmarks/` 下编写机器可读的 JSON 和 Markdown 报告。使用 [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) 作为 v1 发布说明或论文附录模板。

针对一个或多个 GTS 文件运行本地助手：

```bash
cd python
uv run python ../scripts/bench_reader_memory.py ../vectors/25-streamable-source.gts
```

当 Rust、Cargo、Node、npm 和 TypeScript 构建依赖项可用时，助手会为每个文件发出四行：

- `full-reader`：使用当前的 Python 读取器 (reader) 实例化 `Graph`；
- `frame-scan`：一次解码一个 CBOR 项，并在不进行折叠 (folding) 的情况下计数标头/帧 (headers/frames)；
- `streaming-fold`：运行 Rust `read_to_sink_from_reader` sink 基准 (benchmark) 助手，并在 Linux 上报告 Rust 进程的高水位 RSS (`VmHWM`)；
- `typescript-streaming-fold`：在 Node 的 Web Streams 运行时下运行浏览器 `foldStreamToSink` sink 路径，并报告 Node RSS。

Rust 关系导出回归固定装置涵盖了有界行发射路径：DB 加载器将 SQL 流式传输到 `sqlite3`/`duckdb`，在折叠图中保留未缓存的延迟 blob 条目，并在无法解码转换后的 blob 时在 `COMMIT` 之前停止。剩余的架构约束是故意的：`blobs.bytes` 导出 必须 (MUST) 仍为正在写入的行瞬时解码每个内联有效载荷。

未来的流式实现 应该 (SHOULD) 添加特定于引擎的非实例化 sink 基准 (benchmarks)，以按不同术语、最大解码帧 (frame) 大小、验证 sidecar 状态、triples 和 blob 大小报告峰值内存。

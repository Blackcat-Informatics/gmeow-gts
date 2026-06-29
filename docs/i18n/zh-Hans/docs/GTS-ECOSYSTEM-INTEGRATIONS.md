<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-ECOSYSTEM-INTEGRATIONS.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS Ecosystem 集成合约

> [`docs/GTS-ECOSYSTEM-INTEGRATIONS.md`](../../../../docs/GTS-ECOSYSTEM-INTEGRATIONS.md) 的信息性中文翻译。英文文档仍然是集成、高级功能、可选 profile、基准数据、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。

本文档是配合 RDF 库、数据帧 (data frames)、浏览器、服务和对象存储使用 GTS 的公共合约。核心有线格式 (wire format) 在 [GTS-SPEC.md](./GTS-SPEC.md) 中仍然是规范性的；本文档记录了当前引擎公开的内容、支持的示例以及明确推迟 (deferred) 的内容。

## 状态矩阵

| 生态系统 | 当前集成路径 | 推迟 |
|---|---|---|
| Rust RDF | `gmeow_gts::nquads::to_nquads(&graph)` 和 `gmeow_gts::from_nquads::from_nquads(text)` 仍然是针对外部 RDF crate 的零额外依赖桥接；`--features rdf` 为无需依赖的原生 `Dataset` 互操作（不含嵌入式图形存储）启用了 `gmeow_gts::rdf::{to_rdf_dataset, from_rdf_dataset}`；`--features native-store` 启用了使用确定性原生内存 RDF 存储的 `gmeow_gts::native_store::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}` 和 `Writer::from_store`；`--features rdf-codecs` 启用了原生 N-Triples, Turtle, TriG, 和 RDF/XML 文本编解码器；`gmeow_gts::examples::agent_memory` 演示了一个没有额外依赖的下游应用形态；`gts to-sqlite` 默认导出折叠 (folded) 整数表模型，而 `to-duckdb` 和 `to-parquet` 则位于无依赖 Cargo 特性 `duckdb` 之后。 | Rio 仍然被推迟 (deferred)，因为当前的 `rio_api` crate 在上游被标记为未维护；外部 Sophia/Oxigraph/Rio 互操作使用零依赖的 N-Quads 文本桥接，而不是 crate 内适配器。 |
| Python RDF/数据 | `gts.from_rdflib()` 和 `gts.to_rdflib()` 涵盖了 rdflib RDF 1.1 `Graph`/`Dataset` 互操作；`gts to-sqlite`、`to-duckdb` 和 `to-parquet` 涵盖了关系型/数据帧移交。 | RDF 1.2 引用三元组 (quoted-triple) 向 rdflib 的导出默认是严格的，并且仅在明确请求时才是有损的。 |
| TypeScript 浏览器 | `@blackcatinformatics/gmeow-gts/browser` 为非物化 (non-materializing) 的流式读取器 (Streaming Reader) 层暴露了 `foldStreamToSink(ReadableStream<Uint8Array>, options)`，此外还包括返回图形的 `foldStream`、`readStream`、`toNQuads`、渐进式折叠 (fold) 事件，以及基于 WebCrypto 的 COSE Sign1/Encrypt0 密钥提供者钩子。包根目录还带有一个浏览器条件，为打包工具解析到这个更窄的表面。 | 仅限 Node 的 CLI 和文件系统 `pack`/`unpack`/`diff` 辅助程序仍然位于浏览器导出之外。范围获取 (Range fetch) 仍需要经过验证的索引或边界扫描。 |
| Go 服务 | `reader.ReadFrom(ctx, io.Reader, reader.Options)` 提供返回图形的服务集成 (integration)，而 `reader.ReadToSink(ctx, io.Reader, reader.Options, sink)` 则为 HTTP 正文、对象存储对象和管道提供可取消、字节限制的流式折叠 (fold) 事件；Go CLI 还暴露了共享的复制库存谓词。 | 特定于服务的复制编排仍然是构建在共享谓词上的应用代码。 |
| C ABI 包装器 | `rust/capi/` 为 C 兼容的运行时构建 `libgts` 和 `rust/capi/include/gts.h`。C++、.NET、PHP、Lua、Swift、Ruby、R 和 Julia 包装器在将原生缓冲区复制到生态系统拥有的值中时，暴露 ABI 元数据、读取/验证 JSON 报告、注册表驱动的 RDF 文本转换、files-profile 辅助程序以及结构化错误。 | 这些包装器委托给 Rust 引擎，不是独立的对等引擎或新的 CLI 列。可安装的原生 `libgts` 归档通过 `capi-v*` GitHub Release 通道发布；包装器注册表发布自动化仍然与当前的 Rust/Python/Go/TypeScript 引擎发布通道分开。 |
| Tar 兼容归档 | Rust `gts from-tar`、`gts to-tar` 和 `gts tar -c/-x/-t/-d` 在 `--features tar` 之后可用。它们将 `.tar`、`.tar.gz` 和 `.tar.zst` 流桥接到具有摘要寻址文件体、tar 等效元数据、未知 PAX 保留和显式提取选择加入的 files-profile-v2 GTS 归档。 | Python/Go/TypeScript 对等性被故意推迟 (deferred)。在它们的 CLI 声称支持 `from-tar`、`to-tar` 或 `tar` 之前，这些引擎应该实现 files-profile-v2 导入/导出并通过 `vectors/tar/`。 |
| OKF 捆绑包 | Rust `gts from-okf` 和 `gts to-okf` 在 `--features okf` 之后可用。它们将带有 YAML 前置内容的 Markdown OKF 捆绑包转换为 GTS 配置文件 (profile) `okf` 包，并将 OKF-profile 图形映射回捆绑包目录。提交的语料库 (corpus) 包括 `vectors/okf/bigquery-join/` 下的 BigQuery 风格捆绑包，包括与 Google 检入的 Knowledge Catalog 示例相匹配的无前置内容导航 `index.md` 页面。 | Python/Go/TypeScript 对等性被故意推迟 (deferred)。在它们的 CLI 声称支持 `from-okf` 或 `to-okf` 之前，这些引擎应该实现相同的 `gts-okf-v1` 目录合约并通过 OKF 语料库 (corpus)。 |
| 生态系统 | 当前集成路径 | 推迟项 |
|---|---|---|
| Rust RDF | ``gmeow_gts::nquads::to_nquads(&graph)`` 和 ``gmeow_gts::from_nquads::from_nquads(text)`` 仍是外部 RDF crate 的零额外依赖桥接；``--features rdf`` 启用了 ``gmeow_gts::rdf::{to_rdf_dataset, from_rdf_dataset}`` 以实现无依赖的原生 ``Dataset`` 互操作，且无需内置图存储；``--features native-store`` 通过使用确定性原生内存 RDF 存储启用了 ``gmeow_gts::native_store::{graph_to_store, graph_to_store_with_sidecar, store_to_writer}`` 和 ``Writer::from_store``；``--features rdf-codecs`` 启用了原生 N-Triples、Turtle、TriG 和 RDF/XML 文本编解码器；``gmeow_gts::examples::agent_memory`` 展示了无额外依赖的下游应用形态；``gts to-sqlite`` 默认导出折叠 (fold) 整数表模型，而 ``to-duckdb`` 和 ``to-parquet`` 则位于无依赖 Cargo 特性 ``duckdb`` 之后。 | Rio 仍被推迟 (deferred)，因为当前的 ``rio_api`` crate 在上游被标记为未维护；外部 Sophia/Oxigraph/Rio 互操作使用零依赖的 N-Quads 文本桥接，而非 crate 内适配器。 |
| Python RDF/数据 | ``gts.from_rdflib()`` 和 ``gts.to_rdflib()`` 涵盖了 rdflib RDF 1.1 ``Graph``/``Dataset`` 互操作；``gts to-sqlite``、``to-duckdb`` 和 ``to-parquet`` 涵盖了关系型/数据帧 (data-frame) 移交。 | RDF 1.2 引用三元组 (quoted-triple) 向 rdflib 的导出默认是严格的，并且仅在明确请求时才会进行有损操作。 |
| TypeScript 浏览器 | ``@blackcatinformatics/gmeow-gts/browser`` 为非实例化流式读取器 (Streaming Reader) 层级暴露了 ``foldStreamToSink(ReadableStream<Uint8Array>, options)``，此外还有返回图的 ``foldStream``、``readStream``、``toNQuads``、渐进式折叠 (fold) 事件，以及由 WebCrypto 支持的 COSE Sign1/Encrypt0 密钥提供者钩子。包根目录还包含一个 browser 条件，可为打包器解析到此较窄的表面。 | 仅限 Node 的 CLI 和文件系统 ``pack``/``unpack``/``diff`` 助手仍留在浏览器导出之外。范围抓取 (Range fetch) 仍需要经验证的索引或边界扫描。 |
| Go 服务 | ``reader.ReadFrom(ctx, io.Reader, reader.Options)`` 提供了返回图的服务集成，而 ``reader.ReadToSink(ctx, io.Reader, reader.Options, sink)`` 为 HTTP 主体、对象存储对象和管道提供了感知取消且字节受限的流式折叠 (fold) 事件；Go CLI 还暴露了共享的复制清单动词。 | 特定于服务的复制编排仍是构建在共享动词之上的应用代码。 |
| C ABI 包装器 | ``rust/capi/`` 为 C 兼容运行时构建了 ``libgts`` 和 ``rust/capi/include/gts.h``。C++、.NET、PHP、Lua、Swift、Ruby、R 和 Julia 包装器在将原生缓冲区复制到生态系统拥有的值时，会暴露 ABI 元数据、读取/验证 JSON 报告、注册表驱动的 RDF 文本转换、文件配置文件 (files-profile) 助手和结构化错误。 | 这些包装器委托给 Rust 引擎，并非独立的一致性引擎或新的 CLI 列。可安装的原生 ``libgts`` 归档通过 ``capi-v*`` GitHub Release 通道发布；包装器注册表发布自动化仍与当前的 Rust/Python/Go/TypeScript 引擎发布通道保持分离。 |
| 兼容 Tar 的归档 | Rust ``gts from-tar``、``gts to-tar`` 和 ``gts tar -c/-x/-t/-d`` 在 ``--features tar`` 之后可用。它们将 ``.tar``、``.tar.gz`` 和 ``.tar.zst`` 流桥接到具有摘要寻址文件主体、等效于 tar 的元数据、未知 PAX 保留和显式提取选择加入的 `files-profile-v2` GTS 归档。 | Python/Go/TypeScript 的一致性被故意推迟 (deferred)。这些引擎在它们的 CLI 声明 ``from-tar``、``to-tar`` 或 ``tar`` 之前，应该 (SHOULD) 实现 `files-profile-v2` 导入/导出并通过 ``vectors/tar/``。 |
| OKF 捆绑包 | Rust ``gts from-okf`` 和 ``gts to-okf`` 在 ``--features okf`` 之后可用。它们将带有 YAML 前置内容 (frontmatter) 的 Markdown OKF 捆绑包转换为 GTS 配置文件 ``okf`` 包，并将项目 `OKF-profile` 图转换回捆绑包目录。提交的一致性语料库 (conformance corpus) 包含 ``vectors/okf/bigquery-join/`` 下的一个 BigQuery 风格捆绑包，其中包括与 Google 签入的知识目录 (Knowledge Catalog) 样本相匹配的无前置内容导航 ``index.md`` 页面。 | Python/Go/TypeScript 的一致性被故意推迟 (deferred)。这些引擎在它们的 CLI 声明 ``from-okf`` 或 ``to-okf`` 之前，应该 (SHOULD) 实现相同的 ``gts-okf-v1`` 目录合约并通过 OKF 语料库 (corpus)。 |

## C ABI 包装器合约

C ABI 兼容性策略位于 `rust/capi/README.md#compatibility-policy`(../rust/capi/README.md#compatibility-policy)。`GTS_ABI_VERSION` 规范了原生 `gts.h`/`libgts` 边界，并与软件包版本和 JSON 报告模式版本分离。包装器包必须 (MUST) 拒绝不受支持的 ABI 版本，并提供明确的包装器错误、异常或安装/配置失败，而不是在未知的原生合约下静默继续。

文件配置文件 (Files-profile) 路径助手使用 ABI v1 以 NUL 结尾的 UTF-8 C-字符串路径合约。包装器文档不得 (MUST NOT) 将这些助手呈现为具有完整的 Windows 宽字符路径覆盖；未来的宽字符路径函数应该是兼容性策略下新的增量 C ABI 符号。

## Tar 兼容归档桥接

Rust tar 桥接使得 GTS 能够作为已签名、仅追加、已去重的归档表面，供已经熟悉 tar 的用户使用。`gts from-tar` 将 tar 流导入到 files-profile-v2 GTS 归档中，`gts to-tar` 将这些归档导出回 tar，而 `gts tar -c/-x/-t/-d` 提供了熟悉的 create/extract/list/diff 命令形式。该桥接处理普通的 `.tar`、`.tar.gz` 和 `.tar.zst` 流，在配置文件 (profile) 可以表示的情况下保留等效于 tar 的元数据和未知的 PAX 记录。

对于大型归档，Rust 导入/创建路径避免了直接 GTS 创作路径上常驻内存随常规文件有效负载字节缩放的问题：`gts from-tar` 将 tar 输入作为流进行解码，在收集排序元数据的同时缓冲常规文件主体，并从有界块中发出 blob 帧 (frames)；`gts tar -cf out.gts ...` 在有界块中对源文件有效负载进行哈希和写入。折叠 (folded) 的 `to-tar` 路径仍从内存中的 `Graph` 表示形式导出，而 `.tar.zst` 输出仍使用当前物化编码投影的 zstd 后端路径。这些是实现边界，而非格式要求。

当验证至关重要时，规范产物应该是 `.gts` 文件：帧 (frame) ID、可选签名、仅追加修订、抑制和内容寻址 blob 对 GTS 读取器 (readers) 保持可见。传统的 `.tar`、`.tar.gz` 和 `.tar.zst` 输出是针对尚不支持 GTS 的工具链的有用兼容性投影，但当已签名的 GTS 链作为证据记录时，它们应该被视为派生导出。

产物注册表和对象存储可以使用 `application/vnd.blackcat.gts+cbor-seq` 直接携带 GTS 归档。OCI 或发布资产发布者可以将 `.gts` 产物与生成的 tar 投影一起分发：注册表为感知 GTS 的消费者获取单个内容寻址归档，而现有的 tar 消费者保留熟悉的下载路径。同样的划分也适用于 OKF 捆绑包：可编辑的 OKF 目录仍然是人工创作表面，`gts from-okf` 创建语义化的 `okf` 配置文件 (profile) 包，而 files-profile-v2 tar 桥接可以在消费者需要普通归档工具时，将目录字节打包为可验证的 tarball 形状的分发产物。

## OKF：知识编目（Knowledge Catalog）与 BigQuery 捆绑包

OKF 互操作具有两个有用的入口（gates）：

- 提交的、封闭的入口是 `vectors/okf/bigquery-join/`。它模拟了 BigQuery 数据集、表、表连接、扩展前置内容（frontmatter）、Markdown 链接以及导航 `index.md` 文件，而不依赖于 Google 凭证或上游样本漂移。
- 实时生态系统入口是 <https://github.com/GoogleCloudPlatform/knowledge-catalog>。其 `okf/bundles/` 样本由知识编目（Knowledge Catalog）OKF 增强概念验证产生，其可视化工具消耗相同的 Markdown + YAML 前置内容目录界面。

任一入口的 Rust 命令序列为：

```bash
cargo run --features okf --bin gts -- from-okf okf-bundle/ -o bundle.gts
cargo run --bin gts -- verify bundle.gts
cargo run --features okf --bin gts -- to-okf bundle.gts --directory restored-okf/
```

`from-okf` 导入带有 YAML 前置内容的概念文档，并将不带前置内容的 `index.md` 文件视为导航页面。这些页面不是 GTS 配置文件 (profile) 中的概念，因此不会由 `to-okf` 发射；需要静态浏览页面的消费者可以从导出的概念集中重新生成它们。

该桥梁将 OKF 定位为 GMEOW 知识的人工创作前端：人员和代理编辑 Markdown，而 GTS 提供仅追加 (append-only) 打包、内容寻址主体、签名、抑制以及用于审计和机器使用的图投影 (graph projections)。

## Python: rdflib 与数据帧 (Data Frames)

Python 软件包拥有最丰富的生态系统桥接，因为它已经包含了针对 RDF 和数据库目标的可选扩展 (optional extras)：

```bash
pip install 'gmeow-gts[rdf,db]'
```

RDF 1.1 数据集往返：

```python
import gts
from rdflib import Dataset, Literal, URIRef
from rdflib.namespace import RDFS

ds = Dataset()
graph = ds.graph(URIRef("https://example.org/graph"))
graph.add((
    URIRef("https://example.org/Cat"),
    RDFS.label,
    Literal("Cat", lang="en"),
))

data = gts.from_rdflib(ds)
folded = gts.read(data)
assert sorted(gts.to_nquads(folded).splitlines()) == sorted(
    ds.serialize(format="nquads").splitlines()
)

back = gts.to_rdflib(folded)
```

RDF 1.2 限制：

- rdflib 稳定的 RDF 1.1 数据集解析器无法忠实地表示 GTS 引用三元组项 (quoted-triple terms) 或 `rdf:reifies <<( ... )>>` 语法。
- 当 N-Quads 投影包含引用三元组时，`gts.to_rdflib(graph)` 会抛出 `RDF12UnsupportedError`。
- `gts.to_rdflib(graph, allow_rdf12_lossy=True)` 会丢弃包含引用三元组语法的 N-Quads 行，并解析剩余的兼容 RDF 1.1 的图。

关系型/数据帧 (data-frame) 衔接由 Python 和 Rust 公开：

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

Python DuckDB 和 Parquet 导出需要 `pip install 'gmeow-gts[db]'`。Rust 默认使用 `sqlite3` 处理 SQLite。Rust DuckDB/Parquet 导出在构建时包含 `--features duckdb` 即可用；它们不增加 Rust crate 依赖，并在 `PATH` 上外部调用 `duckdb`。

性能预期：这些导出使用整数 ID 折叠 (folded) 模型。`terms`、`quads`、`reifiers`、`annotations` 和 `blobs` 在导出期间进行批量加载而无需解析 IRI；使用者 (consumers) 通过 `terms` 表进行连接。Rust 路径将行增量写入 `sqlite3`/`duckdb`，因此它不会一次性保留所有 SQL 行或完整的加载脚本。`blobs` 表仍然保留有效载荷字节，因此转换后的内联 blob 有效载荷在发射其对应行时会被瞬时解码。SQLite 足以满足小型本地检查。DuckDB 和 Parquet 是 Pandas、Polars、DuckDB SQL 和 Arrow 式扫描的首选路径，因为它们保留了字典编码，并允许目标引擎选择投影/过滤顺序。

## Rust: RDF Crate

目前的 Rust 互操作性保持了默认 crate 的显式和低依赖特性：

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let nquads = gmeow_gts::nquads::to_nquads(&graph);
```

应用程序可以将 `nquads` 提供给 Sophia、Rio、Oxigraph 或其他 RDF crate。这是 v1 的稳定桥梁，因为核心 crate 不应该将图形数据库或 RDF 工具包强加给每个传输用户。

反向纯图形路径也是显式的：

```rust
let bytes = gmeow_gts::from_nquads::from_nquads(nquads.as_str())?;
```

对于原生 Rust 数据模型互操作，启用可选的 `rdf` 特性：

```toml
gmeow-gts = { version = "0.9.10", default-features = false, features = ["rdf"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let dataset = gmeow_gts::rdf::to_rdf_dataset(&graph)?;
let bytes = gmeow_gts::rdf::from_rdf_dataset(&dataset)?;
```

`rdf` 特性使用了 GTS 原生的 RDF 数据集、四元组 (quad)、项 (term)、图形名称、字面量和引用三元组 (quoted-triple) 类型。它刻意不依赖 `oxrdf` crate 或外部 RDF 存储，因此 `--features rdf` 仍然适用于 `wasm32-unknown-unknown` 构建。

对于原生内存中 RDF 存储互操作，启用可选的原生存储：

```toml
gmeow-gts = { version = "0.9.10", default-features = false, features = ["native-store"] }
```

```rust
let graph = gmeow_gts::reader::read(&bytes, true, None);
let package = gmeow_gts::native_store::graph_to_store_with_sidecar(graph)?;
let writer = gmeow_gts::writer::Writer::from_store(&package.store, "dist")?;
```

`native-store` 特性仅依赖于 `rdf`。存储投影是纯 RDF；仅限 GTS 的状态，如 blob、抑制 (suppressions)、签名、诊断、段头和可流式处理布局 (streamable-layout) 元数据，则在旁路文件 (sidecar) 中返回。适配器遍历原生四元组，且不会在热路径中实例化 N-Quads 文本。

对于 Sophia、Oxigraph、Rio 或其他外部 RDF crate，请将依赖关系保持在应用程序边界，并与 GTS 交换 N-Quads 文本。核心 Rust crate 不发布 crate 内置的 Sophia 适配器，因为 Sophia 的 N-Quads 栈会将 UUID 生成引入 all-features 依赖图中。原生的 `rdf`、`native-store` 和 `rdf-codecs` 特性涵盖了 crate 内的结构化和文本互操作路径，同时保留了 `wasm32-unknown-unknown` 构建。
CI 还将 all-features WASM 视为永久的 Rust 库合约：`scripts/check_rust_wasm_dependency_audit.py` 检查 `wasm32-unknown-unknown --all-features` 的常规/构建依赖树，如果出现 Oxigraph/OxRDF/OxTTL/OxRDFXML、Sophia crate、`uuid` 或 `getrandom` 0.3，则构建失败。

默认执行严格导出。GTS 具体化器 (reifiers) 投影到宾语位置的 RDF 1.2 三元组项。如果 GTS 图在原生数据集表面有意不表示的位置（如主语或图形名称位置）使用了引用三元组，`to_rdf_dataset` 将抛出 `RdfAdapterError`。显式的 `to_rdf_dataset_lossy` 路径仅丢弃那些无法表示的行，并由特性门控 (feature-gated) 的测试覆盖。

为了实现应用对等，Rust crate 包含了一个可运行的接地记忆 (grounded-memory) 示例：

```bash
cargo run --example agent_memory
```

`gmeow_gts::examples::agent_memory::Memory` 追加断言 (claims)、修订或抑制断言、记录工具调用出处 (provenance)、通过确定性的令牌重叠召回断言，并生成由 `gts verify` 接收的包。这是基于 GTS 的应用示例，而非核心读取器的前提条件。

Rust 数据帧 (data-frame) 传递使用了与 Python 导出相同的折叠 (folded) 表：

```bash
gts to-sqlite package.gts package.sqlite
gts to-duckdb package.gts package.duckdb
gts to-parquet package.gts out-parquet/
```

Rust 二进制文件将这些保留为运行时工具集成，而非默认的 crate 依赖：`to-sqlite` 在默认构建中调用 `sqlite3`，而 `to-duckdb` 和 `to-parquet` 由无依赖的 `duckdb` Cargo 特性启用，并调用外部的 `duckdb` 二进制文件。Rust 加载器将行 SQL 流式传输到这些工具，并分阶段替换输出；它不会在内存中构建完整的行集或 SQL 脚本。
跟踪的推迟 (Tracked deferral)：额外的原生 Rust RDF 适配器应该仅作为可选特性 (optional features) 添加。Rio 仍然被推迟 (deferred)，直到选定一个维护的 Rio 兼容 crate 或替代路径。任何未来的适配器必须 (MUST) 包含针对 IRIs、空节点 (blank nodes)、语言字面量 (language literals)、数据类型 (datatypes)、命名图 (named graphs) 和 RDF 1.2 reifier 限制的回环测试 (round-trip tests)，不得 (MUST NOT) 对嵌入式数据库添加默认依赖，并且当目标 crate 无法保留该行为时，必须 (MUST) 记录 quoted-triple 行为。

## TypeScript: 浏览器和范围获取 (Range Fetch)

TypeScript 软件包为 Web Streams 暴露了一个浏览器专用的入口点：

```typescript
import { foldStream, foldStreamToSink, readStream, toNQuads } from "@blackcatinformatics/gmeow-gts/browser";

const response = await fetch("/artifacts/example.gts");
const result = await foldStream(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") renderQuad(event.quad);
    if (event.kind === "blob") renderBlob(event.digest, event.size);
  },
});

console.log(toNQuads(result.graph));

await foldStreamToSink(response.body!, {
  onEvent(event) {
    if (event.kind === "quad") projectRow(event.segmentIndex, event.quad);
  },
});
```

浏览器路径还可以使用平台 WebCrypto 进行实际的 COSE 验证和解密：

```typescript
const graph = await readStream(response.body!, {
  keys: {
    verificationKey: (kid) => lookupEd25519PublicKey(kid),
    contentKey: (kid) => lookupAes256GcmContentKey(kid),
  },
});
```

浏览器导出按帧 (frame) 顺序发出 term、quad、reifier、annotation、suppression、blob、opaque、signature、diagnostic、segment-head 和 streamable-layout 事件。`foldStreamToSink` 是 TypeScript 软件包的非实例化 `GTS Streaming Reader` 界面；`foldStream` 和 `readStream` 仍然是返回图形的便捷方法。根 Node `Read(bytes, allowSegments)` API 仍然是一个实例化读取器 (reader)，浏览器代码不得 (MUST NOT) 依赖仅限于 Node 的 CLI/文件系统助手。

范围规则：调用者可以 (MAY) 仅针对从索引帧 (index frame) 或顺序 CBOR 边界扫描中得知的字节跨度使用 HTTP `Range`。切割 CBOR 项的范围属于撕裂追加 (torn append)，必须 (MUST) 被视为不完整的前缀。

## Go：服务和对象存储

Go 调用者应在服务边界使用 `reader.ReadFrom`：

```go
func handleGTS(w http.ResponseWriter, r *http.Request) {
    graph, err := reader.ReadFrom(r.Context(), r.Body, reader.Options{
        AllowSegments: true,
        MaxBytes:      64 << 20,
    })
    if err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }
    _, _ = io.WriteString(w, nquads.ToNQuads(graph))
}
```

同样的 API 也适用于对象存储 SDK 读取器：

```go
obj, err := client.GetObject(ctx, bucket, key)
if err != nil {
    return nil, err
}
defer obj.Body.Close()

graph, err := reader.ReadFrom(ctx, obj.Body, reader.Options{
    AllowSegments: true,
    ExpectedHead:  expectedHead,
    MaxBytes:      512 << 20,
})
```

`ReadFrom` 有意设计为一个有界的全读取器（full-reader）封装。当调用者需要具像化的 `*model.Graph` 时，它为 Go 服务提供惯用的取消（cancellation）机制和资源限制。返回的图（graph）仍然携带读取器诊断信息，而不是将格式诊断转换为 Go 错误。

对于流式折叠（streaming folds），调用者可以将段本地（segment-local）折叠事件发送到接收器（sink），而无需构建最终的联合图（union graph）：

<!-- markdownlint-disable MD010 -->
```go
var sink reader.StreamingSink = reader.StreamingSinkFunc(func(event reader.StreamingEvent) error {
	if event.Kind == reader.StreamingEventQuad {
		// project or forward event.Quad here
	}
	return nil
})

result, err := reader.ReadToSink(ctx, obj.Body, reader.Options{
	AllowSegments: true,
	ExpectedHead:  expectedHead,
	MaxBytes:      512 << 20,
}, sink)
```
<!-- markdownlint-enable MD010 -->

`result.Diagnostics`、`result.SegmentHeads` 和 `result.SegmentStreamable` 与相同输入和选项的全读取器（full reader）相匹配。

## 复制与服务边界

对于当前服务：

- 在任何引擎中使用 `gts heads` / `gts segments` 来清点段 (segment) 头部和字节范围。
- 使用 `gts ls` 或折叠 (folded) 的 `Graph.Blobs`/`BlobMeta` 来清点内联对象。
- 从 HTTP 或对象存储提供字节范围服务时，请使用上述范围规则。
- `gts missing` 和 `gts resume` 在每个引擎中提供稳定的字节范围恢复界面。
  更高层级的服务间协议仍是构建在 [GTS-ADVANCED-PRIMITIVES.md](./GTS-ADVANCED-PRIMITIVES.md) JSON 形状和边界规则之上的应用程序代码。

## 合约守卫

`scripts/check_ecosystem_contract.py` 验证此文档是否保留了状态矩阵、各生态系统章节、推迟语言以及公开文档链接。它是集成承诺的偏移守卫，而非引擎测试的替代品。

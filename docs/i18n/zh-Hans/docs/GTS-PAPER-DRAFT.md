<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-PAPER-DRAFT.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS：一种用于 RDF 图和二进制产物的内容寻址仅追加传输基底

> [`docs/GTS-PAPER-DRAFT.md`](../../../../docs/GTS-PAPER-DRAFT.md) 的信息性中文翻译。英文文档仍然是集成、高级功能、可选 profile、基准数据、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。


Graph Transport Substrate (GTS) 的论文草案叙述。

本文档为参考性研究材料。它不定义 GTS 的规范性行为。
规范性要求保留在 [`GTS-SPEC.md`](./GTS-SPEC.md) 中，可测试的层级与向量
规则位于 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md)，信任/配置文件 (profile) 策略位于
[`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md)，变更控制位于
[`GTS-GOVERNANCE.md`](./GTS-GOVERNANCE.md)。
## 1. 引言

图数据现在跨越了本地优先应用 (local-first applications)、溯源工作流 (provenance workflows)、证据包 (evidence packages)、归档、服务边界和 AI 记忆系统。在这些场景下的传输构件 (transport artifact) 不仅仅需要移动三元组 (triples)：它必须保留二进制有效负载 (binary payloads)，在不重写旧字节的情况下追加历史记录，在缺少编解码器或密钥的情况下存续，并允许独立实现对字节含义达成一致。

GTS 将该问题界定为传输而非存储。其持久构件是一个以 `application/vnd.blackcat.gts+cbor-seq` 形式提供的 `.gts` 文件：这是一个 CBOR Sequence，其逻辑数据集是通过确定性折叠 (deterministic fold) 生成的。查询系统、数据库、对象存储、缓存和领域配置文件 (domain profiles) 围绕构件构建，而非内置于核心格式之中。

本研究的预期贡献包括：

1. 一种用于仅追加的图和二进制日志的 CBOR Sequence 线路格式。
2. 一种涵盖 RDF 1.2 术语 (terms)、四元组 (quads)、具象化器 (reifiers)、注解 (annotations)、二进制大对象 (blobs)、元数据 (metadata)、抑制 (suppression)、快照 (snapshots) 和不透明帧 (opaque frames) 的确定性折叠模型 (deterministic fold model)。
3. 一个具有可选 COSE 签名、可选加密以及针对缺失功能的不透明模型 (opacity model) 的内容寻址 id/prev 链。
4. 通过字节拼接实现的多段 (multi-segment) 组合，以及面向交付构件的可流式处理布局 (streamable layout) 紧缩。
5. 一个跨语言一致性语料库 (conformance corpus) 以及 Rust、Python、Go、TypeScript、Smalltalk/Pharo 和 Kotlin/JVM 的参考实现。
## 1. 引言

图数据现在跨越了本地优先应用、出处工作流、证据包、归档、服务边界以及 AI 记忆系统。在这些场景下的传输构件必须移动的不仅仅是三元组：它必须保留二进制负载，在不重写旧字节的情况下追加历史，在缺少编解码器或密钥的情况下生存，并允许独立实现对字节含义达成一致。

GTS 将该问题框架化为传输而非存储。持久化构件是一个被作为 `application/vnd.blackcat.gts+cbor-seq` 提供的 `.gts` 文件：一个其逻辑数据集由确定性折叠 (fold) 产生的 CBOR Sequence。查询系统、数据库、对象存储、缓存和领域配置文件 (profile) 围绕构件而存在，而非位于核心格式内部。

该工作的预期贡献包括：

1. 一种用于仅追加图和二进制日志的 CBOR Sequence 线路格式。
2. 一种用于 RDF 1.2 术语、四元组、reifiers、注解、blobs、元数据、抑制、快照和不透明帧 (opaque frames) 的确定性折叠 (fold) 模型。
3. 一个带有可选 COSE 签名、可选加密以及针对缺失能力的不透明度模型的内容寻址 id/prev 链。
4. 通过字节拼接实现的多段组合，以及针对面向交付构件的可流式处理布局 (streamable layout) 紧凑化。
5. 一个跨语言的一致性语料库 (conformance corpus) 以及在 Rust、Python、Go、TypeScript、Smalltalk/Pharo 和 Kotlin/JVM 中的参考实现。
## 设计概览

GTS 被设计为一个窄腰结构：

```text
Applications and profiles
generic graphs | files | evidence | images | media packages | GMEOW | agent memory
|
v
GTS narrow waist
CBOR Sequence segments
deterministic-CBOR headers and frames
BLAKE3 id/prev chains
transform catalog
deterministic fold
opaque-node degradation
|
v
Storage and transport
filesystem | HTTP range | object storage | artifact registries | message buses
```

核心格式不绑定于特定的本体、数据库、查询引擎、可变事务模型或信任框架。领域配置文件（profile）在腰部之上增加词汇和验证。部署则在腰部之下选择存储和提供服务的行为。任何一方都不会改变核心头部/帧（frame）语法、内容 ID 原像、段（segment）边界规则或折叠（fold）语义。

当前软件包系列命名为 `gmeow-gts`；格式为 GTS。GMEOW 是一个主要的下游消费者和分发用例，但依赖方向是单向的：GTS 读取器（reader）不需要 GMEOW 词汇、OWL 推理、音乐领域规则或代理内存约定即可解析、验证、折叠（fold）或传输 GTS 文件。
## 3. Wire Format

GTS 文件是由一个或多个 segment/段组成的 CBOR Sequence。一个 segment/段包含一个确定性 CBOR 头部，后跟确定性 CBOR frame/帧。发布的工件所使用的注册临时媒体类型为 `application/vnd.blackcat.gts+cbor-seq`，文件扩展名为 `.gts`。

在叙述层面上，文件结构为：

```text
GTS file
  segment 0
    header: magic/version/profile/catalog/layout/metadata/id
    frame:  type + transform chain + public envelope + payload + prev + id + optional sig
    frame:  ...
  segment 1
    header
    frame
    ...
```

每个 frame/帧的内容标识符是针对确定性字节的 BLAKE3-256 摘要。`prev` 字段将 frame/帧链接到其 segment/段中的前一个项目。由于 segment/段头部和 frame/帧是 CBOR Sequence 项目，因此可以连接独立有效的 segment/段而无需重写其字节。生成的文件 fold/折叠为 segment/段 fold/折叠的有序值并集。

有效负载使用转换目录。基准面包含了核心 reader/读取器所需的强制性结构路径，而可选的编解码器和加密转换则取决于能力。当周围字节保持可恢复时，未知的编解码器、不受支持的 frame/帧类型或不可用的密钥将表示为诊断和不透明节点 (opaque graph nodes)。

可选的索引帧可以携带偏移表、帧类型索引和 MMR 根。目前的支持范围是有意划定的：独立的 MMR 证明验证是跨引擎的，Rust 可以从索引的 GTS 文件创建证明，更广泛的随机访问/证明创建表面仍作为高级原语进行跟踪，而非基准 reader/读取器需求。
## 4. 折叠 (Fold) 语义

折叠 (Fold) 是将段 (segment) 帧 (frame) 确定性地重放为 RDF 数据集形状状态的过程。
相关状态包括：

- RDF 项 (terms)，包括 IRI、字面量、空白节点和被引用的三元组。
- 四元组 (Quads) 和语句级注释。
- 再定体 (Reifier) 绑定。
- 按摘要、媒体类型和大小分类的内联 blob 摘要。
- 段 (segment) 元数据、配置文件 (profiles)、诊断信息和段头。
- 抑制记录和不透明节点 (opaque nodes)。

项 ID (Term ids) 是段本地的。跨段标识基于 RDF 项的值，而非本地整数 ID，且空白节点标签不会在独立生成的段之间合并。因此，追加新段会保持现有字节完整，同时增加另一个折叠 (fold) 贡献。

抑制是累加的。它在先前的图声明之上记录显示或有效性策略，而不会物理删除较早的签名字节。快照压缩可以将图重写为更小的分发构件，但相对于完整的追加历史，这种重写是显式的且有损的。

本论文应将折叠 (fold) 模型视为核心抽象，但不应重新阐述新的规范性规则。形式化符号可以将规范模型总结为：

```text
fold(file) = value_union(fold(segment_0), ..., fold(segment_n))
```


确切的语法、重复行为、抑制行为、诊断和一致性 (conformance) 预期仍归规范和一致性文档所有。
## 5. 完整性、机密性与不透明性

GTS 分离了四个关注点：

- 帧 (frame) 完整性：每个帧都有其自身的 BLAKE3 内容 ID。
- 历史完整性：`prev` 链接将帧提交到其链位置。
- 来源或署名：可选的 COSE 签名可以将签名者绑定到帧 ID。
- 新鲜度或非截断性：需要外部或带内 (in-band) 头部承诺来检测丢失的尾部帧。

前两者是无需密钥的格式属性。后两者是配置文件 (profile) 或部署选择。这种区分对于研究叙述非常重要：有效的签名证明了密钥签署了特定字节，但对该密钥的信任以及 RDF 声明的真实性则是部署或配置文件策略。

不透明性模型也是传输设计的一部分。没有编解码器或密钥的读取器 (reader) 仍可以保留位置、帧类型、公共封包、接收者标识符、签名和诊断信息。内容可以被隐藏，但隐藏内容的存在和链位置仍然是可观察的。这使得降级读取变得显式且可测试，而不是静默地丢弃信息。

当前 v1 加密状态应该进行狭义描述。COSE_Sign1 和单接收者 COSE_Encrypt0 是已实现的可选完整读取器 (Full Reader) 能力。多接收者 COSE_Encrypt 封包和 ECDH 密钥封装在字节级固定装置 (fixtures)、互操作性测试和密钥管理策略存在之前，被推迟 (deferred) 在 v1 一致性之外。
## 6. 一致性和实现状态

存储库包含六个引擎：

| 引擎 | 包表面 | 当前角色 |
|---|---|---|
| Rust | `gmeow-gts`，二进制文件 `gts` | 参考包、事件驱动的投影 API、仅限 Rust 的证明创建、CLI 转换。 |
| Python | `gmeow-gts`，模块 `gts` | 参考语料库生成器和 Python 包。 |
| Go | `go.blackcatinformatics.ca/gts` | Go 包和带有流式接收器证据的 CLI。 |
| TypeScript | `@blackcatinformatics/gmeow-gts` | npm 包、Node 读取器表面以及浏览器渐进流/WebCrypto 表面。 |
| Smalltalk/Pharo | Tonel + Metacello 源包，Docker `gts` 运行时 | 用于公共语料库、CLI 和互操作表面的 Pharo 引擎。 |
| Kotlin/JVM | Gradle 源包和 `gts` 运行时 | 用于公共语料库、CLI 和 Java 可调用库表面的 JVM 引擎。 |

共享的兼容性预测器 (oracle) 是 `vectors/` 下签入的向量语料库，以及 `vectors/manifest*.json` 下的聚合及作用域便携清单。一致性声明列出了层级 (tier)、语料库版本、向量子集、启用的可选功能以及生成证据的命令或测试框架。

论文叙述的相关层级 (tier) 包括：

- 基准读取器 (Baseline Reader)：解析、验证、折叠 (fold)、报告诊断，并将不支持的可恢复帧 (frame) 降级为不透明节点 (opaque node)。
- 流式读取器 (Streaming Reader)：基准读取器行为加上避免将整个图实例化的接收器/事件 API。在当前存储库中，Go 针对 `reader.ReadToSink` 声明了此层级，Rust 针对 `read_to_sink_from_reader` 声明了此层级，TypeScript 浏览器导出针对 `foldStreamToSink` 声明了此层级。
- 全功能读取器 (Full Reader)：基准读取器行为加上声明的可选功能，如 COSE、解密、嵌套 GTS 递归、安全策略或索引/MMR 行为。
- 写入器 (Writer) 和验证工具：确定性输出，以及在做出此类声明时执行更严格的工具/配置文件 (profile) 检查。

实现状态应作为动态的存储库事实呈现，而非标准声明。在本草案撰写时，所有六个引擎都被描述为其公共表面均针对共享语料库进行准入检查，而若干功能则刻意保留在基准之外：并非每个引擎都提供数据库和 Parquet 导出，非 Rust 的证明创建已推迟 (deferred)，范围获取助手仍依赖于已验证的边界，对象存储服务模式是集成 (integration) 合约而非核心格式行为，且多接收者加密仅作为推迟的合约描述符被固定。
## 7. 评估计划

论文应仅报告来自可重复发布制品的测量结果。当前仓库在 [`GTS-BENCHMARK-RELEASE-REPORT.md`](./GTS-BENCHMARK-RELEASE-REPORT.md) 中提供了一个基准 (benchmark) 运行器和报告模板。此草案的实测证据运行结果保留在 [`dist/benchmarks/paper-evidence/release-benchmark-report.md`](../../../../dist/benchmarks/paper-evidence/release-benchmark-report.md) 的生成输出中，而不是覆盖基准 (benchmark) 模板。

评估应涵盖四个主张：

1. 正确性和互操作性：每个引擎都将相同的向量 (vector) 字节折叠 (fold) 为其声称级别所预期的相同图摘要、段 (segment) 头、诊断和不透明原因。
2. 流式处理行为：项边界前缀折叠 (fold) 为有效的中间状态，且声称具有流式读取器 (Streaming Reader) 状态的引擎提供具有有界内存行为的 sink/API 证据。当前论文主张应引用 Go 的级别声明，并将 Rust/TypeScript 仅描述为事件化或渐进式证据。
3. 完整性行为：损坏的帧 (frame)、断裂链、撕裂的追加、截断锚点以及不支持的功能会产生预期的可恢复或致命诊断。
4. 实用性：GTS 可以投射到诸如 N-Quads、SQLite、DuckDB 和 Parquet 等操作底层，只要相关引擎公开了这些转换目标，并报告而非忽略其中的失败和差距。

出版附录的建议表格：

- 按引擎、级别、向量 (vector) 子集和语料库 (corpus) 修订划分的语料库 (corpus) 通过/失败情况；
- 按引擎划分的读取 (read)、折叠 (fold)、写入 (write)、打包 (pack) 以及解包 (unpack) 耗时；
- 全量读取器 (full-reader) 和流式读取器 (streaming-reader) 路径的峰值内存或分配证据；
- 跨编解码器选择和可流式处理压缩的文件大小比较；
- 在有无偏移索引情况下的损坏输入恢复行为；
- 在边界已知后，渐进式交付示例的范围获取 (range-fetch) 字节节省。
## 8. 应用 (Applications)

GTS 旨在支持多种应用族，而不将其中任何一种作为其核心标识：

- 数据集和本体分发：发布一个可验证的图包及其命名的二进制资产。
- GMEOW 分发：将 GMEOW 本体包和配置文件 (profiles) 作为 GTS 制品发运，同时保持 GTS 独立于 GMEOW。
- 归档和文件清单：使用图原生元数据和内容寻址的 blob 封装目录树。
- 证据和监管链：在不重写先前历史的情况下追加观测结果、签名和密封负载。
- 本地优先的图同步：连接独立生成的段 (segments) 并对值并集进行折叠 (fold)。
- 图像和媒体包：以目录元数据和小型表现形式引导，随后在同一可验证流中携带较大的 blob 和来源信息 (provenance)。
- 代理记忆和信念修正：将观测结果、抑制项和来源信息作为一种应用级配置文件 (profile) 追加，而非作为格式的标识。
- 图数据库交换：当相关转换可用时，将折叠 (folded) 的图状态投射到 N-Quads、SQLite、DuckDB、Parquet 或其他系统中。
## 9. 局限性与未来工作

GTS 不是查询语言、推理器、可变数据库、共识协议、密钥发现系统、信任框架或外部 blob 可用性保证。应用级冲突解决仍位于核心折叠 (fold) 之上，部署方仍负责信任锚、签名者授权、密钥轮转、撤销和外部头部承诺 (head commitments)。

已知局限性及当前的推迟 (deferrals) 项包括：

- 截断检测需要头部承诺 (head commitment)，例如签名头部、索引根、发布清单或外部锚点。
- 机密帧 (confidential frames) 仍可能泄露存在性、类型、收件人标识符、签名和链位置。
- 绕过任意字节损坏的恢复需要已知偏移量或外部成帧 (framing)；裸 CBOR 序列在字节受损后可能会失去同步。
- 压缩、解压缩和嵌套 GTS 递归需要显式的资源预算。
- 多收件人 COSE_Encrypt 和 ECDH 密钥包装 (key-wrap) 在 v1 一致性 (conformance) 范围之外。
- 跨引擎证明创建、更深层的范围获取辅助工具以及对象存储/服务工作流属于高级表层，而非核心读取器 (reader) 要求。
- 可选标准和特定领域配置文件 (profiles) 在提出强力主张之前，需要治理、测试向量和明确的兼容性说明。
- 发布和出版主张需要加盖戳记的语料库 (corpus) 修订版，而非检入的清单占位符。
## 10. 相关工作

GTS 故意与几个成熟领域重叠，但它在设计空间中占据了不同的位置：一个单一的传输工件，它是仅追加的、内容寻址的、折叠 (fold) 后呈 RDF 形状的、可感知二进制负载的、部分可读的，并且由跨引擎的一致性语料库 (conformance corpus) 覆盖。

**RDF 序列化与图交换。** W3C RDF 序列化，如 [RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/)、[TriG](https://www.w3.org/TR/rdf12-trig/)、N-Triples/N-Quads、Turtle 和 [JSON-LD 1.1](https://www.w3.org/TR/json-ld11/)，定义了编写 RDF 图或数据集的互操作方式。HDT（W3C 关于 [Header-Dictionary-Triples](https://www.w3.org/submissions/2011/SUBM-HDT-20110330/) 的成员提交）解决了紧凑二进制 RDF 的发布和交换。GTS 的不同之处在于将 RDF 投影视为仅追加二进制日志的折叠 (fold)，该日志还可以携带内容寻址的 blob、转换 (transforms)、签名、不透明度诊断和多段 (multi-segment) 历史。

**二进制编码、序列和数据包。** GTS 重用 [CBOR](https://www.rfc-editor.org/info/rfc8949) 以实现确定的二进制结构，重用 [CBOR Sequences](https://datatracker.ietf.org/doc/html/rfc8742) 以实现自定界的项流。归档和研究数据打包系统，如 [BagIt](https://datatracker.ietf.org/doc/rfc8493/) 和 [RO-Crate](https://www.researchobject.org/specs/)，专注于可靠的文件传输和元数据丰富的研究对象描述。GTS 借鉴了数据包与清单的直觉，但数据包清单本身是折叠 (folded) 的图状态，且每个段 (segment)/帧 (frame) 都参与同一个内容标识链。

**内容寻址系统与仅追加日志。** Git 在其文档中通常被描述为[内容寻址文件系统](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects)，而 IPFS 通过源自加密哈希的 [CIDs](https://docs.ipfs.tech/concepts/content-addressing/) 命名内容。透明度系统（如 [Certificate Transparency](https://datatracker.ietf.org/doc/html/rfc6962)）使用仅追加日志和审计证明来记录全球可观察的发布事件。GTS 在便携式图工件内部应用内容寻址：帧 (frame) ID 和 `prev` 链接提供局部链完整性，可选的 MMR 索引支持脱离的包含证明，而部署配置文件 (profile) 决定是否将头部锚定在外部透明度或发布系统中。

**事件溯源与本地优先同步。** 事件溯源将状态更改记录为一系列事件，并从中重建状态，正如 Martin Fowler 的 [Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) 模式所总结的那样。本地优先 (Local-first) 研究主张用户控制的、具备离线能力的数据，将同步视为增强功能而非核心依赖，特别是在 Ink & Switch 的 [local-first software](https://www.inkandswitch.com/essay/local-first/) 文章中。GTS 不是 CRDT 或应用同步协议，但它的段 (segment) 连接、前缀折叠 (prefix-fold) 有效性和加性抑制模型使其适合作为需要仅追加历史并随后投影到特定于应用程序的合并逻辑的系统的传输工件。

**溯源、保管与研究证据。** W3C [PROV-O](https://www.w3.org/TR/prov-o/) 提供了一个用于表示和交换溯源信息的 RDF/OWL 词汇表。RO-Crate 和 BagIt 为研究对象和数字保存有效负载提供了成熟的打包模式。GTS 可以携带 PROV-O、类 RO-Crate 或特定领域的元数据作为普通图内容，同时将字节完整性和签名验证与部署信任分离：一个有效的 GTS 链证明了字节连续性，而非声明的真实性或签署者的权威性。
**有效载荷安全层。** GTS 使用 COSE 而非自创签名或加密信封：[RFC 9052](https://www.rfc-editor.org/info/rfc9052) 为 CBOR 序列化定义了签名、MAC 和加密结构。JSON 生态系统通常使用 [JWS](https://datatracker.ietf.org/doc/html/rfc7515) 来处理受完整性保护的基于 JSON 的有效载荷。GTS 的独特之处在于不透明性不变式 (opacity invariant)：加密或不受支持的有效载荷可以作为带有诊断信息、公开信封和链位置的不透明节点 (opaque nodes) 在图中保持可见，而不会导致全盘读取失败或从折叠 (fold) 中消失。

**图数据库与投影目标。** SPARQL 1.1 定义了标准的 [RDF 查询语言](https://www.w3.org/TR/sparql11-query/)，而 SQLite、DuckDB 和 Parquet 等系统则提供了持久化或分析型的表格基座。SQLite 记录了一种稳定的 [单文件数据库格式](https://www.sqlite.org/fileformat.html)；DuckDB 是一种 [可嵌入分析型数据库](https://duckdb.org/pdf/SIGMOD2019-demo-duckdb.pdf)；而 Apache Parquet 是一种用于分析的 [列式文件格式](https://parquet.apache.org/)。GTS 并不作为查询引擎与这些系统竞争。相反，它定义了一种便携式、可验证的传输协议，可以从中重新生成 N-Quads、SQLite、DuckDB、Parquet 或原生 RDF 存储。
## 11. 结论

GTS 探索了一种用于图状工件的小型传输层：确定性 CBOR 字节、仅追加帧、内容寻址历史、折叠语义、优雅的不透明性以及跨语言一致性语料库。其价值在于它划定的边界。核心工件是便携且可验证的；更丰富的数据库、配置文件、证明系统、对象存储和领域工作流可以附加在其之上或之下，而无需改变格式的细腰 (narrow waist) 结构。
## 附录草案

未来的论文修订版可以增加：

- 规范中的 CDDL 摘录。
- 与规范算法一致的折叠 (fold) 伪代码。
- 一致性向量 (conformance-vector) 目录摘要。
- 媒体类型 (media type) 注册摘录。
- 针对 read、verify、fold、cat、compact、pack、unpack 和 transform 目标的 CLI 示例。
- 总结了完整性、信任、不透明性 (opacity) 和资源限制假设的安全检查清单。

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: CHANGELOG.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# 更新日志

> [`CHANGELOG.md`](../../../CHANGELOG.md) 的信息性中文翻译。英文文档仍然是治理、安全、发布、许可、贡献、行为义务、披露流程和可执行命令的权威来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。

GTS 的所有显著变更都记录在此。本日志格式基于
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/)。主要引擎通过本仓库中针对各生态系统的发布标签进行发布；跨引擎发布的版本通常保持一致，而仅针对 Rust 的先行发布可能会独立提升 Rust crate 的版本。参见
[`scripts/check-versions.sh`](../../../scripts/check-versions.sh)。

线路格式已为版本 1 系列冻结（`GTS-SPEC.md` 文档版本 `1.0-rc1`）。任何有效的 GTS 线路格式主版本 1 文件，未来的主版本 1 读取器都仍可解析并安全折叠；
破坏性的线路变更需要新的线路格式主版本。

## [未发布]

### 已修复

- 帧签名现在遵循 COSE 受保护头中声明的签名算法（§9.2：读取器必须遵循所声明的算法）。
  Rust、Python、Go、TypeScript（包括浏览器 WebCrypto 路径）、Kotlin 和 Smalltalk 均按
  `alg` 分派：EdDSA 按 Ed25519 验证，任何其他已声明的算法都报告为 `unverified` 并保留其
  `kid`，绝不报告为 `invalid`，也绝不报告为 `valid`。此前每个引擎都无条件运行
  Ed25519，因此其他算法的签名会被报告为 `invalid`（或者，当受保护头声明了其他算法而签名
  实为真正的 Ed25519 签名时，被报告为 `valid`）。受保护头不符合下列语法的 COSE_Sign1 格式错误，其签名为 `invalid`；长度不是 64 字节的 EdDSA 签名在其密钥解析后同样为 `invalid`：

  > 受保护头是一个字节串，其内容是确定性编码的 CBOR（定长、整数和长度取最短形式、
  > 映射键按其编码的字节序排列、无重复键、无多余字节），且恰好构成一个映射。任何深度的
  > 每个映射键都是整数或文本串。任何深度的每个值都是整数、字节串、文本串、数组、映射、
  > null、true 或 false。每个整数（无论是键还是值）都位于 -2^63 至 2^63-1（含）的范围内。
  > 文本串是有效的 UTF-8。嵌套最多 8 层，头部映射本身计为第 1 层。该映射包含整数键 1，
  > 其值为整数或文本串。不满足以上全部要求的头部格式错误，其签名为 `invalid`。

  写出的字节不变：签名输出以及此变更之前已有的每个向量都逐字节相同。新的向量 `vectors/signed/sigalg-*.json`
  使用私有用途算法标识符 -65537 固定这一行为，包括该语法的边界（整数范围、键类型、UTF-8、嵌套深度）；每个引擎的测试都按所述状态检查其中每一个向量。
- 其他行为变更：Rust 不再把格式错误的 COSE_Sign1 报告为 `unverified`；Smalltalk 现在把缺失的
  `kid` 报告为 `invalid`，此前为 `unverified`；Python 读取器不再因无法解码的签名字节而抛出
  异常，也不再为格式错误的 COSE_Sign1 报告 `kid`，与其他引擎一致；Rust 的 `cose::parse`
  不再公开。
- 对脚本的影响：每个引擎的命令行 `verify` 在提供密钥时，只把 `invalid` 视为失败。算法不受支持
  且 `kid` 可解析的签名现在为 `unverified`，退出码为 0，而此前为 1，这与未解析的 `kid` 原有的
  行为一致。必须拒绝 `unverified` 签名的调用方应显式检查。
- Python 读取器不再让其 CBOR 库解释标签。除了包裹头部的 `55799`（§3）外，GTS 不赋予任何标签含义，
  但该库会根据其他标签构造集合、十进制数、日期等值，因此读取器接受什么取决于库的版本。现在每个标签
  都被解码为未解释的 `CBORTag`，无论是文件中的条目、经过转换的负载还是 COSE 结构，在 `cbor2`
  6.1.5 和 6.2.1 下都完全一致。一个完整的、不是帧的 CBOR 条目，无论有无标签，都会被报告为
  `DamagedFrame`（不是映射的帧，或映射的自哈希不匹配），折叠随后继续处理下一个条目。此前，在
  `cbor2` 6.2.1 下，标签 `258` 包裹非数组的条目，或标签 `4`、`5` 带有非整数部分的条目，会使读取器
  报告不完整追加（`TornAppendError`）并停止折叠，尽管该条目是完整的 CBOR。在 6.1.5 下，相同的字节
  被报告为受损帧，后续帧仍被折叠。此次更改涉及格式良好的带标签条目或孤立条目。其他解码失败，例如过度嵌套或
  无效的 UTF-8，不受此次更改影响，且各引擎尚未以相同方式报告。标签 `2` 和 `3` 仍是整数（字节串内容变为其所表示的整数），
  因此帧按整数计算哈希，与 Rust、Go 和 TypeScript 引擎相同，以非首选形式写入的大整数在这里和这些引擎中都是自哈希
  不匹配。
- `decrypt0` 和 `recipient_kid` 现在把无法解码的字节和长度错误的数组报告为畸形的 `COSE_Encrypt0`
  （`missing-key`，或没有 `kid`），而不是抛出 `CBORDecodeError` 或 `IndexError`；位于 `258`
  等标签之下的 COSE 结构也不再被解包为其顺序决定结果的 Python 集合。各引擎对标签不是 `18` 或 `16`
  的 COSE 结构是否接受并不一致，目前尚无向量固定这一点。
- 新向量 `28i-stray-set-between-frames`、`28j-stray-set-last-item`、
  `28k-stray-decimal-between-frames`、`28l-stray-decimal-last-item`、
  `28m-stray-self-reference-in-set` 和 `28n-stray-self-reference-as-key` 固定了帧之间和末尾出现孤立
  带标签条目时的结果：`DamagedFrame`，没有 `TornAppendError`，该条目之后的四元组仍被折叠。
  `28o-suppression-over-damaged-terms` 固定了后续段中的抑制指向受损 `terms` 帧的标识符的情形，
  `28p-tagged-value-in-frame-field` 固定了帧字段内带标签值的情形。
  写入的字节不变：此前存在的每个向量都逐字节相同。
- 当后续段中的抑制指向其所在段受损的 `terms` 帧从未定义的术语标识符时，`read` 不再抛出
  `IndexError`；该标识符按接收原样保留，受损是唯一的诊断，与 Rust、Go 和 TypeScript 一致。
- Kotlin 和 Smalltalk 解码器现在会界定浮点数条目和其他格式良好的简单值，并保持其不被解释。此前，
  包含它们的完整条目（例如带半精度浮点部分的标签 `4` 条目）会被报告为不完整追加并停止折叠；现在
  它是受损帧，折叠继续，符合规范要求。
- 帧结构或块结构无效（不是 zstd、被截断、块头错误）、指名字典、或声明了读取方不会解码的窗口的
  `zstd` 或 `zstd-rsyncable` 负载，现在只使该帧成为一个不透明节点（原因 `damaged`，诊断
  `DamagedFrame`），折叠继续向后进行，适用于 Python、Rust 和 TypeScript（其 Node 与浏览器构建）。
  Go 本来就如此；Kotlin 和 Smalltalk 仅据阅读判断应当如此，因为未运行它们。把读取方只是无法解码的
  完整帧报告为 `damaged`，是规范不透明原因中的一个缺口（§7.6 只列出 `unknown-codec`、`missing-key`
  和 `damaged`），不是规则。此前，Python 和 Rust 在不检查负载的情况下按声明的摘要登记 blob 帧：
  Python 随后会从 `gts ls` 和 `gts extract` 抛出 zstd 库的错误并附带回溯，对被截断的负载则不报任何
  错误而返回更少的字节；Rust 在读取时报告干净的错误，但没有折叠出不透明节点。二者现在在折叠 blob 帧
  时遍历 zstd 的帧头和块头（每 128 KiB 块几个字节，不解压，读取 blob 时也不重复）。块内部的损坏仍在
  首次读取时出现；Python 命令行以 `gts: cannot decode blob` 和状态 1 报告，而不是回溯，从
  `g.blobs[...]` 捕获 `zstandard.ZstdError` 的库调用方现在得到 `PayloadDecodeError`（一个
  `ValueError`）。Go 和 TypeScript 的命令行工具在报告任何 `DamagedFrame` 时仍以状态 1 退出，与之前相同。
- TypeScript 在两种构建中都不再解码声明窗口大于 2**27 字节（128 MiB）的 zstd 帧。其 zstd 库在匹配回溯
  约 256 MiB 时会不报错地返回错误字节，而 2**29 及以上的窗口允许这种情况；2**27 是所有参考读取方经
  测量能正确解码的最大窗口。该限制是两种构建共用模块中的一个具名常量（`MAX_ZSTD_WINDOW_LOG`），
  遍历不接受的任何结构都由遍历本身拒绝，而不是留给比格式更宽松的库。作为该库返回错误字节的其他情形
  （未存储标识的字典、声称有它并不具备的内容大小的单段帧）的后备，解码后字节的哈希与其声明的
  `pub.digest` 不符的 blob，现在在两种构建中都是受损帧；此前它会以错误字节的哈希被列出，且没有诊断。
- 这改变了 Python 读取方所接受的内容。它现在拒绝任何声明窗口大于 2**27 的帧，无论其库是否能解码：
  当全部内容放得进调用方的缓冲区时，库会跳过自己的限制，因此它接受什么取决于缓冲区大小而不是帧本身。
  此前能解码的、手工构造的、声明更大窗口而内容很小且大小已知的帧，现在成为不透明节点；未知有真实的
  编码器会写出这样的帧。Rust 的遍历现在还会在折叠时（而非首次读取时）拒绝大于 2**31 的窗口，这是其
  解码器接受的上限；其他引擎的限制没有变化。
- 此处未解决、格式也未决定：引擎在格式允许的帧上仍有分歧。声明窗口 2**28 的帧，Rust、Go 和 Smalltalk
  解码，Python、TypeScript 和（据阅读）Kotlin 拒绝；2**29 仅 Rust 和 Go 解码；2**30 仅 Rust 和
  Smalltalk 解码。字典帧在折叠时被 Python、Rust、TypeScript 和 Go 拒绝，在 Kotlin 和 Smalltalk 中
  （据阅读）于解码时失败。Smalltalk 对任何不带内容大小的帧都会失败。规范是否应规定统一的窗口限制和
  字典参数，仍未决定。
- Smalltalk 解码器现在会把由多个串联 zstd 帧组成的负载解码到末尾。此前它只按第一帧确定输出大小，因此
  参考写入方为每个单元写一帧、超过一个单元的 `zstd-rsyncable` 负载都被报告为受损帧。
- 新增向量 `22b-zstd-rsyncable-multi-frame`（由三个串联帧组成的负载，可解码，blob 摘要一致）、
  `22c-zstd-declared-window-2-27`、`28q-blob-payload-not-zstd`、`28r-blob-payload-truncated` 和
  `28s-blob-payload-names-dictionary`（各为一个不透明节点，其前后的帧照常折叠）。此前存在的每个向量
  都逐字节相同。

### 已变更

- Rust 引擎的 zstd 实现已更新（`structured-zstd` 从 0.0.48 升至 0.3.0），对部分输入现在会产生不同的压缩字节，这些字节的解码结果完全相同。
- 由于帧标识符覆盖已存储的压缩载荷，通过 Rust 库使用 zstd 编解码器写出的头部会随压缩器版本而变化，因此不得固定此类头部，也不得在不同版本之间比较；请改为比较内容。
- `getrandom`、`aes-gcm`、`quick-xml`、`ed25519-dalek` 和 `visual-hashing` 的更新不影响写出的字节。
- TypeScript 引擎现在使用 `@noble/ed25519` 3.2.0 和 `@noble/hashes` 2.4.0（均为主版本更新），其导入遵循新的模块布局（`sha2.js` 和 `legacy.js` 取代 `sha512.js` 和 `sha1.js`）。写出和签名的字节保持不变：签名、摘要和 OpenPGP 指纹完全相同，已通过前后对比证明。签名验证对退化签名发生变化，这类签名只有签名密钥的持有者才能产生；诚实签名不受影响。旧库会接受一个建立在单位元点上的签名，该签名对同一密钥下的任何消息都有效，新库则拒绝它。新库按收到的签名 `R` 字节验证（ZIP-215），因此此前能通过验证的非规范 `R` 签名现在不再通过，反之亦然。验证策略本身没有改变，仍是需要对整个格式统一决定的事项。
- Python 依赖已更新，其中包括 `cbor2` 6.1.5 升至 6.2.1。写入的字节不变：两个版本下语料库都能逐字节
  相同地重新生成。此次更新消除了一次崩溃：`cbor2` 6.1.5 在遇到用作映射键或集合元素的自引用标签时会以
  段错误终止进程，而 6.2.1 抛出可以捕获的错误。读取器不再把标签交给库去解释（见已修复），因此两个版本
  都不会因此类输入而崩溃，并且折叠结果相同。

## [1.0.0-rc.1] — 2026-09-04

### 已变更

- **破坏性变更（读取器契约）。** 引用三元组的 TERM 现在是自描述的：`terms` 行可以携带
  `"tt": [s, p, o]`，存在时具有权威性。`"rf"` 仍是合法的旧式编码，解析为其具体化器的第一个
  绑定，因此现有字节的读取结果不变——但任何既有的第三方读取器现在都不再符合规范，因为
  `rdf:reifies` 在 RDF 1.2 中并非函数性的，且 `reifies` 帧是一个多值语句层。每一行都以其
  自身的图槽保留；读取器不得在它们之间进行选择、重新排序或丢弃任何一行。只有逐字节相同的行
  才会合并（§7.8）。
- `ConflictingReifier` 现在收窄为唯一真正不一致的形态：一个没有 `"tt"` 的项，其具体化器绑定了
  多于一个不同的三元组。每个有问题的项只报告一次，不丢弃任何行，且该项仍按文件顺序解析为第一个
  绑定。一致性向量 12 从负例变为正例。
- **破坏性变更（§14 关系投影）。** `gts → {sqlite,duckdb,parquet}` 导出的 `terms` 表从 7 列
  增加到 10 列，追加了 `tt_s`、`tt_p`、`tt_o`。现有的位置索引 0-6 保持不变，但具体化器 id 不再
  标识一个三元组，因此任何以 `reifier` 作为键或连接并期望只有一个三元组的使用方都会静默丢行。

### 新增

- 规范性位置约束：在 `reifies` 行中，具体化器必须 (MUST) 是 IRI 或空白节点（`k:0|2`），绝不能是
  字面量或引用三元组。它是每一行所断言的 `R rdf:reifies <<( S P O )>>` 语句的主语，而 RDF 1.2
  在该位置只接受这些类别。违规会被诊断为 `PositionConstraint` 并丢弃该行。
- 一致性向量 `13b-reifier-position-constraint`、`13c-self-describing-triple-term` 和
  `13e-literal-base-direction`。最后一个覆盖 RDF 1.2 字面量基础方向，它已在六个引擎中完整实现，
  却完全没有冻结覆盖。

### 已修复

- 三元组项的解析现在在每一次遍历中都会终止。`reifies` 行可以命名恰好通过它解析的那个项，这可以用
  普通写入器构造，且不需要任何 `"tt"`。此前，折叠这样的文件会使 Go 以
  `fatal error: stack overflow` 中止——不可恢复，并违反 `FuzzRead` 所捍卫的读取器无 panic 契约
  ——并在 Python 中引发 `RecursionError`。两者都是解析器拒绝服务。
- 当权威的 `"tt"` 与旧式 `"rf"` 同时存在时，Go 不再丢弃前者。`Graph → events → Graph` 往返
  曾会降级这样的项。
- Rust 写入器仅为 `k:3` 项发出 `"tt"`。

## [0.9.11] — 2026-06-29

### 已变更

- `reifies` 和 `annot` 行现在携带与 `quads` 相同的可选图 term-id，从而恢复
  RDF 1.2 重述与注解的逐命名图分区。`reifies` 现在是行数组载荷
  (`[rid, s, p, o, g?]`)，而不是以 reifier id 为键的映射。
- 为这次协调的模式更新，发布元数据已对齐至 `0.9.11`。

## [0.9.10] — 2026-06-29

### 新增

- Rust 可以直接从折叠后的图状态生成确定性的图快照，包括 terms、quads、
  reifiers、annotations、blobs 以及快照级 zstd 选项。
- Python 和 Rust 写入器支持为 `zstd` 和 `zstd-rsyncable` 帧设置逐帧 zstd
  压缩级别。

### 已变更

- Rust 的 zstd 处理现在在编码和解码时均使用纯 Rust 的 `structured-zstd`
  后端。
- Python 和 Rust 发布通道的发布元数据已对齐至 `0.9.10`，且门控的伴随元数据保持同步。

### 已修复

- 移除了 Python、Rust、Go、TypeScript、浏览器 TypeScript、Kotlin 和 Smalltalk
  中固定的 16 MiB zstd 解码大小上限。各实现现在依赖流/背压、存储、平台分配失败以及数据损坏错误，
  而不是仅因解码输出超过编解码器级别的字节上限而拒绝。

## [0.9.9]

### 已修复

- Native Turtle/TriG parser now resolves `PN_LOCAL_ESC` escapes in prefixed-name
  local parts: a backslash-escaped delimiter (`\(`, `\)`, `\,`, `\.`, …) is part of
  the local name, not a statement/object delimiter, and expands to the literal
  character in the IRI (e.g. `dbr:Semantic_analysis_\(linguistics\)` →
  `http://dbpedia.org/resource/Semantic_analysis_(linguistics)`). An escaped trailing
  `\.` is kept rather than stripped as the statement terminator.

## [0.9.8]

### 已修复

- Turtle/TriG prefixed names admit internal dots (`repo:README.md`): a `.` is no longer
  a name delimiter; only a trailing dot terminates the statement.
- The N-Triples/N-Quads statement-layer fold HARD-FAILS on a conflicting `rdf:reifies`
  rebind (same reifier subject, different triple term) instead of silently
  last-write-winning (CONSTITUTION P7); an identical rebind stays idempotent.

## [0.9.7]

### 新增

- Full RDF 1.2 reification/annotation surface in the native Turtle/TriG parser:
  `<< s p o [~ id] >>` reifying triples (expanding to `reifier rdf:reifies <<( s p o )>>`),
  `{| … |}` annotation blocks and `~` reifiers in any sequence, `VERSION`/`@version`
  directives, optional predicate-object lists for self-asserting subjects, and
  `rdf:annotationNodeID` (blank-node reifier) in RDF/XML.

### 已修复

- Serialize a triple term in quad-object position: the self-referential reifier entry
  (a triple term keying its own components in the reifiers map) no longer trips the
  event source's `cycle while declaring term N` guard.

## [0.9.6]

### 新增

- Native Turtle/TriG parsing of bare numeric (`xsd:integer`/`decimal`/`double`),
  boolean (`xsd:boolean`), and single- and triple-quoted string literals, with
  lexical forms preserved verbatim (`0.70`, `1.0E0` survive unchanged). This
  closes the gaps that previously forced an oxttl-backed codec fallback, so the
  hand-rolled native codecs fully replace `oxttl`/`oxrdfxml`/`oxrdfio` with no
  text-codec dependency on the Oxigraph family.
- Installable C ABI archive packaging for `libgts`, including pkg-config,
  CMake, checksum, SBOM, provenance, and immutable GitHub Release publication
  through the `capi-v*` tag lane.
- Publishable `gmeow-gts-capi` Rust source crate metadata and a bootstrap
  crates.io release workflow for the C ABI source package.
- Credential-free package dry-run CI for the C ABI wrapper family, covering
  local package/artifact validation without registry publication secrets.

### 已变更

- N-Quads/N-Triples language-tag validation now accepts long BCP-47 private-use
  subtags (e.g. `x-gmeow-norwegiannynorsk`): once the `x` singleton appears the
  8-char per-subtag cap is dropped for the remainder. This is the native
  equivalent of the prior oxttl `.lenient()` mode and lets GMEOW's long
  private-use language tags round-trip through every native text codec.
- Corrected the Go module release tag shape to `go/v<version>` so
  `go.blackcatinformatics.ca/gts` versions in the `go/` subdirectory are
  discoverable by the Go proxy and pkg.go.dev.
- Moved canonical `visual-hashing` publication to
  `Blackcat-Informatics/visual-hashing`; version `0.1.3` is published from that
  repository with crates.io Trusted Publishing provenance, and the old monorepo
  release workflow is retired.
- Replaced the Rust Oxigraph/OxRDF/Sophia adapter surfaces with native RDF
  dataset, text-codec, RDF/XML, ULID, and in-memory store implementations; CI
  now locks the `wasm32-unknown-unknown --all-features` library build and audits
  that dependency tree to keep the removed toolkit blockers out.

## [0.9.5] — 2026-06-22

Rust-only release. Python, Go, TypeScript, and wrapper package versions remain
at `0.9.4`.

### 新增

- Rust RDF event source/sink API for folded GTS graphs, including declaration
  ordering for sinks that require term declarations before references.
- Rust RDF text codecs for N-Triples, Turtle, TriG, and RDF/XML behind the
  `rdf-codecs` feature.
- Rust XSD lexical validation annotations for parsed RDF literals.
- C ABI RDF format discovery and conversion helpers for the new Rust codec
  surface.

### 已变更

- Folded the RDF event protocol back into the main `gmeow-gts` crate so the
  Rust release does not require publishing a second support crate.

## [0.9.4] — 2026-06-21

### 新增

- Rust GTS terms now preserve RDF 1.2 language-tagged literal base direction
  (`"dir": "ltr" | "rtl"`) across the wire model, deterministic writer,
  multi-segment union, N-Quads/TriG projection, YAML-LD import/export, SQL
  exports, and dump-directory JSONL tables.
- Regression coverage for multiple distinct reifiers bound to the same asserted
  triple.
- Regression coverage documenting the blob contract: inline blob frames preserve
  bytes, while external blob records carry metadata and digest references only.

### 已变更

- Bumped lockstep package metadata to `0.9.4` across release manifests,
  citation metadata, C API metadata, Kotlin metadata, and documentation snippets.
- Native `oxrdf` export now refuses RDF 1.2 directional literals rather than
  silently dropping base direction on an adapter surface that cannot carry it.

## [0.9.2] — 2026-06-19

### 新增

- Rust-only YAML-LD-star/JSON-LD-star import/export behind the `yaml-ld`
  Cargo feature, including `gts to-yaml-ld` and `gts from-yaml-ld`. This is a
  transform-only feature with no wire-format, canonical-catalog, or shared
  corpus oracle change, so no GIP is required.
- Rust `gts dump --directory` inspection export for versioned directory trees
  containing folded N-Quads, JSONL tables, unfolded frame views, blob indexes,
  and files-profile payloads.
- TriG import/export support for readable graph-block interchange over the same
  folded RDF content.
- Rust OKF profile import/export, an OKF conformance corpus, and ecosystem
  positioning for Markdown/YAML-frontmatter knowledge bundles.
- Files-profile-v2 metadata support plus the Rust tar bridge: `from-tar`,
  `to-tar`, and tar-compatible `gts tar -c/-x/-t/-d`.
- Tar round-trip conformance fixtures covering ustar, PAX metadata, gzip/zstd,
  links, special nodes, and unsafe archive refusals.

### 已变更

- Documented OKF and tar as intentional Rust-first extension surfaces with
  explicit future parity gates for Python, Go, and TypeScript.
- Strengthened ecosystem and parity drift guards for the new OKF and tar
  documentation surfaces.

### 安全

- Hardened files-profile-v2 and tar extraction behavior around traversal,
  symlinks, hardlinks, special files, ownership, setuid/setgid bits, and unsafe
  output paths.

## [0.9.1] — 2026-06-19

### 新增

- v1.0-rc1 release support docs: release checklist, third-party implementer guide,
  benchmark release report template, paper draft, and public release verification workflow.
- Go and TypeScript `from-nq` parity so all four engines can import N-Quads into GTS.

### 已变更

- Refined README, specification, conformance, vector-manifest, streaming-reader, and
  benchmark documentation for clearer release-candidate evidence and implementer guidance.
- Improved language-specific quality gates and docs: Rust public API docs and `rustdoc` gate,
  Python inline docs and typing, Go docs plus `golangci-lint`, and TypeScript API docs plus
  stricter lint rules.
- Added project DOI and homepage metadata across package and citation surfaces.

### 安全

- Hardened release publication with crates.io Trusted Publishing, GitHub SLSA provenance,
  SPDX SBOM attestations for release artifacts, immutable Go GitHub Releases, and release
  smoke verification.
- Documented the current release SLSA posture as Build Level 2 evidence and the signer
  workflow requirements for any future Build Level 3 claim.

## [0.9.0] — 2026-06-18

### 新增

- Rust parity expansion: N-Quads import, SQLite/DuckDB/Parquet exports, lazy blob
  decoding, verification APIs, policy/nested reader APIs, agent-memory example,
  streaming sink reader, MMR proof support, replication inventory verbs, deterministic
  writer helpers, transform encryption APIs, and native RDF adapters for `oxrdf`,
  Oxigraph, and Sophia.
- Cross-engine service/browser parity: Go streaming reader sinks, replication verbs
  across Rust/Go/TypeScript/Python CLIs, TypeScript browser streaming/WebCrypto export,
  and cross-engine MMR proof verification.
- Repository contracts for API/CLI parity, advanced primitives, ecosystem integrations,
  conformance, security policy, vector manifest metadata, and downstream Rust ontology
  CI coverage.

### 已变更

- Vector corpus and conformance documentation now stamp corpus revision metadata and
  sharpen total-reader, fold-semantics, media-type, CDDL/preimage, and publication
  guidance.
- Release documentation now records visual-hashing publish ordering and lockstep
  release expectations for the four GTS engines.

### 安全

- Added security policy parity for Go and TypeScript, including nested-GTS/profile
  policy coverage, and documented deferred multi-recipient crypto behavior.

## [0.2.0] — 2026-06-17

### 新增

- **`visual-hashing` standalone Rust crate (#16).** The emojihash utility
  (BLAKE3-XOF → 6-bit → 64-emoji visual hash) is GTS-independent, so it now lives
  in its own crate, `visual-hashing/`, ready to publish to crates.io
  independently of GTS. It also carries **randomart** (the OpenSSH "Drunken
  Bishop" ASCII-art fingerprint) — ported to Rust from the Python reference and
  gated by a new frozen corpus (`vectors/randomart/*.json` +
  `scripts/gen_randomart_vectors.py`), so the Rust art reproduces Python's
  byte-for-byte. The `gmeow-gts` crate now depends on `visual-hashing` and
  re-exports it as `gmeow_gts::emojihash`, so existing paths keep working. The
  crate's only dependency is `blake3`; it stays `wasm32`-friendly. (The
  standalone crate is published to crates.io as `visual-hashing 0.1.1`; publish
  it first whenever the Rust `gmeow-gts` crate bumps its compatible
  `visual-hashing` requirement, since `gmeow-gts` packaging resolves that
  dependency from crates.io rather than the in-repo path.)

- **Cross-engine COSE_Encrypt0 (#18): AES-256-GCM payload encryption in all four engines.**
  - `encrypt0` / `decrypt0` / `recipient_kid` (§9.3) in Rust (pure-Rust
    `aes-gcm`), Go (`crypto/aes` + `crypto/cipher`), and TypeScript
    (`@noble/ciphers`) — byte-compatible with the Python reference. The COSE
    structure is `16([protected{1:3}, unprotected{4:kid, 5:iv}, ciphertext])`
    with the Enc_structure (`["Encrypt0", protected, b""]`) bound as AAD.
  - Because AES-GCM uses a random 12-byte IV, the seal is not deterministic, so
    conformance is gated two ways: a **fixed-IV vector**
    (`vectors/encrypt0/basic.json` + `scripts/gen_encrypt0_vectors.py`) freezes
    the exact transform (every engine reproduces the sealed bytes and opens
    them), and each engine round-trips a **random-IV** seal. Verified live
    across engine boundaries (e.g. Go seals → Python/Rust open).
  - The Rust seal/open paths stay pure / wasm-friendly: `aes-gcm` is built with
    `getrandom` off, and the random-IV `encrypt0` convenience (the only RNG user)
    is gated off `wasm32`, so the `wasm32-unknown-unknown` build is still green.
  - This was the last tracked crypto-parity follow-up from #15; signing (#15),
    `extract-key` (#17), and now encryption (#18) all reach four-engine parity.

- **Cross-engine `extract-key` (#17): OpenPGP key inspection in all four engines.**
  - A minimal OpenPGP reader (de-armor → packet parse → raw Ed25519 key + v4
    SHA-1 fingerprint) in Rust (`openpgp.rs`, pure-Rust `sha1`), Go
    (`openpgp` package, `crypto/sha1`), and TypeScript (`openpgp.ts`,
    `@noble/hashes/sha1`) — narrow by design: only the unencrypted armored
    Ed25519 (algorithm 22) certificates GPG emits, mirroring the Python
    `gts.openpgp` reference. SHA-1 appears solely because RFC 4880 mandates it
    for v4 fingerprints; it is not a general-purpose hash here.
  - **`gts extract-key <file>`** in the Rust, Go, and TypeScript CLIs (Python
    already had it, #12): prints the embedded transport key's `kid`, OpenPGP
    fingerprint, emojihash, and armored public key. Gated by a frozen shared
    vector set (`vectors/openpgp/*.json` + `scripts/gen_openpgp_vectors.py`):
    every engine parses the same key to the same raw bytes / fingerprint /
    emojihash, and the CLI reproduces the Python-generated stdout byte-for-byte.
  - The Rust OpenPGP reader stays pure / wasm-friendly (the `wasm32` build is
    still green).
  - COSE_Encrypt0 landed as #18 (above) — four-engine crypto parity is complete.

- **Cross-engine crypto parity (#15): COSE_Sign1 + emojihash in all four engines.**
  - **COSE_Sign1 Ed25519** (`sign_id`/`verify_sig`, detached over the frame id,
    §9.2) in Rust (`ed25519-dalek`), Go (`crypto/ed25519`), and TypeScript
    (`@noble/ed25519`) — byte-compatible with the Python reference and gated by a
    frozen shared vector set (`vectors/cose/*.json` + `scripts/gen_cose_vectors.py`).
    Ed25519 is deterministic, so every engine reproduces the exact COSE bytes and
    verifies them.
  - **emojihash** (BLAKE3-XOF → 6-bit → 64-emoji visual hash) in Rust, Go, and
    TypeScript, byte-identical to Python, gated by `vectors/emojihash/*.json`.
  - **File-level signing**: each engine's `Writer` can COSE-sign every frame
    (`sign_with`/`SignWith`/`signWith`), and each engine verifies the signatures of
    a signed GTS against resolved keys (`cose.verify_signatures` /
    `VerifySignatures` / `verifySignatures`). Gated by a frozen signed-GTS vector
    (`vectors/signed/basic.json`): every engine reproduces the Python-signed file
    byte-for-byte and verifies it (valid / unverified / invalid).
  - **`gts verify --key KID:HEXPUB`** in all four CLIs: verify a signed file's
    COSE signatures against raw Ed25519 public keys (repeatable; exit 1 on any
    invalid signature).
  - All crypto stays pure / wasm-friendly (the Rust `wasm32` build is still green).
  - Cross-engine `extract-key` landed as #17 (above); COSE_Encrypt0
    (random-IV AES-GCM) remains tracked as #18.

- `gts extract-key <file>` (Python CLI, #12): prints the embedded transport
  (verification) key for a signed GTS — `kid`, OpenPGP fingerprint, emojihash,
  and the armored public key.
- `gts from-nq <in.nq> -o <out.gts>` (Python CLI, #14): build a GTS from
  N-Quads — the inverse of `fold`. Native N-Quads(-star) parser (no rdflib);
  handles IRIs, blank nodes, language/datatyped literals, named graphs, and the
  RDF 1.2 reifying style. `-` reads stdin, so `rdflib → n-quads → gts` pipes. Also
  exported as `gts.from_nquads`.
- Relational export (Python CLI, #13): `gts to-sqlite`, `gts to-duckdb`,
  `gts to-parquet` load a folded graph into the integer-id, dictionary-encoded
  five-table schema (`terms`/`quads`/`reifiers`/`annotations`/`blobs`). SQLite
  uses the standard library; DuckDB and Parquet need the new `[db]` extra
  (`pip install 'gmeow-gts[db]'`). Library API: `gts.db.to_sqlite/to_duckdb/to_parquet`.

### 已修复

- **Cross-engine `pack` byte parity (#5).** The TypeScript and Go engines encoded
  the files-profile `mode` as an octal string (`"644"`) instead of the decimal
  `xsd:integer` value (`"420"`) the Rust and Python engines use, and TypeScript
  emitted millisecond-precision `modified` timestamps (`…20.000Z`). All four
  engines now pack identical fixtures to byte-identical output. The `interop`
  check promotes byte-identity to a hard gate and folds every package against a
  single global reference, so writer drift (not just reader disagreement) fails CI.

## [0.1.3] — 2026-06-16

### 新增

- Live cross-engine interoperability check (`scripts/interop.sh`): every engine
  packs a fixture and every other engine folds/unpacks it, asserting identical
  results — interop coverage beyond the Python-generated frozen corpus.
- Fuzzing for the readers: Rust `cargo-fuzz` target and Go native `FuzzRead`,
  seeded from the conformance corpus, run on a schedule and as a PR smoke test.
- Per-ecosystem supply-chain vulnerability scanning (`cargo audit`,
  `govulncheck`, `npm audit`, `pip-audit`) on lockfile changes, on main, and weekly.
- Cross-platform CI (Linux, macOS, Windows) for every engine's build and tests.
- `wasm32-unknown-unknown` build gate for the Rust library.
- Developer `justfile`, a lockstep version-sync check, and this changelog.
- Full release attestations across all four engines: SLSA build-provenance
  attestations and SPDX SBOMs for the crate, the npm tarball (in addition to
  npm's sigstore provenance), the Python distributions, and the Go binaries.
- npm releases use trusted publishing (OIDC); the Go release cross-builds and
  publishes attested binaries (replacing the Pro-only goreleaser monorepo setup).

## [0.1.2] — 2026-06-16

### 新增

- Initial public import of GTS into the standalone `gmeow-gts` repository: four
  interoperable engines (Rust, Python, Go, TypeScript), the `GTS-SPEC.md`
  specification, and the frozen conformance corpus.
- Triple licensing: `MIT OR Apache-2.0 OR proprietary`.

[1.0.0-rc.1]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.11...rust-v1.0.0-rc.1
[0.9.11]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.10...rust-v0.9.11
[0.9.10]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.9...rust-v0.9.10
[0.9.9]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.8...rust-v0.9.9
[0.9.8]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.7...rust-v0.9.8
[0.9.7]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.6...rust-v0.9.7
[0.9.6]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.5...rust-v0.9.6
[0.9.5]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/rust-v0.9.4...rust-v0.9.5
[0.9.4]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.9.2...py-v0.9.4
[0.9.2]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.9.1...py-v0.9.2
[0.9.1]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.9.0...py-v0.9.1
[0.9.0]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.2.0...py-v0.9.0
[0.2.0]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.1.3...py-v0.2.0
[0.1.3]: https://github.com/Blackcat-Informatics/gmeow-gts/compare/py-v0.1.2...py-v0.1.3
[0.1.2]: https://github.com/Blackcat-Informatics/gmeow-gts/releases/tag/py-v0.1.2

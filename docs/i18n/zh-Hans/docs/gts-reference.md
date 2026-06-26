<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/gts-reference.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS Python 参考实现 (`gts`)

> [`docs/gts-reference.md`](../../../docs/gts-reference.md) 的信息性中文翻译。英文文档仍然是兼容性规则、一致性声明、对等矩阵、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../../GLOSSARY.md)，仅供参考。

一个轻量级、低依赖的 **Graph Transport Substrate** 线格式读取器/写入器，该格式在 [`GTS-SPEC.md`](./GTS-SPEC.md) 中规定。`gts` 软件包 (PyPI: `gmeow-gts`) 是 **baseline** 层级：它对规范进行了实证验证，是 Rust、Go 和 TypeScript 引擎所对标的语言无关一致性语料库的唯一事实来源。层级声明和向量子集在 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) 中定义。

## 涵盖范围

- **CBOR 仅追加日志** + RDF 1.2 折叠状态 (`terms` / `quads` /
  `reifies` / `annot`，blobs、元数据、抑制、不透明节点和诊断)，
  以及 `snapshot` 折叠 (§7)。
- **完整性** — 确定性 CBOR + 每帧 BLAKE3 自 `id` 以及 `prev`
  内容 ID 链，包含 header-genesis 原像规则 (§5, §9.1)。
- **转换目录** — `identity` / `gzip` / `zstd`；能力模型会将未知的编解码器或 `encrypt` 编解码器（基准中没有密钥）降级为**不透明节点**，而不是导致读取失败 (§8, §7.6)。
- **鲁棒性** — 撕裂追加检测 (§3)、损坏帧隔离以及规范诊断 (§2.4)，包括 `EmptyFile`、`TornAppendError`、`DamagedFrame`、`BrokenChain`、`UnknownCodec`、`MissingKey`、`ConflictingReifier`、`PositionConstraint`、`ForwardReference`、`SegmentBoundary` 和 `UnknownFrameType`。
- **`RDF -> GTS` 互操作性** — 通过可选的 `[rdf]` 额外组件 (rdflib)，rdflib
  `Graph`/`Dataset` (RDF 1.1 基础图) 可以通过 `gts.from_rdflib` 驻留 (intern)
  到 GTS 字典中；`gts.to_rdflib` 严格遵守 RDF 1.2 引用三元组 (quoted-triple)
  限制。集成合约位于
  [GTS-ECOSYSTEM-INTEGRATIONS.md](./GTS-ECOSYSTEM-INTEGRATIONS.md)。
- **转换输出** — `gts → nquads` (§14) 和 `gts → {sqlite,duckdb}` (整数 ID、字典编码的关系加载；引擎通过 join 解析 ID)。
- **COSE 签名 (§9.2)** — `Writer(signer=…)` 对每帧的
  `id` (EdDSA/Ed25519) 进行 COSE_Sign1 签名；`read(data, keys=…)` 验证并在 `Graph.signatures`
  中记录每帧状态 (`KeyProvider` 下的 `valid`/`invalid`/`unverified`)。此外还包括通过 `read(data, expected_head=…)` → `TruncatedLog` 进行的**截断检测** (#272)。
- **COSE 加密 (§9.3)** — `Writer.add_frame(…, encrypt=(kid, key))` 将
  负载封装为 `COSE_Encrypt0` (最外层转换) 并记录接收者；
  `read(data, keys=…)` 在持有内容密钥时进行解密，否则帧将折叠为
  `missing-key` **不透明节点**，且其接收者可见（不透明度不变性）——
  选择性披露 (#272)。

## 尚未实现（#267 下的后续工作）

多接收者 / ECDH 密钥封装（本次交付单接收者 `COSE_Encrypt0`）；
v1 信任/配置文件策略合约（要求签名、伪名-`kid`
以及有界嵌套 GTS 递归）正在
[`GTS-SECURITY-POLICY.md`](./GTS-SECURITY-POLICY.md) 中进行跟踪。剩余推迟项包括
`index`/MMR 加速（§6.2）、针对极大型输入的
帧流式处理数据库加载，以及打包词汇表扩展。

## 使用

```python
from gts import Writer, Term, TermKind, read, to_nquads

w = Writer(profile="dist")
w.add_terms([
    Term(TermKind.IRI, "https://example.org/Cat"),
    Term(TermKind.IRI, "http://www.w3.org/2000/01/rdf-schema#label"),
    Term(TermKind.LITERAL, "Cat", lang="en"),
])
w.add_quads([(0, 1, 2, None)])
data = w.to_bytes()                      # the GTS file (bytes)

graph = read(data)                       # parse + verify chain + fold
print(to_nquads(graph))                  # <…/Cat> <…/label> "Cat"@en .
```

CLI (`pip install gmeow-gts` 安装 `gts` 二进制文件):

```bash
gts info   file.gts             # frame/term/quad/blob counts + diagnostics
gts fold   file.gts             # fold to N-Quads on stdout
gts verify file.gts             # verify chains; exit 1 on any diagnostic
gts cat -o combined.gts a.gts b.gts   # validating composer
gts pack ./my-dir -o archive.gts      # package a directory (files profile)
gts unpack archive.gts -C ./restore   # extract a files profile
```

跨引擎 API 形状、CLI 一致性矩阵以及仅限 Python 的命令缺口维护在 [`GTS-API-CLI-PARITY.md`](./GTS-API-CLI-PARITY.md) 中。
高级流式处理、索引/MMR/证明、复制、范围获取以及基准测试延期跟踪在 [`GTS-ADVANCED-PRIMITIVES.md`](./GTS-ADVANCED-PRIMITIVES.md) 中。

## 一致性

`python/tests/test_gts.py` 实现了 [`GTS-CONFORMANCE.md`](./GTS-CONFORMANCE.md) 中定义的 non-COSE 一致性语料库子集（最小文件、`zstd`/`gzip` 帧、未知编解码器 → 不透明、损坏的帧、撕裂追加、头部哈希、抑制、数据类型默认设置、冲突的具体化器、位置约束、空白节点局部性、内联 blob、快照折叠）。基准配置文件的符合性读取器故意设计得非常精简 —— 这正是该格式的意义所在。

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-DUMP-DIR.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS 转储目录

> [`docs/GTS-DUMP-DIR.md`](../../../../docs/GTS-DUMP-DIR.md) 的信息性中文翻译。英文文档仍然是集成、高级功能、可选 profile、基准数据、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。


`gts dump <archive.gts> --directory <out-dir>` 将 GTS 存档展开为一个版本化的检查目录。第一个实现仅限 Rust；其布局有意设计为语言无关，以便其他引擎以后可以遵循相同的合约。

转储是一个探索与诊断界面，而非一种新的传输格式。它复制了存档的有用视图，同时默认避免重复大型有效负载字节。
```text
out/
├── README.md
├── .gts-dump/
│   ├── manifest.json
│   ├── heads.json
│   └── segments.json
├── graph/
│   ├── README.md
│   ├── folded.nq
│   └── tables/
│       ├── terms.jsonl
│       ├── quads.jsonl
│       ├── reifiers.jsonl
│       ├── annotations.jsonl
│       ├── meta.jsonl
│       ├── blob-meta.jsonl
│       ├── suppressions.jsonl
│       ├── opaque.jsonl
│       ├── signatures.jsonl
│       └── diagnostics.jsonl
├── frames/
│   ├── README.md
│   ├── inventory.jsonl
│   └── segments/
│       └── 0000/
│           ├── header.json
│           ├── folded.nq
│           ├── frame-0001.nq
│           └── *.jsonl
├── blobs/
│   ├── index.jsonl
│   └── by-digest/
│       └── blake3/
└── files/
    ├── entries.jsonl
    └── tree/
```

当没有相应的归档内容时，目录将被省略。例如，仅当归档包含有效的 files 配置文件 (profile) 编目时，`files/` 才会存在；而仅当转储 (dump) 必须存储尚未通过 `files/tree/` 具象化的 blob 负载字节时，`blobs/by-digest/` 才会存在。
## 图视图

`graph/folded.nq` 是折叠归档的权威 RDF 文本投影。N-Quads 是默认格式，因为它是确定性的、面向行的，并且可以表示命名图。默认不输出 Turtle，因为如果不进行策略选择，它无法表示整个折叠的 RDF 数据集；对于想要更具可读性的 RDF 数据集语法的用户来说，TriG 是更好的未来显式格式。

`graph/tables/*.jsonl` 以简单的面向行的表格形式公开相同的折叠状态。这些适用于 shell 工具、电子表格、DuckDB、Python 笔记本以及不想在检查归档之前了解 RDF 序列化的用户。
## 展开帧

`frames/inventory.jsonl` 记录了段（segment）和帧（frame）的字节范围、帧 ID、帧类型以及有效性。每个 `frames/segments/NNNN/` 目录包含逐段折叠的 N-Quads 和解码后的帧级 JSONL 行。当帧具有可以投影为 N-Quads 的 RDF 贡献时，会生成 `frame-*.nq` 文件。

展开帧视图回答了与 `graph/` 不同的问题：它按顺序显示了追加日志所贡献的内容，而 `graph/` 则显示了最终的折叠状态。
## 有效负载策略

默认采用单副本有效负载具体化 (materialization)：

- files-profile 有效负载被写入 `files/tree/` 下；
- `blobs/index.jsonl` 始终记录 digest、size、media type、suppression state 以及 materialized paths；
- `blobs/by-digest/` 仅针对尚未通过 `files/tree/` 具体化的 blob 有效负载进行写入；
- 被抑制的有效负载会被索引，但除非传递了 `--include-suppressed`，否则不会被具体化；
- `--metadata-only` 在不提取有效负载字节的情况下写入 graph、frame、manifest 和 index 文件。

默认情况下，转储 (dump) 不会复制原始的 `.gts` 文件。source path、size 和 digest 记录在 `.gts-dump/manifest.json` 中。
## 未来导入

`.gts-dump/manifest.json` 中的架构名称为 `gts-dump-v1`。未来的 `undump` 支持应该将清单和物化的有效负载映射视为导入契约。当前的 Rust 命令是单向的：它为往返编辑准备目录形状，但尚未声明支持导入。

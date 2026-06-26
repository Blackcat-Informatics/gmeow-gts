<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- i18n-source: docs/GTS-BENCHMARK-RELEASE-REPORT.md -->
<!-- i18n-locale: zh-Hans -->
<!-- i18n-status: translated -->

# GTS Benchmark 发行报告模板

> [`docs/GTS-BENCHMARK-RELEASE-REPORT.md`](../../../../docs/GTS-BENCHMARK-RELEASE-REPORT.md) 的信息性中文翻译。英文文档仍然是集成、高级功能、可选 profile、基准数据、示例、标识符和机器可读值的规范来源。本翻译遵循 [`docs/i18n/GLOSSARY.md`](../GLOSSARY.md)，仅供参考。


将此模板用于 v1 发行说明、发行候选 (release-candidate) 评审和论文附录证据。
使用以下方式生成填充后的报告：

```bash
just bench-release
```

对于发行候选 (release-candidate) 运行，请使用所有引擎、至少进行三次迭代，并将生成的产物 (artifact) 提交到发行证据包 (release evidence bundle) 而不是此源码树：

```bash
python scripts/bench_release_suite.py \
  --engines rust,python,go,ts,smalltalk \
  --iterations 5 \
  --vectors vectors/01-minimal.gts,vectors/23-files-profile-tree.gts,vectors/25b-streamable-compacted.gts \
  --out-dir dist/benchmarks/v1.0-rc1 \
  --strict
```

运行器 (runner) 写入：

- `release-benchmark-report.json` 用于机器可读证据；
- `release-benchmark-report.md` 用于发行说明或附录文本；
- 运行所使用的确定性写入和归档 fixtures；
- 用于测量写入 (write)、打包 (pack) 和拆包 (unpack) 路径的每引擎产物 (products)。

默认情况下，即使选定的引擎失败或不可用，运行器 (runner) 也会编写一份完整的报告。一旦失败的行预计会阻塞候选版本，请使用 `--strict` 进行发行候选门控 (release-candidate gating)。
## 必需的发布元数据

| 字段 | 值 |
|---|---|
| Release candidate | |
| Generated report path | |
| Runner command line | |
| Repository commit | |
| GTS spec commit | |
| GTS spec blob | |
| Conformance corpus commit | |
| Corpus manifest SHA-256 | |
| Platform | |
| CPU / memory | |
| Runner versions | |
## 基准输入

| Kind | Path | Bytes | SHA-256 | Notes |
|---|---|---:|---|---|
| Conformance vector | | | | read/fold |
| Conformance vector | | | | read/fold |
| Write fixture | | | | `from-nq` input |
| Archive fixture | | | | `pack`/`unpack` input |
## CLI 耗时摘要

在发布说明声明中使用中位数。在报告中保留失败或跳过的行，以便使不可用的引擎可见，而不是被静默忽略。

| 引擎 | 操作 | 输入 | 迭代次数 | 中位数 ms | 最小值 ms | 最大值 ms | 输出证据 |
|---|---|---|---:|---:|---:|---:|---|
| Rust | read-info | | | | | | |
| Rust | fold | | | | | | |
| Rust | write-from-nq | | | | | | |
| Rust | pack | | | | | | |
| Rust | unpack | | | | | | |
| Python | read-info | | | | | | |
| Python | fold | | | | | | |
| Python | write-from-nq | | | | | | |
| Python | pack | | | | | | |
| Python | unpack | | | | | | |
| Go | read-info | | | | | | |
| Go | fold | | | | | | |
| Go | write-from-nq | | | | | | |
| Go | pack | | | | | | |
| Go | unpack | | | | | | |
| TypeScript | read-info | | | | | | |
| TypeScript | fold | | | | | | |
| TypeScript | write-from-nq | | | | | | |
| TypeScript | pack | | | | | | |
| TypeScript | unpack | | | | | | |
| Smalltalk | read-info | | | | | | |
| Smalltalk | fold | | | | | | |
| Smalltalk | write-from-nq | | | | | | |
| Smalltalk | pack | | | | | | |
| Smalltalk | unpack | | | | | | |
## 流式处理内存摘要

流式处理内存证据与 CLI 挂钟时间 (wall time) 不直接具有可比性。请单独引用并说明每个引擎所使用的方法。

| 引擎 | 方法 | 输入 | 耗时 | 峰值内存 / 分配证据 | 备注 |
|---|---|---|---:|---:|---|
| Python | full-reader materialization | | | | |
| Rust | `read_to_sink_from_reader` streaming fold | | | | |
| Go | `go test ./reader -bench ... -benchmem` | | | | |
| TypeScript | browser `foldStreamToSink` harness | | | | |
## 版本说明摘要

`<release>` 的基准测试 (Benchmarks) 已在 `<platform>` 上运行，代码库提交为 `<repo_commit>`，规范 (spec) 提交为 `<spec_commit>`，一致性语料库 (conformance corpus) 提交为 `<corpus_commit>`。read/fold/write/pack/unpack 的中位数耗时列在 `<report path>` 中。流式内存证据 (Streaming-memory evidence) 是单独报告的，因为 Rust 辅助程序报告的是进程高水位 RSS，Go 基准测试 (benchmark) 报告的是运行时分配指标，而浏览器 TypeScript 内存则必须 (MUST) 从用于发布候选版本 (release candidate) 的浏览器测试带 (browser harness) 中获取。

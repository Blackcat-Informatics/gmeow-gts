<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Localization Glossary

Use this glossary as the shared terminology source for GTS translations. English
protocol terms remain the authoritative identifiers; localized terms are reader
aids and should stay consistent across documentation.

## Core Terms

| English | `fr-CA` | `zh-Hans` | Notes |
|---|---|---|---|
| GTS | GTS | GTS | Product and format name; do not translate. |
| Graph Transport Substrate | Graph Transport Substrate | Graph Transport Substrate | Expand once, then keep the English name. |
| frame | trame | 帧 | A CBOR frame in the append-only stream. |
| segment | segment | 段 | A contiguous piece of a GTS stream. |
| fold | repli | 折叠 | The deterministic replay result of the log. |
| reader | lecteur | 读取器 | Component that parses and verifies GTS data. |
| writer | rédacteur | 写入器 | Component that emits GTS data. |
| conformance corpus | corpus de conformité | 一致性语料库 | Frozen cross-engine test corpus. |
| opaque node | noeud opaque | 不透明节点 | Preserve the protocol phrase; avoid inventing RDF identifiers. |
| streamable layout | disposition diffusable en continu | 可流式处理布局 | Layout that can be consumed incrementally. |
| profile | profil | 配置文件 | Domain or application layer above the core transport. |
| BLAKE3 | BLAKE3 | BLAKE3 | Algorithm name; do not translate. |
| N-Quads | N-Quads | N-Quads | RDF serialization name; do not translate. |
| TriG | TriG | TriG | RDF serialization name; do not translate. |
| RDF | RDF | RDF | Standards acronym; do not translate. |
| CBOR | CBOR | CBOR | Standards acronym; do not translate. |

## Normative Keywords

Normative keywords remain recognizable because English is authoritative. Use the
localized term in prose, but keep the English keyword when translating protocol
requirements or tables where ambiguity would matter.

| RFC 2119 keyword | `fr-CA` | `zh-Hans` | Notes |
|---|---|---|---|
| MUST | DOIT | 必须 | Preserve `MUST` when quoting normative English text. |
| MUST NOT | NE DOIT PAS | 不得 | Preserve `MUST NOT` when quoting normative English text. |
| REQUIRED | REQUIS | 必需 | Same force as `MUST`. |
| SHALL | DOIT | 应 | Avoid unless the English source uses `SHALL`. |
| SHALL NOT | NE DOIT PAS | 不应 | Avoid unless the English source uses `SHALL NOT`. |
| SHOULD | DEVRAIT | 应该 | Recommendation with known exceptions. |
| SHOULD NOT | NE DEVRAIT PAS | 不应该 | Recommendation against a behavior. |
| RECOMMENDED | RECOMMANDÉ | 推荐 | Same force as `SHOULD`. |
| MAY | PEUT | 可以 | Optional behavior. |
| OPTIONAL | FACULTATIF | 可选 | Optional feature or field. |

## Protected Literals

Keep these literal classes unchanged in translated files:

- Commands and flags, such as `gts pack` and `--json`.
- Package and import names, such as `gmeow-gts`,
  `@blackcatinformatics/gmeow-gts`, and `go.blackcatinformatics.ca/gts`.
- File paths and URLs.
- Media types, API identifiers, diagnostic codes, JSON keys, enum values,
  status values, profile names, and RDF terms used as identifiers.
- Code blocks, registry snippets, and conformance examples.

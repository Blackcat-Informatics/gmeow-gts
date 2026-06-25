// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fmt;

#[cfg(feature = "rdf-codecs")]
macro_rules! rdf_codecs_usage {
    () => {
        "  to-nt <file>             fold the default graph to N-Triples on stdout
  from-nt <in.nt> [-o out]  build a GTS from N-Triples; '-' reads stdin
  to-rdfxml <file>         fold the default graph to RDF/XML on stdout
  from-rdfxml <in.rdf> [-o out]
                            build a GTS from RDF/XML; '-' reads stdin
  to-turtle <file>         fold the default graph to Turtle on stdout
  from-turtle <in.ttl> [-o out]
                            build a GTS from Turtle; '-' reads stdin
"
    };
}

#[cfg(not(feature = "rdf-codecs"))]
macro_rules! rdf_codecs_usage {
    () => {
        "optional:
  to-nt <file>             build with --features rdf-codecs
  from-nt <in.nt> [-o out] build with --features rdf-codecs
  to-rdfxml <file>         build with --features rdf-codecs
  from-rdfxml <in.rdf> [-o out]
                            build with --features rdf-codecs
  to-turtle <file>         build with --features rdf-codecs
  from-turtle <in.ttl> [-o out]
                            build with --features rdf-codecs
"
    };
}

macro_rules! usage_text {
    ($yamlld:literal, $okf:literal, $relational:literal) => {
        concat!(
            "usage: gts <command> [args]

commands:
  info <file>...            per-segment composition ledger (§14.1)
  fold <file>               fold to N-Quads on stdout
  from-nq <in.nq> [-o out]  build a GTS from N-Quads; '-' reads stdin
  to-trig <file>            fold to TriG on stdout
  from-trig <in.trig> [-o out]
                            build a GTS from TriG; '-' reads stdin
",
            rdf_codecs_usage!(),
            $yamlld,
            $okf,
            "
  from-tar <archive.tar[.gz|.zst]|-> [-o out.gts] [--allow-symlinks] [--allow-special] [--owner]
                            build a files-profile-v2 GTS from tar (feature tar)
  to-tar <file.gts> [-o archive.tar|-] [-z|--gzip|--zstd] [--numeric-owner]
                            export a files-profile-v2 archive as tar (feature tar)
  tar -c[z|--zstd]f <archive> <dir|file>... | -xf <archive> | -tf <archive> | -df <archive> <dir>
                            tar-compatible files-profile surface (feature tar)
  verify [--key kid:hexpubkey] [--policy file] <file>...
                            verify chains, signatures, and optional profile policy
  prove <file> <frame-id>   emit JSON inclusion proof from an index.mmr root
  verify-proof <proof.json> verify detached proof JSON without the GTS file
  heads <file>              JSON segment heads and aggregate comparison digest
  segments <file>           JSON segment byte ranges and layout inventory
  missing --from-head <head> <file>
                            JSON byte ranges needed after a peer head
  resume --after <frame-id> <file>
                            emit bytes after a verified frame boundary
  extract-key <file>        print the embedded transport key: kid, OpenPGP
                            fingerprint, emojihash, and armored public key (§9.2)
  ls <file>                 list inline blobs: digest, size, declared media type
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            extract one blob by content digest; --mt asserts
                            the declared media type (never converts)
  cat -o <out> <file>...    validating composer: refuse degenerate inputs,
                            then byte-concatenate (§3.1, §14.1)
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            rewrite into the streamable layout state: leading
                            streaming index, blobs most-significant-first,
                            trailing index footer (§10.1)
  pack <dir|file>... -o out.gts
                            pack files/directories into a files-profile archive
  unpack <archive> [-C dir] [--include-suppressed] [--allow-symlinks] [--allow-special] [--same-owner|--numeric-owner] [--preserve-setid]
                            unpack a files-profile archive
  diff <archive> <dir>      compare archive to directory by digest
  dump <archive> --directory dir [--include-suppressed] [--force] [--metadata-only]
                            expand an archive into an inspection directory
  to-sqlite <file> <out>    export the folded graph to SQLite (needs sqlite3)",
            $relational
        )
    };
}

#[cfg(all(feature = "duckdb", feature = "yaml-ld", feature = "okf"))]
const USAGE_EN: &str = usage_text!(
    "  to-yaml-ld <file>         fold to YAML-LD-star on stdout
  from-yaml-ld <in.yaml> [-o out]
                            build a GTS from YAML-LD-star; '-' reads stdin",
    "
  to-okf <file> --directory <out> [--inline-body] [--base-iri IRI]
                            export an OKF-profile graph to a Markdown bundle
  from-okf <dir> [-o out] [--inline-body] [--strict-links] [--base-iri IRI]
                            build a GTS from an OKF Markdown bundle",
    "
  to-duckdb <file> <out>    export the folded graph to DuckDB (needs duckdb)
  to-parquet <file> <dir>   export Parquet files, one per non-empty table
                            (needs duckdb)"
);

#[cfg(all(feature = "duckdb", feature = "yaml-ld", not(feature = "okf")))]
const USAGE_EN: &str = usage_text!(
    "  to-yaml-ld <file>         fold to YAML-LD-star on stdout
  from-yaml-ld <in.yaml> [-o out]
                            build a GTS from YAML-LD-star; '-' reads stdin",
    "
optional:
  to-okf <file> --directory <out>
                            build with --features okf
  from-okf <dir> [-o out]   build with --features okf",
    "
  to-duckdb <file> <out>    export the folded graph to DuckDB (needs duckdb)
  to-parquet <file> <dir>   export Parquet files, one per non-empty table
                            (needs duckdb)"
);

#[cfg(all(feature = "duckdb", not(feature = "yaml-ld"), feature = "okf"))]
const USAGE_EN: &str = usage_text!(
    "optional:
  to-yaml-ld <file>         build with --features yaml-ld
  from-yaml-ld <in.yaml> [-o out]
                            build with --features yaml-ld",
    "
  to-okf <file> --directory <out> [--inline-body] [--base-iri IRI]
                            export an OKF-profile graph to a Markdown bundle
  from-okf <dir> [-o out] [--inline-body] [--strict-links] [--base-iri IRI]
                            build a GTS from an OKF Markdown bundle",
    "
  to-duckdb <file> <out>    export the folded graph to DuckDB (needs duckdb)
  to-parquet <file> <dir>   export Parquet files, one per non-empty table
                            (needs duckdb)"
);

#[cfg(all(feature = "duckdb", not(feature = "yaml-ld"), not(feature = "okf")))]
const USAGE_EN: &str = usage_text!(
    "optional:
  to-yaml-ld <file>         build with --features yaml-ld
  from-yaml-ld <in.yaml> [-o out]
                            build with --features yaml-ld",
    "
optional:
  to-okf <file> --directory <out>
                            build with --features okf
  from-okf <dir> [-o out]   build with --features okf",
    "
  to-duckdb <file> <out>    export the folded graph to DuckDB (needs duckdb)
  to-parquet <file> <dir>   export Parquet files, one per non-empty table
                            (needs duckdb)"
);

#[cfg(all(not(feature = "duckdb"), feature = "yaml-ld", feature = "okf"))]
const USAGE_EN: &str = usage_text!(
    "  to-yaml-ld <file>         fold to YAML-LD-star on stdout
  from-yaml-ld <in.yaml> [-o out]
                            build a GTS from YAML-LD-star; '-' reads stdin",
    "
  to-okf <file> --directory <out> [--inline-body] [--base-iri IRI]
                            export an OKF-profile graph to a Markdown bundle
  from-okf <dir> [-o out] [--inline-body] [--strict-links] [--base-iri IRI]
                            build a GTS from an OKF Markdown bundle",
    "
optional:
  to-duckdb <file> <out>    build with --features duckdb; needs duckdb on PATH
  to-parquet <file> <dir>   build with --features duckdb; needs duckdb on PATH"
);

#[cfg(all(not(feature = "duckdb"), feature = "yaml-ld", not(feature = "okf")))]
const USAGE_EN: &str = usage_text!(
    "  to-yaml-ld <file>         fold to YAML-LD-star on stdout
  from-yaml-ld <in.yaml> [-o out]
                            build a GTS from YAML-LD-star; '-' reads stdin",
    "
optional:
  to-okf <file> --directory <out>
                            build with --features okf
  from-okf <dir> [-o out]   build with --features okf",
    "
optional:
  to-duckdb <file> <out>    build with --features duckdb; needs duckdb on PATH
  to-parquet <file> <dir>   build with --features duckdb; needs duckdb on PATH"
);

#[cfg(all(not(feature = "duckdb"), not(feature = "yaml-ld"), feature = "okf"))]
const USAGE_EN: &str = usage_text!(
    "optional:
  to-yaml-ld <file>         build with --features yaml-ld
  from-yaml-ld <in.yaml> [-o out]
                            build with --features yaml-ld",
    "
  to-okf <file> --directory <out> [--inline-body] [--base-iri IRI]
                            export an OKF-profile graph to a Markdown bundle
  from-okf <dir> [-o out] [--inline-body] [--strict-links] [--base-iri IRI]
                            build a GTS from an OKF Markdown bundle",
    "
optional:
  to-duckdb <file> <out>    build with --features duckdb; needs duckdb on PATH
  to-parquet <file> <dir>   build with --features duckdb; needs duckdb on PATH"
);

#[cfg(all(
    not(feature = "duckdb"),
    not(feature = "yaml-ld"),
    not(feature = "okf")
))]
const USAGE_EN: &str = usage_text!(
    "optional:
  to-yaml-ld <file>         build with --features yaml-ld
  from-yaml-ld <in.yaml> [-o out]
                            build with --features yaml-ld",
    "
optional:
  to-okf <file> --directory <out>
                            build with --features okf
  from-okf <dir> [-o out]   build with --features okf",
    "
  to-duckdb <file> <out>    build with --features duckdb; needs duckdb on PATH
  to-parquet <file> <dir>   build with --features duckdb; needs duckdb on PATH"
);

const USAGE_FR_CA: &str = r#"utilisation: gts <command> [args]

commandes:
  info <file>...            affiche le registre de composition par segment
  fold <file>               plie vers N-Quads sur stdout
  from-nq <in.nq> [-o out]  construit un GTS depuis N-Quads; '-' lit stdin
  to-trig <file>            plie vers TriG sur stdout
  from-trig <in.trig> [-o out]
                            construit un GTS depuis TriG; '-' lit stdin
  verify [--key kid:hexpubkey] [--policy file] <file>...
                            verifie les chaines, signatures et politiques
  prove <file> <frame-id>   emet une preuve JSON d'inclusion MMR
  verify-proof <proof.json> verifie une preuve detachee sans fichier GTS
  heads <file>              emet les tetes de segments en JSON
  segments <file>           emet les plages d'octets des segments en JSON
  missing --from-head <head> <file>
                            emet les plages JSON requises apres une tete de pair
  resume --after <frame-id> <file>
                            emet les octets apres une frontiere de trame valide
  extract-key <file>        imprime la cle de transport integree
  ls <file>                 liste les blobs: digest, taille, type media declare
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            extrait un blob par digest de contenu
  cat -o <out> <file>...    compose en validant et refuse les entrees degenerees
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            reecrit vers l'etat de disposition diffusable
  pack <dir|file>... -o out.gts
                            emballe des fichiers en archive de profil files
  unpack <archive> [-C dir] [--include-suppressed]
                            deballe une archive de profil files
  diff <archive> <dir>      compare une archive a un repertoire par digest
  dump <archive> --directory dir
                            developpe une archive dans un repertoire d'inspection"#;

const USAGE_ZH_HANS: &str = r#"用法: gts <command> [args]

命令:
  info <file>...            显示每个段的组合账本
  fold <file>               将内容折叠为 N-Quads 并写到 stdout
  from-nq <in.nq> [-o out]  从 N-Quads 构建 GTS；'-' 读取 stdin
  to-trig <file>            将内容折叠为 TriG 并写到 stdout
  from-trig <in.trig> [-o out]
                            从 TriG 构建 GTS；'-' 读取 stdin
  verify [--key kid:hexpubkey] [--policy file] <file>...
                            验证链、签名和可选配置文件策略
  prove <file> <frame-id>   从 index.mmr 根输出 JSON 包含证明
  verify-proof <proof.json> 在没有 GTS 文件时验证分离的证明 JSON
  heads <file>              输出段头和聚合比较摘要的 JSON
  segments <file>           输出段字节范围和布局清单的 JSON
  missing --from-head <head> <file>
                            输出对等段头之后所需的字节范围 JSON
  resume --after <frame-id> <file>
                            输出已验证帧边界之后的字节
  extract-key <file>        打印内嵌的传输密钥
  ls <file>                 列出内联 blob 的摘要、大小和声明媒体类型
  extract <file> <digest> [-o out] [--mt TYPE] [--include-suppressed]
                            按内容摘要提取一个 blob
  cat -o <out> <file>...    验证后组合，拒绝退化输入
  compact <file> -o <out> --streamable [--seal-original] [--timestamp ISO]
                            重写为可流式布局状态
  pack <dir|file>... -o out.gts
                            将文件或目录打包为 files 配置文件归档
  unpack <archive> [-C dir] [--include-suppressed]
                            解包 files 配置文件归档
  diff <archive> <dir>      按摘要比较归档和目录
  dump <archive> --directory dir
                            将归档展开到检查目录"#;

#[derive(Clone, Copy, Eq, PartialEq)]
enum CliLocale {
    English,
    FrenchCanada,
    ChineseHans,
}

fn locale_from(raw: &str) -> CliLocale {
    let mut value = raw.trim().to_ascii_lowercase().replace('_', "-");
    if let Some(idx) = value.find(['.', '@']) {
        value.truncate(idx);
    }
    match value.as_str() {
        "fr" | "fr-ca" => CliLocale::FrenchCanada,
        "zh" | "zh-cn" | "zh-hans" | "zh-hans-cn" => CliLocale::ChineseHans,
        _ => CliLocale::English,
    }
}

fn cli_locale() -> CliLocale {
    for key in ["GTS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(raw) = std::env::var(key) {
            if !raw.trim().is_empty() {
                return locale_from(&raw);
            }
        }
    }
    CliLocale::English
}

fn usage_text(locale: CliLocale) -> &'static str {
    match locale {
        CliLocale::FrenchCanada => USAGE_FR_CA,
        CliLocale::ChineseHans => USAGE_ZH_HANS,
        CliLocale::English => USAGE_EN,
    }
}

pub struct LocalizedUsage;

impl fmt::Display for LocalizedUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(usage_text(cli_locale()))
    }
}

pub static USAGE: LocalizedUsage = LocalizedUsage;

pub fn unknown_command_message(command: &str) -> String {
    match cli_locale() {
        CliLocale::FrenchCanada => format!("gts: commande inconnue '{command}'\n{USAGE}"),
        CliLocale::ChineseHans => format!("gts: 未知命令 '{command}'\n{USAGE}"),
        CliLocale::English => format!("gts: unknown command '{command}'\n{USAGE}"),
    }
}

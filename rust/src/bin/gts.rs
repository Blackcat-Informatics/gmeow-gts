// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `gts` command-line tool: inspect, fold, verify, and compose GTS files.
//!
//! `cat` and `verify` implement the §14.1 composition-tooling contract: raw
//! byte concatenation is always valid GTS (§3.1), but a publish-class tool
//! refuses pathological states instead of trusting them to be intentional.
//!
//! Exit codes: 0 clean; 1 diagnostics found or input refused; 2 usage/IO error.

use std::collections::HashSet;
use std::process::ExitCode;

use ciborium::value::Value;
use ed25519_dalek::VerifyingKey;
use gmeow_gts::cose::verify_signatures;
use gmeow_gts::emojihash::emojihash;
use gmeow_gts::files::UnpackOptions;
#[cfg(feature = "tar")]
use gmeow_gts::files::{read_entries, FileEntryKind};
use gmeow_gts::from_nquads::from_nquads;
#[cfg(feature = "okf")]
use gmeow_gts::from_okf::{from_okf_with_options, FromOkfOptions};
#[cfg(feature = "tar")]
use gmeow_gts::from_tar::{from_tar_to_writer, FromTarOptions};
#[cfg(not(feature = "rdf-codecs"))]
use gmeow_gts::from_trig::from_trig;
#[cfg(feature = "yaml-ld")]
use gmeow_gts::from_yamlld::from_yaml_ld;
use gmeow_gts::mmr::{parse_hex_32, prove_file, verify_proof, Proof};
use gmeow_gts::model::{Graph, Suppression, TermKind};
use gmeow_gts::nquads::to_nquads;
#[cfg(feature = "okf")]
use gmeow_gts::okf::{to_okf, OkfExportOptions};
use gmeow_gts::policy::{evaluate_profile_policy, Severity, TrustPolicy};
use gmeow_gts::reader::{read, read_file_segments, FileSegments};
use gmeow_gts::replication::{
    heads_json, inventory, missing, missing_json, resume_after, segments_json, MissingStatus,
};
#[cfg(feature = "tar")]
use gmeow_gts::tar::{to_tar, TarCompression, ToTarOptions};
#[cfg(not(feature = "rdf-codecs"))]
use gmeow_gts::trig::to_trig;
use gmeow_gts::verify::{extract_transport_key, format_fingerprint};
use gmeow_gts::wire::{digest_str, hex};
#[cfg(feature = "yaml-ld")]
use gmeow_gts::yamlld::to_yaml_ld;

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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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
const USAGE: &str = usage_text!(
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match cmd.as_str() {
        "info" => cmd_info(&args[1..]),
        "fold" => cmd_fold(&args[1..]),
        "from-nq" => cmd_from_nq(&args[1..]),
        "to-trig" => cmd_to_trig(&args[1..]),
        "from-trig" => cmd_from_trig(&args[1..]),
        #[cfg(feature = "rdf-codecs")]
        "to-nt" => cmd_to_nt(&args[1..]),
        #[cfg(feature = "rdf-codecs")]
        "from-nt" => cmd_from_nt(&args[1..]),
        #[cfg(feature = "rdf-codecs")]
        "to-rdfxml" => cmd_to_rdfxml(&args[1..]),
        #[cfg(feature = "rdf-codecs")]
        "from-rdfxml" => cmd_from_rdfxml(&args[1..]),
        #[cfg(feature = "rdf-codecs")]
        "to-turtle" => cmd_to_turtle(&args[1..]),
        #[cfg(feature = "rdf-codecs")]
        "from-turtle" => cmd_from_turtle(&args[1..]),
        #[cfg(not(feature = "rdf-codecs"))]
        "to-nt" | "from-nt" | "to-rdfxml" | "from-rdfxml" | "to-turtle" | "from-turtle" => {
            cmd_rdf_codecs_disabled(cmd)
        }
        #[cfg(feature = "yaml-ld")]
        "to-yaml-ld" => cmd_to_yaml_ld(&args[1..]),
        #[cfg(feature = "yaml-ld")]
        "from-yaml-ld" => cmd_from_yaml_ld(&args[1..]),
        #[cfg(not(feature = "yaml-ld"))]
        "to-yaml-ld" | "from-yaml-ld" => cmd_yaml_ld_disabled(cmd),
        #[cfg(feature = "okf")]
        "to-okf" => cmd_to_okf(&args[1..]),
        #[cfg(feature = "okf")]
        "from-okf" => cmd_from_okf(&args[1..]),
        #[cfg(not(feature = "okf"))]
        "to-okf" | "from-okf" => cmd_okf_disabled(cmd),
        #[cfg(feature = "tar")]
        "from-tar" => cmd_from_tar(&args[1..]),
        #[cfg(feature = "tar")]
        "to-tar" => cmd_to_tar(&args[1..]),
        #[cfg(feature = "tar")]
        "tar" => cmd_tar(&args[1..]),
        #[cfg(not(feature = "tar"))]
        "from-tar" | "to-tar" | "tar" => cmd_tar_disabled(cmd),
        "verify" => cmd_verify(&args[1..]),
        "prove" => cmd_prove(&args[1..]),
        "verify-proof" => cmd_verify_proof(&args[1..]),
        "heads" => cmd_heads(&args[1..]),
        "segments" => cmd_segments(&args[1..]),
        "missing" => cmd_missing(&args[1..]),
        "resume" => cmd_resume(&args[1..]),
        "extract-key" => cmd_extract_key(&args[1..]),
        "ls" => cmd_ls(&args[1..]),
        "extract" => cmd_extract(&args[1..]),
        "cat" => cmd_cat(&args[1..]),
        "compact" => cmd_compact(&args[1..]),
        "pack" => cmd_pack(&args[1..]),
        "unpack" => cmd_unpack(&args[1..]),
        "diff" => cmd_diff(&args[1..]),
        "dump" => cmd_dump(&args[1..]),
        "to-sqlite" => cmd_to_sqlite(&args[1..]),
        #[cfg(feature = "duckdb")]
        "to-duckdb" => cmd_to_duckdb(&args[1..]),
        #[cfg(feature = "duckdb")]
        "to-parquet" => cmd_to_parquet(&args[1..]),
        #[cfg(not(feature = "duckdb"))]
        "to-duckdb" | "to-parquet" => cmd_duckdb_disabled(cmd),
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("gts: unknown command '{other}'\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(not(feature = "duckdb"))]
fn cmd_duckdb_disabled(cmd: &str) -> ExitCode {
    eprintln!(
        "gts {cmd}: optional DuckDB/Parquet exports are disabled; rebuild with \
         `--features duckdb` and keep the `duckdb` binary on PATH"
    );
    ExitCode::from(2)
}

#[cfg(not(feature = "yaml-ld"))]
fn cmd_yaml_ld_disabled(cmd: &str) -> ExitCode {
    eprintln!("gts {cmd}: YAML-LD-star transforms are disabled; rebuild with `--features yaml-ld`");
    ExitCode::from(2)
}

#[cfg(not(feature = "rdf-codecs"))]
fn cmd_rdf_codecs_disabled(cmd: &str) -> ExitCode {
    eprintln!(
        "gts {cmd}: RDF Turtle-family codecs are disabled; rebuild with \
         `--features rdf-codecs`"
    );
    ExitCode::from(2)
}

#[cfg(not(feature = "okf"))]
fn cmd_okf_disabled(cmd: &str) -> ExitCode {
    eprintln!("gts {cmd}: OKF transforms are disabled; rebuild with `--features okf`");
    ExitCode::from(2)
}

#[cfg(not(feature = "tar"))]
fn cmd_tar_disabled(cmd: &str) -> ExitCode {
    eprintln!("gts {cmd}: tar transforms are disabled; rebuild with `--features tar`");
    ExitCode::from(2)
}

fn load(path: &str) -> Result<Vec<u8>, ExitCode> {
    std::fs::read(path).map_err(|e| {
        eprintln!("gts: cannot read {path}: {e}");
        ExitCode::from(2)
    })
}

fn cmd_prove(args: &[String]) -> ExitCode {
    let [path, frame_id] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let target = match parse_hex_32(frame_id) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("gts prove: invalid frame id: {e}");
            return ExitCode::from(2);
        }
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    match prove_file(&data, &target) {
        Ok(proof) => {
            print!("{}", proof.to_json());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts prove: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_verify_proof(args: &[String]) -> ExitCode {
    let [path] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("gts verify-proof: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let proof = match Proof::from_json(&text) {
        Ok(proof) => proof,
        Err(e) => {
            eprintln!("gts verify-proof: invalid proof JSON: {e}");
            return ExitCode::from(2);
        }
    };
    match verify_proof(&proof) {
        Ok(()) => {
            println!(
                "proof ok: root {} frame {}",
                hex(&proof.root),
                hex(&proof.frame_id)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts verify-proof: invalid proof: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_heads(args: &[String]) -> ExitCode {
    let [path] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let inventory = inventory(&data);
    print!("{}", heads_json(&inventory));
    if inventory.has_problems() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_segments(args: &[String]) -> ExitCode {
    let [path] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let inventory = inventory(&data);
    print!("{}", segments_json(&inventory));
    if inventory.has_problems() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_missing(args: &[String]) -> ExitCode {
    if args.len() != 3 || args[0] != "--from-head" {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let from_head = &args[1];
    let path = &args[2];
    let from_head = match parse_hex_32(from_head) {
        Ok(head) => head,
        Err(e) => {
            eprintln!("gts missing: invalid peer head: {e}");
            return ExitCode::from(2);
        }
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let inventory = inventory(&data);
    let result = missing(&inventory, &from_head);
    print!("{}", missing_json(&result));
    if result.status == MissingStatus::Error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_resume(args: &[String]) -> ExitCode {
    if args.len() != 3 || args[0] != "--after" {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let frame_id = &args[1];
    let path = &args[2];
    let frame_id = match parse_hex_32(frame_id) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("gts resume: invalid frame id: {e}");
            return ExitCode::from(2);
        }
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    match resume_after(&data, &frame_id) {
        Ok(tail) => {
            let mut stdout = std::io::stdout();
            if let Err(e) = std::io::Write::write_all(&mut stdout, tail) {
                eprintln!("gts resume: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts resume: {e}");
            ExitCode::from(1)
        }
    }
}

/// Print the per-segment composition ledger (§14.1 "SHOULD report").
fn print_ledger(path: &str, fs: &FileSegments) {
    println!(
        "{path}: {} segment(s){}",
        fs.segments.len(),
        match fs.torn {
            Some(off) => format!(", TORN at byte {off}"),
            None => String::new(),
        }
    );
    if let Some(fatal) = &fs.fatal {
        println!("  FATAL {}: {}", fatal.code, fatal.detail);
        return;
    }
    for (idx, seg) in fs.segments.iter().enumerate() {
        let head = seg
            .segment_heads
            .first()
            .map(|h| hex(h))
            .unwrap_or_else(|| "<none>".to_string());
        let profile = seg
            .segment_profiles
            .first()
            .map(String::as_str)
            .unwrap_or("<none>");
        let signers = seg
            .signatures
            .iter()
            .filter(|s| s.status != "invalid")
            .count();
        println!(
            "  segment {idx}: head {head} profile {profile} terms {} quads {} \
             reifies {} annot {} blobs {} suppress {} opaque {} sigs {signers}",
            seg.terms.len(),
            seg.quads.len(),
            seg.reifiers.len(),
            seg.annotations.len(),
            seg.blobs.len(),
            seg.suppressions.len(),
            seg.opaque.len(),
        );
        if let Some(layout) = seg.segment_streamable.first() {
            if layout.claimed {
                let head_hex = layout
                    .head
                    .as_deref()
                    .map(hex)
                    .unwrap_or_else(|| "<none>".to_string());
                let tail = if layout.tail > 0 {
                    format!(", accretive tail {} frame(s)", layout.tail)
                } else {
                    String::new()
                };
                println!(
                    "    layout: streamable through frame {} (head {head_hex}){tail}",
                    layout.covered
                );
            }
        }
        for o in &seg.opaque {
            println!("    opaque: {} ({})", o.frame_type, o.reason);
        }
        for d in &seg.diagnostics {
            println!(
                "    diagnostic {}: {}{}",
                d.code,
                d.detail,
                match d.frame_index {
                    Some(i) => format!(" [item {i}]"),
                    None => String::new(),
                }
            );
        }
    }
}

fn has_problems(fs: &FileSegments) -> bool {
    fs.fatal.is_some() || fs.torn.is_some() || fs.segments.iter().any(|s| !s.diagnostics.is_empty())
}

/// Profile → vocabulary namespace registry for the declared-vs-computed check
/// mandated by §14.1.
const PROFILE_VOCABS: &[(&str, &str)] = &[("files", "https://w3id.org/gts/files#")];

fn namespace(iri: &str) -> &str {
    if let Some(i) = iri.rfind('#') {
        &iri[..=i]
    } else if let Some(i) = iri.rfind('/') {
        &iri[..=i]
    } else {
        iri
    }
}

fn term_iri_value(seg: &Graph, tid: usize) -> Option<&str> {
    seg.terms
        .get(tid)
        .and_then(|t| match (t.kind, t.value.as_deref()) {
            (TermKind::Iri, Some(v)) => Some(v),
            _ => None,
        })
}

/// Vocabulary namespaces actually used by IRIs in a segment's quads.
fn used_vocabs(seg: &Graph) -> HashSet<&'static str> {
    let mut out = HashSet::new();
    for &(s, p, o, g) in &seg.quads {
        // The graph slot is a term position like any other (§14.1): a
        // vocabulary IRI used only as a graph name still rots a declaration.
        let ids = [Some(s), Some(p), Some(o), g];
        for iri in ids
            .into_iter()
            .flatten()
            .filter_map(|tid| term_iri_value(seg, tid))
        {
            for &(_prof, vocab) in PROFILE_VOCABS {
                if namespace(iri) == vocab {
                    out.insert(vocab);
                }
            }
        }
    }
    out
}

/// Check declared-vs-computed profile requirements for one segment.
/// Returns (message, is_error) pairs.
fn profile_check(seg: &Graph) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let declared: HashSet<&str> = seg.segment_profiles.iter().map(String::as_str).collect();
    let used = used_vocabs(seg);
    for &(prof, vocab) in PROFILE_VOCABS {
        let declares = declared.contains(prof);
        let uses = used.contains(vocab);
        if uses && !declares {
            out.push((
                format!(
                    "profile error: segment uses {vocab} vocabulary but does not declare '{prof}'"
                ),
                true,
            ));
        }
        if declares && !uses {
            out.push((
                format!(
                    "profile warning: segment declares '{prof}' but uses no {vocab} vocabulary"
                ),
                false,
            ));
        }
    }
    out
}

/// Warn on `stream#` vocabulary in an unclaimed segment (§13.3).
///
/// A warning, never an error: compaction-provenance quads legitimately
/// survive `nq → gts` round trips and re-accretion — the error class is
/// reserved for a claimed layout the bytes contradict (the reader's
/// `StreamableLayoutError`).
fn stream_vocab_check(seg: &Graph) -> Vec<String> {
    let claimed = seg
        .segment_streamable
        .first()
        .is_some_and(|info| info.claimed);
    if claimed {
        return Vec::new();
    }
    let uses = seg.quads.iter().any(|&(s, p, o, g)| {
        [Some(s), Some(p), Some(o), g]
            .into_iter()
            .flatten()
            .any(|tid| {
                term_iri_value(seg, tid)
                    .is_some_and(|iri| iri.starts_with(gmeow_gts::stream::STREAM_NS))
            })
    });
    if uses {
        vec![format!(
            "layout warning: segment uses {} vocabulary but does \
             not claim layout 'streamable' (§13.3)",
            gmeow_gts::stream::STREAM_NS
        )]
    } else {
        Vec::new()
    }
}

fn cmd_info(paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    for path in paths {
        let data = match load(path) {
            Ok(d) => d,
            Err(code) => return code,
        };
        print_ledger(path, &read_file_segments(&data));
    }
    ExitCode::SUCCESS
}

fn cmd_fold(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    print!("{}", to_nquads(&g));
    // The (possibly partial) fold is still emitted, but any diagnostic — or
    // never reaching segmentation at all — is a nonzero exit, so
    // `gts fold … && publish` pipelines fail on damage.
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn cmd_to_trig(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    let trig = {
        #[cfg(feature = "rdf-codecs")]
        {
            match gmeow_gts::rdf_codecs::to_trig(&g) {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("gts to-trig: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        #[cfg(not(feature = "rdf-codecs"))]
        {
            to_trig(&g)
        }
    };
    print!("{trig}");
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_to_nt(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    match gmeow_gts::rdf_codecs::to_ntriples(&g) {
        Ok(text) => print!("{text}"),
        Err(e) => {
            eprintln!("gts to-nt: {e}");
            return ExitCode::from(1);
        }
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_to_turtle(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    match gmeow_gts::rdf_codecs::to_turtle(&g) {
        Ok(text) => print!("{text}"),
        Err(e) => {
            eprintln!("gts to-turtle: {e}");
            return ExitCode::from(1);
        }
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_to_rdfxml(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    match gmeow_gts::rdf_codecs::to_rdf_xml(&g) {
        Ok(text) => print!("{text}"),
        Err(e) => {
            eprintln!("gts to-rdfxml: {e}");
            return ExitCode::from(1);
        }
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn cmd_from_nq(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts from-nq: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            eprintln!("gts from-nq: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("gts from-nq: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match from_nquads(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("gts from-nq: {e}");
            return ExitCode::from(1);
        }
    };

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts from-nq: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-nq: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

fn cmd_from_trig(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts from-trig: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            eprintln!("gts from-trig: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("gts from-trig: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = {
        #[cfg(feature = "rdf-codecs")]
        {
            match gmeow_gts::rdf_codecs::from_trig(&text) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("gts from-trig: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        #[cfg(not(feature = "rdf-codecs"))]
        {
            match from_trig(&text) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("gts from-trig: {e}");
                    return ExitCode::from(1);
                }
            }
        }
    };

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts from-trig: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-trig: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_from_nt(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts from-nt: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            eprintln!("gts from-nt: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("gts from-nt: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match gmeow_gts::rdf_codecs::from_ntriples(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("gts from-nt: {e}");
            return ExitCode::from(1);
        }
    };

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts from-nt: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-nt: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_from_turtle(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts from-turtle: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            eprintln!("gts from-turtle: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("gts from-turtle: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match gmeow_gts::rdf_codecs::from_turtle(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("gts from-turtle: {e}");
            return ExitCode::from(1);
        }
    };

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts from-turtle: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-turtle: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_from_rdfxml(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts from-rdfxml: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            eprintln!("gts from-rdfxml: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("gts from-rdfxml: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match gmeow_gts::rdf_codecs::from_rdf_xml(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("gts from-rdfxml: {e}");
            return ExitCode::from(1);
        }
    };

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts from-rdfxml: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-rdfxml: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "yaml-ld")]
fn cmd_to_yaml_ld(args: &[String]) -> ExitCode {
    let [path] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match to_yaml_ld(&graph) {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts to-yaml-ld: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(feature = "yaml-ld")]
fn cmd_from_yaml_ld(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts from-yaml-ld: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            eprintln!("gts from-yaml-ld: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("gts from-yaml-ld: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match from_yaml_ld(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("gts from-yaml-ld: {e}");
            return ExitCode::from(1);
        }
    };

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts from-yaml-ld: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-yaml-ld: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "okf")]
fn cmd_to_okf(args: &[String]) -> ExitCode {
    let mut input: Option<&str> = None;
    let mut directory: Option<&str> = None;
    let mut options = OkfExportOptions::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--directory" => match it.next() {
                Some(path) => directory = Some(path),
                None => {
                    eprintln!("gts to-okf: --directory requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--inline-body" => options.inline_body = true,
            "--base-iri" => match it.next() {
                Some(iri) => options.base_iri = iri.clone(),
                None => {
                    eprintln!("gts to-okf: --base-iri requires an IRI\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other if input.is_none() => input = Some(other),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let (Some(input), Some(directory)) = (input, directory) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let graph = match export_graph(input) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match to_okf(&graph, std::path::Path::new(directory), &options) {
        Ok(report) => {
            if report.unmapped_triples > 0 {
                eprintln!(
                    "gts to-okf: wrote {} unmapped triple(s) to _unmapped.nq",
                    report.unmapped_triples
                );
            }
            eprintln!(
                "gts to-okf: wrote {} document(s) to {}",
                report.documents,
                report.directory.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts to-okf: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(feature = "okf")]
fn cmd_from_okf(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut input: Option<&str> = None;
    let mut options = FromOkfOptions::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-o" | "--out" => match it.next() {
                Some(path) => out_path = Some(path),
                None => {
                    eprintln!("gts from-okf: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--inline-body" => options.inline_body = true,
            "--strict-links" => options.strict_links = true,
            "--base-iri" => match it.next() {
                Some(iri) => options.base_iri = iri.clone(),
                None => {
                    eprintln!("gts from-okf: --base-iri requires an IRI\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other if input.is_none() => input = Some(other),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(input) = input else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let bytes = match from_okf_with_options(std::path::Path::new(input), &options) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("gts from-okf: {e}");
            return ExitCode::from(1);
        }
    };
    match out_path {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &bytes) {
                eprintln!("gts from-okf: cannot write {path}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts from-okf: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "tar")]
fn cmd_from_tar(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut input: Option<&str> = None;
    let mut options = FromTarOptions::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-o" | "--out" => match it.next() {
                Some(path) => out_path = Some(path),
                None => {
                    eprintln!("gts from-tar: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--allow-symlinks" => options.allow_symlinks = true,
            "--allow-special" => options.allow_special = true,
            "--owner" => options.owner = true,
            other if input.is_none() => input = Some(other),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(input) = input else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    if input == "-" {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        return write_from_tar_output(&mut handle, out_path, &options);
    }

    options.source_name = Some(input.to_string());
    let mut file = match std::fs::File::open(input) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("gts from-tar: cannot read {input}: {e}");
            return ExitCode::from(2);
        }
    };
    write_from_tar_output(&mut file, out_path, &options)
}

#[cfg(feature = "tar")]
fn cmd_to_tar(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut input: Option<&str> = None;
    let mut options = ToTarOptions::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-o" | "--out" => match it.next() {
                Some(path) => out_path = Some(path),
                None => {
                    eprintln!("gts to-tar: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "-z" | "--gzip" => {
                if let Err(e) = set_tar_compression(&mut options, TarCompression::Gzip) {
                    eprintln!("gts to-tar: {e}");
                    return ExitCode::from(2);
                }
            }
            "--zstd" => {
                if let Err(e) = set_tar_compression(&mut options, TarCompression::Zstd) {
                    eprintln!("gts to-tar: {e}");
                    return ExitCode::from(2);
                }
            }
            "--numeric-owner" => options.numeric_owner = true,
            other if input.is_none() => input = Some(other),
            _ => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(input) = input else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let graph = match export_graph(input) {
        Ok(graph) => graph,
        Err(code) => return code,
    };

    match out_path {
        Some("-") | None => {
            let mut stdout = std::io::stdout();
            match to_tar(&graph, &mut stdout, &options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("gts to-tar: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some(path) => {
            let mut file = match std::fs::File::create(path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("gts to-tar: cannot write {path}: {e}");
                    return ExitCode::from(2);
                }
            };
            match to_tar(&graph, &mut file, &options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("gts to-tar: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

#[cfg(feature = "tar")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TarOperation {
    Create,
    Extract,
    List,
    Diff,
}

#[cfg(feature = "tar")]
#[derive(Debug, Default)]
struct TarCliArgs {
    operation: Option<TarOperation>,
    archive: Option<String>,
    sources: Vec<String>,
    directory: Option<String>,
    from_options: FromTarOptions,
    to_options: ToTarOptions,
    unpack_options: UnpackOptions,
}

#[cfg(feature = "tar")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArchiveKind {
    Gts,
    Tar,
}

#[cfg(feature = "tar")]
fn cmd_tar(args: &[String]) -> ExitCode {
    let parsed = match parse_tar_cli_args(args) {
        Ok(parsed) => parsed,
        Err(msg) => {
            eprintln!("gts tar: {msg}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match parsed.operation.expect("parse requires operation") {
        TarOperation::Create => cmd_tar_create(&parsed),
        TarOperation::Extract => cmd_tar_extract(&parsed),
        TarOperation::List => cmd_tar_list(&parsed),
        TarOperation::Diff => cmd_tar_diff(&parsed),
    }
}

#[cfg(feature = "tar")]
fn parse_tar_cli_args(args: &[String]) -> Result<TarCliArgs, String> {
    let mut parsed = TarCliArgs::default();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--" => {
                parsed.sources.extend(args[i + 1..].iter().cloned());
                break;
            }
            "--allow-symlinks" => {
                parsed.from_options.allow_symlinks = true;
                parsed.unpack_options.allow_symlinks = true;
            }
            "--allow-special" => {
                parsed.from_options.allow_special = true;
                parsed.unpack_options.allow_special = true;
            }
            "--same-owner" => {
                parsed.from_options.owner = true;
                parsed.unpack_options.same_owner = true;
            }
            "--numeric-owner" => {
                parsed.from_options.owner = true;
                parsed.to_options.numeric_owner = true;
                parsed.unpack_options.same_owner = true;
            }
            "--preserve-setid" => parsed.unpack_options.preserve_setid = true,
            "--include-suppressed" => parsed.unpack_options.include_suppressed = true,
            "--gzip" => set_tar_compression(&mut parsed.to_options, TarCompression::Gzip)
                .map_err(str::to_string)?,
            "--zstd" => set_tar_compression(&mut parsed.to_options, TarCompression::Zstd)
                .map_err(str::to_string)?,
            other if other.starts_with("--") => {
                return Err(format!("unknown option {other}"));
            }
            other if other.starts_with('-') && other != "-" => {
                parse_tar_short_option(args, &mut i, &mut parsed)?;
            }
            other => parsed.sources.push(other.to_string()),
        }
        i += 1;
    }

    let Some(operation) = parsed.operation else {
        return Err("choose one of -c, -x, -t, or -d".to_string());
    };
    if parsed.archive.is_none() {
        return Err("-f requires an archive path".to_string());
    }
    match operation {
        TarOperation::Create => {
            if parsed.sources.is_empty() {
                return Err("create requires at least one source".to_string());
            }
        }
        TarOperation::Extract | TarOperation::List => {
            if !parsed.sources.is_empty() {
                return Err("entry-name filters are not supported yet".to_string());
            }
        }
        TarOperation::Diff => {
            if parsed.sources.len() > 1 {
                return Err("diff accepts one directory".to_string());
            }
            if parsed.sources.is_empty() && parsed.directory.is_none() {
                return Err("diff requires a directory or -C".to_string());
            }
        }
    }
    Ok(parsed)
}

#[cfg(feature = "tar")]
fn parse_tar_short_option(
    args: &[String],
    index: &mut usize,
    parsed: &mut TarCliArgs,
) -> Result<(), String> {
    let arg = &args[*index];
    let bytes = arg.as_bytes();
    let mut pos = 1;
    while pos < bytes.len() {
        match bytes[pos] as char {
            'c' => set_tar_operation(parsed, TarOperation::Create)?,
            'x' => set_tar_operation(parsed, TarOperation::Extract)?,
            't' => set_tar_operation(parsed, TarOperation::List)?,
            'd' => set_tar_operation(parsed, TarOperation::Diff)?,
            'z' => set_tar_compression(&mut parsed.to_options, TarCompression::Gzip)
                .map_err(str::to_string)?,
            'f' | 'C' => {
                let value = if pos + 1 < bytes.len() {
                    arg[pos + 1..].to_string()
                } else {
                    *index += 1;
                    args.get(*index)
                        .ok_or_else(|| format!("-{} requires a value", bytes[pos] as char))?
                        .clone()
                };
                if bytes[pos] as char == 'f' {
                    parsed.archive = Some(value);
                } else {
                    parsed.directory = Some(value);
                }
                return Ok(());
            }
            other => return Err(format!("unknown option -{other}")),
        }
        pos += 1;
    }
    Ok(())
}

#[cfg(feature = "tar")]
fn set_tar_operation(parsed: &mut TarCliArgs, operation: TarOperation) -> Result<(), String> {
    if parsed
        .operation
        .is_some_and(|existing| existing != operation)
    {
        return Err("choose only one of -c, -x, -t, or -d".to_string());
    }
    parsed.operation = Some(operation);
    Ok(())
}

#[cfg(feature = "tar")]
fn cmd_tar_create(parsed: &TarCliArgs) -> ExitCode {
    let archive = parsed.archive.as_deref().expect("archive is parsed");
    let kind = archive_kind(archive);
    if kind == ArchiveKind::Gts && parsed.to_options.compression != TarCompression::None {
        eprintln!("gts tar: compression flags require tar output, not .gts");
        return ExitCode::from(2);
    }

    let paths = resolve_tar_sources(&parsed.sources, parsed.directory.as_deref());
    let path_refs: Vec<&std::path::Path> = paths.iter().map(std::path::PathBuf::as_path).collect();
    if kind == ArchiveKind::Gts {
        let mut file = match std::fs::File::create(archive) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("gts tar: cannot write {archive}: {e}");
                return ExitCode::from(2);
            }
        };
        return match gmeow_gts::files::pack_to_writer(&path_refs, &mut file) {
            Ok(()) => ExitCode::SUCCESS,
            Err(msg) => {
                eprintln!("gts tar: refusing create: {msg}");
                ExitCode::from(1)
            }
        };
    }

    let data = match gmeow_gts::files::pack(&path_refs) {
        Ok(data) => data,
        Err(msg) => {
            eprintln!("gts tar: refusing create: {msg}");
            return ExitCode::from(1);
        }
    };

    let graph = match graph_from_clean_bytes("tar", archive, &data) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    let mut options = parsed.to_options.clone();
    if let Err(msg) = infer_tar_output_compression(archive, &mut options) {
        eprintln!("gts tar: {msg}");
        return ExitCode::from(2);
    }
    write_tar_output("tar", archive, &graph, &options)
}

#[cfg(feature = "tar")]
fn cmd_tar_extract(parsed: &TarCliArgs) -> ExitCode {
    let archive = parsed.archive.as_deref().expect("archive is parsed");
    let graph = match read_archive_graph_for_tar(parsed, false) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    let dest = parsed.directory.as_deref().unwrap_or(".");
    match gmeow_gts::files::unpack_with_options(
        &graph,
        std::path::Path::new(dest),
        &parsed.unpack_options,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("gts tar: refusing extract from {archive}: {msg}");
            ExitCode::from(1)
        }
    }
}

#[cfg(feature = "tar")]
fn cmd_tar_list(parsed: &TarCliArgs) -> ExitCode {
    let graph = match read_archive_graph_for_tar(parsed, true) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match print_files_profile_listing(&graph) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("gts tar: refusing list: {msg}");
            ExitCode::from(1)
        }
    }
}

#[cfg(feature = "tar")]
fn cmd_tar_diff(parsed: &TarCliArgs) -> ExitCode {
    let graph = match read_archive_graph_for_tar(parsed, true) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    let directory = match tar_diff_directory(parsed) {
        Ok(path) => path,
        Err(msg) => {
            eprintln!("gts tar: {msg}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match gmeow_gts::files::diff(&graph, &directory) {
        Ok(lines) => {
            let has_changes = !lines.is_empty();
            for line in &lines {
                println!("{line}");
            }
            if has_changes {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(msg) => {
            eprintln!("gts tar: refusing diff: {msg}");
            ExitCode::from(1)
        }
    }
}

#[cfg(feature = "tar")]
fn resolve_tar_sources(sources: &[String], directory: Option<&str>) -> Vec<std::path::PathBuf> {
    sources
        .iter()
        .map(|source| path_under_directory(source, directory))
        .collect()
}

#[cfg(feature = "tar")]
fn tar_diff_directory(parsed: &TarCliArgs) -> Result<std::path::PathBuf, String> {
    match (parsed.sources.as_slice(), parsed.directory.as_deref()) {
        ([source], directory) => Ok(path_under_directory(source, directory)),
        ([], Some(directory)) => Ok(std::path::PathBuf::from(directory)),
        ([], None) => Err("diff requires a directory or -C".to_string()),
        _ => Err("diff accepts one directory".to_string()),
    }
}

#[cfg(feature = "tar")]
fn path_under_directory(source: &str, directory: Option<&str>) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(source);
    if path.is_absolute() {
        path
    } else if let Some(directory) = directory {
        std::path::Path::new(directory).join(path)
    } else {
        path
    }
}

#[cfg(feature = "tar")]
fn read_archive_graph_for_tar(parsed: &TarCliArgs, read_only: bool) -> Result<Graph, ExitCode> {
    let archive = parsed.archive.as_deref().expect("archive is parsed");
    match archive_kind(archive) {
        ArchiveKind::Gts => export_graph(archive),
        ArchiveKind::Tar => {
            let mut options = parsed.from_options.clone();
            if read_only {
                options.allow_symlinks = true;
                options.allow_special = true;
            }
            let data = read_tar_as_gts_bytes("tar", archive, &mut options)?;
            graph_from_clean_bytes("tar", archive, &data)
        }
    }
}

#[cfg(feature = "tar")]
fn read_tar_as_gts_bytes(
    command: &str,
    archive: &str,
    options: &mut FromTarOptions,
) -> Result<Vec<u8>, ExitCode> {
    let mut data = Vec::new();
    if archive == "-" {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        if let Err(e) = from_tar_to_writer(&mut handle, &mut data, options) {
            eprintln!("gts {command}: {e}");
            return Err(ExitCode::from(1));
        }
    } else {
        options.source_name = Some(archive.to_string());
        let mut file = std::fs::File::open(archive).map_err(|e| {
            eprintln!("gts {command}: cannot read {archive}: {e}");
            ExitCode::from(2)
        })?;
        if let Err(e) = from_tar_to_writer(&mut file, &mut data, options) {
            eprintln!("gts {command}: {e}");
            return Err(ExitCode::from(1));
        }
    }
    Ok(data)
}

#[cfg(feature = "tar")]
fn graph_from_clean_bytes(command: &str, label: &str, data: &[u8]) -> Result<Graph, ExitCode> {
    let graph = read(data, true, None);
    for d in &graph.diagnostics {
        eprintln!("gts {command}: diagnostic {}: {}", d.code, d.detail);
    }
    if !graph.diagnostics.is_empty() || graph.segment_heads.is_empty() {
        eprintln!("gts {command}: refusing {label}: archive did not read cleanly");
        return Err(ExitCode::from(1));
    }
    Ok(graph)
}

#[cfg(feature = "tar")]
fn write_tar_output(
    command: &str,
    archive: &str,
    graph: &Graph,
    options: &ToTarOptions,
) -> ExitCode {
    if archive == "-" {
        let mut stdout = std::io::stdout();
        match to_tar(graph, &mut stdout, options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("gts {command}: {e}");
                ExitCode::from(1)
            }
        }
    } else {
        let mut file = match std::fs::File::create(archive) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("gts {command}: cannot write {archive}: {e}");
                return ExitCode::from(2);
            }
        };
        match to_tar(graph, &mut file, options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("gts {command}: {e}");
                ExitCode::from(1)
            }
        }
    }
}

#[cfg(feature = "tar")]
fn print_files_profile_listing(graph: &Graph) -> Result<(), String> {
    for entry in read_entries(graph)?.values() {
        let size = entry
            .size
            .map(|size| size.to_string())
            .unwrap_or_else(|| "-".to_string());
        match entry.kind {
            FileEntryKind::Symlink | FileEntryKind::Hardlink => println!(
                "{}\t{}\t{} -> {}",
                entry.kind.as_str(),
                size,
                entry.path,
                entry.link_target.as_deref().unwrap_or("")
            ),
            FileEntryKind::CharDev | FileEntryKind::BlockDev => println!(
                "{}\t{}:{}\t{}",
                entry.kind.as_str(),
                entry
                    .dev_major
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                entry
                    .dev_minor
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                entry.path
            ),
            _ => println!("{}\t{}\t{}", entry.kind.as_str(), size, entry.path),
        }
    }
    Ok(())
}

#[cfg(feature = "tar")]
fn infer_tar_output_compression(
    archive: &str,
    options: &mut ToTarOptions,
) -> Result<(), &'static str> {
    if options.compression != TarCompression::None {
        return Ok(());
    }
    let lower = archive.to_ascii_lowercase();
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") || lower.ends_with(".gz") {
        set_tar_compression(options, TarCompression::Gzip)?;
    } else if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") || lower.ends_with(".zst") {
        set_tar_compression(options, TarCompression::Zstd)?;
    }
    Ok(())
}

#[cfg(feature = "tar")]
fn archive_kind(path: &str) -> ArchiveKind {
    if path.to_ascii_lowercase().ends_with(".gts") {
        ArchiveKind::Gts
    } else {
        ArchiveKind::Tar
    }
}

#[cfg(feature = "tar")]
fn set_tar_compression(
    options: &mut ToTarOptions,
    compression: TarCompression,
) -> Result<(), &'static str> {
    if options.compression != TarCompression::None && options.compression != compression {
        return Err("choose only one compression format");
    }
    options.compression = compression;
    Ok(())
}

#[cfg(feature = "tar")]
fn write_from_tar_output<R: std::io::Read>(
    reader: R,
    out_path: Option<&str>,
    options: &FromTarOptions,
) -> ExitCode {
    match out_path {
        Some("-") | None => {
            let mut stdout = std::io::stdout();
            match from_tar_to_writer(reader, &mut stdout, options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("gts from-tar: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some(path) => {
            let mut file = match std::fs::File::create(path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("gts from-tar: cannot write {path}: {e}");
                    return ExitCode::from(2);
                }
            };
            match from_tar_to_writer(reader, &mut file, options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("gts from-tar: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}

fn export_graph(path: &str) -> Result<Graph, ExitCode> {
    let data = load(path)?;
    let graph = read(&data, true, None);
    for d in &graph.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    if !graph.diagnostics.is_empty() || graph.segment_heads.is_empty() {
        eprintln!("gts: refusing export: input did not read cleanly");
        return Err(ExitCode::from(1));
    }
    Ok(graph)
}

fn cmd_to_sqlite(args: &[String]) -> ExitCode {
    let [path, out] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match gmeow_gts::db::to_sqlite(&graph, out) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gts to-sqlite: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(feature = "duckdb")]
fn cmd_to_duckdb(args: &[String]) -> ExitCode {
    let [path, out] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match gmeow_gts::db::to_duckdb(&graph, out) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gts to-duckdb: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(feature = "duckdb")]
fn cmd_to_parquet(args: &[String]) -> ExitCode {
    let [path, out_dir] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match gmeow_gts::db::to_parquet(&graph, out_dir) {
        Ok(paths) => {
            for path in paths {
                println!("{}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts to-parquet: {e}");
            ExitCode::from(2)
        }
    }
}

/// Parse a `kid:hexpubkey` spec into a verifier entry.
fn parse_key(spec: &str) -> Option<(String, VerifyingKey)> {
    let (kid, hexpub) = spec.rsplit_once(':')?;
    if kid.is_empty() || hexpub.len() != 64 {
        return None;
    }
    let mut raw = [0u8; 32];
    for (i, byte) in raw.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hexpub[i * 2..i * 2 + 2], 16).ok()?;
    }
    VerifyingKey::from_bytes(&raw)
        .ok()
        .map(|k| (kid.to_string(), k))
}

#[cfg(feature = "policy-config")]
fn load_policy(path: &str) -> Result<TrustPolicy, ExitCode> {
    TrustPolicy::from_path(path).map_err(|e| {
        eprintln!("gts verify: {e}");
        ExitCode::from(2)
    })
}

#[cfg(not(feature = "policy-config"))]
fn load_policy(_path: &str) -> Result<TrustPolicy, ExitCode> {
    eprintln!("gts verify: --policy requires rebuilding gmeow-gts with `--features policy-config`");
    Err(ExitCode::from(2))
}

fn cmd_verify(args: &[String]) -> ExitCode {
    let mut paths: Vec<String> = Vec::new();
    let mut keys: std::collections::HashMap<String, VerifyingKey> =
        std::collections::HashMap::new();
    let mut policy: Option<TrustPolicy> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--key" => {
                i += 1;
                let Some(spec) = args.get(i) else {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                };
                match parse_key(spec) {
                    Some((kid, key)) => {
                        keys.insert(kid, key);
                    }
                    None => {
                        eprintln!("gts verify: bad --key {spec:?} (want kid:hexpubkey)");
                        return ExitCode::from(2);
                    }
                }
            }
            "--policy" => {
                i += 1;
                let Some(path) = args.get(i) else {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                };
                match load_policy(path) {
                    Ok(loaded) => policy = Some(loaded),
                    Err(code) => return code,
                }
            }
            other => paths.push(other.to_string()),
        }
        i += 1;
    }
    if paths.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let mut problems = false;
    for path in &paths {
        let data = match load(path) {
            Ok(d) => d,
            Err(code) => return code,
        };
        let mut fs = read_file_segments(&data);
        print_ledger(path, &fs);
        if has_problems(&fs) {
            problems = true;
        }
        // §14.1: declared-vs-computed profile requirements + layout warnings.
        for (idx, seg) in fs.segments.iter_mut().enumerate() {
            if let Some(policy) = policy.as_ref() {
                if !keys.is_empty() {
                    verify_signatures(&mut seg.signatures, |k| keys.get(k).copied());
                }
                for finding in evaluate_profile_policy(seg, Some(policy), Some(idx)) {
                    eprintln!(
                        "  segment {idx}: {}: {}: {}",
                        finding.severity.as_str(),
                        finding.code,
                        finding.detail
                    );
                    if finding.severity == Severity::Error {
                        problems = true;
                    }
                }
            } else {
                for (msg, is_err) in profile_check(seg) {
                    let prefix = if is_err { "error" } else { "warning" };
                    eprintln!("  segment {idx}: {prefix}: {msg}");
                    if is_err {
                        problems = true;
                    }
                }
                for msg in stream_vocab_check(seg) {
                    eprintln!("  segment {idx}: warning: {msg}");
                }
            }
        }
        // §9.2: COSE signature verification against the provided keys.
        if !keys.is_empty() {
            let mut g = read(&data, true, None);
            verify_signatures(&mut g.signatures, |k| keys.get(k).copied());
            for sig in &g.signatures {
                println!(
                    "  signature {}: {}",
                    sig.kid.as_deref().unwrap_or("?"),
                    sig.status
                );
                if sig.status == "invalid" {
                    problems = true;
                }
            }
        }
    }
    if problems {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// `extract-key`: print the embedded transport (verification) key (§9.2).
fn cmd_extract_key(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    let Some(transport) = extract_transport_key(&g) else {
        eprintln!("{path}: no embedded transport key");
        return ExitCode::from(1);
    };
    let kid = transport.kid;
    let gpg = transport.gpg;
    println!("kid:         {kid}");
    match gmeow_gts::openpgp::parse_transport_key(&gpg) {
        Ok(key) => {
            println!("fingerprint: {}", format_fingerprint(&key.fingerprint));
            println!("emojihash:   {}", emojihash(&key.raw_public, 11));
        }
        // A malformed embedded key still prints the kid + armored block below.
        Err(_) => println!("fingerprint: {}", format_fingerprint(&kid)),
    }
    println!("{gpg}");
    ExitCode::SUCCESS
}

fn blob_mt(g: &Graph, digest: &str) -> Option<String> {
    g.blob_meta
        .iter()
        .find(|(d, _)| d == digest)
        .and_then(|(_, meta)| {
            if let Value::Map(entries) = meta {
                entries.iter().find_map(|(k, v)| match (k, v) {
                    (Value::Text(key), Value::Text(text)) if key == "mt" => Some(text.clone()),
                    _ => None,
                })
            } else {
                None
            }
        })
}

/// List inline blobs: digest, size, declared media type (tar's `t`).
fn cmd_ls(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    for (digest, entry) in &g.blobs {
        let mt = blob_mt(&g, digest).unwrap_or_else(|| "-".to_string());
        let size = match entry.decoded_len() {
            Ok(size) => size,
            Err(err) => {
                eprintln!("gts: cannot decode blob {digest}: {err:?}");
                return ExitCode::from(1);
            }
        };
        println!("{digest}  {size:>10}  {mt}");
    }
    ExitCode::SUCCESS
}

fn normalize_digest(digest: &str) -> String {
    if digest.starts_with("blake3:") {
        digest.to_string()
    } else {
        format!("blake3:{digest}")
    }
}

/// Digests hidden by `{"kind": "blob", "digest": …}` targets (§11).
fn suppressed_blob_digests(g: &Graph) -> HashSet<String> {
    let mut out = HashSet::new();
    for sup in &g.suppressions {
        for target in &sup.targets {
            if target_kind(target) != "blob" {
                continue;
            }
            if let Value::Map(entries) = target {
                for (k, v) in entries {
                    if matches!(k, Value::Text(t) if t == "digest") {
                        match v {
                            Value::Bytes(b) => {
                                out.insert(format!("blake3:{}", hex(b)));
                            }
                            Value::Text(t) => {
                                out.insert(normalize_digest(t));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    out
}

/// Extract one blob by content digest (tar's `x`), refuse-don't-trust:
/// verify bytes against the digest, honour §11 suppression unless overridden,
/// and treat `--mt` as an ASSERTION against the declared media type — never
/// a conversion.
fn cmd_extract(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut mt: Option<&str> = None;
    let mut include_suppressed = false;
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--mt" => match it.next() {
                Some(m) => mt = Some(m),
                None => {
                    eprintln!("gts: --mt requires a media type\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--include-suppressed" => include_suppressed = true,
            other => positional.push(other),
        }
    }
    let [path, digest] = positional[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let mut g = read(&data, true, None);
    let digest = normalize_digest(digest);
    if g.blob_entry(&digest).is_none() {
        eprintln!("gts: no inline blob {digest} in {path}");
        return ExitCode::from(1);
    }
    if !include_suppressed && suppressed_blob_digests(&g).contains(&digest) {
        eprintln!(
            "gts: refusing {digest}: suppressed (§11); pass \
             --include-suppressed to extract anyway"
        );
        return ExitCode::from(1);
    }
    if let Some(asserted) = mt {
        let declared = blob_mt(&g, &digest);
        if declared.as_deref() != Some(asserted) {
            eprintln!(
                "gts: refusing {digest}: declared media type {declared:?} \
                 does not match asserted {asserted:?}"
            );
            return ExitCode::from(1);
        }
    }
    let bytes = match g.blob_bytes_cloned(&digest) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            eprintln!("gts: no inline blob {digest} in {path}");
            return ExitCode::from(1);
        }
        Err(err) => {
            eprintln!("gts: cannot decode blob {digest}: {err:?}");
            return ExitCode::from(1);
        }
    };
    if digest_str(&bytes) != digest {
        eprintln!("gts: integrity failure: {digest} bytes re-hash differently");
        return ExitCode::from(1);
    }
    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &bytes) {
                eprintln!("gts: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&bytes) {
                eprintln!("gts: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

/// The validating composer (§14.1): refuse-don't-trust, then `cat`.
fn cmd_cat(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut inputs: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "-o" {
            match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            }
        } else {
            inputs.push(a);
        }
    }
    if inputs.len() < 2 {
        eprintln!("gts: cat needs at least two inputs\n{USAGE}");
        return ExitCode::from(2);
    }

    let mut combined: Vec<u8> = Vec::new();
    for path in &inputs {
        let data = match load(path) {
            Ok(d) => d,
            Err(code) => return code,
        };
        let fs = read_file_segments(&data);
        if has_problems(&fs) {
            eprintln!("gts: refusing {path}: not a clean GTS input");
            print_ledger(path, &fs);
            return ExitCode::from(1);
        }
        // §14.1: a segment that contributes NOTHING (no quads, blobs,
        // reifier bindings, annotations, or suppressions) is almost always a
        // wiring bug — never a real package. Refuse, don't trust.
        for (idx, seg) in fs.segments.iter().enumerate() {
            let contributes = !seg.quads.is_empty()
                || !seg.blobs.is_empty()
                || !seg.reifiers.is_empty()
                || !seg.annotations.is_empty()
                || !seg.suppressions.is_empty();
            if !contributes {
                eprintln!(
                    "gts: refusing {path}: segment {idx} folds to nothing \
                     (no quads/blobs/reifies/annot/suppress) — wiring bug?"
                );
                return ExitCode::from(1);
            }
        }
        combined.extend_from_slice(&data);
    }

    // §14.1: refuse an output in which suppressions would hide every prior
    // frame — a composition that suppresses the whole graph is a mistake.
    let folded = read(&combined, true, None);
    if all_quads_suppressed(&folded) {
        eprintln!(
            "gts: refusing composition: suppressions hide every quad in the \
             folded output"
        );
        return ExitCode::from(1);
    }

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &combined) {
                eprintln!("gts: cannot write {p}: {e}");
                return ExitCode::from(2);
            }
        }
        None => {
            use std::io::Write;
            if let Err(e) = std::io::stdout().write_all(&combined) {
                eprintln!("gts: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ` — the default `--timestamp`.
fn now_utc_iso() -> String {
    let fmt = time::format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]T[hour]:[minute]:[second]Z",
    )
    .expect("static format string parses");
    time::OffsetDateTime::now_utc()
        .format(&fmt)
        .expect("UTC time formats")
}

/// Rewrite a GTS file into the streamable layout state (§10.1, §14.1).
fn cmd_compact(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut streamable = false;
    let mut seal_original = false;
    let mut timestamp: Option<&str> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--streamable" => streamable = true,
            "--seal-original" => seal_original = true,
            "--timestamp" => match it.next() {
                Some(t) => timestamp = Some(t),
                None => {
                    eprintln!("gts: --timestamp requires a value\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => positional.push(other),
        }
    }
    let [path] = positional[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let Some(out_path) = out_path else {
        eprintln!("gts: compact requires -o\n{USAGE}");
        return ExitCode::from(2);
    };
    if !streamable {
        // The verb is reserved for layout rewrites; a future --snapshot mode
        // (§10) would land here. Without a mode the request is ambiguous.
        eprintln!("gts: compact requires --streamable");
        return ExitCode::from(2);
    }
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    // The timestamp defaults to now — pass a fixed value for reproducible
    // output (§14.1 determinism).
    let ts = timestamp.map_or_else(now_utc_iso, str::to_string);
    match gmeow_gts::compact::compact_streamable(&data, &ts, seal_original) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(out_path, &bytes) {
                eprintln!("gts: cannot write {out_path}: {e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("gts: refusing compact: {e}");
            ExitCode::from(1)
        }
    }
}

/// Pack files/directories into a files-profile GTS archive (tar's `c`).
fn cmd_pack(args: &[String]) -> ExitCode {
    let mut out_path: Option<&str> = None;
    let mut sources: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--out" => match it.next() {
                Some(p) => out_path = Some(p),
                None => {
                    eprintln!("gts: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other => sources.push(other),
        }
    }
    if sources.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    let out_path = match out_path {
        Some(p) => p,
        None => {
            eprintln!("gts: pack requires -o\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let paths: Vec<&std::path::Path> = sources.iter().map(std::path::Path::new).collect();
    match gmeow_gts::files::pack(&paths) {
        Ok(data) => {
            if let Err(e) = std::fs::write(out_path, &data) {
                eprintln!("gts: cannot write {out_path}: {e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("gts: refusing pack: {msg}");
            ExitCode::from(1)
        }
    }
}

/// Unpack a files-profile GTS archive (tar's `x`), verifying digests.
fn cmd_unpack(args: &[String]) -> ExitCode {
    let mut dest: Option<&str> = None;
    let mut options = UnpackOptions::default();
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-C" => match it.next() {
                Some(d) => dest = Some(d),
                None => {
                    eprintln!("gts: -C requires a directory\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--include-suppressed" => options.include_suppressed = true,
            "--allow-symlinks" => options.allow_symlinks = true,
            "--allow-special" => options.allow_special = true,
            "--same-owner" | "--numeric-owner" => options.same_owner = true,
            "--preserve-setid" => options.preserve_setid = true,
            other => positional.push(other),
        }
    }
    let [path] = positional[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        eprintln!("gts: refusing unpack: archive did not read cleanly");
        return ExitCode::from(1);
    }
    let dest_path = std::path::Path::new(dest.unwrap_or("."));
    match gmeow_gts::files::unpack_with_options(&g, dest_path, &options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("gts: refusing unpack: {msg}");
            ExitCode::from(1)
        }
    }
}

/// Compare an archive to a directory by content digest (tar's `d`).
fn cmd_diff(args: &[String]) -> ExitCode {
    let [archive, directory] = args else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let data = match load(archive) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        eprintln!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        eprintln!("gts: refusing diff: archive did not read cleanly");
        return ExitCode::from(1);
    }
    match gmeow_gts::files::diff(&g, std::path::Path::new(directory)) {
        Ok(lines) => {
            let has_changes = !lines.is_empty();
            for line in &lines {
                println!("{line}");
            }
            if has_changes {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(msg) => {
            eprintln!("gts: refusing diff: {msg}");
            ExitCode::from(1)
        }
    }
}

/// Expand an archive into a human/tool inspection directory.
fn cmd_dump(args: &[String]) -> ExitCode {
    let mut directory: Option<&str> = None;
    let mut include_suppressed = false;
    let mut force = false;
    let mut metadata_only = false;
    let mut positional: Vec<&str> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--directory" => match it.next() {
                Some(path) => directory = Some(path),
                None => {
                    eprintln!("gts dump: --directory requires a path\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--include-suppressed" => include_suppressed = true,
            "--force" => force = true,
            "--metadata-only" => metadata_only = true,
            other => positional.push(other),
        }
    }
    let [archive] = positional[..] else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let Some(directory) = directory else {
        eprintln!("gts dump: --directory is required\n{USAGE}");
        return ExitCode::from(2);
    };
    let options = gmeow_gts::dumpdir::DumpOptions {
        include_suppressed,
        force,
        metadata_only,
    };
    match gmeow_gts::dumpdir::dump_path(archive, directory, options) {
        Ok(report) => {
            eprintln!(
                "gts dump: wrote {} ({} materialized file(s), {} blob payload(s))",
                report.directory.display(),
                report.materialized_files,
                report.materialized_blobs
            );
            if report.clean {
                ExitCode::SUCCESS
            } else {
                eprintln!(
                    "gts dump: archive had diagnostics or dump warnings ({} warning(s))",
                    report.warnings
                );
                ExitCode::from(1)
            }
        }
        Err(err) => {
            eprintln!("gts dump: {err}");
            match err.kind() {
                gmeow_gts::dumpdir::DumpErrorKind::Refused => ExitCode::from(1),
                gmeow_gts::dumpdir::DumpErrorKind::Io => ExitCode::from(2),
            }
        }
    }
}

fn target_idx(target: &Value, key: &str) -> Option<usize> {
    let Value::Map(entries) = target else {
        return None;
    };
    let v = entries
        .iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v)?;
    if let Value::Integer(i) = v {
        usize::try_from(i128::from(*i)).ok()
    } else {
        None
    }
}

fn target_kind(target: &Value) -> &str {
    if let Value::Map(entries) = target {
        for (k, v) in entries {
            if matches!(k, Value::Text(t) if t == "kind") {
                if let Value::Text(t) = v {
                    return t;
                }
            }
        }
    }
    ""
}

/// True iff the folded graph has quads and EVERY one is hidden by a
/// suppression (a direct quad target, or a term target on any component).
fn all_quads_suppressed(g: &Graph) -> bool {
    if g.quads.is_empty() || g.suppressions.is_empty() {
        return false;
    }
    let mut term_sup: HashSet<usize> = HashSet::new();
    let mut quad_sup: HashSet<Vec<usize>> = HashSet::new();
    for sup in &g.suppressions {
        collect_suppressed(sup, &mut term_sup, &mut quad_sup);
    }
    g.quads.iter().all(|&(s, p, o, gq)| {
        let key = match gq {
            Some(gv) => vec![s, p, o, gv],
            None => vec![s, p, o],
        };
        quad_sup.contains(&key)
            || term_sup.contains(&s)
            || term_sup.contains(&p)
            || term_sup.contains(&o)
            || gq.is_some_and(|gv| term_sup.contains(&gv))
    })
}

fn collect_suppressed(
    sup: &Suppression,
    term_sup: &mut HashSet<usize>,
    quad_sup: &mut HashSet<Vec<usize>>,
) {
    for target in &sup.targets {
        match target_kind(target) {
            "term" | "reifier" => {
                if let Some(id) = target_idx(target, "id") {
                    term_sup.insert(id);
                }
            }
            "quad" => {
                if let Value::Map(entries) = target {
                    let q = entries
                        .iter()
                        .find(|(k, _)| matches!(k, Value::Text(t) if t == "q"))
                        .map(|(_, v)| v);
                    if let Some(Value::Array(ids)) = q {
                        let key: Option<Vec<usize>> = ids
                            .iter()
                            .map(|x| {
                                if let Value::Integer(i) = x {
                                    usize::try_from(i128::from(*i)).ok()
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if let Some(key) = key {
                            quad_sup.insert(key);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

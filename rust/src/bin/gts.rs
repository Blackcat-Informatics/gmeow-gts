// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
use gmeow_gts::cose::{unsupported_algorithm, verify_signatures};
use gmeow_gts::emojihash::emojihash;
use gmeow_gts::files::UnpackOptions;
#[cfg(feature = "okf")]
use gmeow_gts::from_okf::{from_okf_with_options, FromOkfOptions};
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
#[cfg(not(feature = "rdf-codecs"))]
use gmeow_gts::trig::to_trig;
use gmeow_gts::v2::from_nquads;
use gmeow_gts::v2::reader::{read_wire, Dispatch, ReadCapacity};
use gmeow_gts::verify::{extract_transport_key, format_fingerprint};
use gmeow_gts::wire::{digest_str, hex};
#[cfg(feature = "yaml-ld")]
use gmeow_gts::yamlld::to_yaml_ld;

#[macro_use]
#[path = "gts/output.rs"]
mod output;

#[cfg(feature = "tar")]
#[path = "gts/tar_cli.rs"]
mod tar_cli;

#[path = "gts/i18n.rs"]
mod i18n;

#[path = "gts/info_v2.rs"]
mod info_v2;

use i18n::{unknown_command_message, USAGE};

/// Print usage and exit 2 — the CLI's uniform "bad invocation" response.
///
/// Every subcommand rejects a malformed argument list the same way, so the
/// pair of statements lived in 29 places. Naming it once keeps that contract
/// in one spot and makes each call site read as the single decision it is.
fn usage_error() -> ExitCode {
    stderr_line!("{USAGE}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        return usage_error();
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
        "from-tar" => tar_cli::cmd_from_tar(&args[1..]),
        #[cfg(feature = "tar")]
        "to-tar" => tar_cli::cmd_to_tar(&args[1..]),
        #[cfg(feature = "tar")]
        "tar" => tar_cli::cmd_tar(&args[1..]),
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
            stdout_line!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            stderr_line!("{}", unknown_command_message(other));
            ExitCode::from(2)
        }
    }
}

#[cfg(not(feature = "duckdb"))]
fn cmd_duckdb_disabled(cmd: &str) -> ExitCode {
    stderr_line!(
        "gts {cmd}: optional DuckDB/Parquet exports are disabled; rebuild with \
         `--features duckdb` and keep the `duckdb` binary on PATH"
    );
    ExitCode::from(2)
}

#[cfg(not(feature = "yaml-ld"))]
fn cmd_yaml_ld_disabled(cmd: &str) -> ExitCode {
    stderr_line!("gts {cmd}: YAML-LD-star transforms are disabled; rebuild with `--features yaml-ld`");
    ExitCode::from(2)
}

#[cfg(not(feature = "rdf-codecs"))]
fn cmd_rdf_codecs_disabled(cmd: &str) -> ExitCode {
    stderr_line!(
        "gts {cmd}: RDF Turtle-family codecs are disabled; rebuild with \
         `--features rdf-codecs`"
    );
    ExitCode::from(2)
}

#[cfg(not(feature = "okf"))]
fn cmd_okf_disabled(cmd: &str) -> ExitCode {
    stderr_line!("gts {cmd}: OKF transforms are disabled; rebuild with `--features okf`");
    ExitCode::from(2)
}

#[cfg(not(feature = "tar"))]
fn cmd_tar_disabled(cmd: &str) -> ExitCode {
    stderr_line!("gts {cmd}: tar transforms are disabled; rebuild with `--features tar`");
    ExitCode::from(2)
}

fn load(path: &str) -> Result<Vec<u8>, ExitCode> {
    std::fs::read(path).map_err(|e| {
        stderr_line!("gts: cannot read {path}: {e}");
        ExitCode::from(2)
    })
}

fn cmd_prove(args: &[String]) -> ExitCode {
    let [path, frame_id] = args else {
        return usage_error();
    };
    let target = match parse_hex_32(frame_id) {
        Ok(id) => id,
        Err(e) => {
            stderr_line!("gts prove: invalid frame id: {e}");
            return ExitCode::from(2);
        }
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    match prove_file(&data, &target) {
        Ok(proof) => {
            stdout_text!("{}", proof.to_json());
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts prove: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_verify_proof(args: &[String]) -> ExitCode {
    let [path] = args else {
        return usage_error();
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) => {
            stderr_line!("gts verify-proof: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let proof = match Proof::from_json(&text) {
        Ok(proof) => proof,
        Err(e) => {
            stderr_line!("gts verify-proof: invalid proof JSON: {e}");
            return ExitCode::from(2);
        }
    };
    match verify_proof(&proof) {
        Ok(()) => {
            stdout_line!(
                "proof ok: root {} frame {}",
                hex(&proof.root),
                hex(&proof.frame_id)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts verify-proof: invalid proof: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_heads(args: &[String]) -> ExitCode {
    let [path] = args else {
        return usage_error();
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let inventory = inventory(&data);
    stdout_text!("{}", heads_json(&inventory));
    if inventory.has_problems() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_segments(args: &[String]) -> ExitCode {
    let [path] = args else {
        return usage_error();
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let inventory = inventory(&data);
    stdout_text!("{}", segments_json(&inventory));
    if inventory.has_problems() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_missing(args: &[String]) -> ExitCode {
    if args.len() != 3 || args[0] != "--from-head" {
        return usage_error();
    }
    let from_head = &args[1];
    let path = &args[2];
    let from_head = match parse_hex_32(from_head) {
        Ok(head) => head,
        Err(e) => {
            stderr_line!("gts missing: invalid peer head: {e}");
            return ExitCode::from(2);
        }
    };
    let data = match load(path) {
        Ok(data) => data,
        Err(code) => return code,
    };
    let inventory = inventory(&data);
    let result = missing(&inventory, &from_head);
    stdout_text!("{}", missing_json(&result));
    if result.status == MissingStatus::Error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn cmd_resume(args: &[String]) -> ExitCode {
    if args.len() != 3 || args[0] != "--after" {
        return usage_error();
    }
    let frame_id = &args[1];
    let path = &args[2];
    let frame_id = match parse_hex_32(frame_id) {
        Ok(id) => id,
        Err(e) => {
            stderr_line!("gts resume: invalid frame id: {e}");
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
                stderr_line!("gts resume: cannot write stdout: {e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts resume: {e}");
            ExitCode::from(1)
        }
    }
}

/// Print the per-segment composition ledger (§14.1 "SHOULD report").
fn print_ledger(path: &str, fs: &FileSegments) {
    stdout_line!(
        "{path}: {} segment(s){}",
        fs.segments.len(),
        match fs.torn {
            Some(off) => format!(", TORN at byte {off}"),
            None => String::new(),
        }
    );
    if let Some(fatal) = &fs.fatal {
        stdout_line!("  FATAL {}: {}", fatal.code, fatal.detail);
        return;
    }
    for (idx, seg) in fs.segments.iter().enumerate() {
        let head = seg
            .segment_heads
            .first().map_or_else(|| "<none>".to_string(), |h| hex(h));
        let profile = seg
            .segment_profiles
            .first()
            .map_or("<none>", String::as_str);
        let signers = seg
            .signatures
            .iter()
            .filter(|s| s.status != "invalid")
            .count();
        stdout_line!(
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
                    .as_deref().map_or_else(|| "<none>".to_string(), hex);
                let tail = if layout.tail > 0 {
                    format!(", accretive tail {} frame(s)", layout.tail)
                } else {
                    String::new()
                };
                stdout_line!(
                    "    layout: streamable through frame {} (head {head_hex}){tail}",
                    layout.covered
                );
            }
        }
        for o in &seg.opaque {
            stdout_line!("    opaque: {} ({})", o.frame_type, o.reason);
        }
        for d in &seg.diagnostics {
            stdout_line!(
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
    // `#` and `/` are ASCII, so the cut point is always a character boundary.
    let cut = iri.rfind('#').or_else(|| iri.rfind('/'));
    cut.and_then(|i| iri.get(..=i)).unwrap_or(iri)
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
/// Returns (message, `is_error`) pairs.
fn profile_check(seg: &Graph) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let declared: HashSet<&str> = seg.segment_profiles.iter().map(String::as_str).collect();
    let used = used_vocabs(seg);
    for &(prof, vocab) in PROFILE_VOCABS {
        let is_declared = declared.contains(prof);
        let is_used = used.contains(vocab);
        if is_used && !is_declared {
            out.push((
                format!(
                    "profile error: segment uses {vocab} vocabulary but does not declare '{prof}'"
                ),
                true,
            ));
        }
        if is_declared && !is_used {
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
        return usage_error();
    }
    let mut clean = true;
    for path in paths {
        let data = match load(path) {
            Ok(d) => d,
            Err(code) => return code,
        };
        if let Some(inspected) = info_v2::inspect(path, &data) {
            clean &= inspected;
        } else {
            print_ledger(path, &read_file_segments(&data));
        }
    }
    ExitCode::from(u8::from(!clean))
}

fn cmd_fold(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let wire = read_wire(&data, ReadCapacity::default());
    if matches!(
        wire.dispatch,
        Some(Dispatch::SecondGeneration | Dispatch::UnsupportedGeneration(_))
    ) {
        return match wire
            .whole_rdf_projection()
            .map_err(|error| error.to_string())
            .and_then(|rdf| rdf.to_nquads().map_err(|error| error.to_string()))
        {
            Ok(text) => {
                stdout_text!("{text}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                stderr_line!("gts fold: {error}");
                ExitCode::from(1)
            }
        };
    }
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    stdout_text!("{}", to_nquads(&g));
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
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    let trig = {
        #[cfg(feature = "rdf-codecs")]
        {
            match gmeow_gts::rdf_codecs::to_trig(&g) {
                Ok(text) => text,
                Err(e) => {
                    stderr_line!("gts to-trig: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        #[cfg(not(feature = "rdf-codecs"))]
        {
            to_trig(&g)
        }
    };
    stdout_text!("{trig}");
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "rdf-codecs")]
fn cmd_to_nt(paths: &[String]) -> ExitCode {
    let [path] = paths else {
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    match gmeow_gts::rdf_codecs::to_ntriples(&g) {
        Ok(text) => stdout_text!("{text}"),
        Err(e) => {
            stderr_line!("gts to-nt: {e}");
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
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    match gmeow_gts::rdf_codecs::to_turtle(&g) {
        Ok(text) => stdout_text!("{text}"),
        Err(e) => {
            stderr_line!("gts to-turtle: {e}");
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
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    match gmeow_gts::rdf_codecs::to_rdf_xml(&g) {
        Ok(text) => stdout_text!("{text}"),
        Err(e) => {
            stderr_line!("gts to-rdfxml: {e}");
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
            "-o" | "--out" => if let Some(p) = it.next() { out_path = Some(p) } else {
                stderr_line!("gts from-nq: -o requires a path\n{USAGE}");
                return ExitCode::from(2);
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        return usage_error();
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            stderr_line!("gts from-nq: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                stderr_line!("gts from-nq: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match from_nquads(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            stderr_line!("gts from-nq: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts from-nq: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-nq: cannot write stdout: {e}");
            return ExitCode::from(2);
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
            "-o" | "--out" => if let Some(p) = it.next() { out_path = Some(p) } else {
                stderr_line!("gts from-trig: -o requires a path\n{USAGE}");
                return ExitCode::from(2);
            },
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        return usage_error();
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            stderr_line!("gts from-trig: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                stderr_line!("gts from-trig: cannot read {path}: {e}");
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
                    stderr_line!("gts from-trig: {e}");
                    return ExitCode::from(1);
                }
            }
        }
        #[cfg(not(feature = "rdf-codecs"))]
        {
            match from_trig(&text) {
                Ok(bytes) => bytes,
                Err(e) => {
                    stderr_line!("gts from-trig: {e}");
                    return ExitCode::from(1);
                }
            }
        }
    };

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts from-trig: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-trig: cannot write stdout: {e}");
            return ExitCode::from(2);
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
            "-o" | "--out" => {
                let Some(p) = it.next() else {
                    stderr_line!("gts from-nt: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                };
                out_path = Some(p);
            }
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        return usage_error();
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            stderr_line!("gts from-nt: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                stderr_line!("gts from-nt: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match gmeow_gts::rdf_codecs::from_ntriples(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            stderr_line!("gts from-nt: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts from-nt: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-nt: cannot write stdout: {e}");
            return ExitCode::from(2);
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
            "-o" | "--out" => {
                let Some(p) = it.next() else {
                    stderr_line!("gts from-turtle: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                };
                out_path = Some(p);
            }
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        return usage_error();
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            stderr_line!("gts from-turtle: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                stderr_line!("gts from-turtle: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match gmeow_gts::rdf_codecs::from_turtle(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            stderr_line!("gts from-turtle: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts from-turtle: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-turtle: cannot write stdout: {e}");
            return ExitCode::from(2);
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
            "-o" | "--out" => {
                let Some(p) = it.next() else {
                    stderr_line!("gts from-rdfxml: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                };
                out_path = Some(p);
            }
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        return usage_error();
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            stderr_line!("gts from-rdfxml: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                stderr_line!("gts from-rdfxml: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match gmeow_gts::rdf_codecs::from_rdf_xml(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            stderr_line!("gts from-rdfxml: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts from-rdfxml: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-rdfxml: cannot write stdout: {e}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(feature = "yaml-ld")]
fn cmd_to_yaml_ld(args: &[String]) -> ExitCode {
    let [path] = args else {
        return usage_error();
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match to_yaml_ld(&graph) {
        Ok(text) => {
            stdout_text!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts to-yaml-ld: {e}");
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
            "-o" | "--out" => {
                let Some(p) = it.next() else {
                    stderr_line!("gts from-yaml-ld: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                };
                out_path = Some(p);
            }
            other => inputs.push(other),
        }
    }
    let [path] = inputs[..] else {
        return usage_error();
    };

    let text = if path == "-" {
        let mut input = String::new();
        let mut stdin = std::io::stdin();
        if let Err(e) = std::io::Read::read_to_string(&mut stdin, &mut input) {
            stderr_line!("gts from-yaml-ld: cannot read stdin: {e}");
            return ExitCode::from(2);
        }
        input
    } else {
        match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                stderr_line!("gts from-yaml-ld: cannot read {path}: {e}");
                return ExitCode::from(2);
            }
        }
    };

    let bytes = match from_yaml_ld(&text) {
        Ok(bytes) => bytes,
        Err(e) => {
            stderr_line!("gts from-yaml-ld: {e}");
            return ExitCode::from(1);
        }
    };

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts from-yaml-ld: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-yaml-ld: cannot write stdout: {e}");
            return ExitCode::from(2);
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
            "--directory" => {
                let Some(path) = it.next() else {
                    stderr_line!("gts to-okf: --directory requires a path\n{USAGE}");
                    return ExitCode::from(2);
                };
                directory = Some(path);
            }
            "--inline-body" => options.inline_body = true,
            "--base-iri" => {
                let Some(iri) = it.next() else {
                    stderr_line!("gts to-okf: --base-iri requires an IRI\n{USAGE}");
                    return ExitCode::from(2);
                };
                options.base_iri.clone_from(iri);
            }
            other if input.is_none() => input = Some(other),
            _ => {
                return usage_error();
            }
        }
    }
    let (Some(input), Some(directory)) = (input, directory) else {
        return usage_error();
    };
    let graph = match export_graph(input) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match to_okf(&graph, std::path::Path::new(directory), &options) {
        Ok(report) => {
            if report.unmapped_triples > 0 {
                stderr_line!(
                    "gts to-okf: wrote {} unmapped triple(s) to _unmapped.nq",
                    report.unmapped_triples
                );
            }
            stderr_line!(
                "gts to-okf: wrote {} document(s) to {}",
                report.documents,
                report.directory.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts to-okf: {e}");
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
            "-o" | "--out" => {
                let Some(path) = it.next() else {
                    stderr_line!("gts from-okf: -o requires a path\n{USAGE}");
                    return ExitCode::from(2);
                };
                out_path = Some(path);
            }
            "--inline-body" => options.inline_body = true,
            "--strict-links" => options.strict_links = true,
            "--base-iri" => {
                let Some(iri) = it.next() else {
                    stderr_line!("gts from-okf: --base-iri requires an IRI\n{USAGE}");
                    return ExitCode::from(2);
                };
                options.base_iri.clone_from(iri);
            }
            other if input.is_none() => input = Some(other),
            _ => {
                return usage_error();
            }
        }
    }
    let Some(input) = input else {
        return usage_error();
    };
    let bytes = match from_okf_with_options(std::path::Path::new(input), &options) {
        Ok(bytes) => bytes,
        Err(e) => {
            stderr_line!("gts from-okf: {e}");
            return ExitCode::from(1);
        }
    };
    if let Some(path) = out_path {
        if let Err(e) = std::fs::write(path, &bytes) {
            stderr_line!("gts from-okf: cannot write {path}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts from-okf: cannot write stdout: {e}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn export_graph(path: &str) -> Result<Graph, ExitCode> {
    let data = load(path)?;
    let graph = read(&data, true, None);
    for d in &graph.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    if !graph.diagnostics.is_empty() || graph.segment_heads.is_empty() {
        stderr_line!("gts: refusing export: input did not read cleanly");
        return Err(ExitCode::from(1));
    }
    Ok(graph)
}

fn cmd_to_sqlite(args: &[String]) -> ExitCode {
    let [path, out] = args else {
        return usage_error();
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match gmeow_gts::db::to_sqlite(&graph, out) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            stderr_line!("gts to-sqlite: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(feature = "duckdb")]
fn cmd_to_duckdb(args: &[String]) -> ExitCode {
    let [path, out] = args else {
        return usage_error();
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match gmeow_gts::db::to_duckdb(&graph, out) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            stderr_line!("gts to-duckdb: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(feature = "duckdb")]
fn cmd_to_parquet(args: &[String]) -> ExitCode {
    let [path, out_dir] = args else {
        return usage_error();
    };
    let graph = match export_graph(path) {
        Ok(graph) => graph,
        Err(code) => return code,
    };
    match gmeow_gts::db::to_parquet(&graph, out_dir) {
        Ok(paths) => {
            for path in paths {
                stdout_line!("{}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts to-parquet: {e}");
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
        *byte = u8::from_str_radix(hexpub.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    VerifyingKey::from_bytes(&raw)
        .ok()
        .map(|k| (kid.to_string(), k))
}

#[cfg(feature = "policy-config")]
fn load_policy(path: &str) -> Result<TrustPolicy, ExitCode> {
    TrustPolicy::from_path(path).map_err(|e| {
        stderr_line!("gts verify: {e}");
        ExitCode::from(2)
    })
}

#[cfg(not(feature = "policy-config"))]
fn load_policy(_path: &str) -> Result<TrustPolicy, ExitCode> {
    stderr_line!("gts verify: --policy requires rebuilding gmeow-gts with `--features policy-config`");
    Err(ExitCode::from(2))
}

/// The parsed arguments of `verify`.
struct VerifyArgs {
    paths: Vec<String>,
    keys: std::collections::HashMap<String, VerifyingKey>,
    policy: Option<TrustPolicy>,
}

fn parse_verify_args(args: &[String]) -> Result<VerifyArgs, ExitCode> {
    let mut parsed = VerifyArgs {
        paths: Vec::new(),
        keys: std::collections::HashMap::new(),
        policy: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--key" => {
                i += 1;
                let Some(spec) = args.get(i) else {
                    return Err(usage_error());
                };
                if let Some((kid, key)) = parse_key(spec) {
                    parsed.keys.insert(kid, key);
                } else {
                    stderr_line!("gts verify: bad --key {spec:?} (want kid:hexpubkey)");
                    return Err(ExitCode::from(2));
                }
            }
            "--policy" => {
                i += 1;
                let Some(path) = args.get(i) else {
                    return Err(usage_error());
                };
                parsed.policy = Some(load_policy(path)?);
            }
            other => parsed.paths.push(other.to_string()),
        }
        i += 1;
    }
    if parsed.paths.is_empty() {
        return Err(usage_error());
    }
    Ok(parsed)
}

fn cmd_verify(args: &[String]) -> ExitCode {
    let parsed = match parse_verify_args(args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let mut problems = false;
    for path in &parsed.paths {
        let data = match load(path) {
            Ok(d) => d,
            Err(code) => return code,
        };
        let mut fs = read_file_segments(&data);
        print_ledger(path, &fs);
        if has_problems(&fs) {
            problems = true;
        }
        verify_segments(&mut fs, parsed.policy.as_ref(), &parsed.keys, &mut problems);
        if !parsed.keys.is_empty() {
            verify_cose_signatures(&data, &parsed.keys, &mut problems);
        }
    }
    if problems {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Checks each segment of a file against the trust policy, or against the
/// built-in profile rules when no policy is given (§14.1).
fn verify_segments(
    fs: &mut FileSegments,
    policy: Option<&TrustPolicy>,
    keys: &std::collections::HashMap<String, VerifyingKey>,
    problems: &mut bool,
) {
    for (idx, seg) in fs.segments.iter_mut().enumerate() {
        if let Some(policy) = policy {
            if !keys.is_empty() {
                verify_signatures(&mut seg.signatures, |k| keys.get(k).copied());
            }
            for finding in evaluate_profile_policy(seg, Some(policy), Some(idx)) {
                stderr_line!(
                    "  segment {idx}: {}: {}: {}",
                    finding.severity.as_str(),
                    finding.code,
                    finding.detail
                );
                if finding.severity == Severity::Error {
                    *problems = true;
                }
            }
        } else {
            for (msg, is_err) in profile_check(seg) {
                let prefix = if is_err { "error" } else { "warning" };
                stderr_line!("  segment {idx}: {prefix}: {msg}");
                if is_err {
                    *problems = true;
                }
            }
            for msg in stream_vocab_check(seg) {
                stderr_line!("  segment {idx}: warning: {msg}");
            }
        }
    }
}

/// Verifies the COSE signatures of a file against the provided keys and
/// prints one line per signature (§9.2).
fn verify_cose_signatures(
    data: &[u8],
    keys: &std::collections::HashMap<String, VerifyingKey>,
    problems: &mut bool,
) {
    let mut g = read(data, true, None);
    verify_signatures(&mut g.signatures, |k| keys.get(k).copied());
    for sig in &g.signatures {
        let note = sig
            .cose
            .as_deref()
            .and_then(unsupported_algorithm)
            .map_or_default(|alg| format!(" (unsupported algorithm {alg})"));
        stdout_line!(
            "  signature {}: {}{note}",
            sig.kid.as_deref().unwrap_or("?"),
            sig.status
        );
        if sig.status == "invalid" {
            *problems = true;
        }
    }
}

/// `extract-key`: print the embedded transport (verification) key (§9.2).
fn cmd_extract_key(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    let Some(transport) = extract_transport_key(&g) else {
        stderr_line!("{path}: no embedded transport key");
        return ExitCode::from(1);
    };
    let kid = transport.kid;
    let gpg = transport.gpg;
    stdout_line!("kid:         {kid}");
    match gmeow_gts::openpgp::parse_transport_key(&gpg) {
        Ok(key) => {
            stdout_line!("fingerprint: {}", format_fingerprint(&key.fingerprint));
            stdout_line!("emojihash:   {}", emojihash(&key.raw_public, 11));
        }
        // A malformed embedded key still prints the kid + armored block below.
        Err(_) => stdout_line!("fingerprint: {}", format_fingerprint(&kid)),
    }
    stdout_line!("{gpg}");
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
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    for (digest, entry) in &g.blobs {
        let mt = blob_mt(&g, digest).unwrap_or_else(|| "-".to_string());
        let size = match entry.decoded_len() {
            Ok(size) => size,
            Err(err) => {
                stderr_line!("gts: cannot decode blob {digest}: {err:?}");
                return ExitCode::from(1);
            }
        };
        stdout_line!("{digest}  {size:>10}  {mt}");
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
            "-o" | "--out" => if let Some(p) = it.next() { out_path = Some(p) } else {
                stderr_line!("gts: -o requires a path\n{USAGE}");
                return ExitCode::from(2);
            },
            "--mt" => if let Some(m) = it.next() { mt = Some(m) } else {
                stderr_line!("gts: --mt requires a media type\n{USAGE}");
                return ExitCode::from(2);
            },
            "--include-suppressed" => include_suppressed = true,
            other => positional.push(other),
        }
    }
    let [path, digest] = positional[..] else {
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let mut g = read(&data, true, None);
    let digest = normalize_digest(digest);
    if g.blob_entry(&digest).is_none() {
        stderr_line!("gts: no inline blob {digest} in {path}");
        return ExitCode::from(1);
    }
    if !include_suppressed && suppressed_blob_digests(&g).contains(&digest) {
        stderr_line!(
            "gts: refusing {digest}: suppressed (§11); pass \
             --include-suppressed to extract anyway"
        );
        return ExitCode::from(1);
    }
    if let Some(asserted) = mt {
        let declared = blob_mt(&g, &digest);
        if declared.as_deref() != Some(asserted) {
            stderr_line!(
                "gts: refusing {digest}: declared media type {declared:?} \
                 does not match asserted {asserted:?}"
            );
            return ExitCode::from(1);
        }
    }
    let bytes = match g.blob_bytes_cloned(&digest) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            stderr_line!("gts: no inline blob {digest} in {path}");
            return ExitCode::from(1);
        }
        Err(err) => {
            stderr_line!("gts: cannot decode blob {digest}: {err:?}");
            return ExitCode::from(1);
        }
    };
    if digest_str(&bytes) != digest {
        stderr_line!("gts: integrity failure: {digest} bytes re-hash differently");
        return ExitCode::from(1);
    }
    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &bytes) {
            stderr_line!("gts: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&bytes) {
            stderr_line!("gts: cannot write stdout: {e}");
            return ExitCode::from(2);
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
            if let Some(p) = it.next() { out_path = Some(p) } else {
                stderr_line!("gts: -o requires a path\n{USAGE}");
                return ExitCode::from(2);
            }
        } else {
            inputs.push(a);
        }
    }
    if inputs.len() < 2 {
        stderr_line!("gts: cat needs at least two inputs\n{USAGE}");
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
            stderr_line!("gts: refusing {path}: not a clean GTS input");
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
                stderr_line!(
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
        stderr_line!(
            "gts: refusing composition: suppressions hide every quad in the \
             folded output"
        );
        return ExitCode::from(1);
    }

    if let Some(p) = out_path {
        if let Err(e) = std::fs::write(p, &combined) {
            stderr_line!("gts: cannot write {p}: {e}");
            return ExitCode::from(2);
        }
    } else {
        use std::io::Write;
        if let Err(e) = std::io::stdout().write_all(&combined) {
            stderr_line!("gts: cannot write stdout: {e}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ` — the default `--timestamp`.
fn now_utc_iso() -> Result<String, time::Error> {
    let fmt = time::format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]T[hour]:[minute]:[second]Z",
    )?;
    Ok(time::OffsetDateTime::now_utc().format(&fmt)?)
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
            "-o" | "--out" => if let Some(p) = it.next() { out_path = Some(p) } else {
                stderr_line!("gts: -o requires a path\n{USAGE}");
                return ExitCode::from(2);
            },
            "--streamable" => streamable = true,
            "--seal-original" => seal_original = true,
            "--timestamp" => if let Some(t) = it.next() { timestamp = Some(t) } else {
                stderr_line!("gts: --timestamp requires a value\n{USAGE}");
                return ExitCode::from(2);
            },
            other => positional.push(other),
        }
    }
    let [path] = positional[..] else {
        return usage_error();
    };
    let Some(out_path) = out_path else {
        stderr_line!("gts: compact requires -o\n{USAGE}");
        return ExitCode::from(2);
    };
    if !streamable {
        // The verb is reserved for layout rewrites; a future --snapshot mode
        // (§10) would land here. Without a mode the request is ambiguous.
        stderr_line!("gts: compact requires --streamable");
        return ExitCode::from(2);
    }
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    // The timestamp defaults to now — pass a fixed value for reproducible
    // output (§14.1 determinism).
    let ts = match timestamp {
        Some(ts) => ts.to_string(),
        None => match now_utc_iso() {
            Ok(ts) => ts,
            Err(e) => {
                stderr_line!("gts compact: cannot read the current time: {e}");
                return ExitCode::from(2);
            }
        },
    };
    match gmeow_gts::compact::compact_streamable(&data, &ts, seal_original) {
        Ok(bytes) => {
            if let Err(e) = std::fs::write(out_path, &bytes) {
                stderr_line!("gts: cannot write {out_path}: {e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            stderr_line!("gts: refusing compact: {e}");
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
            "-o" | "--out" => if let Some(p) = it.next() { out_path = Some(p) } else {
                stderr_line!("gts: -o requires a path\n{USAGE}");
                return ExitCode::from(2);
            },
            other => sources.push(other),
        }
    }
    if sources.is_empty() {
        return usage_error();
    }
    let Some(out_path) = out_path else {
        stderr_line!("gts: pack requires -o\n{USAGE}");
        return ExitCode::from(2);
    };

    let paths: Vec<&std::path::Path> = sources.iter().map(std::path::Path::new).collect();
    match gmeow_gts::files::pack(&paths) {
        Ok(data) => {
            if let Err(e) = std::fs::write(out_path, &data) {
                stderr_line!("gts: cannot write {out_path}: {e}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(msg) => {
            stderr_line!("gts: refusing pack: {msg}");
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
            "-C" => if let Some(d) = it.next() { dest = Some(d) } else {
                stderr_line!("gts: -C requires a directory\n{USAGE}");
                return ExitCode::from(2);
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
        return usage_error();
    };
    let data = match load(path) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        stderr_line!("gts: refusing unpack: archive did not read cleanly");
        return ExitCode::from(1);
    }
    let dest_path = std::path::Path::new(dest.unwrap_or("."));
    match gmeow_gts::files::unpack_with_options(&g, dest_path, &options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            stderr_line!("gts: refusing unpack: {msg}");
            ExitCode::from(1)
        }
    }
}

/// Compare an archive to a directory by content digest (tar's `d`).
fn cmd_diff(args: &[String]) -> ExitCode {
    let [archive, directory] = args else {
        return usage_error();
    };
    let data = match load(archive) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let g = read(&data, true, None);
    for d in &g.diagnostics {
        stderr_line!("gts: diagnostic {}: {}", d.code, d.detail);
    }
    if !g.diagnostics.is_empty() || g.segment_heads.is_empty() {
        stderr_line!("gts: refusing diff: archive did not read cleanly");
        return ExitCode::from(1);
    }
    match gmeow_gts::files::diff(&g, std::path::Path::new(directory)) {
        Ok(lines) => {
            let has_changes = !lines.is_empty();
            for line in &lines {
                stdout_line!("{line}");
            }
            if has_changes {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(msg) => {
            stderr_line!("gts: refusing diff: {msg}");
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
            "--directory" => if let Some(path) = it.next() { directory = Some(path) } else {
                stderr_line!("gts dump: --directory requires a path\n{USAGE}");
                return ExitCode::from(2);
            },
            "--include-suppressed" => include_suppressed = true,
            "--force" => force = true,
            "--metadata-only" => metadata_only = true,
            other => positional.push(other),
        }
    }
    let [archive] = positional[..] else {
        return usage_error();
    };
    let Some(directory) = directory else {
        stderr_line!("gts dump: --directory is required\n{USAGE}");
        return ExitCode::from(2);
    };
    let options = gmeow_gts::dumpdir::DumpOptions {
        include_suppressed,
        force,
        metadata_only,
    };
    match gmeow_gts::dumpdir::dump_path(archive, directory, options) {
        Ok(report) => {
            stderr_line!(
                "gts dump: wrote {} ({} materialized file(s), {} blob payload(s))",
                report.directory.display(),
                report.materialized_files,
                report.materialized_blobs
            );
            if report.clean {
                ExitCode::SUCCESS
            } else {
                stderr_line!(
                    "gts dump: archive had diagnostics or dump warnings ({} warning(s))",
                    report.warnings
                );
                ExitCode::from(1)
            }
        }
        Err(err) => {
            stderr_line!("gts dump: {err}");
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

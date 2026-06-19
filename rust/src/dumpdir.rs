// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Directory dump export for human and tool inspection.
//!
//! The dump layout intentionally duplicates views of an archive while avoiding
//! duplicate large payload bytes by default. Folded graph tables, unfolded frame
//! rows, and N-Quads projections are cheap inspection surfaces; blob bytes are
//! materialized once unless the caller explicitly asks for metadata only.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ciborium::value::Value;

use crate::model::{Graph, TermKind};
use crate::nquads::to_nquads;
use crate::reader::{read, read_file_segments};
use crate::replication::{heads_json, inventory, segments_json, Inventory, SegmentInventory};
use crate::wire::{digest_str, hex, iter_items, map_get, unwrap_header};

/// Options for writing a directory dump.
#[derive(Clone, Debug, Default)]
pub struct DumpOptions {
    /// Materialize blobs hidden by suppression directives.
    pub include_suppressed: bool,
    /// Replace an existing destination path.
    pub force: bool,
    /// Write indexes, graph exports, and frame tables without payload bytes.
    pub metadata_only: bool,
}

/// Summary returned after a dump completes.
#[derive(Clone, Debug)]
pub struct DumpReport {
    /// Final destination directory.
    pub directory: PathBuf,
    /// True when the input read and inventory were clean.
    pub clean: bool,
    /// Number of inline blob payloads written by this command.
    pub materialized_blobs: usize,
    /// Number of files-profile entries materialized under `files/tree`.
    pub materialized_files: usize,
    /// Number of best-effort warnings captured in the dump.
    pub warnings: usize,
}

/// Failure category for CLI exit-code mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DumpErrorKind {
    /// Input was readable, but requested content could not be safely exported.
    Refused,
    /// Usage, filesystem, or other operational failure.
    Io,
}

/// Displayable dump failure.
#[derive(Debug)]
pub struct DumpError {
    kind: DumpErrorKind,
    detail: String,
}

impl DumpError {
    fn io(detail: impl Into<String>) -> Self {
        Self {
            kind: DumpErrorKind::Io,
            detail: detail.into(),
        }
    }

    fn refused(detail: impl Into<String>) -> Self {
        Self {
            kind: DumpErrorKind::Refused,
            detail: detail.into(),
        }
    }

    /// Return the failure category.
    pub fn kind(&self) -> DumpErrorKind {
        self.kind
    }
}

impl fmt::Display for DumpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for DumpError {}

impl From<io::Error> for DumpError {
    fn from(value: io::Error) -> Self {
        Self::io(value.to_string())
    }
}

struct DumpState {
    source_label: String,
    source_size: usize,
    source_digest: String,
    graph: Graph,
    inventory: Inventory,
    options: DumpOptions,
    materialized_paths: BTreeMap<String, Vec<String>>,
    materialized_blobs: usize,
    materialized_files: usize,
    warnings: Vec<String>,
}

/// Dump a GTS file into a directory.
pub fn dump_path(
    archive: impl AsRef<Path>,
    directory: impl AsRef<Path>,
    options: DumpOptions,
) -> Result<DumpReport, DumpError> {
    let archive = archive.as_ref();
    let data = fs::read(archive)
        .map_err(|e| DumpError::io(format!("cannot read {}: {e}", archive.display())))?;
    dump_bytes(
        &data,
        archive.to_string_lossy().as_ref(),
        directory.as_ref(),
        options,
    )
}

/// Dump GTS bytes into a directory.
pub fn dump_bytes(
    data: &[u8],
    source_label: &str,
    directory: &Path,
    options: DumpOptions,
) -> Result<DumpReport, DumpError> {
    if directory.exists() && !options.force {
        return Err(DumpError::io(format!(
            "destination {} already exists; pass --force to replace it",
            directory.display()
        )));
    }

    let staged = staged_path(directory);
    let _ = fs::remove_dir_all(&staged);
    fs::create_dir_all(&staged).map_err(|e| {
        DumpError::io(format!(
            "cannot create staging directory {}: {e}",
            staged.display()
        ))
    })?;

    let result = write_dump(data, source_label, &staged, options.clone());
    if let Err(err) = result {
        let _ = fs::remove_dir_all(&staged);
        return Err(err);
    }
    let mut report = result.expect("handled Err above");

    if options.force && directory.exists() {
        if directory.is_dir() {
            fs::remove_dir_all(directory).map_err(|e| {
                let _ = fs::remove_dir_all(&staged);
                DumpError::io(format!("cannot replace {}: {e}", directory.display()))
            })?;
        } else {
            fs::remove_file(directory).map_err(|e| {
                let _ = fs::remove_dir_all(&staged);
                DumpError::io(format!("cannot replace {}: {e}", directory.display()))
            })?;
        }
    }
    fs::rename(&staged, directory).map_err(|e| {
        let _ = fs::remove_dir_all(&staged);
        DumpError::io(format!(
            "cannot move dump into {}: {e}",
            directory.display()
        ))
    })?;
    report.directory = directory.to_path_buf();
    Ok(report)
}

fn write_dump(
    data: &[u8],
    source_label: &str,
    root: &Path,
    options: DumpOptions,
) -> Result<DumpReport, DumpError> {
    let inventory = inventory(data);
    let graph = read(data, true, None);
    let mut state = DumpState {
        source_label: source_label.to_string(),
        source_size: data.len(),
        source_digest: digest_str(data),
        graph,
        inventory,
        options,
        materialized_paths: BTreeMap::new(),
        materialized_blobs: 0,
        materialized_files: 0,
        warnings: Vec::new(),
    };

    write_control(root, &state)?;
    write_graph(root, &state.graph)?;
    write_frames(root, data, &state)?;
    write_files_profile(root, &mut state)?;
    write_payloads(root, &mut state)?;
    write_blob_index(root, &state)?;
    write_readmes(root, &state)?;
    write_manifest(root, &state)?;

    Ok(DumpReport {
        directory: root.to_path_buf(),
        clean: dump_is_clean(&state),
        materialized_blobs: state.materialized_blobs,
        materialized_files: state.materialized_files,
        warnings: state.warnings.len(),
    })
}

fn dump_is_clean(state: &DumpState) -> bool {
    !state.inventory.has_problems()
        && state.graph.diagnostics.is_empty()
        && !state.graph.segment_heads.is_empty()
        && state.warnings.is_empty()
}

fn staged_path(directory: &Path) -> PathBuf {
    let parent = directory
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = directory
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "dump".into());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(
        ".{name}.gts-dump.{}.{}.tmp",
        std::process::id(),
        nanos
    ))
}

fn write_control(root: &Path, state: &DumpState) -> Result<(), DumpError> {
    let control = root.join(".gts-dump");
    fs::create_dir_all(&control)?;
    fs::write(control.join("heads.json"), heads_json(&state.inventory))?;
    fs::write(
        control.join("segments.json"),
        segments_json(&state.inventory),
    )?;
    Ok(())
}

fn write_graph(root: &Path, graph: &Graph) -> Result<(), DumpError> {
    let graph_dir = root.join("graph");
    let tables = graph_dir.join("tables");
    fs::create_dir_all(&tables)?;
    fs::write(graph_dir.join("folded.nq"), to_nquads(graph))?;

    write_terms(&tables.join("terms.jsonl"), graph, None)?;
    write_quads(&tables.join("quads.jsonl"), graph, None)?;
    write_reifiers(&tables.join("reifiers.jsonl"), graph, None)?;
    write_annotations(&tables.join("annotations.jsonl"), graph, None)?;
    write_meta(&tables.join("meta.jsonl"), graph, None)?;
    write_blob_meta(&tables.join("blob-meta.jsonl"), graph, None)?;
    write_suppressions(&tables.join("suppressions.jsonl"), graph, None)?;
    write_opaque(&tables.join("opaque.jsonl"), graph, None)?;
    write_signatures(&tables.join("signatures.jsonl"), graph, None)?;
    write_diagnostics(&tables.join("diagnostics.jsonl"), &graph.diagnostics, None)?;
    Ok(())
}

fn write_frames(root: &Path, data: &[u8], state: &DumpState) -> Result<(), DumpError> {
    let frames_root = root.join("frames");
    fs::create_dir_all(&frames_root)?;
    let mut inventory_rows = create_writer(&frames_root.join("inventory.jsonl"))?;
    let (items, _) = iter_items(data);
    let file_segments = read_file_segments(data);

    for segment in &state.inventory.segments {
        writeln!(inventory_rows, "{}", segment_inventory_row(segment))?;
        let segment_dir = frames_root
            .join("segments")
            .join(format!("{:04}", segment.index));
        fs::create_dir_all(&segment_dir)?;
        fs::write(
            segment_dir.join("header.json"),
            header_json(&items, segment)?,
        )?;
        if let Some(segment_graph) = file_segments.segments.get(segment.index) {
            fs::write(segment_dir.join("folded.nq"), to_nquads(segment_graph))?;
        }

        for frame in &segment.frames {
            writeln!(
                inventory_rows,
                "{}",
                frame_inventory_row(segment.index, frame)
            )?;
            let Some((_, Value::Map(frame_map))) = items.get(frame.item_index) else {
                continue;
            };
            write_frame_table_rows(&segment_dir, segment.index, frame.frame_index, frame_map)?;
            if frame_has_projectable_rdf(&frame.frame_type) {
                if let Some(nq) = frame_contribution_nquads(data, segment, frame.start, frame.end) {
                    if !nq.is_empty() {
                        fs::write(
                            segment_dir.join(format!("frame-{:04}.nq", frame.frame_index)),
                            nq,
                        )?;
                    }
                }
            }
        }
        if let Some(segment_graph) = file_segments.segments.get(segment.index) {
            let diagnostics_path = segment_dir.join("diagnostics.jsonl");
            write_diagnostics(&diagnostics_path, &segment_graph.diagnostics, None)?;
        }
    }
    Ok(())
}

fn write_files_profile(root: &Path, state: &mut DumpState) -> Result<(), DumpError> {
    let Ok(entries) = crate::files::read_file_entries(&state.graph) else {
        return Ok(());
    };
    let files_root = root.join("files");
    fs::create_dir_all(&files_root)?;
    let mut out = create_writer(&files_root.join("entries.jsonl"))?;
    let suppressed = suppressed_blob_digests(&state.graph);
    for (path, entry) in &entries {
        let digest = entry.get("digest").cloned().unwrap_or_default();
        let suppressed_entry = suppressed.contains(&digest);
        writeln!(
            out,
            "{{\"path\":{},\"digest\":{},\"size\":{},\"mode\":{},\"modified\":{},\"media_type\":{},\"suppressed\":{}}}",
            json_string(path),
            json_string(&digest),
            json_string(entry.get("size").map(String::as_str).unwrap_or("")),
            json_string(entry.get("mode").map(String::as_str).unwrap_or("")),
            json_string(entry.get("modified").map(String::as_str).unwrap_or("")),
            json_string(entry.get("mediaType").map(String::as_str).unwrap_or("")),
            suppressed_entry
        )?;
    }

    if state.options.metadata_only {
        return Ok(());
    }

    match crate::files::unpack(
        &state.graph,
        &files_root.join("tree"),
        state.options.include_suppressed,
    ) {
        Ok(()) => {
            for (path, entry) in &entries {
                let Some(digest) = entry.get("digest") else {
                    continue;
                };
                if !state.options.include_suppressed && suppressed.contains(digest) {
                    continue;
                }
                state
                    .materialized_paths
                    .entry(digest.clone())
                    .or_default()
                    .push(format!("files/tree/{path}"));
                state.materialized_files += 1;
            }
        }
        Err(msg) => {
            state
                .warnings
                .push(format!("files-profile materialization failed: {msg}"));
            fs::write(
                files_root.join("UNPACK_ERROR.txt"),
                format!("files-profile materialization failed: {msg}\n"),
            )?;
        }
    }
    Ok(())
}

fn write_payloads(root: &Path, state: &mut DumpState) -> Result<(), DumpError> {
    if state.options.metadata_only {
        return Ok(());
    }
    let suppressed = suppressed_blob_digests(&state.graph);
    let blob_root = root.join("blobs").join("by-digest").join("blake3");
    for (digest, entry) in &state.graph.blobs {
        if !state.options.include_suppressed && suppressed.contains(digest) {
            continue;
        }
        if state.materialized_paths.contains_key(digest) {
            continue;
        }
        let data = entry
            .decoded_bytes()
            .map_err(|err| DumpError::refused(format!("cannot decode blob {digest}: {err:?}")))?;
        if digest_str(data.as_ref()) != *digest {
            return Err(DumpError::refused(format!(
                "integrity failure for blob {digest}: decoded bytes re-hash differently"
            )));
        }
        fs::create_dir_all(&blob_root)?;
        let hex_digest = digest.strip_prefix("blake3:").unwrap_or(digest);
        let relpath = format!("blobs/by-digest/blake3/{hex_digest}");
        fs::write(root.join(&relpath), data.as_ref())?;
        state
            .materialized_paths
            .entry(digest.clone())
            .or_default()
            .push(relpath);
        state.materialized_blobs += 1;
    }
    Ok(())
}

fn write_blob_index(root: &Path, state: &DumpState) -> Result<(), DumpError> {
    let blob_dir = root.join("blobs");
    fs::create_dir_all(&blob_dir)?;
    let mut out = create_writer(&blob_dir.join("index.jsonl"))?;
    let suppressed = suppressed_blob_digests(&state.graph);
    for (digest, entry) in &state.graph.blobs {
        let size = entry
            .decoded_len()
            .map(|n| n.to_string())
            .unwrap_or_else(|err| json_string(&format!("decode error: {err:?}")));
        let mt = blob_meta_text(&state.graph, digest, "mt");
        let paths = state
            .materialized_paths
            .get(digest)
            .map(|items| {
                format!(
                    "[{}]",
                    items
                        .iter()
                        .map(|item| json_string(item))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .unwrap_or_else(|| "[]".to_string());
        writeln!(
            out,
            "{{\"digest\":{},\"size\":{},\"media_type\":{},\"suppressed\":{},\"materialized_paths\":{}}}",
            json_string(digest),
            size,
            json_optional_string(mt.as_deref()),
            suppressed.contains(digest),
            paths
        )?;
    }
    Ok(())
}

fn write_manifest(root: &Path, state: &DumpState) -> Result<(), DumpError> {
    let control = root.join(".gts-dump");
    let profiles = state
        .graph
        .segment_profiles
        .iter()
        .map(|profile| json_string(profile))
        .collect::<Vec<_>>()
        .join(",");
    let warnings = state
        .warnings
        .iter()
        .map(|warning| json_string(warning))
        .collect::<Vec<_>>()
        .join(",");
    let materialized = state
        .materialized_paths
        .iter()
        .map(|(digest, paths)| {
            format!(
                "{{\"digest\":{},\"paths\":[{}]}}",
                json_string(digest),
                paths
                    .iter()
                    .map(|path| json_string(path))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        control.join("manifest.json"),
        format!(
            concat!(
                "{{\n",
                "  \"schema\":\"gts-dump-v1\",\n",
                "  \"source\":{{\"path\":{},\"size\":{},\"digest\":{}}},\n",
                "  \"clean\":{},\n",
                "  \"options\":{{\"include_suppressed\":{},\"metadata_only\":{}}},\n",
                "  \"counts\":{{\"segments\":{},\"terms\":{},\"quads\":{},\"blobs\":{},\"diagnostics\":{},\"warnings\":{}}},\n",
                "  \"profiles\":[{}],\n",
                "  \"materialized\":[{}],\n",
                "  \"warnings\":[{}]\n",
                "}}\n"
            ),
            json_string(&state.source_label),
            state.source_size,
            json_string(&state.source_digest),
            dump_is_clean(state),
            state.options.include_suppressed,
            state.options.metadata_only,
            state.inventory.segments.len(),
            state.graph.terms.len(),
            state.graph.quads.len(),
            state.graph.blobs.len(),
            state.graph.diagnostics.len(),
            state.warnings.len(),
            profiles,
            materialized,
            warnings
        ),
    )?;
    Ok(())
}

fn write_readmes(root: &Path, state: &DumpState) -> Result<(), DumpError> {
    fs::write(root.join("README.md"), top_readme(state))?;
    fs::write(root.join("graph").join("README.md"), graph_readme(state))?;
    fs::write(root.join("frames").join("README.md"), frames_readme())?;
    Ok(())
}

fn top_readme(state: &DumpState) -> String {
    let status = if dump_is_clean(state) {
        "clean"
    } else {
        "diagnostics or warnings present"
    };
    let profiles = if state.graph.segment_profiles.is_empty() {
        "generic".to_string()
    } else {
        state.graph.segment_profiles.join(", ")
    };
    let mut lines = vec![
        "# GTS Dump".to_string(),
        String::new(),
        format!("- Source: `{}`", state.source_label),
        format!("- Source digest: `{}`", state.source_digest),
        format!("- Status: {status}"),
        format!("- Profiles: {profiles}"),
        format!("- Segments: {}", state.inventory.segments.len()),
        format!("- Folded quads: {}", state.graph.quads.len()),
        format!("- Inline blobs: {}", state.graph.blobs.len()),
        String::new(),
        "Start with `graph/folded.nq` for RDF tooling, `frames/inventory.jsonl` for the append log, and `blobs/index.jsonl` for payload locations.".to_string(),
        "For files-profile archives, user payloads are under `files/tree/`.".to_string(),
    ];
    if state.options.metadata_only {
        lines.push(String::new());
        lines.push(
            "This dump was created with metadata-only mode; payload bytes were not extracted."
                .to_string(),
        );
    }
    if !state.warnings.is_empty() {
        lines.push(String::new());
        lines.push("## Warnings".to_string());
        for warning in &state.warnings {
            lines.push(format!("- {warning}"));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn graph_readme(state: &DumpState) -> String {
    format!(
        "# Folded Graph\n\n`folded.nq` is the archive-level N-Quads projection. The `tables/` directory exposes the same folded state as line-oriented JSON tables for simple inspection.\n\nCounts: {} terms, {} quads, {} reifier bindings, {} annotations.\n",
        state.graph.terms.len(),
        state.graph.quads.len(),
        state.graph.reifiers.len(),
        state.graph.annotations.len()
    )
}

fn frames_readme() -> String {
    "# Unfolded Frames\n\n`inventory.jsonl` lists segment and frame byte ranges, frame ids, frame types, and validation status. Each `segments/NNNN/` directory contains that segment's folded N-Quads plus decoded frame-level JSONL rows. `frame-*.nq` files are emitted for frames with RDF contributions that can be projected as N-Quads.\n".to_string()
}

fn write_terms(path: &Path, graph: &Graph, frame_index: Option<usize>) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (id, term) in graph.terms.iter().enumerate() {
        writeln!(
            out,
            "{{{}\"id\":{},\"kind\":{},\"value\":{},\"datatype\":{},\"lang\":{},\"reifier\":{}}}",
            frame_prefix(frame_index),
            id,
            json_string(term_kind_name(term.kind)),
            json_optional_string(term.value.as_deref()),
            json_optional_usize(term.datatype),
            json_optional_string(term.lang.as_deref()),
            json_optional_usize(term.reifier)
        )?;
    }
    Ok(())
}

fn write_quads(path: &Path, graph: &Graph, frame_index: Option<usize>) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (s, p, o, g) in &graph.quads {
        writeln!(
            out,
            "{{{}\"s\":{},\"p\":{},\"o\":{},\"g\":{}}}",
            frame_prefix(frame_index),
            s,
            p,
            o,
            json_optional_usize(*g)
        )?;
    }
    Ok(())
}

fn write_reifiers(path: &Path, graph: &Graph, frame_index: Option<usize>) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (r, (s, p, o)) in &graph.reifiers {
        writeln!(
            out,
            "{{{}\"reifier\":{},\"s\":{},\"p\":{},\"o\":{}}}",
            frame_prefix(frame_index),
            r,
            s,
            p,
            o
        )?;
    }
    Ok(())
}

fn write_annotations(
    path: &Path,
    graph: &Graph,
    frame_index: Option<usize>,
) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (r, p, v) in &graph.annotations {
        writeln!(
            out,
            "{{{}\"reifier\":{},\"predicate\":{},\"value\":{}}}",
            frame_prefix(frame_index),
            r,
            p,
            v
        )?;
    }
    Ok(())
}

fn write_meta(path: &Path, graph: &Graph, frame_index: Option<usize>) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (key, value) in &graph.meta {
        writeln!(
            out,
            "{{{}\"key\":{},\"value\":{}}}",
            frame_prefix(frame_index),
            json_string(key),
            cbor_json(value)
        )?;
    }
    Ok(())
}

fn write_blob_meta(
    path: &Path,
    graph: &Graph,
    frame_index: Option<usize>,
) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (digest, value) in &graph.blob_meta {
        writeln!(
            out,
            "{{{}\"digest\":{},\"meta\":{}}}",
            frame_prefix(frame_index),
            json_string(digest),
            cbor_json(value)
        )?;
    }
    Ok(())
}

fn write_suppressions(
    path: &Path,
    graph: &Graph,
    frame_index: Option<usize>,
) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (index, suppression) in graph.suppressions.iter().enumerate() {
        writeln!(
            out,
            "{{{}\"index\":{},\"reason\":{},\"by\":{},\"targets\":[{}]}}",
            frame_prefix(frame_index),
            index,
            json_optional_string(suppression.reason.as_deref()),
            json_optional_usize(suppression.by),
            suppression
                .targets
                .iter()
                .map(cbor_json)
                .collect::<Vec<_>>()
                .join(",")
        )?;
    }
    Ok(())
}

fn write_opaque(path: &Path, graph: &Graph, frame_index: Option<usize>) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (index, opaque) in graph.opaque.iter().enumerate() {
        writeln!(
            out,
            "{{{}\"index\":{},\"id\":{},\"frame_type\":{},\"reason\":{},\"sigstat\":{},\"pub\":{},\"recipients\":{}}}",
            frame_prefix(frame_index),
            index,
            json_string(&hex(&opaque.id)),
            json_string(&opaque.frame_type),
            json_string(&opaque.reason),
            json_string(&opaque.sigstat),
            opaque.pub_meta.as_ref().map(cbor_json).unwrap_or_else(|| "null".to_string()),
            opaque.recipients.as_ref().map(|items| format!("[{}]", items.iter().map(cbor_json).collect::<Vec<_>>().join(","))).unwrap_or_else(|| "null".to_string())
        )?;
    }
    Ok(())
}

fn write_signatures(
    path: &Path,
    graph: &Graph,
    frame_index: Option<usize>,
) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for (index, signature) in graph.signatures.iter().enumerate() {
        writeln!(
            out,
            "{{{}\"index\":{},\"frame_id\":{},\"kid\":{},\"status\":{},\"has_cose\":{}}}",
            frame_prefix(frame_index),
            index,
            json_string(&hex(&signature.frame_id)),
            json_optional_string(signature.kid.as_deref()),
            json_string(&signature.status),
            signature.cose.is_some()
        )?;
    }
    Ok(())
}

fn write_diagnostics(
    path: &Path,
    diagnostics: &[crate::model::Diagnostic],
    frame_index: Option<usize>,
) -> Result<(), DumpError> {
    let mut out = create_writer(path)?;
    for diagnostic in diagnostics {
        writeln!(
            out,
            "{{{}\"code\":{},\"detail\":{},\"frame_index\":{}}}",
            frame_prefix(frame_index),
            json_string(&diagnostic.code),
            json_string(&diagnostic.detail),
            json_optional_usize(diagnostic.frame_index)
        )?;
    }
    Ok(())
}

fn write_frame_table_rows(
    segment_dir: &Path,
    segment_index: usize,
    frame_index: usize,
    frame: &[(Value, Value)],
) -> Result<(), DumpError> {
    let ftype = map_get(frame, "t")
        .and_then(value_text)
        .unwrap_or("<unknown>");
    let payload = map_get(frame, "d").unwrap_or(&Value::Null);
    match ftype {
        "terms" => append_payload_rows(
            segment_dir,
            "terms.jsonl",
            segment_index,
            frame_index,
            payload,
        )?,
        "quads" => append_payload_rows(
            segment_dir,
            "quads.jsonl",
            segment_index,
            frame_index,
            payload,
        )?,
        "reifies" => append_payload_rows(
            segment_dir,
            "reifiers.jsonl",
            segment_index,
            frame_index,
            payload,
        )?,
        "annot" => append_payload_rows(
            segment_dir,
            "annotations.jsonl",
            segment_index,
            frame_index,
            payload,
        )?,
        "meta" => append_payload_rows(
            segment_dir,
            "meta.jsonl",
            segment_index,
            frame_index,
            payload,
        )?,
        "suppress" => append_payload_rows(
            segment_dir,
            "suppressions.jsonl",
            segment_index,
            frame_index,
            payload,
        )?,
        "blob" => append_blob_frame_row(segment_dir, segment_index, frame_index, frame)?,
        _ => {}
    }
    Ok(())
}

fn append_payload_rows(
    segment_dir: &Path,
    file_name: &str,
    segment_index: usize,
    frame_index: usize,
    payload: &Value,
) -> Result<(), DumpError> {
    let mut out = append_jsonl(&segment_dir.join(file_name))?;
    match payload {
        Value::Array(items) => {
            for (row, value) in items.iter().enumerate() {
                writeln!(
                    out,
                    "{{\"segment\":{},\"frame\":{},\"row\":{},\"value\":{}}}",
                    segment_index,
                    frame_index,
                    row,
                    cbor_json(value)
                )?;
            }
        }
        Value::Map(entries) => {
            for (row, (key, value)) in entries.iter().enumerate() {
                writeln!(
                    out,
                    "{{\"segment\":{},\"frame\":{},\"row\":{},\"key\":{},\"value\":{}}}",
                    segment_index,
                    frame_index,
                    row,
                    cbor_json(key),
                    cbor_json(value)
                )?;
            }
        }
        Value::Null => {}
        other => {
            writeln!(
                out,
                "{{\"segment\":{},\"frame\":{},\"row\":0,\"value\":{}}}",
                segment_index,
                frame_index,
                cbor_json(other)
            )?;
        }
    }
    Ok(())
}

fn append_blob_frame_row(
    segment_dir: &Path,
    segment_index: usize,
    frame_index: usize,
    frame: &[(Value, Value)],
) -> Result<(), DumpError> {
    let mut out = append_jsonl(&segment_dir.join("blobs.jsonl"))?;
    let digest = map_get(frame, "pub")
        .and_then(blob_digest_from_meta)
        .or_else(|| match map_get(frame, "d") {
            Some(Value::Bytes(bytes)) if !has_transform(frame) => Some(digest_str(bytes)),
            _ => None,
        });
    let size = match map_get(frame, "d") {
        Some(Value::Bytes(bytes)) if !has_transform(frame) => bytes.len().to_string(),
        Some(Value::Bytes(bytes)) => format!("encoded:{}", bytes.len()),
        _ => "null".to_string(),
    };
    writeln!(
        out,
        "{{\"segment\":{},\"frame\":{},\"digest\":{},\"size\":{},\"pub\":{}}}",
        segment_index,
        frame_index,
        json_optional_string(digest.as_deref()),
        if size == "null" {
            size
        } else {
            json_string(&size)
        },
        map_get(frame, "pub")
            .map(cbor_json)
            .unwrap_or_else(|| "null".to_string())
    )?;
    Ok(())
}

fn create_writer(path: &Path) -> Result<BufWriter<File>, DumpError> {
    Ok(BufWriter::new(File::create(path)?))
}

fn append_jsonl(path: &Path) -> Result<BufWriter<File>, DumpError> {
    Ok(BufWriter::new(
        OpenOptions::new().create(true).append(true).open(path)?,
    ))
}

fn frame_has_projectable_rdf(frame_type: &str) -> bool {
    matches!(frame_type, "quads" | "reifies" | "annot" | "snapshot")
}

fn frame_contribution_nquads(
    data: &[u8],
    segment: &SegmentInventory,
    frame_start: usize,
    frame_end: usize,
) -> Option<String> {
    if frame_end <= segment.start || frame_start < segment.start || frame_end > data.len() {
        return None;
    }
    let previous = read(&data[segment.start..frame_start], true, None);
    let current = read(&data[segment.start..frame_end], true, None);
    let contribution = Graph {
        terms: current.terms.clone(),
        quads: current
            .quads
            .get(previous.quads.len()..)
            .unwrap_or_default()
            .to_vec(),
        reifiers: current
            .reifiers
            .get(previous.reifiers.len()..)
            .unwrap_or_default()
            .to_vec(),
        annotations: current
            .annotations
            .get(previous.annotations.len()..)
            .unwrap_or_default()
            .to_vec(),
        ..Graph::default()
    };
    if contribution.quads.is_empty()
        && contribution.reifiers.is_empty()
        && contribution.annotations.is_empty()
    {
        return None;
    }
    Some(to_nquads(&contribution))
}

fn segment_inventory_row(segment: &SegmentInventory) -> String {
    format!(
        "{{\"kind\":\"segment\",\"segment\":{},\"item_start\":{},\"item_end\":{},\"start\":{},\"end\":{},\"length\":{},\"profile\":{},\"head\":{},\"frame_count\":{},\"diagnostics\":{}}}",
        segment.index,
        segment.item_start,
        segment.item_end,
        segment.start,
        segment.end,
        segment.end.saturating_sub(segment.start),
        json_string(&segment.profile),
        segment
            .head
            .as_deref()
            .map(|head| json_string(&hex(head)))
            .unwrap_or_else(|| "null".to_string()),
        segment.frame_count,
        segment.diagnostics.len()
    )
}

fn frame_inventory_row(segment_index: usize, frame: &crate::replication::FrameInventory) -> String {
    format!(
        "{{\"kind\":\"frame\",\"segment\":{},\"item_index\":{},\"frame\":{},\"start\":{},\"end\":{},\"length\":{},\"id\":{},\"type\":{},\"valid\":{}}}",
        segment_index,
        frame.item_index,
        frame.frame_index,
        frame.start,
        frame.end,
        frame.end.saturating_sub(frame.start),
        json_string(&hex(&frame.id)),
        json_string(&frame.frame_type),
        frame.valid
    )
}

fn header_json(items: &[(usize, Value)], segment: &SegmentInventory) -> Result<String, DumpError> {
    let Some((_, item)) = items.get(segment.item_start) else {
        return Ok("{}\n".to_string());
    };
    let header = unwrap_header(item).map_err(DumpError::refused)?;
    Ok(format!(
        "{{\"segment\":{},\"profile\":{},\"start\":{},\"end\":{},\"header\":{}}}\n",
        segment.index,
        json_string(&segment.profile),
        segment.start,
        segment
            .frames
            .first()
            .map(|frame| frame.start)
            .unwrap_or(segment.end),
        cbor_json(&Value::Map(header.clone()))
    ))
}

fn suppressed_blob_digests(graph: &Graph) -> HashSet<String> {
    let mut out = HashSet::new();
    for sup in &graph.suppressions {
        for target in &sup.targets {
            let Value::Map(entries) = target else {
                continue;
            };
            let mut kind = "";
            let mut digest = None;
            for (k, v) in entries {
                if matches!(k, Value::Text(text) if text == "kind") {
                    if let Value::Text(text) = v {
                        kind = text.as_str();
                    }
                } else if matches!(k, Value::Text(text) if text == "digest") {
                    digest = match v {
                        Value::Text(text) if text.starts_with("blake3:") => Some(text.clone()),
                        Value::Text(text) => Some(format!("blake3:{text}")),
                        Value::Bytes(bytes) => Some(format!("blake3:{}", hex(bytes))),
                        _ => None,
                    };
                }
            }
            if kind == "blob" {
                if let Some(digest) = digest {
                    out.insert(digest);
                }
            }
        }
    }
    out
}

fn has_transform(frame: &[(Value, Value)]) -> bool {
    matches!(map_get(frame, "x"), Some(Value::Array(ids)) if !ids.is_empty())
}

fn blob_digest_from_meta(value: &Value) -> Option<String> {
    let Value::Map(entries) = value else {
        return None;
    };
    match map_get(entries, "digest") {
        Some(Value::Text(text)) if text.starts_with("blake3:") => Some(text.clone()),
        Some(Value::Text(text)) => Some(format!("blake3:{text}")),
        Some(Value::Bytes(bytes)) => Some(format!("blake3:{}", hex(bytes))),
        _ => None,
    }
}

fn blob_meta_text(graph: &Graph, digest: &str, key: &str) -> Option<String> {
    graph
        .blob_meta
        .iter()
        .find(|(stored, _)| stored == digest)
        .and_then(|(_, meta)| {
            let Value::Map(entries) = meta else {
                return None;
            };
            map_get(entries, key)
                .and_then(value_text)
                .map(str::to_string)
        })
}

fn value_text(value: &Value) -> Option<&str> {
    if let Value::Text(text) = value {
        Some(text)
    } else {
        None
    }
}

fn term_kind_name(kind: TermKind) -> &'static str {
    match kind {
        TermKind::Iri => "iri",
        TermKind::Literal => "literal",
        TermKind::Bnode => "bnode",
        TermKind::Triple => "triple",
    }
}

fn frame_prefix(frame_index: Option<usize>) -> String {
    frame_index
        .map(|index| format!("\"frame\":{index},"))
        .unwrap_or_default()
}

fn json_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn cbor_json(value: &Value) -> String {
    match value {
        Value::Integer(i) => i128::from(*i).to_string(),
        Value::Bytes(bytes) => format!("{{\"bytes\":{}}}", json_string(&hex(bytes))),
        Value::Float(f) if f.is_finite() => f.to_string(),
        Value::Float(f) => json_string(&f.to_string()),
        Value::Text(text) => json_string(text),
        Value::Bool(value) => value.to_string(),
        Value::Null => "null".to_string(),
        Value::Tag(tag, inner) => {
            format!("{{\"tag\":{},\"value\":{}}}", tag, cbor_json(inner))
        }
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(cbor_json).collect::<Vec<_>>().join(",")
        ),
        Value::Map(entries) => format!(
            "[{}]",
            entries
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{{\"key\":{},\"value\":{}}}",
                        cbor_json(key),
                        cbor_json(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => json_string(&format!("{value:?}")),
    }
}

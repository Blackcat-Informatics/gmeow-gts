// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::process::ExitCode;

use gmeow_gts::files::{read_entries, FileEntryKind, UnpackOptions};
use gmeow_gts::from_tar::{from_tar_to_writer, FromTarOptions};
use gmeow_gts::model::Graph;
use gmeow_gts::reader::read;
use gmeow_gts::tar::{to_tar, TarCompression, ToTarOptions};

use super::{export_graph, USAGE};

pub(super) fn cmd_from_tar(args: &[String]) -> ExitCode {
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

pub(super) fn cmd_to_tar(args: &[String]) -> ExitCode {
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

    write_tar_output("to-tar", out_path.unwrap_or("-"), &graph, &options)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TarOperation {
    Create,
    Extract,
    List,
    Diff,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArchiveKind {
    Gts,
    Tar,
}

pub(super) fn cmd_tar(args: &[String]) -> ExitCode {
    let parsed = match parse_tar_cli_args(args) {
        Ok(parsed) => parsed,
        Err(msg) => {
            eprintln!("gts tar: {msg}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match parsed.operation {
        Some(TarOperation::Create) => cmd_tar_create(&parsed),
        Some(TarOperation::Extract) => cmd_tar_extract(&parsed),
        Some(TarOperation::List) => cmd_tar_list(&parsed),
        Some(TarOperation::Diff) => cmd_tar_diff(&parsed),
        None => {
            eprintln!("gts tar: choose one of -c, -x, -t, or -d\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

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

fn cmd_tar_create(parsed: &TarCliArgs) -> ExitCode {
    let Some(archive) = parsed.archive.as_deref() else {
        eprintln!("gts tar: -f requires an archive path\n{USAGE}");
        return ExitCode::from(2);
    };
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

fn cmd_tar_extract(parsed: &TarCliArgs) -> ExitCode {
    let Some(archive) = parsed.archive.as_deref() else {
        eprintln!("gts tar: -f requires an archive path\n{USAGE}");
        return ExitCode::from(2);
    };
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

fn resolve_tar_sources(sources: &[String], directory: Option<&str>) -> Vec<std::path::PathBuf> {
    sources
        .iter()
        .map(|source| path_under_directory(source, directory))
        .collect()
}

fn tar_diff_directory(parsed: &TarCliArgs) -> Result<std::path::PathBuf, String> {
    match (parsed.sources.as_slice(), parsed.directory.as_deref()) {
        ([source], directory) => Ok(path_under_directory(source, directory)),
        ([], Some(directory)) => Ok(std::path::PathBuf::from(directory)),
        ([], None) => Err("diff requires a directory or -C".to_string()),
        _ => Err("diff accepts one directory".to_string()),
    }
}

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

fn read_archive_graph_for_tar(parsed: &TarCliArgs, read_only: bool) -> Result<Graph, ExitCode> {
    let Some(archive) = parsed.archive.as_deref() else {
        eprintln!("gts tar: -f requires an archive path\n{USAGE}");
        return Err(ExitCode::from(2));
    };
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

fn archive_kind(path: &str) -> ArchiveKind {
    if path.to_ascii_lowercase().ends_with(".gts") {
        ArchiveKind::Gts
    } else {
        ArchiveKind::Tar
    }
}

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

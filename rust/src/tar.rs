// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tar stream export for files-profile-v2 GTS archives.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Write};

use crate::codec::encode_chain;
use crate::files::{read_entries, FileEntry, FileEntryKind};
use crate::model::{BlobEntry, Graph};

/// Error raised by tar import/export helpers.
#[derive(Debug)]
pub struct TarError {
    detail: String,
}

impl TarError {
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for TarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for TarError {}

/// Compression to apply while writing a tar stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TarCompression {
    #[default]
    None,
    Gzip,
    Zstd,
}

/// Options for [`to_tar`].
#[derive(Clone, Debug, Default)]
pub struct ToTarOptions {
    pub compression: TarCompression,
    pub numeric_owner: bool,
}

/// Write a deterministic tar stream from a folded files-profile graph.
pub fn to_tar<W: Write>(
    graph: &Graph,
    mut writer: W,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    match options.compression {
        TarCompression::None => write_tar_stream(graph, writer, options),
        TarCompression::Gzip => {
            let mut encoder = flate2::GzBuilder::new()
                .mtime(0)
                .write(writer, flate2::Compression::default());
            write_tar_stream(graph, &mut encoder, options)?;
            encoder
                .finish()
                .map_err(|err| TarError::new(format!("gzip encode tar stream: {err}")))?;
            Ok(())
        }
        TarCompression::Zstd => {
            let raw = to_tar_vec(graph, options)?;
            let encoded = encode_chain(&["zstd".to_string()], &raw)
                .map_err(|err| TarError::new(format!("zstd encode tar stream: {err}")))?;
            writer
                .write_all(&encoded)
                .map_err(|err| TarError::new(format!("write tar stream: {err}")))
        }
    }
}

/// Build an uncompressed tar stream from a folded files-profile graph.
pub fn to_tar_vec(graph: &Graph, options: &ToTarOptions) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    write_tar_stream(graph, &mut out, options)?;
    Ok(out)
}

fn write_tar_stream<W: Write>(
    graph: &Graph,
    writer: W,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    let entries = read_entries(graph).map_err(TarError::new)?;
    let blobs: BTreeMap<&str, &BlobEntry> = graph
        .blobs
        .iter()
        .map(|(digest, entry)| (digest.as_str(), entry))
        .collect();
    let mut builder = ::tar::Builder::new(writer);
    for entry in entries.values() {
        append_entry(&mut builder, entry, &blobs, options)?;
    }
    builder
        .finish()
        .map_err(|err| TarError::new(format!("finish tar stream: {err}")))
}

fn append_entry<W: Write>(
    builder: &mut ::tar::Builder<W>,
    entry: &FileEntry,
    blobs: &BTreeMap<&str, &BlobEntry>,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    append_pax_records(builder, entry)?;
    match entry.kind {
        FileEntryKind::File => append_file(builder, entry, blobs, options),
        FileEntryKind::Directory => {
            append_metadata_entry(builder, entry, ::tar::EntryType::Directory, None, options)
        }
        FileEntryKind::Symlink => append_metadata_entry(
            builder,
            entry,
            ::tar::EntryType::Symlink,
            Some(required_link_target(entry, "symlink")?),
            options,
        ),
        FileEntryKind::Hardlink => append_metadata_entry(
            builder,
            entry,
            ::tar::EntryType::Link,
            Some(required_link_target(entry, "hardlink")?),
            options,
        ),
        FileEntryKind::Fifo => {
            append_metadata_entry(builder, entry, ::tar::EntryType::Fifo, None, options)
        }
        FileEntryKind::CharDev => {
            append_device_entry(builder, entry, ::tar::EntryType::Char, options)
        }
        FileEntryKind::BlockDev => {
            append_device_entry(builder, entry, ::tar::EntryType::Block, options)
        }
        FileEntryKind::Socket => Err(TarError::new(format!(
            "tar cannot encode socket entry {}",
            entry.path
        ))),
    }
}

fn append_file<W: Write>(
    builder: &mut ::tar::Builder<W>,
    entry: &FileEntry,
    blobs: &BTreeMap<&str, &BlobEntry>,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    let digest = entry
        .digest
        .as_deref()
        .ok_or_else(|| TarError::new(format!("file entry {} has no digest", entry.path)))?;
    let data = blobs
        .get(digest)
        .ok_or_else(|| TarError::new(format!("missing inline blob for {}: {digest}", entry.path)))?
        .decoded_vec()
        .map_err(|err| TarError::new(format!("decode blob for {}: {err}", entry.path)))?;
    let mut header = base_header(entry, options)?;
    header.set_entry_type(::tar::EntryType::Regular);
    header.set_size(data.len() as u64);
    builder
        .append_data(&mut header, &entry.path, io::Cursor::new(data))
        .map_err(|err| TarError::new(format!("append {}: {err}", entry.path)))
}

fn append_metadata_entry<W: Write>(
    builder: &mut ::tar::Builder<W>,
    entry: &FileEntry,
    entry_type: ::tar::EntryType,
    link_target: Option<&str>,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    let mut header = base_header(entry, options)?;
    header.set_entry_type(entry_type);
    header.set_size(0);
    if let Some(link_target) = link_target {
        builder
            .append_link(&mut header, &entry.path, link_target)
            .map_err(|err| TarError::new(format!("append {}: {err}", entry.path)))?;
        return Ok(());
    }
    builder
        .append_data(&mut header, &entry.path, io::empty())
        .map_err(|err| TarError::new(format!("append {}: {err}", entry.path)))
}

fn required_link_target<'a>(entry: &'a FileEntry, kind: &str) -> Result<&'a str, TarError> {
    entry
        .link_target
        .as_deref()
        .filter(|target| !target.is_empty())
        .ok_or_else(|| TarError::new(format!("{kind} entry {} has no link target", entry.path)))
}

fn append_device_entry<W: Write>(
    builder: &mut ::tar::Builder<W>,
    entry: &FileEntry,
    entry_type: ::tar::EntryType,
    options: &ToTarOptions,
) -> Result<(), TarError> {
    let mut header = base_header(entry, options)?;
    header.set_entry_type(entry_type);
    header.set_size(0);
    header
        .set_device_major(
            entry
                .dev_major
                .ok_or_else(|| TarError::new(format!("{} missing devMajor", entry.path)))?
                .try_into()
                .map_err(|_| TarError::new(format!("{} devMajor exceeds u32 range", entry.path)))?,
        )
        .map_err(|err| TarError::new(format!("set dev major for {}: {err}", entry.path)))?;
    header
        .set_device_minor(
            entry
                .dev_minor
                .ok_or_else(|| TarError::new(format!("{} missing devMinor", entry.path)))?
                .try_into()
                .map_err(|_| TarError::new(format!("{} devMinor exceeds u32 range", entry.path)))?,
        )
        .map_err(|err| TarError::new(format!("set dev minor for {}: {err}", entry.path)))?;
    builder
        .append_data(&mut header, &entry.path, io::empty())
        .map_err(|err| TarError::new(format!("append {}: {err}", entry.path)))
}

fn base_header(entry: &FileEntry, options: &ToTarOptions) -> Result<::tar::Header, TarError> {
    let mut header = ::tar::Header::new_gnu();
    header.set_mode(entry.mode.unwrap_or(default_mode(entry.kind)));
    header.set_mtime(parse_mtime(entry.modified.as_deref())?);
    header.set_uid(entry.uid.unwrap_or(0));
    header.set_gid(entry.gid.unwrap_or(0));
    if !options.numeric_owner {
        if let Some(user_name) = &entry.user_name {
            header
                .set_username(user_name)
                .map_err(|err| TarError::new(format!("set user for {}: {err}", entry.path)))?;
        }
        if let Some(group_name) = &entry.group_name {
            header
                .set_groupname(group_name)
                .map_err(|err| TarError::new(format!("set group for {}: {err}", entry.path)))?;
        }
    }
    Ok(header)
}

fn append_pax_records<W: Write>(
    builder: &mut ::tar::Builder<W>,
    entry: &FileEntry,
) -> Result<(), TarError> {
    const CORE_PAX_KEYS: &[&str] = &[
        "path", "linkpath", "size", "uid", "gid", "uname", "gname", "mtime",
    ];
    let records: Vec<(&str, &[u8])> = entry
        .pax_records
        .iter()
        .filter(|record| !record.key.is_empty() && !CORE_PAX_KEYS.contains(&record.key.as_str()))
        .map(|record| (record.key.as_str(), record.value.as_bytes()))
        .collect();
    builder
        .append_pax_extensions(records)
        .map_err(|err| TarError::new(format!("append pax records for {}: {err}", entry.path)))
}

fn default_mode(kind: FileEntryKind) -> u32 {
    match kind {
        FileEntryKind::Directory => 0o755,
        _ => 0o644,
    }
}

fn parse_mtime(value: Option<&str>) -> Result<u64, TarError> {
    let Some(value) = value else {
        return Ok(0);
    };
    let dt = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .or_else(|_| {
            let text = value.strip_suffix('Z').unwrap_or(value);
            time::OffsetDateTime::parse(
                &(text.to_string() + "+00:00"),
                &time::format_description::well_known::Rfc3339,
            )
        })
        .map_err(|err| TarError::new(format!("parse mtime {value}: {err}")))?;
    let timestamp = dt.unix_timestamp();
    if timestamp < 0 {
        return Err(TarError::new(format!("negative tar mtime: {value}")));
    }
    Ok(timestamp as u64)
}

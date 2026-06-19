// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tar stream import into files-profile-v2 GTS archives.

use std::io::{Cursor, Read};
use std::path::Path;

use crate::codec::{decode_chain, Codec};
use crate::files::{pack_entries_v2, FileEntry, FileEntryKind, FilePaxRecord};
use crate::tar::TarError;

/// Options for [`from_tar`].
#[derive(Clone, Debug, Default)]
pub struct FromTarOptions {
    pub allow_symlinks: bool,
    pub allow_special: bool,
    pub owner: bool,
    /// Optional source label used for compression detection by extension.
    pub source_name: Option<String>,
}

/// Read a tar stream, optionally decompress it, and author a files-profile-v2 GTS archive.
pub fn from_tar<R: Read>(mut reader: R, options: &FromTarOptions) -> Result<Vec<u8>, TarError> {
    let mut data = Vec::new();
    reader
        .read_to_end(&mut data)
        .map_err(|err| TarError::new(format!("read tar input: {err}")))?;
    from_tar_bytes(&data, options)
}

/// Author a files-profile-v2 GTS archive from bytes containing tar, tar.gz, or tar.zst.
pub fn from_tar_bytes(data: &[u8], options: &FromTarOptions) -> Result<Vec<u8>, TarError> {
    let decoded = decode_tar_input(data, options.source_name.as_deref())?;
    let entries = read_tar_entries(Cursor::new(decoded), options)?;
    pack_entries_v2(&entries).map_err(TarError::new)
}

fn decode_tar_input(data: &[u8], source_name: Option<&str>) -> Result<Vec<u8>, TarError> {
    match detect_compression(data, source_name) {
        None => Ok(data.to_vec()),
        Some(name) => decode_chain(
            &[Codec {
                name: name.to_string(),
                cls: "compress".to_string(),
            }],
            data,
        )
        .map_err(|err| TarError::new(format!("{name} decode tar input: {err}"))),
    }
}

fn detect_compression(data: &[u8], source_name: Option<&str>) -> Option<&'static str> {
    if data.starts_with(&[0x1f, 0x8b]) {
        return Some("gzip");
    }
    if data.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Some("zstd");
    }
    let name = source_name?.to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".gz") {
        Some("gzip")
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") || name.ends_with(".zst") {
        Some("zstd")
    } else {
        None
    }
}

fn read_tar_entries<R: Read>(
    reader: R,
    options: &FromTarOptions,
) -> Result<Vec<FileEntry>, TarError> {
    let mut archive = ::tar::Archive::new(reader);
    let mut entries = Vec::new();
    for entry in archive
        .entries()
        .map_err(|err| TarError::new(format!("read tar entries: {err}")))?
    {
        let mut entry = entry.map_err(|err| TarError::new(format!("read tar entry: {err}")))?;
        entries.push(read_tar_entry(&mut entry, options)?);
    }
    if entries.is_empty() {
        return Err(TarError::new("tar archive contains no entries"));
    }
    Ok(entries)
}

fn read_tar_entry<R: Read>(
    entry: &mut ::tar::Entry<'_, R>,
    options: &FromTarOptions,
) -> Result<FileEntry, TarError> {
    let path = archive_path_from_path(
        entry
            .path()
            .map_err(|err| TarError::new(format!("read tar path: {err}")))?,
    )?;
    let header = entry.header().clone();
    let entry_type = header.entry_type();
    let kind = entry_kind(entry_type, &path, options)?;
    let mut out = FileEntry {
        path: path.clone(),
        kind,
        mode: header.mode().ok(),
        modified: header.mtime().ok().map(format_mtime).transpose()?,
        ..FileEntry::default()
    };
    out.pax_records = read_pax_records(entry, &path)?;

    if options.owner {
        out.uid = header.uid().ok();
        out.gid = header.gid().ok();
        out.user_name = header
            .username()
            .map_err(|err| TarError::new(format!("read username for {path}: {err}")))?
            .map(str::to_string);
        out.group_name = header
            .groupname()
            .map_err(|err| TarError::new(format!("read group name for {path}: {err}")))?
            .map(str::to_string);
    }

    match kind {
        FileEntryKind::File => {
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .map_err(|err| TarError::new(format!("read file data for {path}: {err}")))?;
            out.size = Some(data.len() as u64);
            out.data = Some(data);
        }
        FileEntryKind::Symlink | FileEntryKind::Hardlink => {
            let link = entry
                .link_name()
                .map_err(|err| TarError::new(format!("read link target for {path}: {err}")))?
                .ok_or_else(|| TarError::new(format!("link entry {path} has no link target")))?;
            out.link_target = Some(link_target_to_string(&link)?);
        }
        FileEntryKind::CharDev | FileEntryKind::BlockDev => {
            out.dev_major = header
                .device_major()
                .map_err(|err| TarError::new(format!("read dev major for {path}: {err}")))?
                .map(u64::from);
            out.dev_minor = header
                .device_minor()
                .map_err(|err| TarError::new(format!("read dev minor for {path}: {err}")))?
                .map(u64::from);
        }
        FileEntryKind::Directory | FileEntryKind::Fifo | FileEntryKind::Socket => {}
    }
    Ok(out)
}

fn read_pax_records<R: Read>(
    entry: &mut ::tar::Entry<'_, R>,
    path: &str,
) -> Result<Vec<FilePaxRecord>, TarError> {
    let mut out = Vec::new();
    let Some(extensions) = entry
        .pax_extensions()
        .map_err(|err| TarError::new(format!("read pax records for {path}: {err}")))?
    else {
        return Ok(out);
    };
    for extension in extensions {
        let extension =
            extension.map_err(|err| TarError::new(format!("read pax record for {path}: {err}")))?;
        let key = extension
            .key()
            .map_err(|err| TarError::new(format!("read pax key for {path}: {err}")))?;
        let value = extension
            .value()
            .map_err(|err| TarError::new(format!("read pax value for {path}: {err}")))?;
        out.push(FilePaxRecord {
            key: key.to_string(),
            value: value.to_string(),
        });
    }
    Ok(out)
}

fn entry_kind(
    entry_type: ::tar::EntryType,
    path: &str,
    options: &FromTarOptions,
) -> Result<FileEntryKind, TarError> {
    if entry_type.is_file() {
        Ok(FileEntryKind::File)
    } else if entry_type.is_dir() {
        Ok(FileEntryKind::Directory)
    } else if entry_type.is_symlink() {
        if !options.allow_symlinks {
            return Err(TarError::new(format!(
                "refusing symlink entry {path}: use --allow-symlinks"
            )));
        }
        Ok(FileEntryKind::Symlink)
    } else if entry_type.is_hard_link() {
        if !options.allow_symlinks {
            return Err(TarError::new(format!(
                "refusing hardlink entry {path}: use --allow-symlinks"
            )));
        }
        Ok(FileEntryKind::Hardlink)
    } else if entry_type.is_fifo() {
        if !options.allow_special {
            return Err(TarError::new(format!(
                "refusing fifo entry {path}: use --allow-special"
            )));
        }
        Ok(FileEntryKind::Fifo)
    } else if entry_type.is_character_special() {
        if !options.allow_special {
            return Err(TarError::new(format!(
                "refusing character device entry {path}: use --allow-special"
            )));
        }
        Ok(FileEntryKind::CharDev)
    } else if entry_type.is_block_special() {
        if !options.allow_special {
            return Err(TarError::new(format!(
                "refusing block device entry {path}: use --allow-special"
            )));
        }
        Ok(FileEntryKind::BlockDev)
    } else {
        Err(TarError::new(format!(
            "unsupported tar entry type {:?} for {path}",
            entry_type.as_byte()
        )))
    }
}

fn archive_path_from_path(path: impl AsRef<Path>) -> Result<String, TarError> {
    let raw = path.as_ref().to_string_lossy().replace('\\', "/");
    let stripped = raw
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    if stripped.is_empty() {
        return Err(TarError::new("empty tar path"));
    }
    Ok(stripped)
}

fn link_target_to_string(path: &Path) -> Result<String, TarError> {
    let value = path.to_string_lossy().replace('\\', "/");
    if value.is_empty() {
        return Err(TarError::new("empty tar link target"));
    }
    Ok(value)
}

fn format_mtime(seconds: u64) -> Result<String, TarError> {
    let dt = time::OffsetDateTime::from_unix_timestamp(seconds as i64)
        .map_err(|err| TarError::new(format!("invalid tar mtime {seconds}: {err}")))?;
    dt.format(&time::format_description::well_known::Rfc3339)
        .map(|text| text.replace("+00:00", "Z"))
        .map_err(|err| TarError::new(format!("format tar mtime {seconds}: {err}")))
}

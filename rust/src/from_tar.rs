// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tar stream import into files-profile-v2 GTS archives.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use crate::files::{
    pack_entries_v2_with_blob_paths, FileBlobSource, FileEntry, FileEntryKind, FilePaxRecord,
};
use crate::tar::TarError;
use crate::wire::hex;

const STREAM_CHUNK_SIZE: usize = 128 * 1024;

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
pub fn from_tar<R: Read>(reader: R, options: &FromTarOptions) -> Result<Vec<u8>, TarError> {
    let mut out = Vec::new();
    from_tar_to_writer(reader, &mut out, options)?;
    Ok(out)
}

/// Read a tar stream and author files-profile-v2 GTS bytes to `writer`.
///
/// Regular-file payloads are spooled to temporary files while tar metadata is
/// collected, then emitted as GTS blob frames in bounded chunks. This preserves
/// deterministic metadata ordering without retaining whole file payloads or
/// decompressed tar streams in memory.
pub fn from_tar_to_writer<R: Read, W: Write>(
    reader: R,
    writer: W,
    options: &FromTarOptions,
) -> Result<(), TarError> {
    let mut reader = BufReader::new(reader);
    let compression = detect_reader_compression(&mut reader, options.source_name.as_deref())?;
    let (entries, temp_blobs) = match compression {
        None => read_tar_entries_spooled(reader, options)?,
        Some("gzip") => read_tar_entries_spooled(flate2::read::GzDecoder::new(reader), options)?,
        Some("zstd") => {
            let decoder = structured_zstd::decoding::StreamingDecoder::new(reader)
                .map_err(|err| TarError::new(format!("zstd decode tar input: {err}")))?;
            read_tar_entries_spooled(decoder, options)?
        }
        Some(other) => {
            return Err(TarError::new(format!(
                "unsupported tar compression: {other}"
            )))
        }
    };
    let blob_sources: BTreeMap<String, FileBlobSource> = temp_blobs
        .iter()
        .map(|(digest, blob)| {
            (
                digest.clone(),
                FileBlobSource {
                    path: blob.path.clone(),
                    size: blob.size,
                    media_type: blob.media_type.clone(),
                    representation: None,
                },
            )
        })
        .collect();
    pack_entries_v2_with_blob_paths(&entries, &blob_sources, writer).map_err(TarError::new)
}

/// Author a files-profile-v2 GTS archive from bytes containing tar, tar.gz, or tar.zst.
pub fn from_tar_bytes(data: &[u8], options: &FromTarOptions) -> Result<Vec<u8>, TarError> {
    from_tar(Cursor::new(data), options)
}

fn detect_reader_compression<R: BufRead>(
    reader: &mut R,
    source_name: Option<&str>,
) -> Result<Option<&'static str>, TarError> {
    let prefix = reader
        .fill_buf()
        .map_err(|err| TarError::new(format!("peek tar input: {err}")))?;
    Ok(detect_compression(prefix, source_name))
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

struct TempBlob {
    path: PathBuf,
    size: u64,
    media_type: Option<String>,
}

impl Drop for TempBlob {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn read_tar_entries_spooled<R: Read>(
    reader: R,
    options: &FromTarOptions,
) -> Result<(Vec<FileEntry>, BTreeMap<String, TempBlob>), TarError> {
    let mut archive = ::tar::Archive::new(reader);
    let mut entries = Vec::new();
    let mut blobs: BTreeMap<String, TempBlob> = BTreeMap::new();
    for entry in archive
        .entries()
        .map_err(|err| TarError::new(format!("read tar entries: {err}")))?
    {
        let mut entry = entry.map_err(|err| TarError::new(format!("read tar entry: {err}")))?;
        let (file_entry, blob) = read_tar_entry_spooled(&mut entry, options)?;
        if let Some((digest, blob)) = blob {
            blobs.entry(digest).or_insert(blob);
        }
        entries.push(file_entry);
    }
    if entries.is_empty() {
        return Err(TarError::new("tar archive contains no entries"));
    }
    Ok((entries, blobs))
}

fn read_tar_entry_spooled<R: Read>(
    entry: &mut ::tar::Entry<'_, R>,
    options: &FromTarOptions,
) -> Result<(FileEntry, Option<(String, TempBlob)>), TarError> {
    let mut out = read_tar_entry_metadata(entry, options)?;
    if out.kind != FileEntryKind::File {
        return Ok((out, None));
    }
    let path = out.path.clone();
    let (digest, size, blob) = spool_file_entry(entry, &path)?;
    out.digest = Some(digest.clone());
    out.size = Some(size);
    Ok((out, Some((digest, blob))))
}

fn read_tar_entry_metadata<R: Read>(
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
        FileEntryKind::File => {}
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

fn create_temp_blob_file() -> Result<(PathBuf, fs::File), std::io::Error> {
    let temp_dir = std::env::temp_dir();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for attempt in 0..1000_u32 {
        let path = temp_dir.join(format!(
            "gmeow-gts-tar-{}-{now}-{attempt}.blob",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not create unique temporary blob path",
    ))
}

fn spool_file_entry<R: Read>(
    entry: &mut ::tar::Entry<'_, R>,
    path: &str,
) -> Result<(String, u64, TempBlob), TarError> {
    let (temp_path, mut temp) = create_temp_blob_file()
        .map_err(|err| TarError::new(format!("create temporary blob for {path}: {err}")))?;
    let mut hasher = blake3::Hasher::new();
    let mut size = 0_u64;
    let mut buf = [0_u8; STREAM_CHUNK_SIZE];
    loop {
        let n = entry
            .read(&mut buf)
            .map_err(|err| TarError::new(format!("read file data for {path}: {err}")))?;
        if n == 0 {
            break;
        }
        temp.write_all(&buf[..n])
            .map_err(|err| TarError::new(format!("spool file data for {path}: {err}")))?;
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    temp.flush()
        .map_err(|err| TarError::new(format!("flush temporary blob for {path}: {err}")))?;
    let digest = format!("blake3:{}", hex(hasher.finalize().as_bytes()));
    Ok((
        digest,
        size,
        TempBlob {
            path: temp_path,
            size,
            media_type: None,
        },
    ))
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

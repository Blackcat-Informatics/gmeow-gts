// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Write the value and blob regions in P4 member and pack order.

mod chunker;

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, mem,
};

use ciborium::value::Value;

use super::{Digest, SegmentWriteError, SegmentWriter};
use crate::{
    v2::{
        attachments::{unit_digest, PACK},
        BLAKE3_256,
    },
    wire,
};

const EXTERNAL: u64 = 12;
const PACK_BYTES: usize = 1 << 24;

/// One entry of the folded blob set, including optional verified bytes.
///
/// An empty slice represents the present empty blob. An absent slice represents
/// a declaration whose bytes are external.
#[derive(Clone, Copy, Debug)]
pub struct Blob<'a> {
    pub digest: Digest,
    pub length: u64,
    pub bytes: Option<&'a [u8]>,
    pub media_types: &'a [&'a str],
}

/// An attachment input or segment append that cannot be represented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttachmentWriteError {
    EmptyValue,
    BlobLength { expected: u64, actual: u64 },
    BlobDigest,
    DuplicateBlob,
    Append(SegmentWriteError),
}

impl fmt::Display for AttachmentWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue => formatter.write_str("value octets must be nonempty"),
            Self::BlobLength { expected, actual } => {
                write!(
                    formatter,
                    "blob declares {expected} bytes but holds {actual}"
                )
            }
            Self::BlobDigest => {
                formatter.write_str("blob bytes do not match their declared digest")
            }
            Self::DuplicateBlob => formatter.write_str("blob set repeats a digest"),
            Self::Append(error) => error.fmt(formatter),
        }
    }
}

impl Error for AttachmentWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::Append(error) = self {
            Some(error)
        } else {
            None
        }
    }
}

impl From<SegmentWriteError> for AttachmentWriteError {
    fn from(error: SegmentWriteError) -> Self {
        Self::Append(error)
    }
}

impl SegmentWriter {
    /// Append the P4 value packs, blob packs, and external declarations.
    ///
    /// Supply the distinct blob entries of the fold and the octets used by
    /// its value references. The writer deduplicates equal values and units,
    /// orders the regions and uses plain frame storage. The caller places
    /// these regions before the terms and row frames of a canonical segment.
    pub fn append_attachment_regions<'a>(
        &mut self,
        values: impl IntoIterator<Item = &'a [u8]>,
        blobs: impl IntoIterator<Item = Blob<'a>>,
    ) -> Result<(), AttachmentWriteError> {
        let mut sorted_values = BTreeMap::new();
        for bytes in values {
            if bytes.is_empty() {
                return Err(AttachmentWriteError::EmptyValue);
            }
            sorted_values.insert(unit_digest(bytes), bytes);
        }
        let sorted_blobs = plan_blobs(blobs)?;
        let start = self.bytes.len();
        let head = self.head;
        let range = self.range.clone();
        let result =
            write_values(self, sorted_values).and_then(|()| write_blobs(self, sorted_blobs));
        if result.is_err() {
            self.bytes.truncate(start);
            self.head = head;
            self.range = range;
        }
        result.map_err(Into::into)
    }
}

#[derive(Clone, Copy)]
struct Unit<'a> {
    digest: Digest,
    bytes: &'a [u8],
}

struct PlannedBlob<'a> {
    length: u64,
    units: Vec<Unit<'a>>,
    media_types: Vec<Option<&'a str>>,
}

fn units(bytes: &[u8]) -> Vec<Unit<'_>> {
    chunker::chunks(bytes)
        .map(|bytes| Unit {
            digest: unit_digest(bytes),
            bytes,
        })
        .collect()
}

fn plan_blobs<'a>(
    blobs: impl IntoIterator<Item = Blob<'a>>,
) -> Result<BTreeMap<Digest, PlannedBlob<'a>>, AttachmentWriteError> {
    let mut planned = BTreeMap::new();
    for blob in blobs {
        let parts = if let Some(bytes) = blob.bytes {
            if bytes.len() as u64 != blob.length {
                return Err(AttachmentWriteError::BlobLength {
                    expected: blob.length,
                    actual: bytes.len() as u64,
                });
            }
            if unit_digest(bytes) != blob.digest {
                return Err(AttachmentWriteError::BlobDigest);
            }
            if bytes.is_empty() {
                vec![Unit {
                    digest: blob.digest,
                    bytes,
                }]
            } else {
                units(bytes)
            }
        } else {
            Vec::new()
        };
        let mut media_types: Vec<_> = blob.media_types.iter().copied().map(Some).collect();
        // For present text options, CBOR compares the text head before its
        // bytes. Shorter byte lengths sort first, then the text bytes.
        media_types.sort_unstable_by_key(|&media| media.map(|text| (text.len(), text)));
        media_types.dedup();
        if media_types.is_empty() {
            media_types.push(None);
        }
        if planned
            .insert(
                blob.digest,
                PlannedBlob {
                    length: blob.length,
                    units: parts,
                    media_types,
                },
            )
            .is_some()
        {
            return Err(AttachmentWriteError::DuplicateBlob);
        }
    }
    Ok(planned)
}

fn uint(number: usize) -> Value {
    Value::from(number as u64)
}

fn digest_value(digest: &Digest) -> Value {
    Value::Bytes(digest.to_vec())
}

fn media_value(media: Option<&str>) -> Value {
    Value::Array(match media {
        Some(text) => vec![Value::from(2), Value::Text(text.to_owned())],
        None => vec![Value::from(1)],
    })
}

fn entries(units: &[Unit<'_>]) -> Vec<u8> {
    let entries = units
        .iter()
        .map(|unit| Value::Array(vec![digest_value(&unit.digest), uint(unit.bytes.len())]))
        .collect();
    wire::encode(&Value::Array(entries))
}

#[derive(Clone, Copy)]
enum MemberKind {
    Chunk = 1,
    Blob = 2,
    Recipe = 3,
    Value = 4,
}

#[derive(Default)]
struct Pack {
    directory: Vec<Value>,
    bodies: Vec<u8>,
    unit_length: usize,
}

impl Pack {
    fn push(
        &mut self,
        writer: &mut SegmentWriter,
        kind: MemberKind,
        content: Vec<Value>,
        body: &[u8],
    ) -> Result<(), SegmentWriteError> {
        if !self.directory.is_empty()
            && (self.bodies.len() > PACK_BYTES || body.len() > PACK_BYTES - self.bodies.len())
        {
            self.flush(writer)?;
        }
        if matches!(kind, MemberKind::Chunk | MemberKind::Blob) {
            self.unit_length += body.len();
        }
        self.directory.push(Value::Array(vec![
            Value::from(kind as u64),
            Value::Array(content),
            uint(body.len()),
        ]));
        self.bodies.extend_from_slice(body);
        Ok(())
    }

    fn push_unit(
        &mut self,
        writer: &mut SegmentWriter,
        unit: Unit<'_>,
        one_unit_blob: Option<&PlannedBlob<'_>>,
    ) -> Result<(), SegmentWriteError> {
        let mut content = vec![digest_value(&unit.digest)];
        let kind = if let Some(blob) = one_unit_blob {
            content.push(media_value(blob.media_types[0]));
            MemberKind::Blob
        } else {
            MemberKind::Chunk
        };
        self.push(writer, kind, content, unit.bytes)
    }

    fn flush(&mut self, writer: &mut SegmentWriter) -> Result<(), SegmentWriteError> {
        if self.directory.is_empty() {
            return Ok(());
        }
        let content = wire::encode(&Value::Array(vec![
            uint(self.directory.len()),
            uint(mem::take(&mut self.unit_length)),
        ]));
        let payload = wire::encode(&Value::Array(vec![
            Value::Array(mem::take(&mut self.directory)),
            Value::Bytes(mem::take(&mut self.bodies)),
        ]));
        writer.append(PACK, &content, &payload)?;
        Ok(())
    }
}

fn write_values(
    writer: &mut SegmentWriter,
    values: BTreeMap<Digest, &[u8]>,
) -> Result<(), SegmentWriteError> {
    let mut pack = Pack::default();
    let mut emitted = BTreeSet::new();
    for (digest, bytes) in values {
        let parts = units(bytes);
        for &part in &parts {
            if emitted.insert(part.digest) {
                pack.push_unit(writer, part, None)?;
            }
        }
        pack.push(
            writer,
            MemberKind::Value,
            vec![digest_value(&digest), uint(bytes.len()), uint(parts.len())],
            &entries(&parts),
        )?;
    }
    pack.flush(writer)
}

fn write_blobs(
    writer: &mut SegmentWriter,
    blobs: BTreeMap<Digest, PlannedBlob<'_>>,
) -> Result<(), SegmentWriteError> {
    let mut pack = Pack::default();
    let mut emitted = BTreeSet::new();
    for (digest, blob) in &blobs {
        for &part in &blob.units {
            if emitted.insert(part.digest) {
                let single = blobs
                    .get(&part.digest)
                    .filter(|candidate| candidate.units.len() == 1);
                pack.push_unit(writer, part, single)?;
            }
        }
        if blob.units.len() > 1 {
            pack.push(
                writer,
                MemberKind::Recipe,
                vec![
                    digest_value(digest),
                    Value::from(blob.length),
                    media_value(blob.media_types[0]),
                    uint(blob.units.len()),
                ],
                &entries(&blob.units),
            )?;
        }
    }
    pack.flush(writer)?;

    for (digest, blob) in blobs {
        let stored_media = usize::from(!blob.units.is_empty());
        for media in blob.media_types.into_iter().skip(stored_media) {
            let name = Value::Array(vec![Value::from(BLAKE3_256), digest_value(&digest)]);
            let content = wire::encode(&Value::Array(vec![
                name,
                Value::from(blob.length),
                media_value(media),
            ]));
            writer.append(EXTERNAL, &content, &[])?;
        }
    }
    Ok(())
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{Decoder, HeaderError, MalformedReason::{UnregisteredValue, OutOfRange, InconsistentCounts, WrongLength}};

pub(super) const UNIT_CONTEXT: &str = "gts2 unit";
pub(super) const CHUNK: u64 = 8;
pub(super) const BLOB: u64 = 9;
pub(super) const RECIPE: u64 = 10;
pub(super) const VALUE: u64 = 11;
pub(super) const PACK: u64 = 13;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct Reference {
    pub digest: [u8; 32],
    pub length: u64,
}

pub(super) fn unit_digest(bytes: &[u8]) -> [u8; 32] {
    blake3::derive_key(UNIT_CONTEXT, bytes)
}

pub(super) struct Attachment<'a> {
    pub kind: u64,
    pub reference: Reference,
    pub bytes: &'a [u8],
    pub entries: Vec<Reference>,
}

struct Fields {
    kind: u64,
    digest: [u8; 32],
    length: u64,
    entries: u64,
}

impl Fields {
    fn read(cursor: &mut Decoder<'_>, kind: u64) -> Result<Self, HeaderError> {
        let arity = match kind {
            CHUNK => 1,
            BLOB => 2,
            RECIPE => 4,
            VALUE => 3,
            _ => return Err(cursor.error(UnregisteredValue)),
        };
        cursor.record(arity)?;
        let digest = *cursor.fixed::<32>()?;
        let length = if kind >= RECIPE { cursor.uint()? } else { 0 };
        if matches!(kind, BLOB | RECIPE) {
            cursor.option(Decoder::text)?;
        }
        let entries = if kind >= RECIPE { cursor.uint()? } else { 0 };
        Ok(Self {
            kind,
            digest,
            length,
            entries,
        })
    }

    fn attach(self, bytes: &[u8], offset: usize) -> Result<Attachment<'_>, HeaderError> {
        let Self {
            kind,
            digest,
            length,
            entries,
        } = self;
        if kind == CHUNK && bytes.is_empty() {
            return Err(HeaderError {
                reason: OutOfRange,
                offset,
            });
        }
        let (length, entries) = if kind <= BLOB {
            (bytes.len() as u64, Vec::new())
        } else {
            let minimum = if kind == RECIPE { 2 } else { 1 };
            let entries =
                read_entries(bytes, entries, length, minimum).map_err(|error| HeaderError {
                    offset: offset + error.offset,
                    ..error
                })?;
            (length, entries)
        };
        Ok(Attachment {
            kind,
            reference: Reference { digest, length },
            bytes,
            entries,
        })
    }
}

fn read_entries(
    bytes: &[u8],
    expected: u64,
    length: u64,
    minimum: u64,
) -> Result<Vec<Reference>, HeaderError> {
    let mut cursor = Decoder::new(bytes, 0);
    if expected < minimum {
        return Err(cursor.error(OutOfRange));
    }
    let count = cursor.head(4)?;
    let entries = (0..count)
        .map(|_| {
            cursor.record(2)?;
            Ok(Reference {
                digest: *cursor.fixed::<32>()?,
                length: cursor.uint()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    cursor.end()?;
    if count != expected {
        return Err(cursor.error(InconsistentCounts));
    }
    let sum = entries
        .iter()
        .try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.length)
                .filter(|_| entry.length != 0)
        })
        .ok_or_else(|| cursor.error(OutOfRange))?;
    if sum != length {
        return Err(cursor.error(InconsistentCounts));
    }
    Ok(entries)
}

pub(super) fn decode_attachment<'a>(
    kind: u64,
    content: &[u8],
    bytes: &'a [u8],
) -> Result<Attachment<'a>, HeaderError> {
    let mut fields = Decoder::new(content, 0);
    let attachment = Fields::read(&mut fields, kind)?;
    fields.end()?;
    attachment.attach(bytes, 0)
}

#[expect(clippy::map_err_ignore, reason = "the conversion error carries no detail, so the typed error replaces it")]
pub(super) fn decode_pack<'a>(
    content: &[u8],
    bytes: &'a [u8],
) -> Result<Vec<Attachment<'a>>, HeaderError> {
    let mut fields = Decoder::new(content, 0);
    fields.record(2)?;
    let expected = fields.positive()?;
    let unit_length = fields.uint()?;
    fields.end()?;

    let mut cursor = Decoder::new(bytes, 0);
    cursor.record(2)?;
    let count = cursor.head(4)?;
    let directory = (0..count)
        .map(|_| {
            let error = cursor.error(WrongLength);
            let arity = cursor.head(4)?;
            if arity == 0 {
                return Err(error);
            }
            let kind = cursor.code(4)? + CHUNK - 1;
            if arity != 3 {
                return Err(error);
            }
            Ok((Fields::read(&mut cursor, kind)?, cursor.uint()?))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bodies = cursor.byte_string(None)?;
    cursor.end()?;
    let error = cursor.error(InconsistentCounts);
    if count != expected {
        return Err(error);
    }
    let mut all = 0_u64;
    let mut units = 0_u64;
    for (member, length) in &directory {
        all = all.checked_add(*length).ok_or(error)?;
        if member.kind <= BLOB {
            units = units.checked_add(*length).ok_or(error)?;
        }
    }
    if all != bodies.len() as u64 || units != unit_length {
        return Err(error);
    }
    let mut remaining = bodies;
    directory
        .into_iter()
        .map(|(member, length)| {
            let offset = bytes.len() - remaining.len();
            let length = usize::try_from(length).map_err(|_| error)?;
            let (body, rest) = remaining.split_at_checked(length).ok_or(error)?;
            remaining = rest;
            member.attach(body, offset)
        })
        .collect()
}

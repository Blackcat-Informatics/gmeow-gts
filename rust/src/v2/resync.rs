// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Walked resync openers and charged candidates (P1 §§2.4a, 6.7).

use super::{
    payload::PayloadBudget, Decoder, Header, HeaderError, MalformedReason, BLAKE3_256,
    HEADER_CONTEXT,
};

pub(super) const RESYNC_MAX: usize = 4096;
const PATTERN: [u8; 20] = [
    0x8b, 0x05, 0x02, 0x50, 0xfc, 0xf3, 0xeb, 0x32, 0xa2, 0xf0, 0x26, 0x7d, 0xd1, 0x6a, 0x53, 0xf0,
    0x9b, 0xff, 0x87, 0x92,
];

/// A resync opener and its original advisory bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedResync<'a> {
    /// The decoded header of the resync item.
    pub header: Header<'a>,
    /// The offset that the resync item claims for its segment.
    pub claimed_offset: u64,
    /// The bytes that precede the claimed offset, if any.
    pub past: Option<&'a [u8]>,
    /// The marks of the resync item.
    pub marks: &'a [u8],
    /// The identifier stored in the resync item.
    pub stored_identifier: &'a [u8],
    /// The identifier computed from the item, if it is valid.
    pub identifier: Option<[u8; 32]>,
    /// The offset of the first byte after the resync item.
    pub next_offset: usize,
}

fn decode_prefix<'a>(
    cursor: &mut Decoder<'a>,
    candidate: bool,
) -> Result<(Header<'a>, u64), HeaderError> {
    use MalformedReason::{OutOfRange, UnregisteredValue};
    cursor.record(11)?;
    let kind_error = cursor.error(UnregisteredValue);
    if cursor.uint()? != 5 {
        return Err(kind_error);
    }
    let generation_error = cursor.error(OutOfRange);
    if cursor.uint()? != 2 {
        return Err(generation_error);
    }
    let marker_error = cursor.error(OutOfRange);
    let marker = &PATTERN[4..];
    if cursor.head(2)? != marker.len() as u64 || cursor.take(marker.len())? != marker {
        return Err(marker_error);
    }
    let (registry_version, hash_algorithm, profile) = cursor.opener_fields(candidate)?;
    let lineage = cursor.option(|cursor| cursor.lineage(hash_algorithm == BLAKE3_256))?;
    let offset = cursor.uint()?;
    Ok((
        Header {
            registry_version,
            hash_algorithm,
            profile,
            canonical: None,
            lineage,
        },
        offset,
    ))
}

fn read_marks<'a>(cursor: &mut Decoder<'a>) -> Result<&'a [u8], HeaderError> {
    use MalformedReason::{OutOfRange, UnorderedOrRepeated, WrongType};
    let marks_at = cursor.offset;
    let count = cursor.head(4)?;
    let mut last: &[u8] = &[];
    for index in 0..count {
        let mark_at = cursor.offset;
        let elements = cursor.head(4)?;
        if elements == 0 {
            return Err(HeaderError {
                reason: WrongType,
                offset: mark_at,
            });
        }
        // The first element must be a uint by shape before count and order.
        if cursor.peek_byte()? >> 5 != 0 {
            return Err(HeaderError {
                reason: WrongType,
                offset: mark_at,
            });
        }
        if index >= 64 {
            return Err(HeaderError {
                reason: OutOfRange,
                offset: mark_at,
            });
        }
        // A complete item cannot be a proper prefix of another complete item.
        let available = &cursor.bytes[mark_at..];
        let prefix = &available[..available.len().min(last.len())];
        if !last.is_empty()
            && prefix <= last
            && (prefix.len() == last.len() || !last.starts_with(prefix))
        {
            return Err(HeaderError {
                reason: UnorderedOrRepeated,
                offset: mark_at,
            });
        }
        for _ in 0..elements {
            cursor.closed_item()?;
        }
        last = &cursor.bytes[mark_at..cursor.offset];
    }
    Ok(&cursor.bytes[marks_at..cursor.offset])
}

fn decode_tail<'a>(
    cursor: &mut Decoder<'a>,
    offset: usize,
    header: Header<'a>,
    claimed_offset: u64,
) -> Result<DecodedResync<'a>, HeaderError> {
    let known = header.hash_algorithm == BLAKE3_256;
    let digest_size = known.then_some(32);
    let past = cursor.option(|cursor| cursor.byte_string(digest_size))?;
    let marks = read_marks(cursor)?;
    let body = &cursor.bytes[offset + 1..cursor.offset];
    let stored_identifier = cursor.byte_string(digest_size)?;
    if let Some(limit) = cursor
        .field_start_limit
        .filter(|&limit| cursor.offset > limit)
    {
        return Err(HeaderError {
            reason: MalformedReason::OutOfRange,
            offset: limit,
        });
    }
    let identifier = known.then(|| {
        let mut hash = blake3::Hasher::new_derive_key(HEADER_CONTEXT);
        hash.update(&[0x8a]);
        hash.update(body);
        *hash.finalize().as_bytes()
    });
    Ok(DecodedResync {
        header,
        claimed_offset,
        past,
        marks,
        stored_identifier,
        identifier,
        next_offset: cursor.offset,
    })
}

pub(super) fn decode_resync(
    data: &[u8],
    offset: usize,
    end: usize,
) -> Result<DecodedResync<'_>, HeaderError> {
    let limit = offset.saturating_add(RESYNC_MAX);
    // Walked items use the complete head when its first byte is within the cap.
    let mut cursor = Decoder::new(&data[..end], offset);
    cursor.field_start_limit = Some(limit);
    decode_prefix(&mut cursor, false)
        .and_then(|(header, claimed)| decode_tail(&mut cursor, offset, header, claimed))
}

pub(super) enum Candidate<'a> {
    Accepted {
        offset: usize,
        value: DecodedResync<'a>,
    },
    Refused {
        offset: usize,
        count: u64,
    },
}

pub(super) struct ResyncScanner<'a> {
    data: &'a [u8],
    tested_until: usize,
}

impl<'a> ResyncScanner<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            tested_until: 0,
        }
    }

    pub(super) fn scan(
        &mut self,
        start: usize,
        end: usize,
        budget: &mut PayloadBudget,
    ) -> Option<Candidate<'a>> {
        if budget.total_refused() {
            return None;
        }
        let mut next = start.max(self.tested_until);
        while next < end {
            let Some(found) = self.data[next..end]
                .iter()
                .position(|&byte| byte == PATTERN[0])
            else {
                break;
            };
            let offset = next + found;
            next = offset + 1;
            if !self.data[offset..].starts_with(&PATTERN) {
                continue;
            }
            self.tested_until = next;
            let limit = offset.saturating_add(RESYNC_MAX).min(self.data.len());
            let mut cursor = Decoder::new(&self.data[..limit], offset);
            let Ok((header, claimed)) = decode_prefix(&mut cursor, true) else {
                continue;
            };
            if claimed != offset as u64 {
                continue;
            }
            if let Err(count) = budget.charge(RESYNC_MAX as u64) {
                return Some(Candidate::Refused { offset, count });
            }
            if let Ok(value) = decode_tail(&mut cursor, offset, header, claimed) {
                if value
                    .identifier
                    .as_ref()
                    .is_some_and(|id| id == value.stored_identifier)
                {
                    return Some(Candidate::Accepted { offset, value });
                }
            }
        }
        self.tested_until = self.tested_until.max(end);
        None
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Stored and decoded checks with a file-wide capacity (P1 §§5.3–5.4).

use std::{borrow::Cow, io::Read};

use structured_zstd::decoding::{BlockDecodingStrategy, FrameDecoder};

use super::{
    frame::{check_stored_chunks, payload_digest, Frame, Storage},
    framing::byte_chunks,
    Decoder, HeaderError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadCapacity {
    pub stored_bytes_per_frame: u64,
    pub decoded_bytes_per_frame: u64,
    pub decoded_bytes_per_read: u64,
    pub items_per_read: u64,
}

impl Default for ReadCapacity {
    fn default() -> Self {
        Self {
            stored_bytes_per_frame: 1 << 26,
            decoded_bytes_per_frame: 1 << 26,
            decoded_bytes_per_read: 1 << 36,
            items_per_read: 1 << 26,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadDamage {
    MalformedFields,
    CodecConstraint,
    DecodeFailed,
    DecodedLength,
    PayloadDigest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadRefusal {
    StoredSize(u64),
    DecodedSize(u64),
    TotalDecoded(u64),
    UnknownCodec,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadState<'a> {
    Verified(Cow<'a, [u8]>),
    Damaged(PayloadDamage),
    Unchecked(PayloadRefusal),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadCheck<'a> {
    pub stored_matches: Option<bool>,
    pub state: PayloadState<'a>,
}

pub(super) struct PayloadFields<'a> {
    storage: Storage<'a>,
    stored: &'a [u8],
    decoded_length: u64,
    digest: &'a [u8; 32],
    content_size: usize,
    signatures_size: usize,
}

impl<'a> From<&Frame<'a>> for PayloadFields<'a> {
    fn from(frame: &Frame<'a>) -> Self {
        Self {
            storage: frame.storage,
            stored: frame.stored_payload,
            decoded_length: frame.decoded_length,
            digest: frame.payload_digest,
            content_size: frame.content.len(),
            signatures_size: frame.encoded_signatures_size,
        }
    }
}

pub(super) fn malformed_payload<'a>(
    data: &'a [u8],
    fields: &[usize; 11],
    newer: bool,
) -> (Option<bool>, Result<PayloadFields<'a>, HeaderError>) {
    let decoded = || Decoder::new(data, fields[4]).uint();
    let mut storage = Decoder::new(data, fields[6]);
    let mut stored = Decoder::new(data, fields[7]);
    let storage = storage.storage(newer, 1);
    let stored = stored.byte_string(None);
    let matches = storage
        .as_ref()
        .ok()
        .and_then(|storage| check_stored_chunks(*storage, byte_chunks(data, fields[7])?));
    let parse = || {
        let storage = storage?;
        let stored = stored?;
        let mut digest = Decoder::new(data, fields[5]);
        Ok(PayloadFields {
            storage,
            stored,
            decoded_length: decoded()?,
            digest: digest.fixed::<32>()?,
            content_size: fields[4] - fields[3],
            signatures_size: fields[10] - fields[9],
        })
    };
    (matches, parse())
}

pub(super) struct PayloadBudget {
    capacity: ReadCapacity,
    total: u64,
    refused: bool,
}

impl PayloadBudget {
    pub(super) fn new(capacity: ReadCapacity) -> Self {
        Self {
            capacity,
            total: 0,
            refused: false,
        }
    }

    fn admit(&mut self, fields: &PayloadFields<'_>) -> Result<(), PayloadRefusal> {
        if !self.refused {
            for length in [
                fields.stored.len(),
                fields.content_size,
                fields.signatures_size,
            ] {
                let length = length as u64;
                if length > self.capacity.stored_bytes_per_frame {
                    return Err(PayloadRefusal::StoredSize(length));
                }
            }
            if fields.decoded_length > self.capacity.decoded_bytes_per_frame {
                return Err(PayloadRefusal::DecodedSize(fields.decoded_length));
            }
        }
        self.charge(fields.decoded_length)
            .map_err(PayloadRefusal::TotalDecoded)
    }

    pub(super) fn total_refused(&self) -> bool {
        self.refused
    }

    pub(super) fn charge(&mut self, length: u64) -> Result<(), u64> {
        let next = self.total.checked_add(length);
        if let Some(admitted) =
            next.filter(|&value| !self.refused && value <= self.capacity.decoded_bytes_per_read)
        {
            self.total = admitted;
            return Ok(());
        }
        self.refused = true;
        Err(next.unwrap_or(u64::MAX))
    }

    pub(super) fn check<'a>(
        &mut self,
        fields: PayloadFields<'a>,
        stored_matches: Option<bool>,
    ) -> PayloadCheck<'a> {
        let state = match self.admit(&fields) {
            Ok(()) => check_payload(fields),
            Err(reason) => PayloadState::Unchecked(reason),
        };
        PayloadCheck {
            stored_matches,
            state,
        }
    }
}

fn check_payload(fields: PayloadFields<'_>) -> PayloadState<'_> {
    use PayloadDamage::{CodecConstraint, DecodeFailed, DecodedLength, PayloadDigest};
    let decoded = match fields.storage {
        Storage::Plain if fields.stored.len() as u64 == fields.decoded_length => {
            Cow::Borrowed(fields.stored)
        }
        Storage::Plain => return PayloadState::Damaged(DecodedLength),
        Storage::Compressed { codec: 1, .. } => {
            if fields.decoded_length == 0
                || !fields
                    .stored
                    .starts_with(&zstd_header(fields.decoded_length))
            {
                return PayloadState::Damaged(CodecConstraint);
            }
            let Some(decoded) = decompress(fields.stored, fields.decoded_length) else {
                return PayloadState::Damaged(DecodeFailed);
            };
            Cow::Owned(decoded)
        }
        _ => return PayloadState::Unchecked(PayloadRefusal::UnknownCodec),
    };
    if &payload_digest(&decoded) == fields.digest {
        PayloadState::Verified(decoded)
    } else {
        PayloadState::Damaged(PayloadDigest)
    }
}

fn zstd_header(size: u64) -> Vec<u8> {
    let mut header = vec![0x28, 0xb5, 0x2f, 0xfd];
    let bytes = size.to_le_bytes();
    match size {
        1..=255 => header.extend([0x20, bytes[0]]),
        256..=65_791 => {
            header.push(0x60);
            header.extend(((size - 256) as u16).to_le_bytes());
        }
        65_792..=67_108_864 => {
            header.push(0xa0);
            header.extend(&bytes[..4]);
        }
        67_108_865..=4_294_967_294 => {
            header.extend([0x80, 0x80]);
            header.extend(&bytes[..4]);
        }
        _ => {
            header.extend([0xc0, 0x80]);
            header.extend(bytes);
        }
    }
    header
}

fn decompress(mut stored: &[u8], size: u64) -> Option<Vec<u8>> {
    let size = usize::try_from(size).ok()?;
    let mut decoded = Vec::new();
    let mut decoder = FrameDecoder::new();
    decoder.set_max_window_size(1 << 26).ok()?;
    decoder.init(&mut stored).ok()?;
    let mut written = 0_usize;
    while !decoder.is_finished() {
        decoder
            .decode_blocks(&mut stored, BlockDecodingStrategy::UptoBlocks(1))
            .ok()?;
        let available = decoder.can_collect();
        let end = written.checked_add(available)?;
        if end > size {
            return None;
        }
        decoded.try_reserve(available).ok()?;
        decoded.resize(end, 0);
        decoder.read_exact(&mut decoded[written..end]).ok()?;
        written = end;
    }
    decoder.verify_content_size().ok()?;
    (written == size && stored.is_empty()).then_some(decoded)
}

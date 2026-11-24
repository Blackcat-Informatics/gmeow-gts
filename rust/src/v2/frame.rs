// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frame envelopes and the BLAKE3 preimages of their wire fields (P1 §§3–4).

use std::{error::Error, fmt, str};

use ciborium::value::Value;

use super::{Decoder, HeaderError, MalformedReason, REGISTRY_VERSION};
use crate::wire;

const FRAME: u64 = 2;
const CHUNK: u64 = 8;
const EXTERNAL: u64 = 12;
pub(super) const COMMIT: u64 = 14;
pub(super) const PARITY: u64 = 16;
const DIGEST_HEAD: [u8; 2] = [0x58, 0x20];

/// The first malformed field of a frame and its absolute input offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameError {
    pub reason: MalformedReason,
    pub offset: usize,
}

impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} at offset {}", self.reason, self.offset)
    }
}

impl Error for FrameError {}

impl From<HeaderError> for FrameError {
    fn from(error: HeaderError) -> Self {
        Self {
            reason: error.reason,
            offset: error.offset,
        }
    }
}

/// The storage record, including the original bytes of a future storage form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Storage<'a> {
    Plain,
    Compressed {
        codec: u64,
        stored_check: &'a [u8; 16],
    },
    Unknown {
        tag: u64,
        encoded: &'a [u8],
    },
}

/// The signature fields in envelope order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Signature<'a> {
    pub algorithm: u64,
    pub key_id: &'a [u8; 32],
    pub value: &'a [u8],
}

/// A parsed frame, borrowing the original encoded fields and stored payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame<'a> {
    pub frame_type: u64,
    pub critical: bool,
    pub transparent: bool,
    pub content: &'a [u8],
    pub decoded_length: u64,
    pub payload_digest: &'a [u8; 32],
    pub storage: Storage<'a>,
    pub stored_payload: &'a [u8],
    pub identifier: &'a [u8; 32],
    pub signatures: Vec<Signature<'a>>,
    pub encoded_signatures_size: usize,
    pub identity_fields: &'a [u8],
    pub next_offset: usize,
}

impl Frame<'_> {
    /// Compute the frame ID from the predecessor and original identity fields.
    #[must_use]
    pub fn computed_identifier(&self, predecessor: &[u8; 32]) -> [u8; 32] {
        frame_identifier(predecessor, self.identity_fields)
    }

    /// Compare the stored check of a compressed frame, including a future codec.
    #[must_use]
    pub fn check_storage(&self) -> Option<bool> {
        check_stored_chunks(self.storage, std::iter::once(self.stored_payload))
    }

    /// Compare the decoded bytes with the envelope length and payload digest.
    #[must_use]
    pub fn check_decoded_payload(&self, decoded: &[u8]) -> bool {
        if u64::try_from(decoded.len()).ok() != Some(self.decoded_length) {
            return false;
        }
        &payload_digest(decoded) == self.payload_digest
    }
}

pub(super) fn check_stored_chunks<'a>(
    storage: Storage<'_>,
    chunks: impl IntoIterator<Item = &'a [u8]>,
) -> Option<bool> {
    let Storage::Compressed {
        codec,
        stored_check,
    } = storage
    else {
        return None;
    };
    let mut hash = blake3::Hasher::new_derive_key("gts2 stored");
    hash.update(&wire::encode(&Value::Array(vec![Value::from(codec)])));
    for chunk in chunks {
        hash.update(chunk);
    }
    Some(hash.finalize().as_bytes()[..16] == stored_check[..])
}

pub(super) fn frame_identifier(predecessor: &[u8; 32], fields: &[u8]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new_derive_key("gts2 frame");
    hash.update(&[0x87]);
    hash.update(&DIGEST_HEAD);
    hash.update(predecessor);
    hash.update(fields);
    *hash.finalize().as_bytes()
}

pub(super) fn payload_digest(decoded: &[u8]) -> [u8; 32] {
    blake3::derive_key("gts2 payload", decoded)
}

fn valid_decoded_length(frame_type: u64, length: u64) -> bool {
    match frame_type {
        COMMIT | EXTERNAL => length == 0,
        CHUNK => length != 0,
        _ => true,
    }
}

impl<'a> Decoder<'a> {
    pub(super) fn byte_string(&mut self, expected: Option<usize>) -> Result<&'a [u8], HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        let length = usize::try_from(self.head(2)?).map_err(|_| error)?;
        if expected.is_some_and(|value| value != length) {
            return Err(error);
        }
        self.take(length)
    }

    pub(super) fn fixed<const SIZE: usize>(&mut self) -> Result<&'a [u8; SIZE], HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        self.byte_string(Some(SIZE))?.try_into().map_err(|_| error)
    }

    pub(super) fn text(&mut self) -> Result<&'a str, HeaderError> {
        let length_error = self.error(MalformedReason::WrongLength);
        let type_error = self.error(MalformedReason::WrongType);
        let size = usize::try_from(self.head(3)?).map_err(|_| length_error)?;
        str::from_utf8(self.take(size)?).map_err(|_| type_error)
    }

    pub(super) fn boolean(&mut self) -> Result<bool, HeaderError> {
        let error = self.error(MalformedReason::WrongType);
        let code = self.head(7)?;
        if self.offset - error.offset == 1 {
            match code {
                20 => return Ok(false),
                21 => return Ok(true),
                _ => {}
            }
        }
        Err(error)
    }

    pub(super) fn closed_item(&mut self) -> Result<(), HeaderError> {
        let mut pending = 1_u64;
        while pending != 0 {
            let first = self
                .bytes
                .get(self.offset)
                .ok_or_else(|| self.error(MalformedReason::WrongLength))?;
            let major = first >> 5;
            pending -= 1;
            match major {
                0 | 1 => {
                    self.head(major)?;
                }
                2 => {
                    self.byte_string(None)?;
                }
                3 => {
                    self.text()?;
                }
                4 => pending = pending.saturating_add(self.head(4)?),
                7 => {
                    self.boolean()?;
                }
                _ => return self.head(0).map(|_| ()),
            }
        }
        Ok(())
    }

    fn registry_code(&mut self, last: u64, newer: bool) -> Result<u64, HeaderError> {
        self.code(if newer { u64::MAX } else { last })
    }

    fn media_type_shape(&mut self) -> Result<(), HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        let size = self.head(4)?;
        if size == 0 {
            return Err(error);
        }
        let tag = self.uint()?;
        match tag {
            1 | 2 if size != tag => return Err(error),
            2 => {
                self.text()?;
            }
            1 => {}
            _ => {
                for _ in 1..size {
                    self.closed_item()?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn frame_content(&mut self, frame_type: u64) -> Result<(), HeaderError> {
        if frame_type == 1 {
            self.record(4)?;
            self.uint()?;
            for _ in 0..self.head(4)? {
                self.record(2)?;
                self.uint()?;
                self.uint()?;
            }
            self.uint()?;
            self.uint()?;
            return Ok(());
        }

        let shape: &[u8] = match frame_type {
            2..=4 | 13 => b"uu",
            5..=7 => b"u",
            8 => b"d",
            9 => b"dm",
            10 => b"dumu",
            11 => b"duu",
            12 => b"num",
            14 | 15 => b"ud",
            16 => b"uuudd",
            _ => return self.closed_item(),
        };
        self.record(shape.len() as u64)?;
        for field in shape {
            match field {
                b'u' => {
                    self.uint()?;
                }
                b'd' => {
                    self.fixed::<32>()?;
                }
                b'm' => self.media_type_shape()?,
                b'n' => {
                    self.record(2)?;
                    let algorithm = self.uint()?;
                    self.byte_string((algorithm == 1).then_some(32))?;
                }
                _ => unreachable!("unknown wire field in a registry shape"),
            }
        }
        Ok(())
    }

    pub(super) fn storage(
        &mut self,
        newer: bool,
        decoded_length: u64,
    ) -> Result<Storage<'a>, HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        let size = self.head(4)?;
        if size == 0 {
            return Err(error);
        }
        let tag = self.registry_code(2, newer)?;
        match tag {
            1 if size == 1 => Ok(Storage::Plain),
            2 if size == 3 => {
                let codec = self.registry_code(1, newer)?;
                let stored_check = self.fixed::<16>()?;
                if decoded_length == 0 {
                    return Err(HeaderError {
                        reason: MalformedReason::ForbiddenCombination,
                        ..error
                    });
                }
                Ok(Storage::Compressed {
                    codec,
                    stored_check,
                })
            }
            1 | 2 => Err(error),
            _ => {
                for _ in 1..size {
                    self.closed_item()?;
                }
                Ok(Storage::Unknown {
                    tag,
                    encoded: &self.bytes[error.offset..self.offset],
                })
            }
        }
    }

    pub(super) fn signatures(&mut self, newer: bool) -> Result<Vec<Signature<'a>>, HeaderError> {
        let mut previous: &[u8] = &[];
        let mut signatures = Vec::new();
        for _ in 0..self.head(4)? {
            let error = self.error(MalformedReason::UnorderedOrRepeated);
            self.record(3)?;
            let algorithm = self.registry_code(2, newer)?;
            let key_id = self.fixed::<32>()?;
            let value = self.byte_string(None)?;
            let encoded = &self.bytes[error.offset..self.offset];
            if encoded <= previous {
                return Err(error);
            }
            previous = encoded;
            signatures.push(Signature {
                algorithm,
                key_id,
                value,
            });
        }
        Ok(signatures)
    }

    pub(super) fn frame_identity(
        &mut self,
        newer: Option<bool>,
    ) -> Result<Identity<'a>, HeaderError> {
        let start = self.offset;
        let frame_type = match newer {
            Some(newer) => self.registry_code(PARITY, newer)?,
            None => self.uint()?,
        };
        let check_flags = newer.is_some() && frame_type <= PARITY;
        let error = self.error(MalformedReason::ForbiddenCombination);
        let critical = self.boolean()?;
        if check_flags && critical != (frame_type < COMMIT) {
            return Err(error);
        }
        let error = self.error(MalformedReason::ForbiddenCombination);
        let transparent = self.boolean()?;
        if check_flags && transparent != (frame_type == PARITY) {
            return Err(error);
        }
        let content_start = self.offset;
        self.frame_content(frame_type)?;
        let content = &self.bytes[content_start..self.offset];
        let length_offset = self.offset;
        let decoded_length = self.uint()?;
        let payload_digest = self.fixed::<32>()?;
        Ok(Identity {
            frame_type,
            critical,
            transparent,
            content,
            length_offset,
            decoded_length,
            payload_digest,
            encoded: &self.bytes[start..self.offset],
        })
    }
}

pub(super) struct Identity<'a> {
    frame_type: u64,
    critical: bool,
    transparent: bool,
    content: &'a [u8],
    length_offset: usize,
    decoded_length: u64,
    payload_digest: &'a [u8; 32],
    pub encoded: &'a [u8],
}

/// Read one deterministic frame envelope at a known item boundary.
///
/// Link decisions, payload processing, capacity, and the meaning of each
/// content value are handled by the reader after envelope decoding.
pub fn decode_frame(
    data: &[u8],
    offset: usize,
    registry_version: u64,
) -> Result<Frame<'_>, FrameError> {
    let mut decoder = Decoder::new(data, offset);
    let newer = registry_version > REGISTRY_VERSION;
    decoder.record(11)?;
    let kind_error = decoder.error(MalformedReason::UnregisteredValue);
    if decoder.uint()? != FRAME {
        return Err(kind_error.into());
    }
    let Identity {
        frame_type,
        critical,
        transparent,
        content,
        length_offset,
        decoded_length,
        payload_digest,
        encoded: identity_fields,
    } = decoder.frame_identity(Some(newer))?;

    let error = decoder.error(MalformedReason::ForbiddenCombination);
    let storage = decoder.storage(newer, decoded_length)?;
    if frame_type == PARITY && storage != Storage::Plain {
        return Err(error.into());
    }
    let stored_payload = decoder.byte_string(None)?;
    let identifier = decoder.fixed::<32>()?;
    let error = decoder.error(MalformedReason::ForbiddenCombination);
    let signatures = decoder.signatures(newer)?;
    if frame_type == PARITY && !signatures.is_empty() {
        return Err(error.into());
    }
    if !valid_decoded_length(frame_type, decoded_length) {
        return Err(FrameError {
            reason: MalformedReason::OutOfRange,
            offset: length_offset,
        });
    }

    Ok(Frame {
        frame_type,
        critical,
        transparent,
        content,
        decoded_length,
        payload_digest,
        storage,
        stored_payload,
        identifier,
        signatures,
        encoded_signatures_size: decoder.offset - error.offset,
        identity_fields,
        next_offset: decoder.offset,
    })
}

/// Write a registry 1 frame with plain storage and no signatures.
///
/// Supply the deterministic encoding of the content record and decoded bytes
/// that satisfy the rules of the selected frame type.
pub fn encode_plain_frame(
    predecessor: &[u8; 32],
    frame_type: u64,
    content: &[u8],
    payload: &[u8],
) -> Result<Vec<u8>, FrameError> {
    encode_plain_frame_with_identifier(predecessor, frame_type, content, payload)
        .map(|(encoded, _)| encoded)
}

pub(super) fn encode_plain_frame_with_identifier(
    predecessor: &[u8; 32],
    frame_type: u64,
    content: &[u8],
    payload: &[u8],
) -> Result<(Vec<u8>, [u8; 32]), FrameError> {
    let mut encoded = vec![0x8b, 0x02];
    if !(1..=PARITY).contains(&frame_type) {
        return Err(FrameError {
            reason: MalformedReason::UnregisteredValue,
            offset: encoded.len(),
        });
    }
    encoded.extend(wire::encode(&Value::from(frame_type)));
    encoded.push(if frame_type < COMMIT { 0xf5 } else { 0xf4 });
    encoded.push(if frame_type == PARITY { 0xf5 } else { 0xf4 });
    let offset = encoded.len();
    let mut decoder = Decoder::new(content, 0);
    decoder
        .frame_content(frame_type)
        .map_err(|error| FrameError {
            reason: error.reason,
            offset: offset + error.offset,
        })?;
    if decoder.offset != content.len() {
        return Err(FrameError {
            reason: MalformedReason::WrongLength,
            offset: offset + decoder.offset,
        });
    }
    encoded.extend_from_slice(content);

    let error = FrameError {
        reason: MalformedReason::OutOfRange,
        offset: encoded.len(),
    };
    let length = u64::try_from(payload.len()).map_err(|_| error)?;
    if !valid_decoded_length(frame_type, length) {
        return Err(error);
    }
    let mut length_head = wire::encode(&Value::from(length));
    encoded.extend_from_slice(&length_head);
    encoded.extend_from_slice(&DIGEST_HEAD);
    encoded.extend_from_slice(&payload_digest(payload));
    let identifier = frame_identifier(predecessor, &encoded[2..]);

    encoded.extend_from_slice(&[0x81, 0x01]);
    // A byte string and a uint use the same preferred argument encoding.
    length_head[0] |= 0x40;
    encoded.extend(length_head);
    encoded.reserve(payload.len() + DIGEST_HEAD.len() + identifier.len() + 1);
    encoded.extend_from_slice(payload);
    encoded.extend_from_slice(&DIGEST_HEAD);
    encoded.extend_from_slice(&identifier);
    encoded.push(0x80);
    Ok((encoded, identifier))
}

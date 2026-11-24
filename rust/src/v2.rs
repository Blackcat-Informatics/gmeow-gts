// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Wire records of generation 2.

use std::{error::Error, fmt};

use ciborium::value::Value;

use crate::wire;

pub mod frame;
mod framing;
mod payload;
pub mod reader;
pub mod writer;

const REGISTRY_VERSION: u64 = 1;
const BLAKE3_256: u64 = 1;
const CANONICAL_FORM: u64 = 1;
const LAST_PROFILE: u64 = 6;
const HEADER_CONTEXT: &str = "gts2 header";

/// A claimed previous segment head and lineage ordinal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lineage<'a> {
    pub previous_head: &'a [u8],
    pub ordinal: u64,
}

/// The fields of a GTS v2 header after the kind and generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header<'a> {
    pub registry_version: u64,
    pub hash_algorithm: u64,
    pub profile: Option<u64>,
    pub canonical: Option<u64>,
    pub lineage: Option<Lineage<'a>>,
}

impl Default for Header<'_> {
    fn default() -> Self {
        Self {
            registry_version: REGISTRY_VERSION,
            hash_algorithm: BLAKE3_256,
            profile: None,
            canonical: None,
            lineage: None,
        }
    }
}

/// The decoded header and the first byte after it in the input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedHeader<'a> {
    pub header: Header<'a>,
    pub next_offset: usize,
    /// Computed over the encoded header. A hash from a later registry can be unavailable.
    pub identifier: Option<[u8; 32]>,
}

/// A P1 malformed-value reason for a wire field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MalformedReason {
    NotDeterministic,
    WrongType,
    WrongLength,
    OutOfRange,
    UnregisteredValue,
    ForbiddenCombination,
    UnorderedOrRepeated,
    InconsistentCounts,
}

/// The first malformed header field and its absolute input offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeaderError {
    pub reason: MalformedReason,
    pub offset: usize,
}

impl fmt::Display for HeaderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} at offset {}", self.reason, self.offset)
    }
}

impl Error for HeaderError {}

/// A header field outside the values the writer can emit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeaderWriteError {
    RegistryVersion(u64),
    HashAlgorithm(u64),
    Profile(u64),
    CanonicalForm(u64),
    Lineage,
}

impl fmt::Display for HeaderWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryVersion(value) => {
                write!(formatter, "cannot write registry version {value}")
            }
            Self::HashAlgorithm(value) => write!(formatter, "cannot write hash algorithm {value}"),
            Self::Profile(value) => write!(formatter, "cannot write profile {value}"),
            Self::CanonicalForm(value) => write!(formatter, "cannot write canonical form {value}"),
            Self::Lineage => {
                formatter.write_str("lineage requires a 32-byte head and positive ordinal")
            }
        }
    }
}

impl Error for HeaderWriteError {}

struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
    exhausted: bool,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8], offset: usize) -> Self {
        Self {
            bytes,
            offset,
            exhausted: false,
        }
    }

    fn error(&self, reason: MalformedReason) -> HeaderError {
        HeaderError {
            reason,
            offset: self.offset,
        }
    }

    fn take(&mut self, size: usize) -> Result<&'a [u8], HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        let Some(end) = self
            .offset
            .checked_add(size)
            .filter(|&end| end <= self.bytes.len())
        else {
            self.exhausted = true;
            return Err(error);
        };
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn head(&mut self, expected: u8) -> Result<u64, HeaderError> {
        let start = self.offset;
        let first = self.take(1)?[0];
        let major = first >> 5;
        let additional = first & 31;
        let error = |reason| HeaderError {
            reason,
            offset: start,
        };

        let argument = match additional {
            0..=23 => u64::from(additional),
            24..=27 => {
                let width_index = usize::from(additional - 24);
                let value = self
                    .take(1 << width_index)?
                    .iter()
                    .fold(0, |value, byte| (value << 8) | u64::from(*byte));
                if major <= 4 && value < [24, 256, 65_536, 1 << 32][width_index] {
                    return Err(error(MalformedReason::NotDeterministic));
                }
                if major == 7 && additional == 24 && value < 32 {
                    return Err(error(MalformedReason::WrongLength));
                }
                value
            }
            31 if (2..=4).contains(&major) => {
                return Err(error(MalformedReason::NotDeterministic));
            }
            31 if major == 5 => return Err(error(MalformedReason::WrongType)),
            _ => return Err(error(MalformedReason::WrongLength)),
        };
        if major != expected {
            return Err(error(MalformedReason::WrongType));
        }
        Ok(argument)
    }

    fn record(&mut self, arity: u64) -> Result<(), HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        if self.head(4)? != arity {
            return Err(error);
        }
        Ok(())
    }

    fn uint(&mut self) -> Result<u64, HeaderError> {
        self.head(0)
    }

    fn positive(&mut self) -> Result<u64, HeaderError> {
        let error = self.error(MalformedReason::OutOfRange);
        let value = self.uint()?;
        if value == 0 {
            return Err(error);
        }
        Ok(value)
    }

    fn code(&mut self, last: u64) -> Result<u64, HeaderError> {
        let error = self.error(MalformedReason::UnregisteredValue);
        let value = self.uint()?;
        if !(1..=last).contains(&value) {
            return Err(error);
        }
        Ok(value)
    }

    fn option<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, HeaderError>,
    ) -> Result<Option<T>, HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        let arity = self.head(4)?;
        if arity == 0 {
            return Err(error);
        }
        let tag = self.code(2)?;
        if tag != arity {
            return Err(error);
        }
        if tag == 2 {
            read(self).map(Some)
        } else {
            Ok(None)
        }
    }

    fn lineage(&mut self, known_hash: bool) -> Result<Lineage<'a>, HeaderError> {
        self.record(2)?;
        let error = self.error(MalformedReason::WrongLength);
        let length = self.head(2)?;
        if known_hash && length != 32 {
            return Err(error);
        }
        let length = usize::try_from(length).map_err(|_| error)?;
        Ok(Lineage {
            previous_head: self.take(length)?,
            ordinal: self.positive()?,
        })
    }

    fn opener_fields(&mut self) -> Result<(u64, u64, Option<u64>), HeaderError> {
        let registry = self.positive()?;
        let hash = self.code(if registry == REGISTRY_VERSION {
            BLAKE3_256
        } else {
            u64::MAX
        })?;
        let profile = self.option(|decoder| decoder.code(u64::MAX))?;
        Ok((registry, hash, profile))
    }
}

/// Decode a header at an established item boundary (P1 §2.4).
///
/// The first malformed field determines the error. A readable header's identifier
/// uses the original bytes. A higher registry with an unknown hash retains its fields.
pub fn decode_header(data: &[u8], offset: usize) -> Result<DecodedHeader<'_>, HeaderError> {
    let mut decoder = Decoder::new(data, offset);
    decoder.record(7)?;
    decoder.code(1)?;
    let generation_error = decoder.error(MalformedReason::OutOfRange);
    if decoder.uint()? != 2 {
        return Err(generation_error);
    }
    let (registry_version, hash_algorithm, profile) = decoder.opener_fields()?;
    let last_known = |last| {
        if registry_version > REGISTRY_VERSION {
            u64::MAX
        } else {
            last
        }
    };
    let known_hash = hash_algorithm == BLAKE3_256;
    let canonical = decoder.option(|decoder| decoder.code(last_known(CANONICAL_FORM)))?;
    let lineage = decoder.option(|decoder| decoder.lineage(known_hash))?;
    let next_offset = decoder.offset;
    let identifier =
        known_hash.then(|| blake3::derive_key(HEADER_CONTEXT, &data[offset..next_offset]));
    Ok(DecodedHeader {
        header: Header {
            registry_version,
            hash_algorithm,
            profile,
            canonical,
            lineage,
        },
        next_offset,
        identifier,
    })
}

fn option(value: Option<Value>) -> Value {
    Value::Array(match value {
        Some(inner) => vec![Value::from(2), inner],
        None => vec![Value::from(1)],
    })
}

/// Encode a registry version 1 header for a segment writer.
///
/// Supply the lineage of the preceding segment and any canonical claim from
/// the writer's segment construction.
pub fn encode_header(header: Header<'_>) -> Result<Vec<u8>, HeaderWriteError> {
    if header.registry_version != REGISTRY_VERSION {
        return Err(HeaderWriteError::RegistryVersion(header.registry_version));
    }
    if header.hash_algorithm != BLAKE3_256 {
        return Err(HeaderWriteError::HashAlgorithm(header.hash_algorithm));
    }
    if let Some(profile) = header.profile {
        if !(1..=LAST_PROFILE).contains(&profile) {
            return Err(HeaderWriteError::Profile(profile));
        }
    }
    if let Some(canonical) = header.canonical {
        if canonical != CANONICAL_FORM {
            return Err(HeaderWriteError::CanonicalForm(canonical));
        }
    }
    let lineage = match header.lineage {
        Some(lineage) if lineage.previous_head.len() == 32 && lineage.ordinal > 0 => {
            Some(Value::Array(vec![
                Value::Bytes(lineage.previous_head.to_vec()),
                Value::from(lineage.ordinal),
            ]))
        }
        Some(_) => return Err(HeaderWriteError::Lineage),
        None => None,
    };
    Ok(wire::encode(&Value::Array(vec![
        Value::from(1),
        Value::from(2),
        Value::from(header.registry_version),
        Value::from(header.hash_algorithm),
        option(header.profile.map(Value::from)),
        option(header.canonical.map(Value::from)),
        option(lineage),
    ])))
}

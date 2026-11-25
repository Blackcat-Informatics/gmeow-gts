// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Wire records of generation 2.

use std::{error::Error, fmt};

use ciborium::value::Value;

use crate::wire;

mod attachments;
mod durability;
pub mod frame;
mod framing;
mod import;
mod index;
mod payload;
mod range;
pub mod rdf;
pub mod reader;
mod resync;
pub mod writer;

pub use import::{from_nquads, ImportError};

const REGISTRY_VERSION: u64 = 1;
const BLAKE3_256: u64 = 1;
const CANONICAL_FORM: u64 = 1;
const LAST_PROFILE: u64 = 6;
const HEADER_CONTEXT: &str = "gts2 header";
const FORMED_BYTES_PER_READ: u64 = 1 << 37;
const DEFAULT_VALUE_TEXT_LIMIT: u64 = 256 << 20;

/// A claimed previous segment head and lineage ordinal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lineage<'a> {
    /// The head identifier of the predecessor segment.
    pub previous_head: &'a [u8],
    /// The position of this segment in its lineage.
    pub ordinal: u64,
}

/// The fields of a GTS v2 header after the kind and generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header<'a> {
    /// The registry version the header declares.
    pub registry_version: u64,
    /// The registry code of the hash algorithm.
    pub hash_algorithm: u64,
    /// The registry code of the profile, if declared.
    pub profile: Option<u64>,
    /// The registry code of the canonical form, if declared.
    pub canonical: Option<u64>,
    /// The lineage that this segment continues, if declared.
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
    /// The decoded header.
    pub header: Header<'a>,
    /// The offset of the first byte after the header.
    pub next_offset: usize,
    /// Computed over the encoded header. A hash from a later registry can be unavailable.
    pub identifier: Option<[u8; 32]>,
}

/// A P1 malformed-value reason for a wire field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MalformedReason {
    /// The item is not in deterministic CBOR form.
    NotDeterministic,
    /// A field has a CBOR type that the format does not allow.
    WrongType,
    /// A field has a length that the format does not allow.
    WrongLength,
    /// A field value is outside its allowed range.
    OutOfRange,
    /// A field names a registry value that is not registered.
    UnregisteredValue,
    /// Two fields are set in a combination that the format forbids.
    ForbiddenCombination,
    /// A map or array is out of order or repeats an entry.
    UnorderedOrRepeated,
    /// Declared counts disagree with the items present.
    InconsistentCounts,
}

/// The first malformed header field and its absolute input offset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeaderError {
    /// The reason the field is malformed.
    pub reason: MalformedReason,
    /// The absolute input offset of the malformed field.
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
    /// The registry version is not supported by this writer.
    RegistryVersion(u64),
    /// The hash algorithm code is not supported by this writer.
    HashAlgorithm(u64),
    /// The profile code is not supported by this writer.
    Profile(u64),
    /// The canonical form code is not supported by this writer.
    CanonicalForm(u64),
    /// The lineage is malformed or inconsistent with the header.
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
    field_start_limit: Option<usize>,
}

impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8], offset: usize) -> Self {
        Self {
            bytes,
            offset,
            field_start_limit: None,
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
            return Err(error);
        };
        let value = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(value)
    }

    fn peek_byte(&mut self) -> Result<u8, HeaderError> {
        self.take(1).map(|byte| {
            self.offset -= 1;
            byte[0]
        })
    }

    fn head(&mut self, expected: u8) -> Result<u64, HeaderError> {
        if let Some(limit) = self.field_start_limit.filter(|limit| self.offset >= *limit) {
            return Err(HeaderError {
                reason: MalformedReason::OutOfRange,
                offset: limit,
            });
        }
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

    fn int(&mut self) -> Result<i128, HeaderError> {
        let negative = self
            .bytes
            .get(self.offset)
            .is_some_and(|first| first >> 5 == 1);
        let argument = i128::from(self.head(u8::from(negative))?);
        Ok(if negative { -1 - argument } else { argument })
    }

    #[expect(
        clippy::map_err_ignore,
        reason = "the conversion error carries no detail, so the typed error replaces it"
    )]
    fn delta(&mut self, previous: u64) -> Result<u64, HeaderError> {
        let error = self.error(MalformedReason::OutOfRange);
        u64::try_from(i128::from(previous) + self.int()?).map_err(|_| error)
    }

    fn column_len(&mut self, count: u64) -> Result<(), HeaderError> {
        let error = self.error(MalformedReason::InconsistentCounts);
        if self.head(4)? == count {
            Ok(())
        } else {
            Err(error)
        }
    }

    #[expect(
        clippy::map_err_ignore,
        reason = "the conversion error carries no detail, so the typed error replaces it"
    )]
    fn list_len(&mut self) -> Result<usize, HeaderError> {
        let error = self.error(MalformedReason::WrongLength);
        let count = usize::try_from(self.head(4)?).map_err(|_| error)?;
        if count > self.bytes.len() - self.offset {
            return Err(error);
        }
        Ok(count)
    }

    fn list<T>(
        &mut self,
        mut read: impl FnMut(&mut Self) -> Result<T, HeaderError>,
    ) -> Result<Vec<T>, HeaderError> {
        let count = self.list_len()?;
        (0..count).map(|_| read(self)).collect()
    }

    fn column<T>(
        &mut self,
        count: u64,
        mut read: impl FnMut(&mut Self) -> Result<T, HeaderError>,
    ) -> Result<Vec<T>, HeaderError> {
        self.column_len(count)?;
        if count > (self.bytes.len() - self.offset) as u64 {
            return Err(self.error(MalformedReason::WrongLength));
        }
        (0..count).map(|_| read(self)).collect()
    }

    fn runs<T: Copy + Eq>(
        &mut self,
        count: u64,
        mut value: impl FnMut(&mut Self) -> Result<T, HeaderError>,
        mut add: impl FnMut(&Self, T, u64, u64) -> Result<(), HeaderError>,
    ) -> Result<(), HeaderError> {
        let mut first = 0_u64;
        let mut previous = None;
        for _ in 0..self.list_len()? {
            self.record(2)?;
            let value = value(self)?;
            let size = self.positive()?;
            if previous == Some(value) {
                return Err(self.error(MalformedReason::NotDeterministic));
            }
            let end = first
                .checked_add(size)
                .ok_or_else(|| self.error(MalformedReason::OutOfRange))?;
            add(self, value, first, size)?;
            first = end;
            previous = Some(value);
        }
        if first != count {
            return Err(self.error(MalformedReason::InconsistentCounts));
        }
        Ok(())
    }

    fn end(&self) -> Result<(), HeaderError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(self.error(MalformedReason::WrongLength))
        }
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

    #[expect(
        clippy::map_err_ignore,
        reason = "the conversion error carries no detail, so the typed error replaces it"
    )]
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

    fn opener_fields(
        &mut self,
        known_hash_only: bool,
    ) -> Result<(u64, u64, Option<u64>), HeaderError> {
        let registry = self.positive()?;
        let hash = self.code(if known_hash_only || registry == REGISTRY_VERSION {
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
///
/// # Errors
///
/// Returns [`HeaderError`] at the first malformed field, carrying its
/// [`MalformedReason`] and absolute input offset.
pub fn decode_header(data: &[u8], offset: usize) -> Result<DecodedHeader<'_>, HeaderError> {
    let mut decoder = Decoder::new(data, offset);
    decoder.record(7)?;
    decoder.code(1)?;
    let generation_error = decoder.error(MalformedReason::OutOfRange);
    if decoder.uint()? != 2 {
        return Err(generation_error);
    }
    let (registry_version, hash_algorithm, profile) = decoder.opener_fields(false)?;
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
///
/// # Errors
///
/// Returns [`HeaderWriteError`] when the registry version, hash algorithm,
/// profile or canonical form is not supported, or when the lineage is not a
/// 32-byte previous head with a non-zero ordinal.
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

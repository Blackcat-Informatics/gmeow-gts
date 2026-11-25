// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::sync::Arc;

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::v2::{Decoder, HeaderError, MalformedReason::{OutOfRange, WrongLength}};

const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const UINT: &str = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
const INT: &str = "http://www.w3.org/2001/XMLSchema#integer";
const TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
pub(super) const OCTETS: &str = "http://www.w3.org/2001/XMLSchema#hexBinary";

pub(super) fn append_hex(lexical: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for &octet in bytes {
        lexical.push(char::from(HEX[usize::from(octet >> 4)]));
        lexical.push(char::from(HEX[usize::from(octet & 15)]));
    }
}

pub(super) struct InlineValues<'a> {
    bytes: &'a [u8],
    start: usize,
    count: u64,
}

impl<'a> InlineValues<'a> {
    pub(super) fn read(cursor: &mut Decoder<'a>) -> Result<Self, HeaderError> {
        let count = cursor.head(4)?;
        let start = cursor.offset;
        for _ in 0..count {
            InlineValue::read(cursor)?;
        }
        Ok(Self {
            bytes: &cursor.bytes[..cursor.offset],
            start,
            count,
        })
    }

    pub(super) fn count_matches(&self, expected: u64) -> bool {
        self.count == expected
    }

    pub(super) fn project(
        self,
    ) -> impl Iterator<Item = Result<(Arc<str>, &'static str), HeaderError>> + 'a {
        let mut cursor = Decoder::new(self.bytes, self.start);
        (0..self.count).map(move |_| {
            let error = cursor.error(OutOfRange);
            InlineValue::read(&mut cursor)?.literal(error)
        })
    }
}

enum InlineValue<'a> {
    Boolean(bool),
    Uint(u64),
    Int(i128),
    Time(OffsetDateTime),
    Octets(&'a [u8]),
}

impl<'a> InlineValue<'a> {
    fn read(cursor: &mut Decoder<'a>) -> Result<Self, HeaderError> {
        let length_error = cursor.error(WrongLength);
        let length = cursor.head(4)?;
        if length == 0 {
            return Err(length_error);
        }
        let kind = cursor.code(5)?;
        if length != if kind == 4 { 3 } else { 2 } {
            return Err(length_error);
        }
        match kind {
            1 => Ok(Self::Boolean(cursor.boolean()?)),
            2 => Ok(Self::Uint(cursor.uint()?)),
            3 => Ok(Self::Int(cursor.int()?)),
            4 => Self::time(cursor),
            _ => Ok(Self::Octets(cursor.byte_string(None)?)),
        }
    }

    #[expect(clippy::map_err_ignore, reason = "the conversion error carries no detail, so the typed error replaces it")]
    fn time(cursor: &mut Decoder<'a>) -> Result<Self, HeaderError> {
        let error = cursor.error(OutOfRange);
        let seconds = i64::try_from(cursor.int()?).map_err(|_| error)?;
        if !(-62_135_596_800..=253_402_300_799).contains(&seconds) {
            return Err(error);
        }
        let instant = OffsetDateTime::from_unix_timestamp(seconds).map_err(|_| error)?;
        let error = cursor.error(OutOfRange);
        let nanoseconds = u32::try_from(cursor.uint()?).map_err(|_| error)?;
        instant
            .replace_nanosecond(nanoseconds)
            .map(Self::Time)
            .map_err(|_| error)
    }

    #[expect(
        clippy::map_err_ignore,
        reason = "HeaderError is Copy and carries only a reason and offset; the time format error detail has no field to occupy"
    )]
    fn literal(self, error: HeaderError) -> Result<(Arc<str>, &'static str), HeaderError> {
        let (lexical, datatype) = match self {
            Self::Boolean(value) => (Arc::from(if value { "true" } else { "false" }), BOOLEAN),
            Self::Uint(value) => (Arc::from(value.to_string()), UINT),
            Self::Int(value) => (Arc::from(value.to_string()), INT),
            Self::Time(value) => (Arc::from(value.format(&Rfc3339).map_err(|_| error)?), TIME),
            Self::Octets(value) => {
                let capacity = value
                    .len()
                    .checked_mul(2)
                    .filter(|&size| isize::try_from(size).is_ok())
                    .ok_or(error)?;
                let mut lexical = String::with_capacity(capacity);
                append_hex(&mut lexical, value);
                (Arc::from(lexical), OCTETS)
            }
        };
        Ok((lexical, datatype))
    }
}

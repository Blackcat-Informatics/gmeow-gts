// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR encoding of the parity findings of a read result (P7 section 1).

use super::reader::{ParityCode, ParityFinding, ParityReason, WireSegment};

/// A parity finding that cannot be encoded from the reader's result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadResultError {
    /// The position, offset, length or count of a finding does not fit in the encoding's `u64`.
    OutOfRange {
        /// The failed conversion of the value.
        source: std::num::TryFromIntError,
    },
}

impl std::fmt::Display for ReadResultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange { .. } => {
                write!(formatter, "a parity finding value is out of range")
            }
        }
    }
}

impl std::error::Error for ReadResultError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutOfRange { source } => Some(source),
        }
    }
}

/// Encode the parity findings of `segment` in input order, each as the seven-field record of P7
/// section 1: `[code, reason, subject, position, offset, length, count]`, where an absent field is
/// `[1]` and a present one is `[2, value]`.
///
/// The findings are those that the reader gives in [`WireSegment::parity_findings`]: a damaged
/// frame gives `parity_damaged` with its position alone, and the other codes carry the fields
/// their finding takes.
///
/// # Errors
///
/// Returns [`ReadResultError::OutOfRange`] when a position, offset, length or count does not fit
/// in a `u64`.
pub fn parity_findings(segment: &WireSegment<'_>) -> Result<Vec<Vec<u8>>, ReadResultError> {
    segment.parity_findings.iter().map(encode).collect()
}

/// The seven-field record of one finding; the subject is always absent for a parity finding.
fn encode(finding: &ParityFinding) -> Result<Vec<u8>, ReadResultError> {
    let number = |value: Option<usize>| {
        value
            .map(|value| {
                u64::try_from(value).map_err(|source| ReadResultError::OutOfRange { source })
            })
            .transpose()
    };
    let mut out = Vec::new();
    head(0, code(finding.code), &mut out);
    optional(finding.reason.map(reason), &mut out);
    optional(None, &mut out);
    optional(number(finding.position)?, &mut out);
    optional(number(finding.offset)?, &mut out);
    optional(number(finding.length)?, &mut out);
    optional(finding.count, &mut out);
    Ok(out)
}

/// The code of a finding in the registry of finding codes.
fn code(code: ParityCode) -> u64 {
    match code {
        ParityCode::ExceedsCapacity => 20,
        ParityCode::ParityDamaged => 30,
        ParityCode::ParityMismatch => 31,
        ParityCode::ParityUnexplained => 32,
        ParityCode::ParityStale => 33,
        ParityCode::RepairFailed => 34,
        ParityCode::Repaired => 35,
        ParityCode::UnprotectedTail => 36,
    }
}

/// The code of a reason in the registry of reason codes.
fn reason(reason: ParityReason) -> u64 {
    match reason {
        ParityReason::DecodedSize => 30,
        ParityReason::StoredSize => 31,
        ParityReason::TotalDecoded => 86,
        ParityReason::Data => 40,
        ParityReason::Parity => 41,
        ParityReason::TooManyErasures => 42,
        ParityReason::VerifyFailed => 43,
    }
}

/// Append an option: `[1]` when absent, else `[2, value]`.
fn optional(value: Option<u64>, out: &mut Vec<u8>) {
    match value {
        None => out.extend([0x81, 0x01]),
        Some(value) => {
            out.extend([0x82, 0x02]);
            head(0, value, out);
        }
    }
}

/// Append the head of a CBOR item of `major` type with `argument`, in its shortest form.
fn head(major: u8, argument: u64, out: &mut Vec<u8>) {
    let major = major << 5;
    if let Ok(value) = u8::try_from(argument) {
        if value < 24 {
            out.push(major | value);
        } else {
            out.extend([major | 0x18, value]);
        }
    } else if let Ok(value) = u16::try_from(argument) {
        out.push(major | 0x19);
        out.extend(value.to_be_bytes());
    } else if let Ok(value) = u32::try_from(argument) {
        out.push(major | 0x1a);
        out.extend(value.to_be_bytes());
    } else {
        out.push(major | 0x1b);
        out.extend(argument.to_be_bytes());
    }
}

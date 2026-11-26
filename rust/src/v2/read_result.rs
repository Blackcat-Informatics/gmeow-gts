// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The CBOR encoding of the parity findings of a read result (P7 section 1).

use super::{
    payload::{PayloadRefusal, PayloadState},
    reader::{Judgement, WireSegment},
};

/// The code of the finding `parity_damaged` in the registry of finding codes.
const PARITY_DAMAGED: u64 = 30;
/// The code of the finding `exceeds_capacity` in the registry of finding codes.
const EXCEEDS_CAPACITY: u64 = 20;
/// The reason `decoded_size` in the registry of reason codes.
const DECODED_SIZE: u64 = 30;
/// The reason `stored_size` in the registry of reason codes.
const STORED_SIZE: u64 = 31;
/// The reason `total_decoded` in the registry of reason codes.
const TOTAL_DECODED: u64 = 86;

/// A parity finding that cannot be encoded from the reader's result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadResultError {
    /// The frame at `position` has no item in its segment, so it has no offset or length.
    MissingItem {
        /// The position of the frame in its segment.
        position: usize,
    },
    /// The frame at `position` has an `exceeds_capacity` judgement whose payload was not refused
    /// by a size limit.
    NotSizeRefusal {
        /// The position of the frame in its segment.
        position: usize,
    },
    /// The offset or length of the item at `position` does not fit in the encoding's `u64`.
    OutOfRange {
        /// The position of the frame in its segment.
        position: usize,
        /// The failed conversion of the offset or length.
        source: std::num::TryFromIntError,
    },
}

impl std::fmt::Display for ReadResultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingItem { position } => {
                write!(formatter, "frame at position {position} has no item")
            }
            Self::NotSizeRefusal { position } => write!(
                formatter,
                "frame at position {position} is above capacity but not refused by a size limit"
            ),
            Self::OutOfRange { position, .. } => {
                write!(
                    formatter,
                    "item of frame at position {position} is out of range"
                )
            }
        }
    }
}

impl std::error::Error for ReadResultError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutOfRange { source, .. } => Some(source),
            Self::MissingItem { .. } | Self::NotSizeRefusal { .. } => None,
        }
    }
}

/// Encode the parity findings of `segment` in frame order, each as the seven-field record of P7
/// section 1: `[code, reason, subject, position, offset, length, count]`, where an absent field is
/// `[1]` and a present one is `[2, value]`.
///
/// A frame whose judgement is `Damaged` gives `parity_damaged` with its position alone. A frame whose
/// judgement is `ExceedsCapacity` gives `exceeds_capacity` with the reason and count of its size
/// refusal and the offset and length of its item. Other judgements give no finding, and damage takes
/// precedence over the capacity report because the reader gives a frame one judgement.
///
/// # Errors
///
/// Returns [`ReadResultError::MissingItem`] when a judged frame has no item at its position,
/// [`ReadResultError::NotSizeRefusal`] when an `ExceedsCapacity` frame's payload was refused for
/// another reason, and [`ReadResultError::OutOfRange`] when an offset does not fit in a `u64`.
pub fn parity_findings(segment: &WireSegment<'_>) -> Result<Vec<Vec<u8>>, ReadResultError> {
    let mut findings = Vec::new();
    for frame in &segment.frames {
        let position = frame.position;
        match frame.judgement {
            Some(Judgement::Damaged) => findings.push(record(
                PARITY_DAMAGED,
                None,
                Some(
                    u64::try_from(position)
                        .map_err(|source| ReadResultError::OutOfRange { position, source })?,
                ),
                None,
                None,
                None,
            )),
            Some(Judgement::ExceedsCapacity) => {
                let item = segment
                    .items
                    .get(position)
                    .ok_or(ReadResultError::MissingItem { position })?;
                let (reason, count) = size_refusal(&frame.payload.state)
                    .ok_or(ReadResultError::NotSizeRefusal { position })?;
                let span = |value: usize| {
                    u64::try_from(value)
                        .map_err(|source| ReadResultError::OutOfRange { position, source })
                };
                findings.push(record(
                    EXCEEDS_CAPACITY,
                    Some(reason),
                    Some(span(position)?),
                    Some(span(item.offset)?),
                    Some(span(item.end - item.offset)?),
                    Some(count),
                ));
            }
            Some(Judgement::Unchecked | Judgement::Verified) | None => {}
        }
    }
    Ok(findings)
}

/// The reason code and the count of a payload that a size limit refused.
fn size_refusal(state: &PayloadState<'_>) -> Option<(u64, u64)> {
    match state {
        PayloadState::Unchecked(PayloadRefusal::StoredSize(count)) => Some((STORED_SIZE, *count)),
        PayloadState::Unchecked(PayloadRefusal::DecodedSize(count)) => Some((DECODED_SIZE, *count)),
        PayloadState::Unchecked(PayloadRefusal::TotalDecoded(count)) => {
            Some((TOTAL_DECODED, *count))
        }
        PayloadState::Verified(_)
        | PayloadState::Damaged(_)
        | PayloadState::Unchecked(PayloadRefusal::UnknownCodec) => None,
    }
}

/// The seven-field finding record; the subject is always absent for a parity finding.
fn record(
    code: u64,
    reason: Option<u64>,
    position: Option<u64>,
    offset: Option<u64>,
    length: Option<u64>,
    count: Option<u64>,
) -> Vec<u8> {
    let mut out = Vec::new();
    head(0, code, &mut out);
    optional(reason, &mut out);
    optional(None, &mut out);
    optional(position, &mut out);
    optional(offset, &mut out);
    optional(length, &mut out);
    optional(count, &mut out);
    out
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

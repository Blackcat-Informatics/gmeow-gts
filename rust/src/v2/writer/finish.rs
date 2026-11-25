// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Close a buffered indexed segment with parity and storage claims.

use std::{error::Error, fmt};

use ciborium::value::Value;

use super::{parity, Digest, SegmentWriter};
use crate::{
    v2::{
        frame::{decode_frame, FrameError},
        REGISTRY_VERSION,
    },
    wire,
};

const INDEX: u64 = 15;

/// A buffered segment that cannot be closed with generated parity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentFinishError {
    /// The segment has no closing index.
    MissingClosingIndex,
    /// The segment already holds a parity frame.
    ExistingParity {
        /// The offset of the existing parity frame.
        offset: usize,
    },
    /// The closing frame could not be written.
    Frame(FrameError),
}

impl fmt::Display for SegmentFinishError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingClosingIndex => {
                formatter.write_str("the segment must end with an index frame")
            }
            Self::ExistingParity { offset } => {
                write!(
                    formatter,
                    "the segment already contains a parity frame at byte {offset}"
                )
            }
            Self::Frame(error) => error.fmt(formatter),
        }
    }
}

impl Error for SegmentFinishError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::MissingClosingIndex | Self::ExistingParity { .. } => None,
        }
    }
}

impl From<FrameError> for SegmentFinishError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl SegmentWriter {
    /// Close a segment whose final frame is the caller's index.
    ///
    /// Add parity after each frame that reaches the end of a span, including
    /// the final span at the closing index. Append the offsets item and the
    /// locator over the resulting stored bytes. The opener and ordinary
    /// frames retain their bytes and identifiers.
    ///
    /// The caller supplies the index's content and payload when appending it.
    /// This method consumes the buffered segment and requires that it holds
    /// no previously emitted parity.
    pub fn finish_indexed(self) -> Result<Vec<u8>, SegmentFinishError> {
        let mut output = ClosingItems {
            bytes: Vec::with_capacity(self.bytes.len()),
            lengths: Vec::new(),
        };
        output.push(&self.bytes[..self.header_len]);

        let mut span_start = 0;
        let mut offset = self.header_len;
        let mut index = None;
        while offset < self.bytes.len() {
            let frame = decode_frame(&self.bytes, offset, REGISTRY_VERSION)?;
            if frame.transparent {
                return Err(SegmentFinishError::ExistingParity { offset });
            }

            let end = frame.next_offset;
            index = (frame.frame_type == INDEX).then_some(output.bytes.len());
            output.push(&self.bytes[offset..end]);
            while end - span_start >= parity::SPAN_BYTES {
                let span_end = span_start + parity::SPAN_BYTES;
                output.push_span(
                    &self.bytes[span_start..span_end],
                    span_start,
                    &self.opener,
                    frame.identifier,
                )?;
                span_start = span_end;
            }
            offset = end;
        }

        let index = index.ok_or(SegmentFinishError::MissingClosingIndex)?;
        if span_start < offset {
            output.push_span(
                &self.bytes[span_start..offset],
                span_start,
                &self.opener,
                &self.head,
            )?;
        }
        Ok(output.with_accelerators(index))
    }
}

struct ClosingItems {
    bytes: Vec<u8>,
    lengths: Vec<Value>,
}

impl ClosingItems {
    fn push(&mut self, item: &[u8]) {
        self.bytes.extend_from_slice(item);
        self.lengths.push(Value::from(item.len() as u64));
    }

    fn push_span(
        &mut self,
        bytes: &[u8],
        first: usize,
        opener: &Digest,
        anchor: &Digest,
    ) -> Result<(), FrameError> {
        self.push(&parity::encode_span(bytes, first, opener, anchor)?);
        Ok(())
    }

    fn with_accelerators(mut self, index: usize) -> Vec<u8> {
        self.bytes.extend(wire::encode(&Value::Array(vec![
            Value::from(4),
            Value::Array(self.lengths),
        ])));
        let distance = (self.bytes.len() - index) as u64;
        self.bytes.extend_from_slice(&[0x82, 0x03, 0x1b]);
        self.bytes.extend_from_slice(&distance.to_be_bytes());
        self.bytes
    }
}

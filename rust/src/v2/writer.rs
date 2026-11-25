// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Write header-led segments with plain frames and P5 commits.

mod index;
pub use index::{IndexFold, IndexWriteError};

use std::{error::Error, fmt};

use ciborium::value::Value;

use super::{
    encode_header,
    frame::{encode_plain_frame_with_identifier, FrameError, COMMIT, PARITY},
    range::{Digest, Range, RangeFull},
    Header, HeaderWriteError, HEADER_CONTEXT,
};
use crate::wire;

mod attachments;
pub(super) use attachments::write_values;
pub use attachments::{AttachmentWriteError, Blob};

/// A segment append that cannot be represented.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentWriteError {
    Frame(FrameError),
    CommitRequired,
    RangeFull,
}

impl fmt::Display for SegmentWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(error) => error.fmt(formatter),
            Self::CommitRequired => formatter.write_str("use commit to append a commit frame"),
            Self::RangeFull => formatter.write_str("segment range leaf count exceeds uint64"),
        }
    }
}

impl Error for SegmentWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::CommitRequired | Self::RangeFull => None,
        }
    }
}

/// Assemble an unsigned GTS v2 segment with plain storage.
///
/// The caller prepares content records and payloads for their frame type. The
/// writer derives the head, frame identifiers and commit count and root.
pub struct SegmentWriter {
    bytes: Vec<u8>,
    header_len: usize,
    opener: Digest,
    head: Digest,
    range: Range,
}

impl SegmentWriter {
    /// Encode a header and use its identifier as the initial head and leaf.
    pub fn new(header: Header<'_>) -> Result<Self, HeaderWriteError> {
        let bytes = encode_header(header)?;
        let head = blake3::derive_key(HEADER_CONTEXT, &bytes);
        Ok(Self {
            header_len: bytes.len(),
            bytes,
            opener: head,
            head,
            range: Range::new(&head),
        })
    }

    /// Return the head after the ordinary frames written so far.
    pub fn head(&self) -> &Digest {
        &self.head
    }

    /// Append one frame and return its identifier.
    ///
    /// Use [`Self::commit`] to append a commit computed from the prefix.
    pub fn append(
        &mut self,
        frame_type: u64,
        content: &[u8],
        payload: &[u8],
    ) -> Result<Digest, SegmentWriteError> {
        if frame_type == COMMIT {
            return Err(SegmentWriteError::CommitRequired);
        }
        self.append_frame(frame_type, content, payload)
    }

    /// Append the commit over the opener and the preceding ordinary frames.
    pub fn commit(&mut self) -> Result<Digest, SegmentWriteError> {
        let content = wire::encode(&Value::Array(vec![
            Value::from(self.range.ordinary_count()),
            Value::Bytes(self.range.root().to_vec()),
        ]));
        self.append_frame(COMMIT, &content, &[])
    }

    /// Return the encoded segment in item order.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Take ownership of the encoded segment.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn append_frame(
        &mut self,
        frame_type: u64,
        content: &[u8],
        payload: &[u8],
    ) -> Result<Digest, SegmentWriteError> {
        let (encoded, identifier) =
            encode_plain_frame_with_identifier(&self.head, frame_type, content, payload)
                .map_err(SegmentWriteError::Frame)?;
        if frame_type != PARITY {
            self.range
                .append(&identifier)
                .map_err(|RangeFull| SegmentWriteError::RangeFull)?;
            self.head = identifier;
        }
        self.bytes.extend(encoded);
        Ok(identifier)
    }
}

mod finish;
mod parity;
pub use finish::SegmentFinishError;

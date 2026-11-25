// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Form the P5 index from the written frames and the labelled fold.

mod rows;

use std::{collections::BTreeMap, error::Error, fmt};

use ciborium::value::Value;

use super::{Digest, SegmentWriteError, SegmentWriter};
use crate::{
    v2::{
        attachments::{decode_pack, BLOB, PACK},
        decode_header,
        frame::{decode_frame, Frame, FrameError, Storage},
        Decoder, HeaderError, MalformedReason, REGISTRY_VERSION,
    },
    wire,
};

const INDEX: u64 = 15;

/// The labelled fold's counts and content digest, used by the closing index.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IndexFold {
    /// The number of lines folded into the index.
    pub lines: u64,
    /// The number of records folded into the index.
    pub records: u64,
    /// The number of blank nodes folded into the index.
    pub blank_nodes: u64,
    /// The digest of the content, if one is recorded.
    pub content_digest: Option<Digest>,
}

/// An index that could not be derived from the encoded prefix or appended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexWriteError {
    /// The header of the index could not be opened.
    Opener(HeaderError),
    /// The index envelope could not be written.
    Envelope(FrameError),
    /// The fields of a frame are malformed.
    FrameFields {
        /// The position of the frame.
        position: u64,
        /// The malformation of the frame fields.
        error: HeaderError,
    },
    /// A frame is stored compressed, which the index cannot read.
    CompressedFrame {
        /// The position of the compressed frame.
        position: u64,
    },
    /// The index frame could not be appended to the segment.
    Append(SegmentWriteError),
}

impl fmt::Display for IndexWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opener(error) => write!(formatter, "decode segment opener: {error}"),
            Self::Envelope(error) => write!(formatter, "decode frame envelope: {error}"),
            Self::FrameFields { position, error } => {
                write!(
                    formatter,
                    "index fields of ordinary frame {position}: {error}"
                )
            }
            Self::CompressedFrame { position } => {
                write!(
                    formatter,
                    "index writer requires decoded bytes for ordinary frame {position}"
                )
            }
            Self::Append(error) => error.fmt(formatter),
        }
    }
}

impl Error for IndexWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Opener(error) | Self::FrameFields { error, .. } => Some(error),
            Self::Envelope(error) => Some(error),
            Self::Append(error) => Some(error),
            Self::CompressedFrame { .. } => None,
        }
    }
}

impl SegmentWriter {
    /// Append the index of the segment's existing ordinary frames.
    ///
    /// The caller supplies valid preceding frame content and the labelled
    /// fold's counts and digest. The writer derives the head, root, type runs, graph ranges, per-predicate
    /// row counts, and unit locations from the encoded segment. This method
    /// leaves the segment unchanged if it cannot construct the index.
    pub fn append_index(&mut self, fold: IndexFold) -> Result<Digest, IndexWriteError> {
        let mut prefix = IndexPrefix::default();
        let mut offset = decode_header(&self.bytes, 0)
            .map_err(IndexWriteError::Opener)?
            .next_offset;
        while offset < self.bytes.len() {
            let frame = decode_frame(&self.bytes, offset, REGISTRY_VERSION)
                .map_err(IndexWriteError::Envelope)?;
            offset = frame.next_offset;
            if frame.transparent {
                continue;
            }
            if !matches!(frame.storage, Storage::Plain) {
                return Err(IndexWriteError::CompressedFrame {
                    position: prefix.count,
                });
            }
            prefix
                .add(&frame)
                .map_err(|error| IndexWriteError::FrameFields {
                    position: prefix.count,
                    error,
                })?;
        }
        let content = record([Value::from(prefix.count), digest(self.head)]);
        let payload = prefix.payload(fold, self.range.root());
        self.append_frame(INDEX, &wire::encode(&content), &wire::encode(&payload))
            .map_err(IndexWriteError::Append)
    }
}

#[derive(Default)]
struct IndexPrefix {
    count: u64,
    types: Vec<(u64, u64)>,
    graphs: BTreeMap<Option<u64>, Vec<GraphRange>>,
    last_range: [Option<(Option<u64>, usize)>; 4],
    predicates: BTreeMap<u64, u64>,
    members: Vec<(Digest, u64, u64)>,
}

struct GraphRange {
    frame_type: u64,
    first_frame: u64,
    first_row: u64,
    rows: u64,
}

fn record<const N: usize>(fields: [Value; N]) -> Value {
    Value::Array(Vec::from(fields))
}

fn integers<const N: usize>(fields: [u64; N]) -> Value {
    record(fields.map(Value::from))
}

fn digest(bytes: Digest) -> Value {
    Value::Bytes(bytes.to_vec())
}

impl IndexPrefix {
    fn add(&mut self, frame: &Frame<'_>) -> Result<(), HeaderError> {
        match frame.frame_type {
            2..=5 => self.add_rows(frame)?,
            PACK => self.add_pack(frame)?,
            _ => {}
        }
        if let Some((_, run)) = self
            .types
            .last_mut()
            .filter(|(code, _)| *code == frame.frame_type)
        {
            *run += 1;
        } else {
            self.types.push((frame.frame_type, 1));
        }
        self.count += 1;
        Ok(())
    }

    fn payload(mut self, fold: IndexFold, root: Digest) -> Value {
        let types = self
            .types
            .into_iter()
            .map(|(code, run)| integers([code, run]));
        let directory = self.graphs.into_iter().map(|(graph, ranges)| {
            let name = graph.map_or_else(|| integers([1]), |id| integers([2, id]));
            let ranges = ranges.into_iter().map(|range| {
                integers([
                    range.frame_type,
                    range.first_frame,
                    range.first_row,
                    range.rows,
                ])
            });
            record([name, Value::Array(ranges.collect())])
        });
        let predicates = self
            .predicates
            .into_iter()
            .map(|(id, rows)| integers([id, rows]));
        let statistics = record([
            Value::from(fold.lines),
            Value::from(fold.records),
            Value::from(fold.blank_nodes),
            Value::Array(predicates.collect()),
        ]);
        let content_digest = fold.content_digest.map_or_else(
            || integers([1]),
            |bytes| record([Value::from(2), digest(bytes)]),
        );
        self.members.sort_unstable();
        let members = self.members.into_iter().map(|(unit, position, index)| {
            record([digest(unit), Value::from(position), Value::from(index)])
        });
        record([
            Value::Array(types.collect()),
            Value::Array(directory.collect()),
            statistics,
            digest(root),
            content_digest,
            Value::Array(members.collect()),
        ])
    }

    fn add_pack(&mut self, frame: &Frame<'_>) -> Result<(), HeaderError> {
        for (index, member) in decode_pack(frame.content, frame.stored_payload)?
            .into_iter()
            .enumerate()
        {
            if member.kind <= BLOB {
                self.members
                    .push((member.reference.digest, self.count, index as u64));
            }
        }
        Ok(())
    }
}

fn checked_sum(cursor: &Decoder<'_>, sum: u64, value: u64) -> Result<u64, HeaderError> {
    sum.checked_add(value)
        .ok_or_else(|| cursor.error(MalformedReason::OutOfRange))
}

fn require_end(cursor: &Decoder<'_>) -> Result<(), HeaderError> {
    if cursor.offset == cursor.bytes.len() {
        Ok(())
    } else {
        Err(cursor.error(MalformedReason::WrongLength))
    }
}

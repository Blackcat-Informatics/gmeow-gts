// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Form the P5 index from the written frames and the labelled fold.

mod rows;

use std::{collections::BTreeMap, error::Error, fmt};

use ciborium::value::Value;

use super::{Digest, SegmentWriteError, SegmentWriter};
use crate::{
    v2::{
        decode_header,
        frame::{decode_frame, Frame, FrameError, Storage},
        Decoder, HeaderError, MalformedReason, REGISTRY_VERSION,
    },
    wire,
};

const INDEX: u64 = 15;
const PACK: u64 = 13;

/// The labelled fold's counts and content digest, used by the closing index.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IndexFold {
    pub lines: u64,
    pub records: u64,
    pub blank_nodes: u64,
    pub content_digest: Option<Digest>,
}

/// An index that could not be derived from the encoded prefix or appended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexWriteError {
    Opener(HeaderError),
    Envelope(FrameError),
    FrameFields { position: u64, error: HeaderError },
    CompressedFrame { position: u64 },
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
        let mut content = Decoder::new(frame.content, 0);
        content.record(2)?;
        let member_count = content.positive()?;
        let unit_length = content.uint()?;

        let mut cursor = Decoder::new(frame.stored_payload, 0);
        cursor.record(2)?;
        if cursor.head(4)? != member_count {
            return Err(cursor.error(MalformedReason::InconsistentCounts));
        }
        let mut all_bytes = 0_u64;
        let mut unit_bytes = 0_u64;
        for index in 0..member_count {
            cursor.record(3)?;
            let kind = cursor.code(4)?;
            let start = cursor.offset;
            cursor.frame_content(7 + kind)?;
            let length = cursor.uint()?;
            all_bytes = checked_sum(&cursor, all_bytes, length)?;
            if kind <= 2 {
                if kind == 1 && length == 0 {
                    return Err(cursor.error(MalformedReason::OutOfRange));
                }
                let mut unit = Decoder::new(frame.stored_payload, start);
                unit.record(kind)?;
                self.members.push((*unit.fixed::<32>()?, self.count, index));
                unit_bytes = checked_sum(&cursor, unit_bytes, length)?;
            }
        }
        let bodies = cursor.byte_string(None)?;
        require_end(&cursor)?;
        if bodies.len() as u64 != all_bytes || unit_bytes != unit_length {
            return Err(cursor.error(MalformedReason::InconsistentCounts));
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

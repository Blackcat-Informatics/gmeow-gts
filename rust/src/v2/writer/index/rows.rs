// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::{checked_sum, require_end, GraphRange, IndexPrefix};
use crate::v2::{frame::Frame, Decoder, HeaderError, MalformedReason};

const QUADS: u64 = 2;
const EMPTY_GRAPHS: u64 = 5;

impl IndexPrefix {
    pub(super) fn add_rows(&mut self, frame: &Frame<'_>) -> Result<(), HeaderError> {
        let mut content = Decoder::new(frame.content, 0);
        let mut cursor = Decoder::new(frame.stored_payload, 0);
        let frame_type = frame.frame_type;
        if frame_type == EMPTY_GRAPHS {
            content.record(1)?;
            let count = content.positive()?;
            cursor.record(1)?;
            require_count(&mut cursor, count)?;
            let mut previous = 0;
            for row in 0..count {
                previous = decode_delta(&mut cursor, previous)?;
                self.add_graph(&cursor, frame_type, Some(previous), row, 1)?;
            }
            return require_end(&cursor);
        }

        content.record(2)?;
        content.code(2)?;
        let count = content.positive()?;
        let delta_columns = if frame_type == 4 { 4 } else { 2 };
        cursor.record(1 + u64::from(frame_type == QUADS) + delta_columns)?;
        self.add_graph_runs(&mut cursor, frame_type, count)?;
        if frame_type == QUADS {
            self.add_predicate_runs(&mut cursor, count)?;
        }
        for column in 0..delta_columns {
            require_count(&mut cursor, count)?;
            let mut previous = 0;
            for _ in 0..count {
                previous = decode_delta(&mut cursor, previous)?;
                if frame_type == 4 && column == 3 && previous == 0 {
                    return Err(cursor.error(MalformedReason::OutOfRange));
                }
            }
        }
        require_end(&cursor)
    }

    fn add_graph_runs(
        &mut self,
        cursor: &mut Decoder<'_>,
        frame_type: u64,
        count: u64,
    ) -> Result<(), HeaderError> {
        let mut first = 0;
        let mut previous = None;
        for _ in 0..cursor.head(4)? {
            cursor.record(2)?;
            let graph = cursor.option(Decoder::uint)?;
            let rows = cursor.positive()?;
            if previous == Some(graph) {
                return Err(cursor.error(MalformedReason::NotDeterministic));
            }
            self.add_graph(cursor, frame_type, graph, first, rows)?;
            first = checked_sum(cursor, first, rows)?;
            previous = Some(graph);
        }
        if first != count {
            return Err(cursor.error(MalformedReason::InconsistentCounts));
        }
        Ok(())
    }

    fn add_graph(
        &mut self,
        cursor: &Decoder<'_>,
        frame_type: u64,
        graph: Option<u64>,
        first_row: u64,
        rows: u64,
    ) -> Result<(), HeaderError> {
        let previous = &mut self.last_range[(frame_type - QUADS) as usize];
        let ranges = self.graphs.entry(graph).or_default();
        if let Some((prior_graph, index)) = *previous {
            if prior_graph == graph {
                let range = &mut ranges[index];
                range.rows = checked_sum(cursor, range.rows, rows)?;
                return Ok(());
            }
        }
        *previous = Some((graph, ranges.len()));
        ranges.push(GraphRange {
            frame_type,
            first_frame: self.count,
            first_row,
            rows,
        });
        Ok(())
    }

    fn add_predicate_runs(
        &mut self,
        cursor: &mut Decoder<'_>,
        count: u64,
    ) -> Result<(), HeaderError> {
        let mut total = 0;
        let mut previous = None;
        for _ in 0..cursor.head(4)? {
            cursor.record(2)?;
            let predicate = cursor.uint()?;
            let rows = cursor.positive()?;
            if previous == Some(predicate) {
                return Err(cursor.error(MalformedReason::NotDeterministic));
            }
            let aggregate = self.predicates.entry(predicate).or_default();
            *aggregate = checked_sum(cursor, *aggregate, rows)?;
            total = checked_sum(cursor, total, rows)?;
            previous = Some(predicate);
        }
        if total != count {
            return Err(cursor.error(MalformedReason::InconsistentCounts));
        }
        Ok(())
    }
}

fn require_count(cursor: &mut Decoder<'_>, count: u64) -> Result<(), HeaderError> {
    if cursor.head(4)? == count {
        Ok(())
    } else {
        Err(cursor.error(MalformedReason::InconsistentCounts))
    }
}

fn decode_delta(cursor: &mut Decoder<'_>, previous: u64) -> Result<u64, HeaderError> {
    let negative = cursor
        .bytes
        .get(cursor.offset)
        .is_some_and(|first| first >> 5 == 1);
    let argument = i128::from(cursor.head(u8::from(negative))?);
    let delta = if negative { -1 - argument } else { argument };
    u64::try_from(i128::from(previous) + delta)
        .map_err(|_| cursor.error(MalformedReason::OutOfRange))
}

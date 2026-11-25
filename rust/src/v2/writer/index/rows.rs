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
            cursor.column_len(count)?;
            let mut previous = 0;
            for row in 0..count {
                previous = cursor.delta(previous)?;
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
            cursor.column_len(count)?;
            let mut previous = 0;
            for _ in 0..count {
                previous = cursor.delta(previous)?;
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
        cursor.runs(
            count,
            |cursor| cursor.option(Decoder::uint),
            |cursor, graph, first, rows| self.add_graph(cursor, frame_type, graph, first, rows),
        )
    }

    fn add_graph(
        &mut self,
        cursor: &Decoder<'_>,
        frame_type: u64,
        graph: Option<u64>,
        first_row: u64,
        rows: u64,
    ) -> Result<(), HeaderError> {
        let previous = usize::try_from(frame_type - QUADS)
            .ok()
            .and_then(|slot| self.last_range.get_mut(slot))
            .ok_or_else(|| cursor.error(MalformedReason::OutOfRange))?;
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
        cursor.runs(count, Decoder::uint, |cursor, predicate, _, rows| {
            let aggregate = self.predicates.entry(predicate).or_default();
            *aggregate = checked_sum(cursor, *aggregate, rows)?;
            Ok(())
        })
    }
}

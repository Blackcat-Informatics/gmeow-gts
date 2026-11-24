// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Import N-Quads as one committed GTS v2 segment.

use std::{collections::HashMap, error::Error, fmt};

use ciborium::Value;

use super::{
    writer::{SegmentWriteError, SegmentWriter},
    Header, HeaderWriteError,
};
use crate::{
    from_nquads::{parse_v2_quads, NQuadsParseError},
    model::{Quad, Term, TermKind},
    wire,
};

const TERMS: u64 = 1;
const QUADS: u64 = 2;
const TERM_ORDER_ARRIVAL: u64 = 2;
const ROW_ORDER_UNORDERED: u64 = 2;
const RESTART_GROUP: usize = 64;
const TERMS_PER_FRAME: usize = 4096;
const ROWS_PER_FRAME: usize = 16_384;

/// N-Quads input or an imported segment that cannot be represented.
#[derive(Debug)]
pub enum ImportError {
    Syntax(NQuadsParseError),
    Header(HeaderWriteError),
    Write(SegmentWriteError),
    IncompleteTerm { index: usize },
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => error.fmt(formatter),
            Self::Header(error) => error.fmt(formatter),
            Self::Write(error) => error.fmt(formatter),
            Self::IncompleteTerm { index } => {
                write!(
                    formatter,
                    "parsed N-Quads term {index} is missing required fields"
                )
            }
        }
    }
}

impl Error for ImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Syntax(error) => Some(error),
            Self::Header(error) => Some(error),
            Self::Write(error) => Some(error),
            Self::IncompleteTerm { .. } => None,
        }
    }
}

impl From<SegmentWriteError> for ImportError {
    fn from(error: SegmentWriteError) -> Self {
        Self::Write(error)
    }
}

/// Import N-Quads into one committed GTS v2 segment with plain storage.
///
/// Stores RDF statements as quad lines, including statements whose predicate
/// is `rdf:reifies`. The segment's dictionary uses arrival order. Quoted
/// triples are accepted in object position, as in the P3 term model.
pub fn from_nquads(text: &str) -> Result<Vec<u8>, ImportError> {
    let (terms, mut quads) = parse_v2_quads(text).map_err(ImportError::Syntax)?;
    let mut writer = SegmentWriter::new(Header::default()).map_err(ImportError::Header)?;
    for (batch, terms) in terms.chunks(TERMS_PER_FRAME).enumerate() {
        append_terms(&mut writer, batch * TERMS_PER_FRAME, terms)?;
    }

    quads.sort_unstable_by_key(|&(subject, predicate, object, graph)| {
        (graph, predicate, object, subject)
    });
    quads.dedup();
    for rows in quads.chunks(ROWS_PER_FRAME) {
        append_quads(&mut writer, rows)?;
    }
    writer.commit()?;
    Ok(writer.into_bytes())
}

fn uint(value: usize) -> Value {
    Value::from(value as u64)
}

fn record<const N: usize>(fields: [Value; N]) -> Value {
    Value::Array(Vec::from(fields))
}

fn kind_code(kind: TermKind) -> u64 {
    match kind {
        TermKind::Iri => 1,
        TermKind::Literal => 2,
        TermKind::Bnode => 3,
        TermKind::Triple => 4,
    }
}

#[derive(Default)]
struct TermBlocks<'a> {
    languages: Vec<Value>,
    language_ids: HashMap<&'a str, usize>,
}

impl<'a> TermBlocks<'a> {
    fn language_id(&mut self, text: &'a str) -> usize {
        *self.language_ids.entry(text).or_insert_with(|| {
            let id = self.languages.len();
            self.languages.push(Value::Text(text.to_owned()));
            id
        })
    }

    fn block(&mut self, run: &'a [Term], first: usize) -> Result<Value, ImportError> {
        let kind = run[0].kind;
        match kind {
            TermKind::Iri | TermKind::Literal => self.text_block(run, first),
            TermKind::Bnode => {
                let labels = run
                    .iter()
                    .enumerate()
                    .map(|(index, term)| {
                        term_text(term, first + index)
                            .map(|text| Value::Bytes(text.as_bytes().to_vec()))
                    })
                    .collect::<Result<_, _>>()?;
                Ok(record([Value::from(kind_code(kind)), Value::Array(labels)]))
            }
            TermKind::Triple => {
                let mut subjects = Vec::with_capacity(run.len());
                let mut predicates = Vec::with_capacity(run.len());
                let mut objects = Vec::with_capacity(run.len());
                for (index, term) in run.iter().enumerate() {
                    let (s, p, o) = term.triple.ok_or(ImportError::IncompleteTerm {
                        index: first + index,
                    })?;
                    subjects.push(uint(s));
                    predicates.push(uint(p));
                    objects.push(uint(o));
                }
                Ok(record([
                    Value::from(kind_code(kind)),
                    Value::Array(subjects),
                    Value::Array(predicates),
                    Value::Array(objects),
                ]))
            }
        }
    }

    fn text_block(&mut self, run: &'a [Term], first: usize) -> Result<Value, ImportError> {
        let kind = run[0].kind;
        let mut prefixes = Vec::with_capacity(run.len());
        let mut lengths = Vec::with_capacity(run.len());
        let mut suffixes = Vec::new();
        let mut forms = Vec::new();
        let mut datatypes = Vec::new();
        let mut language_indexes = Vec::new();
        let mut previous = &[][..];

        for (index, term) in run.iter().enumerate() {
            let id = first + index;
            let text = term_text(term, id)?.as_bytes();
            let prefix = if index % RESTART_GROUP == 0 {
                0
            } else {
                previous
                    .iter()
                    .zip(text)
                    .take_while(|(a, b)| a == b)
                    .count()
            };
            prefixes.push(uint(prefix));
            let suffix = &text[prefix..];
            lengths.push(uint(suffix.len()));
            suffixes.extend_from_slice(suffix);
            previous = text;

            if kind == TermKind::Literal {
                let form = match (
                    term.datatype,
                    term.lang.as_deref(),
                    term.direction.as_deref(),
                ) {
                    (Some(datatype), None, None) => {
                        datatypes.push(uint(datatype));
                        1
                    }
                    (None, Some(language), direction) => {
                        let form = match direction {
                            None => 2,
                            Some("ltr") => 3,
                            Some("rtl") => 4,
                            _ => return Err(ImportError::IncompleteTerm { index: id }),
                        };
                        language_indexes.push(uint(self.language_id(language)));
                        form
                    }
                    _ => return Err(ImportError::IncompleteTerm { index: id }),
                };
                forms.push(Value::from(form));
            }
        }

        let mut block = vec![
            Value::from(kind_code(kind)),
            Value::Array(prefixes),
            Value::Array(lengths),
            Value::Bytes(suffixes),
        ];
        if kind == TermKind::Literal {
            block.extend([
                Value::Array(forms),
                Value::Array(datatypes),
                Value::Array(language_indexes),
            ]);
        }
        Ok(Value::Array(block))
    }
}

fn term_text(term: &Term, index: usize) -> Result<&str, ImportError> {
    term.value
        .as_deref()
        .ok_or(ImportError::IncompleteTerm { index })
}

fn append_terms(
    writer: &mut SegmentWriter,
    base: usize,
    terms: &[Term],
) -> Result<(), ImportError> {
    let mut tables = TermBlocks::default();
    let mut runs = Vec::new();
    let mut blocks = Vec::new();
    let mut first = base;
    for run in terms.chunk_by(|a, b| a.kind == b.kind) {
        runs.push(record([
            Value::from(kind_code(run[0].kind)),
            uint(run.len()),
        ]));
        blocks.push(tables.block(run, first)?);
        first += run.len();
    }
    let content = record([
        uint(base),
        Value::Array(runs),
        Value::from(TERM_ORDER_ARRIVAL),
        uint(RESTART_GROUP),
    ]);
    let payload = record([Value::Array(blocks), Value::Array(tables.languages)]);
    writer.append(TERMS, &wire::encode(&content), &wire::encode(&payload))?;
    Ok(())
}

fn delta_column(values: impl Iterator<Item = usize>) -> Value {
    // Term indices come from a Vec of terms and fit in isize, so any
    // difference between two indices fits in i64.
    Value::Array(
        values
            .scan(0_i64, |prior, current| {
                let current = current as i64;
                let delta = current - *prior;
                *prior = current;
                Some(Value::from(delta))
            })
            .collect(),
    )
}

fn append_quads(writer: &mut SegmentWriter, rows: &[Quad]) -> Result<(), ImportError> {
    let graphs = rows.chunk_by(|a, b| a.3 == b.3).map(|run| {
        let name = run[0].3.map_or_else(
            || record([Value::from(1)]),
            |id| record([Value::from(2), uint(id)]),
        );
        record([name, uint(run.len())])
    });
    let predicates = rows
        .chunk_by(|a, b| a.1 == b.1)
        .map(|run| record([uint(run[0].1), uint(run.len())]));
    let content = record([Value::from(ROW_ORDER_UNORDERED), uint(rows.len())]);
    let payload = record([
        Value::Array(graphs.collect()),
        Value::Array(predicates.collect()),
        delta_column(rows.iter().map(|row| row.2)),
        delta_column(rows.iter().map(|row| row.0)),
    ]);
    writer.append(QUADS, &wire::encode(&content), &wire::encode(&payload))?;
    Ok(())
}

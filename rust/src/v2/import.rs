// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Import N-Quads as one committed GTS v2 segment.

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fmt,
};

use ciborium::Value;

use super::{
    attachments::{unit_digest, PACK},
    reader::ReadCapacity,
    writer::{write_values, SegmentWriteError, SegmentWriter},
    Header, HeaderWriteError, DEFAULT_VALUE_TEXT_LIMIT, FORMED_BYTES_PER_READ,
};
use crate::{
    from_nquads::{parse_v2_quads, NQuadsParseError},
    model::{Quad, Term, TermKind},
    wire,
};

const TERMS: u64 = 1;
const QUADS: u64 = 2;
const LITERAL_REF: u64 = 5;
const TERM_ORDER_ARRIVAL: u64 = 2;
const ROW_ORDER_UNORDERED: u64 = 2;
const RESTART_GROUP: usize = 64;
const TERMS_PER_FRAME: usize = 4096;
const ROWS_PER_FRAME: usize = 16_384;

/// N-Quads input or an imported segment that cannot be represented.
#[derive(Debug)]
pub enum ImportError {
    /// The input is not valid N-Quads.
    Syntax(NQuadsParseError),
    /// The header could not be written.
    Header(HeaderWriteError),
    /// A frame could not be written.
    Write(SegmentWriteError),
    /// A term is missing a required component.
    IncompleteTerm {
        /// The index of the incomplete term.
        index: usize,
    },
    /// A term index is too large for the signed delta encoding.
    IndexOutOfRange {
        /// The index that cannot be encoded.
        index: usize,
    },
    /// An import limit was exceeded.
    Capacity {
        /// The name of the exhausted resource.
        resource: &'static str,
        /// The amount of the resource that the input requires.
        required: u64,
        /// The maximum amount of the resource that is allowed.
        limit: u64,
    },
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
            Self::IndexOutOfRange { index } => {
                write!(
                    formatter,
                    "term index {index} exceeds the signed delta range"
                )
            }
            Self::Capacity {
                resource,
                required,
                limit,
            } => {
                write!(
                    formatter,
                    "N-Quads import requires {required} {resource}, limit {limit}"
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
            Self::IncompleteTerm { .. } | Self::IndexOutOfRange { .. } | Self::Capacity { .. } => {
                None
            }
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
/// Literals too large for a terms frame use shared value declarations.
/// Frames and referenced text fit the default wire and projection capacities.
///
/// # Errors
///
/// Returns [`ImportError::Syntax`] when `text` is not valid N-Quads for this
/// profile. Returns the other [`ImportError`] variants when the header or a
/// frame cannot be written, or when a value exceeds the default capacities.
pub fn from_nquads(text: &str) -> Result<Vec<u8>, ImportError> {
    let (terms, quads) = parse_v2_quads(text).map_err(ImportError::Syntax)?;
    write_dataset(&terms, quads, ReadCapacity::default())
}

fn write_dataset(
    terms: &[Term],
    mut quads: Vec<Quad>,
    capacity: ReadCapacity,
) -> Result<Vec<u8>, ImportError> {
    let mut writer = ImportWriter {
        segment: SegmentWriter::new(Header::default()).map_err(ImportError::Header)?,
        capacity,
        items: 2, // The header and the closing commit.
        decoded: 0,
        formed: 0,
    };
    let TermPlan { terms, values } = TermPlan::new(terms, writer.payload_limit())?;
    write_values(values, |content, payload| {
        writer.append(PACK, content, payload, 0)
    })?;
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
    writer.segment.commit()?;
    Ok(writer.segment.into_bytes())
}

struct ImportWriter {
    segment: SegmentWriter,
    capacity: ReadCapacity,
    items: u64,
    decoded: u64,
    formed: u64,
}

impl ImportWriter {
    fn payload_limit(&self) -> u64 {
        self.capacity
            .stored_bytes_per_frame
            .min(self.capacity.decoded_bytes_per_frame)
    }

    fn append(
        &mut self,
        kind: u64,
        content: &[u8],
        payload: &[u8],
        formed: u64,
    ) -> Result<(), ImportError> {
        let decoded = self.decoded.saturating_add(payload.len() as u64);
        let formed = self.formed.saturating_add(formed);
        let items = self.items + 1;
        for (required, limit, resource) in [
            (
                content.len() as u64,
                self.capacity.stored_bytes_per_frame,
                "content bytes in one frame",
            ),
            (
                payload.len() as u64,
                self.payload_limit(),
                "payload bytes in one frame",
            ),
            (
                decoded,
                self.capacity.decoded_bytes_per_read,
                "decoded bytes",
            ),
            (formed, FORMED_BYTES_PER_READ, "formed prefix bytes"),
            (items, self.capacity.items_per_read, "items"),
        ] {
            if required > limit {
                return Err(ImportError::Capacity {
                    resource,
                    required,
                    limit,
                });
            }
        }
        self.segment.append(kind, content, payload)?;
        self.items = items;
        self.decoded = decoded;
        self.formed = formed;
        Ok(())
    }
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

struct StoredTerm<'a> {
    term: &'a Term,
    value: Option<[u8; 32]>,
}

impl StoredTerm<'_> {
    fn code(&self) -> u64 {
        if self.value.is_some() {
            LITERAL_REF
        } else {
            kind_code(self.term.kind)
        }
    }
}

struct TermPlan<'a> {
    terms: Vec<StoredTerm<'a>>,
    values: BTreeMap<[u8; 32], &'a [u8]>,
}

impl<'a> TermPlan<'a> {
    fn new(input: &'a [Term], frame_limit: u64) -> Result<Self, ImportError> {
        let mut values = BTreeMap::new();
        let mut names = HashMap::new();
        let mut referenced_bytes = 0_u64;
        for (index, term) in input
            .iter()
            .enumerate()
            .filter(|(_, term)| term.kind == TermKind::Literal)
        {
            let text = term_text(term, index)?;
            if text.is_empty()
                || singleton_literal_size(term, text) <= frame_limit
                || names.contains_key(text)
            {
                continue;
            }
            referenced_bytes = referenced_bytes.saturating_add(text.len() as u64);
            if referenced_bytes > DEFAULT_VALUE_TEXT_LIMIT {
                return Err(ImportError::Capacity {
                    resource: "referenced literal bytes",
                    required: referenced_bytes,
                    limit: DEFAULT_VALUE_TEXT_LIMIT,
                });
            }
            let digest = unit_digest(text.as_bytes());
            names.insert(text, digest);
            values.insert(digest, text.as_bytes());
        }
        let terms = input
            .iter()
            .map(|term| StoredTerm {
                value: if term.kind == TermKind::Literal {
                    term.value
                        .as_deref()
                        .and_then(|text| names.get(text).copied())
                } else {
                    None
                },
                term,
            })
            .collect();
        Ok(Self { terms, values })
    }
}

fn cbor_head(value: u64) -> u64 {
    match value {
        0..=23 => 1,
        24..=255 => 2,
        256..=65_535 => 3,
        65_536..=4_294_967_295 => 5,
        _ => 9,
    }
}

fn singleton_literal_size(term: &Term, lexical: &str) -> u64 {
    // The fixed bytes are the list heads, tag, zero prefix, and form code.
    // One lexical length also appears in the bytes head. Each literal uses
    // either a datatype ID or a language table entry and its zero index.
    let length = lexical.len() as u64;
    let form_bytes = if let Some(language) = &term.lang {
        let size = language.len() as u64;
        1 + cbor_head(size) + size
    } else {
        term.datatype.map_or(0, |id| cbor_head(id as u64))
    };
    (12 + 2 * cbor_head(length) + form_bytes).saturating_add(length)
}

fn front_coded_prefix(previous: &[u8], text: &[u8], index: usize) -> usize {
    if index.is_multiple_of(RESTART_GROUP) {
        0
    } else {
        previous
            .iter()
            .zip(text)
            .take_while(|(a, b)| a == b)
            .count()
    }
}

#[derive(Default)]
struct TermBlocks<'a> {
    languages: Vec<Value>,
    language_ids: HashMap<&'a str, usize>,
    formed: u64,
}

impl<'a> TermBlocks<'a> {
    fn language_id(&mut self, text: &'a str) -> usize {
        *self.language_ids.entry(text).or_insert_with(|| {
            let id = self.languages.len();
            self.languages.push(Value::Text(text.to_owned()));
            id
        })
    }

    fn block(&mut self, run: &[StoredTerm<'a>], first: usize) -> Result<Value, ImportError> {
        let kind = run[0].term.kind;
        match kind {
            TermKind::Iri | TermKind::Literal => self.text_block(run, first),
            TermKind::Bnode => {
                let labels = run
                    .iter()
                    .enumerate()
                    .map(|(index, stored)| {
                        term_text(stored.term, first + index)
                            .map(|text| Value::Bytes(text.as_bytes().to_vec()))
                    })
                    .collect::<Result<_, _>>()?;
                Ok(record([Value::from(kind_code(kind)), Value::Array(labels)]))
            }
            TermKind::Triple => {
                let mut subjects = Vec::with_capacity(run.len());
                let mut predicates = Vec::with_capacity(run.len());
                let mut objects = Vec::with_capacity(run.len());
                for (index, stored) in run.iter().enumerate() {
                    let (s, p, o) = stored.term.triple.ok_or(ImportError::IncompleteTerm {
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

    fn text_block(&mut self, run: &[StoredTerm<'a>], first: usize) -> Result<Value, ImportError> {
        let kind = run[0].term.kind;
        let code = run[0].code();
        let mut digests = Vec::new();
        let mut prefixes = Vec::with_capacity(run.len());
        let mut lengths = Vec::with_capacity(run.len());
        let mut suffixes = Vec::new();
        let mut forms = Vec::new();
        let mut datatypes = Vec::new();
        let mut language_indexes = Vec::new();
        let mut previous = &[][..];

        for (index, stored) in run.iter().enumerate() {
            let id = first + index;
            let term = stored.term;
            let text = term_text(term, id)?.as_bytes();
            if let Some(digest) = stored.value {
                digests.push(Value::Bytes(digest.to_vec()));
                lengths.push(uint(text.len()));
            } else {
                let prefix = front_coded_prefix(previous, text, index);
                prefixes.push(uint(prefix));
                self.formed = self.formed.saturating_add(prefix as u64);
                let suffix = &text[prefix..];
                lengths.push(uint(suffix.len()));
                suffixes.extend_from_slice(suffix);
                previous = text;
            }

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

        let mut block = if code == LITERAL_REF {
            vec![
                Value::from(code),
                Value::Array(digests),
                Value::Array(lengths),
            ]
        } else {
            vec![
                Value::from(code),
                Value::Array(prefixes),
                Value::Array(lengths),
                Value::Bytes(suffixes),
            ]
        };
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

fn term_body_size(base: usize, terms: &[StoredTerm<'_>], limit: u64) -> Result<u64, ImportError> {
    let mut languages = HashSet::new();
    let mut size = 0_u64;
    let mut first = base;
    for run in terms.chunk_by(|a, b| a.code() == b.code()) {
        let mut previous = &[][..];
        for (index, stored) in run.iter().enumerate() {
            if stored.term.kind != TermKind::Triple && stored.value.is_none() {
                let text = term_text(stored.term, first + index)?.as_bytes();
                let prefix = if stored.term.kind == TermKind::Bnode {
                    0
                } else {
                    front_coded_prefix(previous, text, index)
                };
                size = size.saturating_add((text.len() - prefix) as u64);
                previous = text;
            }
            if let Some(language) = stored.term.lang.as_deref() {
                if languages.insert(language) {
                    size = size.saturating_add(language.len() as u64);
                }
            }
            if size > limit {
                return Ok(size);
            }
        }
        first += run.len();
    }
    Ok(size)
}

fn append_terms(
    writer: &mut ImportWriter,
    base: usize,
    terms: &[StoredTerm<'_>],
) -> Result<(), ImportError> {
    let limit = writer.payload_limit();
    let size = term_body_size(base, terms, limit)?;
    if size <= limit {
        let (content, payload, formed) = encode_terms(base, terms)?;
        if terms.len() == 1
            || payload.len() as u64 <= limit
                && content.len() as u64 <= writer.capacity.stored_bytes_per_frame
        {
            return writer.append(TERMS, &content, &payload, formed);
        }
    }
    if terms.len() == 1 {
        return Err(ImportError::Capacity {
            resource: "payload bytes in one frame",
            required: size,
            limit,
        });
    }
    let middle = terms.len() / 2;
    let (left, right) = terms.split_at(middle);
    append_terms(writer, base, left)?;
    append_terms(writer, base + middle, right)
}

fn encode_terms(
    base: usize,
    terms: &[StoredTerm<'_>],
) -> Result<(Vec<u8>, Vec<u8>, u64), ImportError> {
    let mut tables = TermBlocks::default();
    let mut runs = Vec::new();
    let mut blocks = Vec::new();
    let mut first = base;
    for run in terms.chunk_by(|a, b| a.code() == b.code()) {
        runs.push(record([Value::from(run[0].code()), uint(run.len())]));
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
    Ok((
        wire::encode(&content),
        wire::encode(&payload),
        tables.formed,
    ))
}

/// Encode indices as differences from the prior index, starting from zero.
///
/// # Errors
///
/// Returns [`ImportError::IndexOutOfRange`] when an index exceeds `i64::MAX`.
fn delta_column(values: impl Iterator<Item = usize>) -> Result<Value, ImportError> {
    let mut deltas = Vec::new();
    let mut prior = 0_i64;
    for index in values {
        let Ok(current) = i64::try_from(index) else {
            return Err(ImportError::IndexOutOfRange { index });
        };
        deltas.push(Value::from(current - prior));
        prior = current;
    }
    Ok(Value::Array(deltas))
}

fn append_quads(writer: &mut ImportWriter, rows: &[Quad]) -> Result<(), ImportError> {
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
        delta_column(rows.iter().map(|row| row.2))?,
        delta_column(rows.iter().map(|row| row.0))?,
    ]);
    writer.append(QUADS, &wire::encode(&content), &wire::encode(&payload), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::{attachments::decode_pack, reader::read_wire};

    #[test]
    fn singleton_storage_choice_matches_encoded_payload_at_the_limit() {
        for suffix in ["", "@en--rtl", "^^<urn:datatype>"] {
            for length in [23, 24, 255, 256] {
                let lexical = "x".repeat(length);
                let source = format!("<urn:s> <urn:p> \"{lexical}\"{suffix} .");
                let (terms, _) = parse_v2_quads(&source).unwrap();
                let term = terms
                    .iter()
                    .find(|term| term.kind == TermKind::Literal)
                    .unwrap();
                let inline = StoredTerm { term, value: None };
                let (_, payload, _) = encode_terms(1000, &[inline]).unwrap();
                let limit = payload.len() as u64;
                assert!(TermPlan::new(&terms, limit).unwrap().values.is_empty());
                assert_eq!(TermPlan::new(&terms, limit - 1).unwrap().values.len(), 1);
            }
        }
    }

    #[test]
    fn imported_literal_forms_share_a_value_and_charge_pack_frames() {
        let length = 1 << 20;
        let lexical = "a".repeat(length);
        let source = format!(
            "<urn:s> <urn:p> \"{lexical}\"^^<urn:datatype> .\n<urn:s> <urn:p> \"{lexical}\"@en--rtl .\n"
        );
        let (terms, quads) = parse_v2_quads(&source).unwrap();
        let capacity = ReadCapacity {
            stored_bytes_per_frame: length as u64,
            ..ReadCapacity::default()
        };
        let data = write_dataset(&terms, quads.clone(), capacity).unwrap();
        let wire = read_wire(&data, capacity);
        let frames = &wire.segments[0].frames;
        assert_eq!(frames[0].frame_type, Some(PACK));
        let frame = frames[0].envelope.as_ref().unwrap();
        let members = decode_pack(frame.content, frame.stored_payload).unwrap();
        assert_eq!(
            members
                .iter()
                .filter(|member| member.kind == super::super::attachments::VALUE)
                .count(),
            1
        );
        let projected = wire.whole_rdf_projection().unwrap().to_nquads().unwrap();
        for line in source.lines() {
            assert!(projected.lines().any(|projected| projected == line));
        }
        let items = ReadCapacity {
            items_per_read: 4,
            ..capacity
        };
        assert!(matches!(
            write_dataset(&terms, quads, items),
            Err(ImportError::Capacity {
                resource: "items",
                ..
            })
        ));
    }
}

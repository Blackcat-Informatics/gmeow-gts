// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{
    collections::{HashMap, HashSet},
    str,
    sync::Arc,
};

use super::{
    rebuild::Rebuild,
    values::{InlineValues, OCTETS},
    ValueIssue,
};
use super::{Direction, FrameIssue, Projector, Term};
use crate::v2::attachments::Reference;
use crate::v2::{Decoder, HeaderError, MalformedReason::*, FORMED_BYTES_PER_READ};

fn read_list<'a, T>(
    cursor: &mut Decoder<'a>,
    mut read: impl FnMut(&mut Decoder<'a>) -> Result<T, HeaderError>,
) -> Result<Vec<T>, HeaderError> {
    let count = cursor.head(4)?;
    (0..count).map(|_| read(cursor)).collect()
}

struct FrontCoded<'a> {
    prefixes: Vec<u64>,
    lengths: Vec<u64>,
    suffixes: &'a [u8],
    formed: Vec<(usize, Vec<u8>)>,
}

impl<'a> FrontCoded<'a> {
    fn read(cursor: &mut Decoder<'a>) -> Result<Self, HeaderError> {
        Ok(Self {
            prefixes: read_list(cursor, Decoder::uint)?,
            lengths: read_list(cursor, Decoder::uint)?,
            suffixes: cursor.byte_string(None)?,
            formed: Vec::new(),
        })
    }

    fn form(&mut self, restart: u64) -> Result<(), HeaderError> {
        let length = self
            .lengths
            .iter()
            .try_fold(0_u64, |total, &length| total.checked_add(length));
        if length != Some(self.suffixes.len() as u64) {
            return Err(HeaderError {
                reason: InconsistentCounts,
                offset: 0,
            });
        }
        let mut suffixes = self.suffixes;
        for (index, (&prefix, &length)) in self.prefixes.iter().zip(&self.lengths).enumerate() {
            let error = |reason| HeaderError {
                reason,
                offset: self.suffixes.len() - suffixes.len(),
            };
            let is_restart = (index as u64).is_multiple_of(restart);
            let previous = self.formed.last().map_or(&[][..], |(_, bytes)| bytes);
            if index != 0 && prefix > previous.len() as u64 {
                return Err(error(OutOfRange));
            }
            if is_restart && prefix != 0 {
                return Err(error(ForbiddenCombination));
            }
            let prefix = usize::try_from(prefix).map_err(|_| error(OutOfRange))?;
            let head = previous.get(..prefix).ok_or_else(|| error(OutOfRange))?;
            let length = usize::try_from(length).map_err(|_| error(InconsistentCounts))?;
            let (tail, rest) = suffixes
                .split_at_checked(length)
                .ok_or_else(|| error(InconsistentCounts))?;
            if !is_restart
                && previous
                    .get(prefix)
                    .zip(tail.first())
                    .is_some_and(|(a, b)| a == b)
            {
                return Err(error(NotDeterministic));
            }
            let mut bytes = Vec::with_capacity(prefix + length);
            bytes.extend_from_slice(head);
            bytes.extend_from_slice(tail);
            self.formed.push((error(WrongType).offset, bytes));
            suffixes = rest;
        }
        Ok(())
    }

    fn texts(self) -> impl Iterator<Item = Result<Arc<str>, HeaderError>> {
        self.formed.into_iter().map(|(offset, bytes)| {
            str::from_utf8(&bytes)
                .map(Arc::from)
                .map_err(|_| HeaderError {
                    reason: WrongType,
                    offset,
                })
        })
    }
}

enum LiteralForm {
    Typed(u64),
    Language(u64, Option<Direction>),
}

enum LiteralStyle {
    Typed(usize),
    Language(Arc<str>, Option<Direction>),
}

impl LiteralStyle {
    fn term(self, lexical: Arc<str>) -> Term {
        match self {
            Self::Typed(datatype) => Term::Literal { lexical, datatype },
            Self::Language(language, direction) => Term::Language {
                lexical,
                language,
                direction,
            },
        }
    }
}

struct LiteralColumns {
    forms: Vec<u64>,
    datatypes: Vec<u64>,
    languages: Vec<u64>,
}

impl LiteralColumns {
    fn read(cursor: &mut Decoder<'_>) -> Result<Self, HeaderError> {
        Ok(Self {
            forms: read_list(cursor, |cursor| cursor.code(4))?,
            datatypes: read_list(cursor, Decoder::uint)?,
            languages: read_list(cursor, Decoder::uint)?,
        })
    }

    fn count_matches(&self, count: u64) -> bool {
        let typed = self.forms.iter().filter(|&&form| form == 1).count();
        self.forms.len() as u64 == count
            && self.datatypes.len() == typed
            && self.languages.len() == self.forms.len() - typed
    }
}

enum Block<'a> {
    Iri(FrontCoded<'a>),
    Literal(FrontCoded<'a>, LiteralColumns),
    Blank(Vec<&'a [u8]>),
    Triple([Vec<u64>; 3]),
    Inline(InlineValues<'a>),
    Reference(ReferenceColumns),
}

struct ReferenceColumns {
    digests: Vec<[u8; 32]>,
    lengths: Vec<u64>,
    forms: Option<LiteralColumns>,
}

impl<'a> Block<'a> {
    fn read(cursor: &mut Decoder<'a>) -> Result<(u64, Self), FrameIssue> {
        let length_error = cursor.error(WrongLength);
        let actual_arity = cursor.head(4)?;
        if actual_arity == 0 {
            return Err(length_error.into());
        }
        let tag = cursor.code(7)?;
        let arity = match tag {
            1 | 4 => 4,
            2 => 7,
            3 | 6 => 2,
            5 => 6,
            7 => 3,
            kind => return Err(FrameIssue::UnsupportedTermKind(kind)),
        };
        if actual_arity != arity {
            return Err(length_error.into());
        }
        let block = match tag {
            1 => Self::Iri(FrontCoded::read(cursor)?),
            2 => Self::Literal(FrontCoded::read(cursor)?, LiteralColumns::read(cursor)?),
            3 => Self::Blank(read_list(cursor, |cursor| cursor.byte_string(None))?),
            6 => Self::Inline(InlineValues::read(cursor)?),
            5 | 7 => Self::Reference(ReferenceColumns {
                digests: read_list(cursor, |cursor| cursor.fixed::<32>().copied())?,
                lengths: read_list(cursor, Decoder::uint)?,
                forms: (tag == 5)
                    .then(|| LiteralColumns::read(cursor))
                    .transpose()?,
            }),
            _ => Self::Triple([
                read_list(cursor, Decoder::uint)?,
                read_list(cursor, Decoder::uint)?,
                read_list(cursor, Decoder::uint)?,
            ]),
        };
        Ok((tag, block))
    }

    fn prefixes(&self) -> &[u64] {
        match self {
            Self::Iri(texts) | Self::Literal(texts, _) => &texts.prefixes,
            _ => &[],
        }
    }

    fn count_matches(&self, count: u64) -> bool {
        let same = |length: usize| length as u64 == count;
        let front =
            |codes: &FrontCoded<'_>| same(codes.prefixes.len()) && same(codes.lengths.len());
        match self {
            Self::Iri(codes) => front(codes),
            Self::Literal(codes, forms) => front(codes) && forms.count_matches(count),
            Self::Blank(labels) => same(labels.len()),
            Self::Triple(columns) => columns.iter().all(|column| same(column.len())),
            Self::Inline(values) => values.count_matches(count),
            Self::Reference(references) => {
                same(references.digests.len())
                    && same(references.lengths.len())
                    && references
                        .forms
                        .as_ref()
                        .is_none_or(|forms| forms.count_matches(count))
            }
        }
    }
}

fn literal_forms(columns: LiteralColumns) -> impl Iterator<Item = Option<LiteralForm>> {
    let mut datatypes = columns.datatypes.into_iter();
    let mut languages = columns.languages.into_iter();
    columns.forms.into_iter().map(move |form| {
        if form == 1 {
            datatypes.next().map(LiteralForm::Typed)
        } else {
            let direction = match form {
                3 => Some(Direction::Ltr),
                4 => Some(Direction::Rtl),
                _ => None,
            };
            languages
                .next()
                .map(|id| LiteralForm::Language(id, direction))
        }
    })
}

struct Languages {
    entries: Vec<Arc<str>>,
}

impl Languages {
    fn read(cursor: &mut Decoder<'_>) -> Result<Self, HeaderError> {
        Ok(Self {
            entries: read_list(cursor, Decoder::text)?
                .into_iter()
                .map(Arc::from)
                .collect(),
        })
    }

    fn get(&self, id: u64, cursor: &Decoder<'_>) -> Result<Arc<str>, HeaderError> {
        let index = usize::try_from(id).map_err(|_| cursor.error(OutOfRange))?;
        let text = self
            .entries
            .get(index)
            .ok_or_else(|| cursor.error(OutOfRange))?;
        Ok(Arc::clone(text))
    }

    fn check(&self, blocks: &[(u64, Block<'_>)], cursor: &Decoder<'_>) -> Result<(), HeaderError> {
        let indexes = || {
            blocks
                .iter()
                .filter_map(|(_, block)| match block {
                    Block::Literal(_, forms) => Some(forms),
                    Block::Reference(references) => references.forms.as_ref(),
                    _ => None,
                })
                .flat_map(|forms| forms.languages.iter().copied())
        };
        if indexes().any(|id| id >= self.entries.len() as u64) {
            return Err(cursor.error(OutOfRange));
        }
        let mut used = 0_u64;
        for id in indexes() {
            if id > used {
                return Err(cursor.error(NotDeterministic));
            }
            if id == used {
                used += 1;
            }
        }
        if used != self.entries.len() as u64
            || self.entries.iter().collect::<HashSet<_>>().len() != self.entries.len()
        {
            return Err(cursor.error(NotDeterministic));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Definition {
    rdf: usize,
    written: usize,
}

#[derive(Hash, PartialEq, Eq)]
struct WrittenKey {
    kind: u64,
    rdf: usize,
    components: Option<[usize; 3]>,
    value: Option<usize>,
}

#[derive(Default)]
pub(super) struct Dictionary {
    entries: Vec<Definition>,
    written: HashMap<WrittenKey, usize>,
}

impl Dictionary {
    fn push(&mut self, kind: u64, rdf: usize, components: Option<[usize; 3]>) {
        self.push_key(WrittenKey {
            kind,
            rdf,
            components,
            value: None,
        });
    }

    fn push_key(&mut self, key: WrittenKey) {
        let next = self.written.len();
        let rdf = key.rdf;
        let written = *self.written.entry(key).or_insert(next);
        self.entries.push(Definition { rdf, written });
    }

    fn get(&self, id: u64) -> Option<Definition> {
        usize::try_from(id)
            .ok()
            .and_then(|id| self.entries.get(id).copied())
    }

    pub(super) fn written(&self, id: u64) -> Result<usize, u64> {
        self.get(id).map(|term| term.written).ok_or(id)
    }
}

pub(super) fn resolve(
    dictionary: &Dictionary,
    id: u64,
    cursor: &Decoder<'_>,
) -> Result<usize, HeaderError> {
    dictionary
        .get(id)
        .map(|term| term.rdf)
        .ok_or_else(|| cursor.error(OutOfRange))
}

impl Projector {
    pub(super) fn read_terms(
        &mut self,
        dictionary: &mut Dictionary,
        segment: usize,
        content: &[u8],
        bytes: &[u8],
        rebuild: &mut Rebuild<'_>,
    ) -> Result<(), FrameIssue> {
        let mut fields = Decoder::new(content, 0);
        fields.record(4)?;
        if fields.uint()? != dictionary.entries.len() as u64 {
            return Err(fields.error(InconsistentCounts).into());
        }
        let runs = fields.list(|cursor| {
            cursor.record(2)?;
            Ok((cursor.code(7)?, cursor.positive()?))
        })?;
        if runs.is_empty() {
            return Err(fields.error(OutOfRange).into());
        }
        if runs.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            return Err(fields.error(NotDeterministic).into());
        }
        fields.code(2)?;
        let restart = fields.positive()?;
        fields.end()?;

        let mut cursor = Decoder::new(bytes, 0);
        cursor.record(2)?;
        let count = cursor.head(4)?;
        let mut blocks = (0..count)
            .map(|_| Block::read(&mut cursor))
            .collect::<Result<Vec<_>, _>>()?;
        let languages = Languages::read(&mut cursor)?;
        cursor.end()?;
        if blocks.len() != runs.len()
            || blocks
                .iter()
                .zip(&runs)
                .any(|((_, block), &(_, count))| !block.count_matches(count))
        {
            return Err(cursor.error(InconsistentCounts).into());
        }
        if blocks
            .iter()
            .zip(runs)
            .any(|((tag, _), (kind, _))| *tag != kind)
        {
            return Err(cursor.error(ForbiddenCombination).into());
        }
        self.formed_bytes = blocks
            .iter()
            .flat_map(|(_, block)| block.prefixes())
            .try_fold(self.formed_bytes, |total, &prefix| {
                total
                    .checked_add(prefix)
                    .filter(|&next| next <= FORMED_BYTES_PER_READ)
            })
            .ok_or(FrameIssue::FormedCapacity)?;
        for (_, block) in &mut blocks {
            if let Block::Iri(texts) | Block::Literal(texts, _) = block {
                texts.form(restart)?;
            }
        }
        languages.check(&blocks, &cursor)?;

        for (kind, block) in blocks {
            match block {
                Block::Iri(texts) => {
                    for text in texts.texts() {
                        dictionary.push(kind, self.intern(Term::Iri(text?)), None);
                    }
                }
                Block::Literal(texts, forms) => {
                    for (lexical, form) in texts.texts().zip(literal_forms(forms)) {
                        let lexical = lexical?;
                        let style = self.literal_style(dictionary, &languages, &cursor, form)?;
                        dictionary.push(kind, self.intern(style.term(lexical)), None);
                    }
                }
                Block::Blank(labels) => {
                    for label in labels {
                        dictionary.push(
                            kind,
                            self.intern(Term::Blank {
                                segment,
                                label: Arc::from(label),
                            }),
                            None,
                        );
                    }
                }
                Block::Triple([subjects, predicates, objects]) => {
                    for ((subject, predicate), object) in
                        subjects.into_iter().zip(predicates).zip(objects)
                    {
                        let triple = (
                            self.node(dictionary, subject, &cursor)?,
                            self.iri(dictionary, predicate, &cursor)?,
                            resolve(dictionary, object, &cursor)?,
                        );
                        let components =
                            [subject, predicate, object].map(|id| dictionary.written(id));
                        let [Ok(subject), Ok(predicate), Ok(object)] = components else {
                            return Err(cursor.error(OutOfRange).into());
                        };
                        dictionary.push(
                            kind,
                            self.intern(Term::Triple(triple)),
                            Some([subject, predicate, object]),
                        );
                    }
                }
                Block::Inline(values) => {
                    for value in values.project() {
                        let (lexical, datatype) = value?;
                        let datatype = self.intern(Term::Iri(Arc::from(datatype)));
                        dictionary.push(
                            kind,
                            self.intern(Term::Literal { lexical, datatype }),
                            None,
                        );
                    }
                }
                Block::Reference(columns) => {
                    self.add_references(dictionary, &languages, &cursor, rebuild, columns)?;
                }
            }
        }
        Ok(())
    }

    fn literal_style(
        &self,
        dictionary: &Dictionary,
        languages: &Languages,
        cursor: &Decoder<'_>,
        form: Option<LiteralForm>,
    ) -> Result<LiteralStyle, HeaderError> {
        match form.ok_or_else(|| cursor.error(InconsistentCounts))? {
            LiteralForm::Typed(id) => Ok(LiteralStyle::Typed(self.iri(dictionary, id, cursor)?)),
            LiteralForm::Language(id, direction) => Ok(LiteralStyle::Language(
                languages.get(id, cursor)?,
                direction,
            )),
        }
    }

    fn add_references(
        &mut self,
        dictionary: &mut Dictionary,
        languages: &Languages,
        cursor: &Decoder<'_>,
        rebuild: &mut Rebuild<'_>,
        columns: ReferenceColumns,
    ) -> Result<(), FrameIssue> {
        let mut forms = columns.forms.map(literal_forms);
        let octets = forms.is_none();
        for (digest, length) in columns.digests.into_iter().zip(columns.lengths) {
            let reference = Reference { digest, length };
            let style = if let Some(forms) = &mut forms {
                let checked = rebuild.check_text(reference);
                if checked == Err(ValueIssue::InvalidUtf8) {
                    return Err(ValueIssue::InvalidUtf8.into());
                }
                let style =
                    self.literal_style(dictionary, languages, cursor, forms.next().flatten())?;
                checked?;
                style
            } else {
                LiteralStyle::Typed(self.intern(Term::Iri(Arc::from(OCTETS))))
            };
            let (value, lexical) = rebuild.lexical(reference, octets)?;
            let rdf = self.intern(style.term(lexical));
            dictionary.push_key(WrittenKey {
                kind: if octets { 7 } else { 5 },
                rdf,
                components: None,
                value: Some(value),
            });
        }
        Ok(())
    }

    pub(super) fn iri(
        &self,
        dictionary: &Dictionary,
        id: u64,
        cursor: &Decoder<'_>,
    ) -> Result<usize, HeaderError> {
        self.position(dictionary, id, cursor, |term| matches!(term, Term::Iri(_)))
    }

    pub(super) fn node(
        &self,
        dictionary: &Dictionary,
        id: u64,
        cursor: &Decoder<'_>,
    ) -> Result<usize, HeaderError> {
        self.position(dictionary, id, cursor, |term| {
            matches!(term, Term::Iri(_) | Term::Blank { .. })
        })
    }

    pub(super) fn position(
        &self,
        dictionary: &Dictionary,
        id: u64,
        cursor: &Decoder<'_>,
        valid: impl FnOnce(&Term) -> bool,
    ) -> Result<usize, HeaderError> {
        let index = resolve(dictionary, id, cursor)?;
        if valid(&self.projection.terms[index]) {
            Ok(index)
        } else {
            Err(cursor.error(ForbiddenCombination))
        }
    }
}

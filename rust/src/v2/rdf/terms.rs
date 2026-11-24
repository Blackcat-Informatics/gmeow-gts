// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{collections::HashSet, str, sync::Arc};

use super::{Direction, FrameIssue, Projector, Term, FORMED_BYTES_PER_READ};
use crate::v2::{Decoder, HeaderError, MalformedReason::*};

struct FrontCoded<'a> {
    prefixes: Vec<u64>,
    lengths: Vec<u64>,
    suffixes: &'a [u8],
}

impl<'a> FrontCoded<'a> {
    fn read(cursor: &mut Decoder<'a>, count: u64) -> Result<Self, HeaderError> {
        Ok(Self {
            prefixes: cursor.column(count, Decoder::uint)?,
            lengths: cursor.column(count, Decoder::uint)?,
            suffixes: cursor.byte_string(None)?,
        })
    }

    fn texts(self, restart: u64) -> Result<Vec<Arc<str>>, HeaderError> {
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
        let mut texts: Vec<Arc<str>> = Vec::new();
        for (index, (prefix, length)) in self.prefixes.into_iter().zip(self.lengths).enumerate() {
            let error = |reason| HeaderError {
                reason,
                offset: self.suffixes.len() - suffixes.len(),
            };
            let is_restart = (index as u64).is_multiple_of(restart);
            if is_restart && prefix != 0 {
                return Err(error(ForbiddenCombination));
            }
            let previous = texts.last().map_or(&[][..], |text| text.as_bytes());
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
            let text = str::from_utf8(&bytes).map_err(|_| error(WrongType))?;
            texts.push(Arc::from(text));
            suffixes = rest;
        }
        Ok(texts)
    }
}

enum LiteralForm {
    Typed(u64),
    Language(u64, Option<Direction>),
}

enum Block<'a> {
    Iri(FrontCoded<'a>),
    Literal(FrontCoded<'a>, Vec<LiteralForm>),
    Blank(Vec<&'a [u8]>),
    Triple([Vec<u64>; 3]),
}

impl<'a> Block<'a> {
    fn read(cursor: &mut Decoder<'a>, count: u64) -> Result<(u64, Self), FrameIssue> {
        let length_error = cursor.error(WrongLength);
        let actual_arity = cursor.head(4)?;
        if actual_arity == 0 {
            return Err(length_error.into());
        }
        let tag = cursor.code(7)?;
        let arity = match tag {
            1 | 4 => 4,
            2 => 7,
            3 => 2,
            kind => return Err(FrameIssue::UnsupportedTermKind(kind)),
        };
        if actual_arity != arity {
            return Err(length_error.into());
        }
        let block = match tag {
            1 => Self::Iri(FrontCoded::read(cursor, count)?),
            2 => Self::Literal(FrontCoded::read(cursor, count)?, read_forms(cursor, count)?),
            3 => Self::Blank(cursor.column(count, |cursor| cursor.byte_string(None))?),
            _ => Self::Triple([
                cursor.column(count, Decoder::uint)?,
                cursor.column(count, Decoder::uint)?,
                cursor.column(count, Decoder::uint)?,
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
}

fn read_forms(cursor: &mut Decoder<'_>, count: u64) -> Result<Vec<LiteralForm>, HeaderError> {
    let forms = cursor.column(count, |cursor| cursor.code(4))?;
    let typed = forms.iter().filter(|&&form| form == 1).count() as u64;
    let mut datatypes = cursor.column(typed, Decoder::uint)?.into_iter();
    let mut languages = cursor.column(count - typed, Decoder::uint)?.into_iter();
    let missing = cursor.error(InconsistentCounts);
    forms
        .into_iter()
        .map(|form| {
            if form == 1 {
                datatypes.next().map(LiteralForm::Typed).ok_or(missing)
            } else {
                let direction = match form {
                    3 => Some(Direction::Ltr),
                    4 => Some(Direction::Rtl),
                    _ => None,
                };
                languages
                    .next()
                    .map(|id| LiteralForm::Language(id, direction))
                    .ok_or(missing)
            }
        })
        .collect()
}

struct Languages {
    entries: Vec<Arc<str>>,
    used: usize,
}

impl Languages {
    fn read(cursor: &mut Decoder<'_>) -> Result<Self, HeaderError> {
        Ok(Self {
            entries: cursor
                .list(Decoder::text)?
                .into_iter()
                .map(Arc::from)
                .collect(),
            used: 0,
        })
    }

    fn get(&mut self, id: u64, cursor: &Decoder<'_>) -> Result<Arc<str>, HeaderError> {
        let index = usize::try_from(id).map_err(|_| cursor.error(OutOfRange))?;
        let text = self
            .entries
            .get(index)
            .ok_or_else(|| cursor.error(OutOfRange))?;
        if index > self.used {
            return Err(cursor.error(NotDeterministic));
        }
        if index == self.used {
            self.used += 1;
        }
        Ok(Arc::clone(text))
    }
}

pub(super) fn resolve(
    dictionary: &[usize],
    id: u64,
    cursor: &Decoder<'_>,
) -> Result<usize, HeaderError> {
    usize::try_from(id)
        .ok()
        .and_then(|index| dictionary.get(index).copied())
        .ok_or_else(|| cursor.error(OutOfRange))
}

impl Projector {
    pub(super) fn read_terms(
        &mut self,
        dictionary: &mut Vec<usize>,
        segment: usize,
        content: &[u8],
        bytes: &[u8],
    ) -> Result<(), FrameIssue> {
        let mut fields = Decoder::new(content, 0);
        fields.record(4)?;
        if fields.uint()? != dictionary.len() as u64 {
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
        cursor.column_len(runs.len() as u64)?;
        let blocks = runs
            .iter()
            .map(|&(_, count)| Block::read(&mut cursor, count))
            .collect::<Result<Vec<_>, _>>()?;
        let mut languages = Languages::read(&mut cursor)?;
        cursor.end()?;
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

        for (_, block) in blocks {
            match block {
                Block::Iri(texts) => {
                    for text in texts.texts(restart)? {
                        dictionary.push(self.intern(Term::Iri(text)));
                    }
                }
                Block::Literal(texts, forms) => {
                    for (lexical, form) in texts.texts(restart)?.into_iter().zip(forms) {
                        let term = match form {
                            LiteralForm::Typed(id) => Term::Literal {
                                lexical,
                                datatype: self.iri(dictionary, id, &cursor)?,
                            },
                            LiteralForm::Language(id, direction) => Term::Language {
                                lexical,
                                language: languages.get(id, &cursor)?,
                                direction,
                            },
                        };
                        dictionary.push(self.intern(term));
                    }
                }
                Block::Blank(labels) => {
                    for label in labels {
                        dictionary.push(self.intern(Term::Blank {
                            segment,
                            label: Arc::from(label),
                        }));
                    }
                }
                Block::Triple([subjects, predicates, objects]) => {
                    for ((subject, predicate), object) in
                        subjects.into_iter().zip(predicates).zip(objects)
                    {
                        let subject = self.node(dictionary, subject, &cursor)?;
                        let predicate = self.iri(dictionary, predicate, &cursor)?;
                        let object = resolve(dictionary, object, &cursor)?;
                        dictionary.push(self.intern(Term::Triple((subject, predicate, object))));
                    }
                }
            }
        }
        if languages.used != languages.entries.len()
            || languages.entries.iter().collect::<HashSet<_>>().len() != languages.entries.len()
        {
            return Err(cursor.error(NotDeterministic).into());
        }
        Ok(())
    }

    pub(super) fn iri(
        &self,
        dictionary: &[usize],
        id: u64,
        cursor: &Decoder<'_>,
    ) -> Result<usize, HeaderError> {
        self.position(dictionary, id, cursor, |term| matches!(term, Term::Iri(_)))
    }

    pub(super) fn node(
        &self,
        dictionary: &[usize],
        id: u64,
        cursor: &Decoder<'_>,
    ) -> Result<usize, HeaderError> {
        self.position(dictionary, id, cursor, |term| {
            matches!(term, Term::Iri(_) | Term::Blank { .. })
        })
    }

    pub(super) fn position(
        &self,
        dictionary: &[usize],
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

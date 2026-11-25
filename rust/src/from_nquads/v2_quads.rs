// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Intern N-Quads statements for the v2 RDF import mapping.

use super::{Interner, NQuadsParseError, Tokenizer};
use crate::model::{Quad, Term, TermKind, XSD_STRING};

pub(crate) fn parse_v2_quads(text: &str) -> Result<(Vec<Term>, Vec<Quad>), NQuadsParseError> {
    let mut interner = Interner::new(Some(XSD_STRING));
    let mut rows = Vec::new();
    for line in text.split(['\r', '\n']) {
        let mut tokenizer = Tokenizer {
            text: line,
            pos: 0,
            rdf12: true,
        };
        tokenizer.skip_ws();
        match tokenizer.peek_char() {
            None | Some('#') => continue,
            _ => {}
        }
        if tokenizer.consume("VERSION") {
            tokenizer.skip_ws();
            tokenizer.string()?;
            tokenizer.finish_line()?;
        } else {
            rows.push(tokenizer.quad(&mut interner)?);
        }
    }
    Ok((interner.terms, rows))
}

impl Tokenizer<'_> {
    fn consume(&mut self, token: &str) -> bool {
        if !self.rest().starts_with(token) {
            return false;
        }
        self.pos += token.len();
        true
    }

    fn finish_line(&mut self) -> Result<(), NQuadsParseError> {
        self.skip_ws();
        if matches!(self.peek_char(), None | Some('#')) {
            Ok(())
        } else {
            Err(NQuadsParseError::new(format!(
                "unexpected text after statement in {:?}",
                self.text
            )))
        }
    }

    fn quad(&mut self, interner: &mut Interner) -> Result<Quad, NQuadsParseError> {
        let subject = self.resource(interner, false)?;
        let predicate = self.resource(interner, true)?;
        let object = self.object(interner)?;
        self.skip_ws();
        let graph = if matches!(self.peek_char(), Some('<' | '_')) {
            let graph = self.resource(interner, false)?;
            self.skip_ws();
            Some(graph)
        } else {
            None
        };
        if !self.consume(".") {
            return Err(NQuadsParseError::new(format!(
                "expected '.' at the end of {:?}",
                self.text
            )));
        }
        self.finish_line()?;
        Ok((subject, predicate, object, graph))
    }

    fn resource(
        &mut self,
        interner: &mut Interner,
        iri_only: bool,
    ) -> Result<usize, NQuadsParseError> {
        self.skip_ws();
        let atom = self.atom()?;
        if atom.kind == TermKind::Iri || (!iri_only && atom.kind == TermKind::Bnode) {
            Ok(interner.atom(&atom))
        } else {
            let expected = if iri_only {
                "an IRI predicate"
            } else {
                "an IRI or blank node"
            };
            Err(NQuadsParseError::new(format!(
                "expected {expected} in {:?}",
                self.text
            )))
        }
    }

    fn object(&mut self, interner: &mut Interner) -> Result<usize, NQuadsParseError> {
        let mut parents = Vec::new();
        self.skip_ws();
        // Only the object can itself be a triple term. Intern the left
        // components on descent and the triple IDs on the way back out.
        while self.consume("<<(") {
            let subject = self.resource(interner, false)?;
            let predicate = self.resource(interner, true)?;
            parents.push((subject, predicate));
            self.skip_ws();
        }
        let mut object = interner.atom(&self.atom()?);
        for (subject, predicate) in parents.into_iter().rev() {
            self.close_triple()?;
            object = interner.triple(subject, predicate, object);
        }
        Ok(object)
    }

    pub(super) fn rdf12_iri(&mut self) -> Result<String, NQuadsParseError> {
        self.pos += 1;
        let mut text = String::new();
        while let Some(ch) = self.bump_char() {
            match ch {
                '>' => {
                    oxiri::Iri::parse(text.as_str()).map_err(|error| {
                        NQuadsParseError::new(format!(
                            "invalid RDF IRI in {:?}: {error}",
                            self.text
                        ))
                    })?;
                    return Ok(text);
                }
                '\\' => match self.bump_char() {
                    Some(width @ ('u' | 'U')) => text.push(self.unicode_escape(width)?),
                    _ => {
                        return Err(NQuadsParseError::new(format!(
                            "invalid IRI escape in {:?}",
                            self.text
                        )))
                    }
                },
                ch => text.push(ch),
            }
        }
        Err(NQuadsParseError::new(format!(
            "unterminated IRI in {:?}",
            self.text
        )))
    }

    pub(super) fn rdf12_blank_label(&mut self) -> Result<String, NQuadsParseError> {
        let start = self.pos;
        if !self.peek_char().is_some_and(blank_start) {
            return Err(NQuadsParseError::new(format!(
                "invalid blank node label in {:?}",
                self.text
            )));
        }
        self.bump_char();
        let mut end = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch == '.' {
                self.pos += 1;
            } else if blank_continue(ch) {
                self.pos += ch.len_utf8();
                end = self.pos;
            } else {
                break;
            }
        }
        self.pos = end;
        Ok(self.span(start, end)?.to_owned())
    }
}

fn blank_start(ch: char) -> bool {
    matches!(
        ch,
        '0'..='9' | '_' | 'A'..='Z' | 'a'..='z'
            | '\u{00c0}'..='\u{00d6}' | '\u{00d8}'..='\u{00f6}' | '\u{00f8}'..='\u{02ff}'
            | '\u{0370}'..='\u{037d}' | '\u{037f}'..='\u{1fff}'
            | '\u{200c}'..='\u{200d}' | '\u{2070}'..='\u{218f}'
            | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}'
            | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}' | '\u{10000}'..='\u{effff}'
    )
}

fn blank_continue(ch: char) -> bool {
    blank_start(ch)
        || matches!(ch, '-' | '\u{00b7}' | '\u{0300}'..='\u{036f}' | '\u{203f}'..='\u{2040}')
}

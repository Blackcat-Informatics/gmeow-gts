// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{error::Error, fmt};

use super::{Direction, RdfProjection, Term};
use crate::{
    model::{RDF_DIR_LANG_STRING, RDF_LANG_STRING, XSD_STRING},
    nquads::escape_literal,
};

/// A projected term whose text cannot be written as N-Quads.
#[derive(Debug)]
pub enum NQuadsExportError {
    /// An IRI in the term is malformed.
    Iri(oxiri::IriParseError),
    /// A language tag in the term is malformed.
    Language(oxilangtag::LanguageTagParseError),
    /// The term at the given index cannot be written as N-Quads.
    InvalidTerm(usize),
    /// The term at the given index pairs a language tag with a datatype.
    LanguageDatatype(usize),
}

impl fmt::Display for NQuadsExportError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Iri(error) => write!(out, "invalid RDF IRI: {error}"),
            Self::Language(error) => write!(out, "invalid RDF language tag: {error}"),
            Self::InvalidTerm(id) => write!(out, "term {id} breaks an RDF position rule"),
            Self::LanguageDatatype(id) => {
                write!(out, "term {id} has a language datatype without a language")
            }
        }
    }
}

impl Error for NQuadsExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Iri(error) => Some(error),
            Self::Language(error) => Some(error),
            Self::InvalidTerm(_) | Self::LanguageDatatype(_) => None,
        }
    }
}

fn iri(out: &mut String, text: &str) -> Result<(), NQuadsExportError> {
    oxiri::Iri::parse(text).map_err(NQuadsExportError::Iri)?;
    out.push('<');
    out.push_str(text);
    out.push('>');
    Ok(())
}

fn literal(out: &mut String, lexical: &str) {
    out.push('"');
    out.push_str(&escape_literal(lexical));
    out.push('"');
}

impl RdfProjection {
    /// Encode the projected quads as N-Quads. Empty named graphs are available
    /// through [Self::empty_graphs].
    pub fn to_nquads(&self) -> Result<String, NQuadsExportError> {
        let mut out = String::new();
        for &(subject, predicate, object, graph) in &self.quads {
            self.write_term(&mut out, subject)?;
            out.push(' ');
            self.write_term(&mut out, predicate)?;
            out.push(' ');
            self.write_term(&mut out, object)?;
            if let Some(graph) = graph {
                out.push(' ');
                self.write_term(&mut out, graph)?;
            }
            out.push_str(" .\n");
        }
        Ok(out)
    }

    fn write_term(&self, out: &mut String, mut id: usize) -> Result<(), NQuadsExportError> {
        let mut depth = 0;
        while let Term::Triple((subject, predicate, object)) = self.terms[id] {
            // Triple terms can nest only through the object. Each opener
            // writes two atomic terms and leaves one closer for the end.
            out.push_str("<<( ");
            self.write_atom(out, subject)?;
            out.push(' ');
            self.write_atom(out, predicate)?;
            out.push(' ');
            depth += 1;
            id = object;
        }
        self.write_atom(out, id)?;
        for _ in 0..depth {
            out.push_str(" )>>");
        }
        Ok(())
    }

    fn write_atom(&self, out: &mut String, id: usize) -> Result<(), NQuadsExportError> {
        match &self.terms[id] {
            Term::Iri(text) => iri(out, text)?,
            Term::Blank { .. } => {
                out.push_str("_:b");
                out.push_str(&id.to_string());
            }
            Term::Literal { lexical, datatype } => {
                let Term::Iri(datatype) = &self.terms[*datatype] else {
                    return Err(NQuadsExportError::InvalidTerm(*datatype));
                };
                if matches!(datatype.as_ref(), RDF_LANG_STRING | RDF_DIR_LANG_STRING) {
                    return Err(NQuadsExportError::LanguageDatatype(id));
                }
                literal(out, lexical);
                if datatype.as_ref() != XSD_STRING {
                    out.push_str("^^");
                    iri(out, datatype)?;
                }
            }
            Term::Language {
                lexical,
                language,
                direction,
            } => {
                oxilangtag::LanguageTag::parse(language.as_ref())
                    .map_err(NQuadsExportError::Language)?;
                literal(out, lexical);
                out.push('@');
                out.push_str(language);
                match direction {
                    Some(Direction::Ltr) => out.push_str("--ltr"),
                    Some(Direction::Rtl) => out.push_str("--rtl"),
                    None => {}
                }
            }
            Term::Triple(_) => return Err(NQuadsExportError::InvalidTerm(id)),
        }
        Ok(())
    }
}

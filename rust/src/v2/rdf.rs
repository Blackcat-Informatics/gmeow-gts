// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Project checked GTS v2 RDF rows and their segment dictionaries (P3 §10a).

mod nquads;
mod rebuild;
mod rows;
mod terms;
mod values;

pub use nquads::NQuadsExportError;

use std::{
    collections::{BTreeSet, HashMap},
    error::Error,
    fmt,
    sync::Arc,
};

use super::{
    attachments::{BLOB, CHUNK, PACK, RECIPE, VALUE},
    frame::{FrameError, COMMIT, PARITY},
    reader::{
        Dispatch, ItemKind, LinkState, Opener, PayloadDamage, PayloadRefusal, PayloadState,
        ReadStop, WireRead,
    },
    HeaderError, REGISTRY_VERSION,
};
use crate::model::{Quad, Triple3};

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const DEFAULT_VALUE_TEXT_LIMIT: u64 = 256 << 20;

/// The direction of a GTS language literal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    Ltr,
    Rtl,
}

/// A term in an RDF projection. An index refers to the projection's term table.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Term {
    Iri(Arc<str>),
    Literal {
        lexical: Arc<str>,
        datatype: usize,
    },
    Language {
        lexical: Arc<str>,
        language: Arc<str>,
        direction: Option<Direction>,
    },
    Blank {
        segment: usize,
        label: Arc<[u8]>,
    },
    Triple(Triple3),
}

/// The quads and empty named graphs of a GTS v2 RDF projection.
///
/// Term indices are local to this result. Quoted triples refer to their
/// components by index, so they can be inspected without recursion.
#[derive(Debug, Default)]
pub struct RdfProjection {
    terms: Vec<Term>,
    quads: BTreeSet<Quad>,
    empty_graphs: BTreeSet<usize>,
}

impl RdfProjection {
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }

    /// The set of (subject, predicate, object, optional graph) term indices.
    pub fn quads(&self) -> &BTreeSet<Quad> {
        &self.quads
    }

    /// Graph name indices for named graphs that carry no statements.
    pub fn empty_graphs(&self) -> &BTreeSet<usize> {
        &self.empty_graphs
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameIssue {
    Envelope(FrameError),
    Link(LinkState),
    StoredCheck,
    DamagedPayload(PayloadDamage),
    UncheckedPayload(PayloadRefusal),
    Content(HeaderError),
    UnsupportedFrameType(u64),
    UnsupportedTermKind(u64),
    FormedCapacity,
    Value(ValueIssue),
}

/// A value reference that the RDF projection cannot represent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueIssue {
    MissingValue,
    MissingUnit,
    UnitLength,
    ValueDigest,
    InvalidUtf8,
    RebuiltCapacity,
    MaterializedCapacity,
}

impl From<ValueIssue> for FrameIssue {
    fn from(issue: ValueIssue) -> Self {
        Self::Value(issue)
    }
}

impl From<HeaderError> for FrameIssue {
    fn from(error: HeaderError) -> Self {
        Self::Content(error)
    }
}

/// The condition that prevented a complete RDF projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    Generation(Option<Dispatch>),
    Framing(ReadStop),
    Opener {
        segment: usize,
    },
    Item {
        segment: usize,
        offset: usize,
        kind: ItemKind,
    },
    Frame {
        segment: usize,
        position: usize,
        issue: FrameIssue,
    },
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generation(dispatch) => {
                write!(out, "expected GTS generation 2, found {dispatch:?}")
            }
            Self::Framing(stop) => write!(out, "GTS v2 framing stopped at {stop:?}"),
            Self::Opener { segment } => {
                write!(out, "unreadable or unsupported opener in segment {segment}")
            }
            Self::Item {
                segment,
                offset,
                kind,
            } => write!(out, "segment {segment} item at byte {offset}: {kind:?}"),
            Self::Frame {
                segment,
                position,
                issue,
            } => write!(out, "segment {segment} frame at item {position}: {issue:?}"),
        }
    }
}

impl Error for ProjectionError {}

#[derive(Default)]
struct Projector {
    projection: RdfProjection,
    interned: HashMap<Term, usize>,
    formed_bytes: u64,
}

impl Projector {
    fn intern(&mut self, term: Term) -> usize {
        *self.interned.entry(term.clone()).or_insert_with(|| {
            let id = self.projection.terms.len();
            self.projection.terms.push(term);
            id
        })
    }

    fn finish(mut self) -> RdfProjection {
        for &(_, _, _, graph) in &self.projection.quads {
            if let Some(graph) = graph {
                self.projection.empty_graphs.remove(&graph);
            }
        }
        self.projection
    }
}

impl WireRead<'_> {
    /// Compute the whole RDF projection of header-led registry version 1
    /// segments with intact wire checks.
    ///
    /// Supports inline and value-backed literals and all four RDF row types.
    /// A suppression frame or unreadable term causes refusal. The projection
    /// returns RDF only and defaults to 256 MiB of materialized reference text.
    pub fn whole_rdf_projection(&self) -> Result<RdfProjection, ProjectionError> {
        self.whole_rdf_projection_with_value_limit(DEFAULT_VALUE_TEXT_LIMIT)
    }

    /// Project RDF with a caller-supplied aggregate limit on materialized
    /// reference text. Repeated use of cached lexical or hex text costs only
    /// once per representation. The P4 rebuild budget applies independently.
    pub fn whole_rdf_projection_with_value_limit(
        &self,
        value_text_bytes: u64,
    ) -> Result<RdfProjection, ProjectionError> {
        if self.dispatch != Some(Dispatch::SecondGeneration) {
            return Err(ProjectionError::Generation(self.dispatch));
        }
        if let Some(stop) = self.stop {
            return Err(ProjectionError::Framing(stop));
        }
        let mut projector = Projector::default();
        let mut rebuild = rebuild::Rebuild::new(value_text_bytes);
        for (segment, wire) in self.segments.iter().enumerate() {
            let Opener::Header(Ok(opener)) = wire.opener else {
                return Err(ProjectionError::Opener { segment });
            };
            if opener.identifier.is_none() || opener.header.registry_version != REGISTRY_VERSION {
                return Err(ProjectionError::Opener { segment });
            }
            if let Some(item) = wire.items.iter().find(|item| {
                !matches!(
                    item.kind,
                    ItemKind::Header | ItemKind::Frame | ItemKind::Locator | ItemKind::Offsets
                )
            }) {
                return Err(ProjectionError::Item {
                    segment,
                    offset: item.offset,
                    kind: item.kind,
                });
            }
            let mut dictionary = terms::Dictionary::default();
            rebuild.start_segment();
            for observed in &wire.frames {
                let at = |issue| ProjectionError::Frame {
                    segment,
                    position: observed.position,
                    issue,
                };
                let frame = observed
                    .envelope
                    .as_ref()
                    .map_err(|error| at(FrameIssue::Envelope(*error)))?;
                if observed.link.state != LinkState::Linked {
                    return Err(at(FrameIssue::Link(observed.link.state)));
                }
                if observed.payload.stored_matches == Some(false) {
                    return Err(at(FrameIssue::StoredCheck));
                }
                let bytes = match &observed.payload.state {
                    PayloadState::Verified(bytes) => bytes,
                    PayloadState::Damaged(reason) => {
                        return Err(at(FrameIssue::DamagedPayload(*reason)))
                    }
                    PayloadState::Unchecked(reason) => {
                        return Err(at(FrameIssue::UncheckedPayload(*reason)))
                    }
                };
                match frame.frame_type {
                    1 => projector
                        .read_terms(&mut dictionary, segment, frame.content, bytes, &mut rebuild)
                        .map_err(at)?,
                    2..=5 => projector
                        .read_rows(&dictionary, frame.frame_type, frame.content, bytes)
                        .map_err(|error| at(error.into()))?,
                    CHUNK | BLOB | RECIPE | VALUE | PACK => rebuild
                        .frame(frame.frame_type, frame.content, bytes)
                        .map_err(|error| at(error.into()))?,
                    6 | 12 | COMMIT | 15 | PARITY => {}
                    kind => return Err(at(FrameIssue::UnsupportedFrameType(kind))),
                }
            }
        }
        Ok(projector.finish())
    }
}

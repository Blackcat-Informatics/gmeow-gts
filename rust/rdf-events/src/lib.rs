// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF event protocol shared by GMEOW graph tooling.
//!
//! The protocol is deliberately small and dependency-free. Sources may emit
//! term references before declarations for low-latency streaming sinks; sinks
//! that require declarations first return `true` from
//! [`RdfEventSink::declares_before_reference`], and sources that can reorder
//! bounded folded data must satisfy that contract before emitting references.
//! Every id is scoped by `start_scope`/`end_scope`; unresolved references are
//! diagnosed by the sink at `finish`.

use std::error::Error;
use std::fmt;

/// Scope-local term identifier used by RDF event sources.
pub type EventTermId = u64;

/// Identifier for a source scope, such as one folded GTS file.
pub type EventScopeId = u64;

/// Optional source location attached to diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventLocation {
    /// GTS frame index, line number, or other source-native record index.
    pub frame_index: Option<u64>,
    /// 1-based line number where available.
    pub line: Option<u64>,
    /// 1-based column number where available.
    pub column: Option<u64>,
}

/// Diagnostic emitted as part of an RDF event stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable detail.
    pub detail: String,
    /// Optional source location.
    pub location: Option<EventLocation>,
}

/// RDF-star triple value carried by a term or reifier binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventTriple {
    /// Subject term id.
    pub subject: EventTermId,
    /// Predicate term id.
    pub predicate: EventTermId,
    /// Object term id.
    pub object: EventTermId,
}

/// RDF quad value carried by an event stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventQuad {
    /// Subject term id.
    pub subject: EventTermId,
    /// Predicate term id.
    pub predicate: EventTermId,
    /// Object term id.
    pub object: EventTermId,
    /// Graph-name term id, or `None` for the default graph.
    pub graph_name: Option<EventTermId>,
}

/// RDF 1.2 literal base direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventLiteralDirection {
    /// Left-to-right.
    Ltr,
    /// Right-to-left.
    Rtl,
}

/// RDF term payload for an event term declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventTermKind {
    /// IRI reference.
    Iri { value: String },
    /// Blank node label, scoped to the active event scope.
    BlankNode { label: String },
    /// Literal value.
    Literal {
        /// Literal lexical value.
        lexical: String,
        /// Datatype IRI term id, when explicit in the source.
        datatype: Option<EventTermId>,
        /// Language tag.
        language: Option<String>,
        /// RDF 1.2 literal base direction.
        direction: Option<EventLiteralDirection>,
    },
    /// RDF-star triple term. `reifier` carries the term id of the reifier
    /// bound to this quoted triple, when the source records one.
    Triple {
        /// Quoted triple value.
        triple: EventTriple,
        /// Optional reifier term id.
        reifier: Option<EventTermId>,
    },
}

/// Term declaration in an RDF event stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventTerm {
    /// Scope-local term id.
    pub id: EventTermId,
    /// Term payload.
    pub kind: EventTermKind,
}

/// High-level category for RDF event errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventErrorKind {
    /// Sink rejected an event.
    Sink,
    /// Source graph or stream is internally inconsistent.
    InvalidSource,
    /// A term id was declared twice in one scope.
    DuplicateDeclaration,
    /// A referenced term id was not declared by `finish`.
    UnresolvedReference,
    /// Events were emitted after their scope was closed.
    ClosedScope,
    /// A quoted-triple term exceeded the configured nesting limit.
    TripleNestingLimit,
    /// Source or sink aborted before freezing final state.
    Cancelled,
}

/// Concrete error type shared by RDF event sources and sinks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventError {
    kind: EventErrorKind,
    detail: String,
}

impl EventError {
    /// Create an error with a high-level kind and detail string.
    pub fn new(kind: EventErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// Create a sink rejection error.
    pub fn sink(detail: impl Into<String>) -> Self {
        Self::new(EventErrorKind::Sink, detail)
    }

    /// Create an invalid-source error.
    pub fn invalid_source(detail: impl Into<String>) -> Self {
        Self::new(EventErrorKind::InvalidSource, detail)
    }

    /// Create a quoted-triple nesting-limit error.
    pub fn triple_nesting_limit(limit: usize) -> Self {
        Self::new(
            EventErrorKind::TripleNestingLimit,
            format!("quoted triple nesting exceeds configured limit {limit}"),
        )
    }

    /// Return the error category.
    pub fn kind(&self) -> &EventErrorKind {
        &self.kind
    }

    /// Return human-readable detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}

impl Error for EventError {}

/// Consumer of RDF graph events.
///
/// Implementations should treat cancellation as non-final: if any callback
/// returns `Err`, no partial state should be frozen as successfully imported.
pub trait RdfEventSink {
    /// Whether this sink requires term declarations before any reference.
    fn declares_before_reference(&self) -> bool {
        false
    }

    /// Maximum quoted-triple nesting accepted by this sink.
    fn triple_term_nesting_limit(&self) -> usize {
        64
    }

    /// Open a new id scope.
    fn start_scope(&mut self, _scope: EventScopeId) -> Result<(), EventError> {
        Ok(())
    }

    /// Declare a term id.
    fn term(&mut self, _term: EventTerm) -> Result<(), EventError> {
        Ok(())
    }

    /// Emit a quad.
    fn quad(&mut self, _quad: EventQuad) -> Result<(), EventError> {
        Ok(())
    }

    /// Bind a reifier term to a triple.
    fn reifier(&mut self, _reifier: EventTermId, _triple: EventTriple) -> Result<(), EventError> {
        Ok(())
    }

    /// Emit an annotation triple `(reifier, predicate, value)`.
    fn annotation(&mut self, _annotation: EventTriple) -> Result<(), EventError> {
        Ok(())
    }

    /// Emit a source diagnostic.
    fn diagnostic(&mut self, _diagnostic: EventDiagnostic) -> Result<(), EventError> {
        Ok(())
    }

    /// Close an id scope.
    fn end_scope(&mut self, _scope: EventScopeId) -> Result<(), EventError> {
        Ok(())
    }

    /// Finish the stream and let the sink resolve deferred references.
    fn finish(&mut self) -> Result<(), EventError> {
        Ok(())
    }
}

/// Producer of RDF graph events.
pub trait RdfEventSource {
    /// Drive events into a concrete sink.
    fn drive<S: RdfEventSink>(&self, sink: &mut S) -> Result<(), EventError>
    where
        Self: Sized;

    /// Drive events into a trait-object sink.
    fn drive_erased(&self, sink: &mut dyn RdfEventSink) -> Result<(), EventError>;
}

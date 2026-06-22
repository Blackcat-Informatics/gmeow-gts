// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF event adapter for folded GTS graphs.
//!
//! This module adapts the GTS reader to an RDF event protocol used by RDF text
//! codecs. It intentionally sits above the GTS frame-level
//! [`crate::reader::StreamingSink`] API: the event stream is about RDF dataset
//! semantics, not container mechanics.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::model::{Diagnostic, Graph, Quad, Term, TermKind, Triple3};
use crate::reader::{read_with_options, ReadOptions};

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

/// RDF 1.2 triple value carried by a term or reifier binding.
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
    /// RDF 1.2 triple term. `reifier` carries the term id of the reifier
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

/// Infallible visitor over a folded [`Graph`].
///
/// The visitor preserves the graph's folded order and keeps callbacks scoped to
/// already-materialized graph state. Codecs that need fallible event streaming
/// should use [`GraphRdfEventSource`] instead.
pub trait RdfDatasetVisitor {
    /// Visit one folded term.
    fn term(&mut self, _id: usize, _term: &Term) {}
    /// Visit one folded quad.
    fn quad(&mut self, _quad: Quad) {}
    /// Visit one folded reifier binding.
    fn reifier(&mut self, _reifier: usize, _triple: Triple3) {}
    /// Visit one folded annotation.
    fn annotation(&mut self, _annotation: Triple3) {}
    /// Visit one reader diagnostic.
    fn diagnostic(&mut self, _diagnostic: &Diagnostic) {}
}

/// Visit the folded RDF dataset projection of a graph.
pub fn visit_dataset(graph: &Graph, visitor: &mut impl RdfDatasetVisitor) {
    for (id, term) in graph.terms.iter().enumerate() {
        visitor.term(id, term);
    }
    for &quad in &graph.quads {
        visitor.quad(quad);
    }
    for &(reifier, triple) in &graph.reifiers {
        visitor.reifier(reifier, triple);
    }
    for &annotation in &graph.annotations {
        visitor.annotation(annotation);
    }
    for diagnostic in &graph.diagnostics {
        visitor.diagnostic(diagnostic);
    }
}

/// RDF event source backed by an already-folded GTS graph.
pub struct GraphRdfEventSource<'a> {
    graph: &'a Graph,
    scope: EventScopeId,
}

impl<'a> GraphRdfEventSource<'a> {
    /// Create a source using scope id `0`.
    pub fn new(graph: &'a Graph) -> Self {
        Self::with_scope(graph, 0)
    }

    /// Create a source with an explicit scope id.
    pub fn with_scope(graph: &'a Graph, scope: EventScopeId) -> Self {
        Self { graph, scope }
    }
}

impl RdfEventSource for GraphRdfEventSource<'_> {
    fn drive<S: RdfEventSink>(&self, sink: &mut S) -> Result<(), EventError> {
        drive_graph(self.graph, self.scope, sink)
    }

    fn drive_erased(&self, sink: &mut dyn RdfEventSink) -> Result<(), EventError> {
        drive_graph(self.graph, self.scope, sink)
    }
}

/// RDF event source backed by GTS bytes.
pub struct ReaderRdfEventSource<'a> {
    data: &'a [u8],
    options: ReadOptions<'a>,
    scope: EventScopeId,
}

impl<'a> ReaderRdfEventSource<'a> {
    /// Create a source using the normal `reader::read` options.
    pub fn new(data: &'a [u8], allow_segments: bool, expected_head: Option<&'a [u8]>) -> Self {
        Self::with_options(data, ReadOptions::new(allow_segments, expected_head))
    }

    /// Create a source using full reader options.
    pub fn with_options(data: &'a [u8], options: ReadOptions<'a>) -> Self {
        Self {
            data,
            options,
            scope: 0,
        }
    }

    /// Set the source scope id.
    pub fn with_scope(mut self, scope: EventScopeId) -> Self {
        self.scope = scope;
        self
    }
}

impl RdfEventSource for ReaderRdfEventSource<'_> {
    fn drive<S: RdfEventSink>(&self, sink: &mut S) -> Result<(), EventError> {
        let graph = read_with_options(self.data, self.options);
        GraphRdfEventSource::with_scope(&graph, self.scope).drive(sink)
    }

    fn drive_erased(&self, sink: &mut dyn RdfEventSink) -> Result<(), EventError> {
        let graph = read_with_options(self.data, self.options);
        GraphRdfEventSource::with_scope(&graph, self.scope).drive_erased(sink)
    }
}

/// Fold GTS bytes and drive RDF events into a sink.
pub fn read_to_rdf_events(
    data: &[u8],
    allow_segments: bool,
    expected_head: Option<&[u8]>,
    sink: &mut dyn RdfEventSink,
) -> Result<(), EventError> {
    ReaderRdfEventSource::new(data, allow_segments, expected_head).drive_erased(sink)
}

fn drive_graph(
    graph: &Graph,
    scope: EventScopeId,
    sink: &mut dyn RdfEventSink,
) -> Result<(), EventError> {
    sink.start_scope(scope)?;
    if sink.declares_before_reference() {
        DeclarationOrderEmitter::new(graph, sink).emit_all()?;
    } else {
        emit_fold_order(graph, sink)?;
    }
    for diagnostic in &graph.diagnostics {
        sink.diagnostic(event_diagnostic(diagnostic)?)?;
    }
    sink.end_scope(scope)?;
    sink.finish()
}

fn emit_fold_order(graph: &Graph, sink: &mut dyn RdfEventSink) -> Result<(), EventError> {
    for (id, term) in graph.terms.iter().enumerate() {
        sink.term(event_term(graph, id, term)?)?;
    }
    for &(reifier, triple) in &graph.reifiers {
        validate_term_ref(graph, reifier, "reifier")?;
        validate_triple_refs(graph, triple, "reifier")?;
        sink.reifier(event_id(reifier)?, event_triple(triple)?)?;
    }
    for &quad in &graph.quads {
        validate_quad_refs(graph, quad)?;
        sink.quad(event_quad(quad)?)?;
    }
    for &annotation in &graph.annotations {
        validate_triple_refs(graph, annotation, "annotation")?;
        sink.annotation(event_triple(annotation)?)?;
    }
    Ok(())
}

struct DeclarationOrderEmitter<'a, 's> {
    graph: &'a Graph,
    sink: &'s mut dyn RdfEventSink,
    emitted_terms: HashSet<usize>,
    visiting_terms: HashSet<usize>,
    emitted_reifiers: HashSet<usize>,
}

impl<'a, 's> DeclarationOrderEmitter<'a, 's> {
    fn new(graph: &'a Graph, sink: &'s mut dyn RdfEventSink) -> Self {
        Self {
            graph,
            sink,
            emitted_terms: HashSet::new(),
            visiting_terms: HashSet::new(),
            emitted_reifiers: HashSet::new(),
        }
    }

    fn emit_all(&mut self) -> Result<(), EventError> {
        for id in 0..self.graph.terms.len() {
            self.emit_term(id, 0)?;
        }
        for &(reifier, triple) in &self.graph.reifiers {
            self.emit_reifier(reifier, triple, 0)?;
        }
        for &quad in &self.graph.quads {
            validate_quad_refs(self.graph, quad)?;
            self.sink.quad(event_quad(quad)?)?;
        }
        for &annotation in &self.graph.annotations {
            validate_triple_refs(self.graph, annotation, "annotation")?;
            self.sink.annotation(event_triple(annotation)?)?;
        }
        Ok(())
    }

    fn emit_term(&mut self, id: usize, depth: usize) -> Result<(), EventError> {
        if self.emitted_terms.contains(&id) {
            return Ok(());
        }
        if !self.visiting_terms.insert(id) {
            return Err(EventError::invalid_source(format!(
                "cycle while declaring term {id}"
            )));
        }
        if depth > self.sink.triple_term_nesting_limit() {
            return Err(EventError::triple_nesting_limit(
                self.sink.triple_term_nesting_limit(),
            ));
        }

        let term = self.graph.terms.get(id).ok_or_else(|| {
            EventError::invalid_source(format!("term id {id} is outside graph terms"))
        })?;

        match term.kind {
            TermKind::Literal => {
                if let Some(datatype) = term.datatype {
                    self.emit_term(datatype, depth)?;
                }
            }
            TermKind::Triple => {
                if let Some(reifier) = term.reifier {
                    if let Some(triple) = self.graph.reifier(reifier) {
                        self.emit_reifier(reifier, triple, depth + 1)?;
                    }
                }
            }
            TermKind::Iri | TermKind::Bnode => {}
        }

        self.sink.term(event_term(self.graph, id, term)?)?;
        self.visiting_terms.remove(&id);
        self.emitted_terms.insert(id);
        Ok(())
    }

    fn emit_reifier(
        &mut self,
        reifier: usize,
        triple: Triple3,
        depth: usize,
    ) -> Result<(), EventError> {
        if !self.emitted_reifiers.insert(reifier) {
            return Ok(());
        }
        self.emit_term(reifier, depth)?;
        self.emit_triple_deps(triple, depth)?;
        self.sink.reifier(event_id(reifier)?, event_triple(triple)?)
    }

    fn emit_triple_deps(&mut self, triple: Triple3, depth: usize) -> Result<(), EventError> {
        let (subject, predicate, object) = triple;
        self.emit_term(subject, depth)?;
        self.emit_term(predicate, depth)?;
        self.emit_term(object, depth)?;
        Ok(())
    }
}

fn validate_term_ref(graph: &Graph, id: usize, context: &str) -> Result<(), EventError> {
    if id < graph.terms.len() {
        Ok(())
    } else {
        Err(EventError::invalid_source(format!(
            "{context} references term id {id} outside graph terms"
        )))
    }
}

fn validate_triple_refs(graph: &Graph, triple: Triple3, context: &str) -> Result<(), EventError> {
    let (subject, predicate, object) = triple;
    validate_term_ref(graph, subject, context)?;
    validate_term_ref(graph, predicate, context)?;
    validate_term_ref(graph, object, context)
}

fn validate_quad_refs(graph: &Graph, quad: Quad) -> Result<(), EventError> {
    let (subject, predicate, object, graph_name) = quad;
    validate_term_ref(graph, subject, "quad")?;
    validate_term_ref(graph, predicate, "quad")?;
    validate_term_ref(graph, object, "quad")?;
    if let Some(graph_name) = graph_name {
        validate_term_ref(graph, graph_name, "quad")?;
    }
    Ok(())
}

fn event_id(id: usize) -> Result<EventTermId, EventError> {
    EventTermId::try_from(id)
        .map_err(|_| EventError::invalid_source(format!("term id {id} exceeds event id range")))
}

fn event_direction(direction: &str) -> Option<EventLiteralDirection> {
    match direction {
        "ltr" => Some(EventLiteralDirection::Ltr),
        "rtl" => Some(EventLiteralDirection::Rtl),
        _ => None,
    }
}

fn event_term(graph: &Graph, id: usize, term: &Term) -> Result<EventTerm, EventError> {
    let kind = match term.kind {
        TermKind::Iri => EventTermKind::Iri {
            value: term.value.clone().unwrap_or_default(),
        },
        TermKind::Bnode => EventTermKind::BlankNode {
            label: term.value.clone().unwrap_or_default(),
        },
        TermKind::Literal => EventTermKind::Literal {
            lexical: term.value.clone().unwrap_or_default(),
            datatype: term.datatype.map(event_id).transpose()?,
            language: term.lang.clone(),
            direction: term.direction.as_deref().and_then(event_direction),
        },
        TermKind::Triple => {
            let triple = term
                .reifier
                .and_then(|reifier| graph.reifier(reifier))
                .ok_or_else(|| {
                    EventError::invalid_source(format!(
                        "triple term {id} does not have a resolvable reifier binding"
                    ))
                })?;
            EventTermKind::Triple {
                triple: event_triple(triple)?,
                reifier: term.reifier.map(event_id).transpose()?,
            }
        }
    };
    Ok(EventTerm {
        id: event_id(id)?,
        kind,
    })
}

fn event_triple(triple: Triple3) -> Result<EventTriple, EventError> {
    let (subject, predicate, object) = triple;
    Ok(EventTriple {
        subject: event_id(subject)?,
        predicate: event_id(predicate)?,
        object: event_id(object)?,
    })
}

fn event_quad(quad: Quad) -> Result<EventQuad, EventError> {
    let (subject, predicate, object, graph_name) = quad;
    Ok(EventQuad {
        subject: event_id(subject)?,
        predicate: event_id(predicate)?,
        object: event_id(object)?,
        graph_name: graph_name.map(event_id).transpose()?,
    })
}

fn event_diagnostic(diagnostic: &Diagnostic) -> Result<EventDiagnostic, EventError> {
    Ok(EventDiagnostic {
        code: diagnostic.code.clone(),
        detail: diagnostic.detail.clone(),
        location: Some(EventLocation {
            frame_index: diagnostic
                .frame_index
                .map(u64::try_from)
                .transpose()
                .map_err(|_| {
                    EventError::invalid_source("diagnostic frame index exceeds event range")
                })?,
            line: None,
            column: None,
        }),
    })
}

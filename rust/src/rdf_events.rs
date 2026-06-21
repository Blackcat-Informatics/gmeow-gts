// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RDF event adapter for folded GTS graphs.
//!
//! This module adapts the GTS reader to the standalone `gmeow-rdf-events`
//! protocol used by RDF text codecs. It intentionally sits above the GTS
//! frame-level [`crate::reader::StreamingSink`] API: the event stream is about
//! RDF dataset semantics, not container mechanics.

use std::collections::HashSet;

pub use gmeow_rdf_events::{
    EventDiagnostic, EventError, EventErrorKind, EventLiteralDirection, EventLocation, EventQuad,
    EventScopeId, EventTerm, EventTermId, EventTermKind, EventTriple, RdfEventSink, RdfEventSource,
};

use crate::model::{Diagnostic, Graph, Quad, Term, TermKind, Triple3};
use crate::reader::{read_with_options, ReadOptions};

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
        sink.reifier(event_id(reifier)?, event_triple(triple)?)?;
    }
    for &quad in &graph.quads {
        sink.quad(event_quad(quad)?)?;
    }
    for &annotation in &graph.annotations {
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
            self.emit_quad_deps(quad, 0)?;
            self.sink.quad(event_quad(quad)?)?;
        }
        for &annotation in &self.graph.annotations {
            self.emit_triple_deps(annotation, 0)?;
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
                if let Some(triple) = term.reifier.and_then(|reifier| self.graph.reifier(reifier)) {
                    self.emit_triple_deps(triple, depth + 1)?;
                    if let Some(reifier) = term.reifier {
                        self.emit_term(reifier, depth + 1)?;
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

    fn emit_quad_deps(&mut self, quad: Quad, depth: usize) -> Result<(), EventError> {
        let (subject, predicate, object, graph_name) = quad;
        self.emit_term(subject, depth)?;
        self.emit_term(predicate, depth)?;
        self.emit_term(object, depth)?;
        if let Some(graph_name) = graph_name {
            self.emit_term(graph_name, depth)?;
        }
        Ok(())
    }

    fn emit_triple_deps(&mut self, triple: Triple3, depth: usize) -> Result<(), EventError> {
        let (subject, predicate, object) = triple;
        self.emit_term(subject, depth)?;
        self.emit_term(predicate, depth)?;
        self.emit_term(object, depth)?;
        Ok(())
    }
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

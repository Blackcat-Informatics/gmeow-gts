// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optional RDF 1.2 Turtle-family and line-format codecs.
//!
//! This module is compiled only with `--features rdf-codecs`. It uses
//! `oxttl` for shared N-Triples/Turtle/TriG parsing and serialization, while
//! GTS import and export stay routed through the crate's native RDF adapter and
//! RDF event contract.

use std::collections::BTreeMap;
use std::fmt;

use oxrdf::{Dataset, GraphNameRef, TripleRef};
use oxttl::{
    NTriplesParser, NTriplesSerializer, TriGParser, TriGSerializer, TurtleParser, TurtleSerializer,
};

use crate::model::{Diagnostic, Graph, Quad, Term, TermKind, Triple3};
use crate::rdf::{from_oxrdf_dataset, to_oxrdf_quads, RdfAdapterError};
use crate::rdf_events::{
    EventDiagnostic, EventError, EventErrorKind, EventLiteralDirection, EventQuad, EventScopeId,
    EventTerm, EventTermId, EventTermKind, EventTriple, GraphRdfEventSource, RdfEventSink,
    RdfEventSource,
};

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// Error raised by the optional Turtle-family codec layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdfCodecError {
    detail: String,
}

impl RdfCodecError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Human-readable error detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RdfCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for RdfCodecError {}

impl From<RdfAdapterError> for RdfCodecError {
    fn from(error: RdfAdapterError) -> Self {
        Self::new(format!("RDF adapter error: {error}"))
    }
}

impl From<oxttl::TurtleSyntaxError> for RdfCodecError {
    fn from(error: oxttl::TurtleSyntaxError) -> Self {
        Self::new(format!("RDF text syntax error: {error}"))
    }
}

impl From<oxrdf::IriParseError> for RdfCodecError {
    fn from(error: oxrdf::IriParseError) -> Self {
        Self::new(format!("invalid serializer IRI: {error}"))
    }
}

impl From<std::io::Error> for RdfCodecError {
    fn from(error: std::io::Error) -> Self {
        Self::new(format!("RDF text serialization error: {error}"))
    }
}

impl From<std::string::FromUtf8Error> for RdfCodecError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        Self::new(format!(
            "RDF text serializer emitted invalid UTF-8: {error}"
        ))
    }
}

impl From<EventError> for RdfCodecError {
    fn from(error: EventError) -> Self {
        Self::new(format!("RDF event error: {error}"))
    }
}

/// Parse N-Triples text into a GTS byte stream using the `dist` profile.
pub fn from_ntriples(text: &str) -> Result<Vec<u8>, RdfCodecError> {
    let mut dataset = Dataset::new();
    for triple in NTriplesParser::new().for_slice(text.as_bytes()) {
        let triple = triple?;
        dataset.insert(triple.as_ref().in_graph(GraphNameRef::DefaultGraph));
    }
    from_oxrdf_dataset(&dataset).map_err(Into::into)
}

/// Parse Turtle text into a GTS byte stream using the `dist` profile.
pub fn from_turtle(text: &str) -> Result<Vec<u8>, RdfCodecError> {
    let mut dataset = Dataset::new();
    for triple in TurtleParser::new().for_slice(text.as_bytes()) {
        let triple = triple?;
        dataset.insert(triple.as_ref().in_graph(GraphNameRef::DefaultGraph));
    }
    from_oxrdf_dataset(&dataset).map_err(Into::into)
}

/// Parse TriG text into a GTS byte stream using the `dist` profile.
pub fn from_trig(text: &str) -> Result<Vec<u8>, RdfCodecError> {
    let mut dataset = Dataset::new();
    for quad in TriGParser::new().for_slice(text.as_bytes()) {
        let quad = quad?;
        dataset.insert(quad.as_ref());
    }
    from_oxrdf_dataset(&dataset).map_err(Into::into)
}

/// Serialize a folded graph to N-Triples through the RDF event contract.
///
/// N-Triples has only a default graph. This returns an error if the folded
/// graph's RDF projection contains named-graph quads.
pub fn to_ntriples(graph: &Graph) -> Result<String, RdfCodecError> {
    to_ntriples_from_source(&GraphRdfEventSource::new(graph))
}

/// Serialize a folded graph to Turtle through the RDF event contract.
///
/// Turtle has only a default graph. This returns an error if the folded graph's
/// RDF projection contains named-graph quads.
pub fn to_turtle(graph: &Graph) -> Result<String, RdfCodecError> {
    to_turtle_from_source(&GraphRdfEventSource::new(graph))
}

/// Serialize a folded graph to TriG through the RDF event contract.
pub fn to_trig(graph: &Graph) -> Result<String, RdfCodecError> {
    to_trig_from_source(&GraphRdfEventSource::new(graph))
}

/// Materialize a graph from an RDF event source.
pub fn graph_from_source<S: RdfEventSource>(source: &S) -> Result<Graph, RdfCodecError> {
    let mut sink = EventGraphSink::default();
    source.drive(&mut sink)?;
    sink.into_graph()
}

/// Materialize a graph from a trait-object RDF event source.
pub fn graph_from_erased_source(source: &dyn RdfEventSource) -> Result<Graph, RdfCodecError> {
    let mut sink = EventGraphSink::default();
    source.drive_erased(&mut sink)?;
    sink.into_graph()
}

/// Serialize an RDF event source to N-Triples.
pub fn to_ntriples_from_source<S: RdfEventSource>(source: &S) -> Result<String, RdfCodecError> {
    serialize_ntriples_graph(&graph_from_source(source)?)
}

/// Serialize a trait-object RDF event source to N-Triples.
pub fn to_ntriples_from_erased_source(
    source: &dyn RdfEventSource,
) -> Result<String, RdfCodecError> {
    serialize_ntriples_graph(&graph_from_erased_source(source)?)
}

/// Serialize an RDF event source to Turtle.
pub fn to_turtle_from_source<S: RdfEventSource>(source: &S) -> Result<String, RdfCodecError> {
    serialize_turtle_graph(&graph_from_source(source)?)
}

/// Serialize a trait-object RDF event source to Turtle.
pub fn to_turtle_from_erased_source(source: &dyn RdfEventSource) -> Result<String, RdfCodecError> {
    serialize_turtle_graph(&graph_from_erased_source(source)?)
}

/// Serialize an RDF event source to TriG.
pub fn to_trig_from_source<S: RdfEventSource>(source: &S) -> Result<String, RdfCodecError> {
    serialize_trig_graph(&graph_from_source(source)?)
}

/// Serialize a trait-object RDF event source to TriG.
pub fn to_trig_from_erased_source(source: &dyn RdfEventSource) -> Result<String, RdfCodecError> {
    serialize_trig_graph(&graph_from_erased_source(source)?)
}

fn serialize_ntriples_graph(graph: &Graph) -> Result<String, RdfCodecError> {
    let mut serializer = NTriplesSerializer::new().for_writer(Vec::new());

    for quad in to_oxrdf_quads(graph)? {
        let quad = quad.as_ref();
        if !quad.graph_name.is_default_graph() {
            return Err(RdfCodecError::new(format!(
                "N-Triples cannot serialize named graph {}",
                quad.graph_name
            )));
        }
        serializer.serialize_triple(TripleRef::new(quad.subject, quad.predicate, quad.object))?;
    }

    String::from_utf8(serializer.finish()).map_err(Into::into)
}

fn serialize_turtle_graph(graph: &Graph) -> Result<String, RdfCodecError> {
    let mut serializer = TurtleSerializer::new()
        .with_prefix("rdf", RDF_NS)?
        .with_prefix("xsd", XSD_NS)?
        .for_writer(Vec::new());

    for quad in to_oxrdf_quads(graph)? {
        let quad = quad.as_ref();
        if !quad.graph_name.is_default_graph() {
            return Err(RdfCodecError::new(format!(
                "Turtle cannot serialize named graph {}",
                quad.graph_name
            )));
        }
        serializer.serialize_triple(TripleRef::new(quad.subject, quad.predicate, quad.object))?;
    }

    String::from_utf8(serializer.finish()?).map_err(Into::into)
}

fn serialize_trig_graph(graph: &Graph) -> Result<String, RdfCodecError> {
    let mut serializer = TriGSerializer::new()
        .with_prefix("rdf", RDF_NS)?
        .with_prefix("xsd", XSD_NS)?
        .for_writer(Vec::new());

    for quad in to_oxrdf_quads(graph)? {
        serializer.serialize_quad(quad.as_ref())?;
    }

    String::from_utf8(serializer.finish()?).map_err(Into::into)
}

/// RDF event sink that materializes one event scope as a folded graph.
#[derive(Debug, Default)]
pub struct EventGraphSink {
    terms: BTreeMap<EventTermId, EventTermKind>,
    quads: Vec<EventQuad>,
    reifiers: BTreeMap<EventTermId, EventTriple>,
    annotations: Vec<EventTriple>,
    diagnostics: Vec<EventDiagnostic>,
    active_scope: Option<EventScopeId>,
    saw_scope: bool,
    finished: bool,
}

impl EventGraphSink {
    /// Consume the sink and return the materialized graph.
    pub fn into_graph(self) -> Result<Graph, RdfCodecError> {
        self.try_into_graph().map_err(Into::into)
    }

    fn try_into_graph(self) -> Result<Graph, EventError> {
        let EventGraphSink {
            terms,
            quads,
            reifiers,
            annotations,
            diagnostics,
            ..
        } = self;

        let id_map: BTreeMap<EventTermId, usize> = terms
            .keys()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();

        let terms = terms
            .into_iter()
            .map(|(id, kind)| event_term_to_model(id, kind, &id_map, &reifiers))
            .collect::<Result<Vec<_>, _>>()?;
        let quads = quads
            .into_iter()
            .map(|quad| event_quad_to_model(quad, &id_map))
            .collect::<Result<Vec<_>, _>>()?;
        let reifiers = reifiers
            .into_iter()
            .map(|(reifier, triple)| {
                Ok((
                    map_event_id(&id_map, reifier, "reifier")?,
                    event_triple_to_model(triple, &id_map)?,
                ))
            })
            .collect::<Result<Vec<_>, EventError>>()?;
        let annotations = annotations
            .into_iter()
            .map(|annotation| event_triple_to_model(annotation, &id_map))
            .collect::<Result<Vec<_>, _>>()?;
        let diagnostics = diagnostics
            .into_iter()
            .map(event_diagnostic_to_model)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Graph {
            terms,
            quads,
            reifiers,
            annotations,
            diagnostics,
            ..Default::default()
        })
    }

    fn ensure_not_finished(&self, event: &str) -> Result<(), EventError> {
        if self.finished {
            Err(EventError::new(
                EventErrorKind::ClosedScope,
                format!("{event} emitted after finish"),
            ))
        } else {
            Ok(())
        }
    }

    fn bind_reifier(
        &mut self,
        reifier: EventTermId,
        triple: EventTriple,
    ) -> Result<(), EventError> {
        if let Some(previous) = self.reifiers.get(&reifier) {
            if *previous != triple {
                return Err(EventError::invalid_source(format!(
                    "reifier term {reifier} has conflicting triple bindings"
                )));
            }
            return Ok(());
        }
        self.reifiers.insert(reifier, triple);
        Ok(())
    }
}

impl RdfEventSink for EventGraphSink {
    fn declares_before_reference(&self) -> bool {
        true
    }

    fn start_scope(&mut self, scope: EventScopeId) -> Result<(), EventError> {
        self.ensure_not_finished("start_scope")?;
        if self.saw_scope {
            return Err(EventError::invalid_source(
                "event graph sink accepts one RDF event scope",
            ));
        }
        self.saw_scope = true;
        self.active_scope = Some(scope);
        Ok(())
    }

    fn term(&mut self, term: EventTerm) -> Result<(), EventError> {
        self.ensure_not_finished("term")?;
        let EventTerm { id, kind } = term;
        if self.terms.contains_key(&id) {
            return Err(EventError::new(
                EventErrorKind::DuplicateDeclaration,
                format!("term id {id} declared more than once"),
            ));
        }
        if let EventTermKind::Triple { triple, reifier } = &kind {
            self.bind_reifier(reifier.unwrap_or(id), *triple)?;
        }
        self.terms.insert(id, kind);
        Ok(())
    }

    fn quad(&mut self, quad: EventQuad) -> Result<(), EventError> {
        self.ensure_not_finished("quad")?;
        self.quads.push(quad);
        Ok(())
    }

    fn reifier(&mut self, reifier: EventTermId, triple: EventTriple) -> Result<(), EventError> {
        self.ensure_not_finished("reifier")?;
        self.bind_reifier(reifier, triple)
    }

    fn annotation(&mut self, annotation: EventTriple) -> Result<(), EventError> {
        self.ensure_not_finished("annotation")?;
        self.annotations.push(annotation);
        Ok(())
    }

    fn diagnostic(&mut self, diagnostic: EventDiagnostic) -> Result<(), EventError> {
        self.ensure_not_finished("diagnostic")?;
        self.diagnostics.push(diagnostic);
        Ok(())
    }

    fn end_scope(&mut self, scope: EventScopeId) -> Result<(), EventError> {
        self.ensure_not_finished("end_scope")?;
        if self.active_scope != Some(scope) {
            return Err(EventError::invalid_source(format!(
                "end_scope {scope} does not match active scope {:?}",
                self.active_scope
            )));
        }
        self.active_scope = None;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), EventError> {
        if self.active_scope.is_some() {
            return Err(EventError::invalid_source(format!(
                "finish called before closing active scope {:?}",
                self.active_scope
            )));
        }
        self.finished = true;
        Ok(())
    }
}

fn event_term_to_model(
    id: EventTermId,
    kind: EventTermKind,
    id_map: &BTreeMap<EventTermId, usize>,
    reifiers: &BTreeMap<EventTermId, EventTriple>,
) -> Result<Term, EventError> {
    let term = match kind {
        EventTermKind::Iri { value } => Term {
            kind: TermKind::Iri,
            value: Some(value),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        EventTermKind::BlankNode { label } => Term {
            kind: TermKind::Bnode,
            value: Some(label),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        EventTermKind::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => Term {
            kind: TermKind::Literal,
            value: Some(lexical),
            datatype: datatype
                .map(|datatype| map_event_id(id_map, datatype, "literal datatype"))
                .transpose()?,
            lang: language,
            direction: direction.map(|direction| match direction {
                EventLiteralDirection::Ltr => "ltr".to_string(),
                EventLiteralDirection::Rtl => "rtl".to_string(),
            }),
            reifier: None,
        },
        EventTermKind::Triple { reifier, .. } => {
            let reifier = reifier.unwrap_or(id);
            if !reifiers.contains_key(&reifier) {
                return Err(EventError::new(
                    EventErrorKind::UnresolvedReference,
                    format!("triple term {id} references unbound reifier term {reifier}"),
                ));
            }
            Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(map_event_id(id_map, reifier, "triple term reifier")?),
            }
        }
    };
    Ok(term)
}

fn event_quad_to_model(
    quad: EventQuad,
    id_map: &BTreeMap<EventTermId, usize>,
) -> Result<Quad, EventError> {
    Ok((
        map_event_id(id_map, quad.subject, "quad subject")?,
        map_event_id(id_map, quad.predicate, "quad predicate")?,
        map_event_id(id_map, quad.object, "quad object")?,
        quad.graph_name
            .map(|graph_name| map_event_id(id_map, graph_name, "quad graph name"))
            .transpose()?,
    ))
}

fn event_triple_to_model(
    triple: EventTriple,
    id_map: &BTreeMap<EventTermId, usize>,
) -> Result<Triple3, EventError> {
    Ok((
        map_event_id(id_map, triple.subject, "triple subject")?,
        map_event_id(id_map, triple.predicate, "triple predicate")?,
        map_event_id(id_map, triple.object, "triple object")?,
    ))
}

fn event_diagnostic_to_model(diagnostic: EventDiagnostic) -> Result<Diagnostic, EventError> {
    let frame_index = diagnostic
        .location
        .and_then(|location| location.frame_index)
        .map(|frame_index| {
            usize::try_from(frame_index).map_err(|_| {
                EventError::invalid_source("diagnostic frame index exceeds usize range")
            })
        })
        .transpose()?;
    Ok(Diagnostic {
        code: diagnostic.code,
        detail: diagnostic.detail,
        frame_index,
    })
}

fn map_event_id(
    id_map: &BTreeMap<EventTermId, usize>,
    id: EventTermId,
    role: &str,
) -> Result<usize, EventError> {
    id_map.get(&id).copied().ok_or_else(|| {
        EventError::new(
            EventErrorKind::UnresolvedReference,
            format!("{role} references undeclared term id {id}"),
        )
    })
}

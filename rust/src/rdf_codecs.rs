// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optional RDF 1.2 text codecs.
//!
//! This module is compiled only with `--features rdf-codecs`. N-Triples,
//! Turtle, TriG, and RDF/XML use the crate's native parsers and serializers.
//! GTS import and export stay routed through the native RDF adapter and RDF
//! event contract.

use std::collections::BTreeMap;
use std::fmt;

use crate::model::{Diagnostic, Graph, Quad, Term, TermKind, Triple3};
use crate::rdf::{to_rdf_quads, RdfAdapterError};
use crate::rdf_events::{
    EventDiagnostic, EventError, EventErrorKind, EventLiteralDirection, EventQuad, EventScopeId,
    EventTerm, EventTermId, EventTermKind, EventTriple, GraphRdfEventSource, RdfEventSink,
    RdfEventSource,
};
use crate::reader::read;
use crate::writer::Writer;
use crate::xsd::annotate_ill_typed_literals;

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// Error raised by the optional RDF text codec layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdfCodecError {
    detail: String,
}

impl RdfCodecError {
    pub(crate) fn new(detail: impl Into<String>) -> Self {
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
    let bytes = crate::from_nquads::from_ntriples(text)
        .map_err(|error| RdfCodecError::new(format!("RDF text syntax error: {error}")))?;
    annotate_native_import(bytes)
}

/// Parse RDF/XML text into a GTS byte stream using the `dist` profile.
pub fn from_rdf_xml(text: &str) -> Result<Vec<u8>, RdfCodecError> {
    crate::rdf_xml::from_rdf_xml(text, None)
}

/// Parse RDF/XML text with an explicit document base IRI.
pub fn from_rdf_xml_with_base_iri(text: &str, base_iri: &str) -> Result<Vec<u8>, RdfCodecError> {
    crate::rdf_xml::from_rdf_xml(text, Some(base_iri))
}

/// Parse Turtle text into a GTS byte stream using the `dist` profile.
pub fn from_turtle(text: &str) -> Result<Vec<u8>, RdfCodecError> {
    let bytes = crate::from_trig::from_turtle(text)
        .map_err(|error| RdfCodecError::new(format!("RDF text syntax error: {error}")))?;
    annotate_native_import(bytes)
}

/// Parse TriG text into a GTS byte stream using the `dist` profile.
pub fn from_trig(text: &str) -> Result<Vec<u8>, RdfCodecError> {
    let bytes = crate::from_trig::from_trig(text)
        .map_err(|error| RdfCodecError::new(format!("RDF text syntax error: {error}")))?;
    annotate_native_import(bytes)
}

fn annotate_native_import(bytes: Vec<u8>) -> Result<Vec<u8>, RdfCodecError> {
    let mut graph = read(&bytes, true, None);
    let meta_len = graph.meta.len();
    annotate_ill_typed_literals(&mut graph);
    if graph.meta.len() == meta_len {
        return Ok(bytes);
    }
    Writer::deterministic(&graph, "dist")
        .map(|writer| writer.to_bytes())
        .map_err(|error| RdfCodecError::new(format!("RDF text serialization error: {error}")))
}

/// Serialize a folded graph to N-Triples through the RDF event contract.
///
/// N-Triples has only a default graph. This returns an error if the folded
/// graph's RDF projection contains named-graph quads.
pub fn to_ntriples(graph: &Graph) -> Result<String, RdfCodecError> {
    to_ntriples_from_source(&GraphRdfEventSource::new(graph))
}

/// Serialize a folded graph to RDF/XML through the RDF event contract.
///
/// RDF/XML is a graph format. This returns an error if the folded graph's RDF
/// projection contains named-graph quads.
pub fn to_rdf_xml(graph: &Graph) -> Result<String, RdfCodecError> {
    to_rdf_xml_from_source(&GraphRdfEventSource::new(graph))
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

/// Serialize an RDF event source to RDF/XML.
pub fn to_rdf_xml_from_source<S: RdfEventSource>(source: &S) -> Result<String, RdfCodecError> {
    serialize_rdf_xml_graph(&graph_from_source(source)?)
}

/// Serialize a trait-object RDF event source to RDF/XML.
pub fn to_rdf_xml_from_erased_source(source: &dyn RdfEventSource) -> Result<String, RdfCodecError> {
    serialize_rdf_xml_graph(&graph_from_erased_source(source)?)
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
    ensure_default_graph_projection(graph, "N-Triples")?;
    Ok(crate::nquads::to_nquads(graph))
}

fn serialize_rdf_xml_graph(graph: &Graph) -> Result<String, RdfCodecError> {
    crate::rdf_xml::to_rdf_xml(graph)
}

fn serialize_turtle_graph(graph: &Graph) -> Result<String, RdfCodecError> {
    ensure_default_graph_projection(graph, "Turtle")?;
    let body = crate::nquads::to_nquads(graph);
    if body.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!(
            "@prefix rdf: <{RDF_NS}> .\n@prefix xsd: <{XSD_NS}> .\n\n{body}"
        ))
    }
}

fn serialize_trig_graph(graph: &Graph) -> Result<String, RdfCodecError> {
    to_rdf_quads(graph)?;
    Ok(crate::trig::to_trig(graph))
}

fn ensure_default_graph_projection(graph: &Graph, format: &str) -> Result<(), RdfCodecError> {
    for quad in to_rdf_quads(graph)? {
        if !quad.graph_name.is_default_graph() {
            return Err(RdfCodecError::new(format!(
                "{format} cannot serialize named graph {}",
                quad.graph_name
            )));
        }
    }
    Ok(())
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

        let mut graph = Graph {
            terms,
            quads,
            reifiers,
            annotations,
            diagnostics,
            ..Default::default()
        };
        annotate_ill_typed_literals(&mut graph);
        Ok(graph)
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

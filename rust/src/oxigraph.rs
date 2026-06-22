// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optional Oxigraph store adapter.
//!
//! This module is compiled only with `--features oxigraph-adapter`. The feature
//! is intentionally opt-in and uses Oxigraph's in-memory `Store` without its
//! default RocksDB feature, so default GTS transport users do not inherit a
//! graph database dependency.

use std::fmt;

use ::oxigraph::model::{
    BaseDirection as OxBaseDirection, BlankNode as OxBlankNode, BlankNodeRef as OxBlankNodeRef,
    GraphName as OxGraphName, GraphNameRef as OxGraphNameRef, Literal as OxLiteral,
    LiteralRef as OxLiteralRef, NamedNode as OxNamedNode, NamedNodeRef as OxNamedNodeRef,
    NamedOrBlankNode as OxNamedOrBlankNode, NamedOrBlankNodeRef as OxNamedOrBlankNodeRef,
    Quad as OxQuad, QuadRef as OxQuadRef, Term as OxTerm, TermRef as OxTermRef, Triple as OxTriple,
    TripleRef as OxTripleRef,
};
use ::oxigraph::store::{StorageError, Store};
use ciborium::value::Value;

use crate::model::{
    BlobEntry, Diagnostic, Graph, OpaqueNode, Signature, StreamableInfo, Suppression,
};
use crate::rdf::{
    to_rdf_quads, writer_from_rdf_dataset_with_profile, BaseDirection, BlankNode, Dataset,
    GraphName, Iri, Literal, NamedOrBlankNode, RdfAdapterError, RdfQuad, RdfTerm, RdfTriple,
};
use crate::writer::Writer;

/// Error raised by the optional Oxigraph adapter.
#[derive(Debug)]
pub enum OxigraphAdapterError {
    /// RDF model conversion failed before reaching the store.
    Rdf(RdfAdapterError),
    /// Oxigraph storage operation failed.
    Storage(StorageError),
}

impl fmt::Display for OxigraphAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rdf(err) => write!(f, "RDF adapter error: {err}"),
            Self::Storage(err) => write!(f, "Oxigraph storage error: {err}"),
        }
    }
}

impl std::error::Error for OxigraphAdapterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Rdf(err) => Some(err),
            Self::Storage(err) => Some(err),
        }
    }
}

impl From<RdfAdapterError> for OxigraphAdapterError {
    fn from(value: RdfAdapterError) -> Self {
        Self::Rdf(value)
    }
}

impl From<StorageError> for OxigraphAdapterError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

/// GTS-specific state carried next to a pure RDF store projection.
///
/// The Oxigraph store receives only RDF quads. Suppressions, signatures, blobs,
/// diagnostics, segment heads, profiles, and metadata stay in this sidecar so
/// callers can inspect them without encoding GTS implementation details into
/// the RDF graph.
#[derive(Clone, Debug, Default)]
pub struct GtsSidecar {
    pub blob_meta: Vec<(String, Value)>,
    pub blobs: Vec<(String, BlobEntry)>,
    pub meta: Vec<(String, Value)>,
    pub suppressions: Vec<Suppression>,
    pub opaque: Vec<OpaqueNode>,
    pub signatures: Vec<Signature>,
    pub diagnostics: Vec<Diagnostic>,
    pub segment_heads: Vec<Vec<u8>>,
    pub segment_profiles: Vec<String>,
    pub segment_meta: Vec<Vec<(String, Value)>>,
    pub segment_streamable: Vec<StreamableInfo>,
}

impl GtsSidecar {
    /// Clone the GTS-only state from a folded graph.
    pub fn from_graph(graph: &Graph) -> Self {
        Self {
            blob_meta: graph.blob_meta.clone(),
            blobs: graph.blobs.clone(),
            meta: graph.meta.clone(),
            suppressions: graph.suppressions.clone(),
            opaque: graph.opaque.clone(),
            signatures: graph.signatures.clone(),
            diagnostics: graph.diagnostics.clone(),
            segment_heads: graph.segment_heads.clone(),
            segment_profiles: graph.segment_profiles.clone(),
            segment_meta: graph.segment_meta.clone(),
            segment_streamable: graph.segment_streamable.clone(),
        }
    }
}

/// Oxigraph store plus the GTS-only sidecar state split out of the source graph.
pub struct StoreWithSidecar {
    pub store: Store,
    pub sidecar: GtsSidecar,
}

/// Owning iterator over Oxigraph quads projected from a GTS graph.
pub struct IntoQuads {
    inner: std::vec::IntoIter<OxQuad>,
}

impl Iterator for IntoQuads {
    type Item = OxQuad;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Project a folded graph into Oxigraph quads without materializing N-Quads text.
pub fn graph_into_quads(graph: Graph) -> Result<IntoQuads, OxigraphAdapterError> {
    let quads = to_rdf_quads(&graph)?
        .into_iter()
        .map(|quad| oxigraph_quad(&quad))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IntoQuads {
        inner: quads.into_iter(),
    })
}

/// Project a folded graph into Oxigraph quads and return GTS-only sidecar state.
pub fn graph_into_quads_with_sidecar(
    graph: Graph,
) -> Result<(IntoQuads, GtsSidecar), OxigraphAdapterError> {
    let sidecar = GtsSidecar::from_graph(&graph);
    Ok((graph_into_quads(graph)?, sidecar))
}

/// Project a folded graph into a new in-memory Oxigraph store.
pub fn graph_to_store(graph: &Graph) -> Result<Store, OxigraphAdapterError> {
    let store = Store::new()?;
    for quad in to_rdf_quads(graph)? {
        let quad = oxigraph_quad(&quad)?;
        store.insert(quad.as_ref())?;
    }
    Ok(store)
}

/// Project a folded graph into an Oxigraph store and split GTS-only state aside.
pub fn graph_to_store_with_sidecar(graph: Graph) -> Result<StoreWithSidecar, OxigraphAdapterError> {
    let sidecar = GtsSidecar::from_graph(&graph);
    Ok(StoreWithSidecar {
        store: graph_to_store(&graph)?,
        sidecar,
    })
}

/// Build a deterministic GTS writer from an Oxigraph store.
///
/// The conversion walks native Oxigraph quads and never serializes through
/// N-Quads. GTS-only sidecar state is intentionally not folded back into the
/// pure RDF writer; callers that need signatures, blobs, suppressions, or
/// diagnostics should keep the [`GtsSidecar`] next to the store.
pub fn store_to_writer(store: &Store, profile: &str) -> Result<Writer, OxigraphAdapterError> {
    let mut dataset = Dataset::new();
    for quad in store.iter() {
        let quad = quad?;
        dataset.insert(native_quad_from_oxigraph(quad.as_ref())?);
    }
    Ok(writer_from_rdf_dataset_with_profile(&dataset, profile)?)
}

/// Build GTS bytes from an Oxigraph store.
pub fn store_to_gts_bytes(store: &Store, profile: &str) -> Result<Vec<u8>, OxigraphAdapterError> {
    Ok(store_to_writer(store, profile)?.to_bytes())
}

fn native_quad_from_oxigraph(quad: OxQuadRef<'_>) -> Result<RdfQuad, OxigraphAdapterError> {
    Ok(RdfQuad::new(
        native_named_or_blank_from_oxigraph(quad.subject)?,
        native_iri_from_oxigraph(quad.predicate)?,
        native_term_from_oxigraph(quad.object)?,
        native_graph_name_from_oxigraph(quad.graph_name)?,
    ))
}

fn native_graph_name_from_oxigraph(
    graph_name: OxGraphNameRef<'_>,
) -> Result<GraphName, OxigraphAdapterError> {
    match graph_name {
        OxGraphNameRef::DefaultGraph => Ok(GraphName::DefaultGraph),
        OxGraphNameRef::NamedNode(node) => Ok(native_iri_from_oxigraph(node)?.into()),
        OxGraphNameRef::BlankNode(node) => Ok(native_blank_node_from_oxigraph(node)?.into()),
    }
}

fn native_named_or_blank_from_oxigraph(
    node: OxNamedOrBlankNodeRef<'_>,
) -> Result<NamedOrBlankNode, OxigraphAdapterError> {
    match node {
        OxNamedOrBlankNodeRef::NamedNode(node) => Ok(native_iri_from_oxigraph(node)?.into()),
        OxNamedOrBlankNodeRef::BlankNode(node) => Ok(native_blank_node_from_oxigraph(node)?.into()),
    }
}

fn native_term_from_oxigraph(term: OxTermRef<'_>) -> Result<RdfTerm, OxigraphAdapterError> {
    match term {
        OxTermRef::NamedNode(node) => Ok(native_iri_from_oxigraph(node)?.into()),
        OxTermRef::BlankNode(node) => Ok(native_blank_node_from_oxigraph(node)?.into()),
        OxTermRef::Literal(literal) => Ok(native_literal_from_oxigraph(literal)?.into()),
        OxTermRef::Triple(triple) => Ok(native_triple_from_oxigraph(triple.into())?.into()),
    }
}

fn native_triple_from_oxigraph(triple: OxTripleRef<'_>) -> Result<RdfTriple, OxigraphAdapterError> {
    Ok(RdfTriple::new(
        native_named_or_blank_from_oxigraph(triple.subject)?,
        native_iri_from_oxigraph(triple.predicate)?,
        native_term_from_oxigraph(triple.object)?,
    ))
}

fn native_literal_from_oxigraph(
    literal: OxLiteralRef<'_>,
) -> Result<Literal, OxigraphAdapterError> {
    if let Some(direction) = literal.direction() {
        let language = literal
            .language()
            .ok_or_else(|| invalid_rdf("directional literal is missing its language tag"))?;
        let direction = match direction {
            OxBaseDirection::Ltr => BaseDirection::Ltr,
            OxBaseDirection::Rtl => BaseDirection::Rtl,
        };
        return Literal::new_directional_language_tagged_literal(
            literal.value(),
            language,
            direction,
        )
        .map_err(Into::into);
    }
    if let Some(language) = literal.language() {
        return Literal::new_language_tagged_literal(literal.value(), language).map_err(Into::into);
    }
    let datatype = literal.datatype().as_str();
    if datatype == crate::model::XSD_STRING {
        Ok(Literal::new_simple_literal(literal.value()))
    } else {
        Ok(Literal::new_typed_literal(
            literal.value(),
            Iri::new(datatype)?,
        ))
    }
}

fn native_iri_from_oxigraph(node: OxNamedNodeRef<'_>) -> Result<Iri, OxigraphAdapterError> {
    Iri::new(node.as_str()).map_err(Into::into)
}

fn native_blank_node_from_oxigraph(
    node: OxBlankNodeRef<'_>,
) -> Result<BlankNode, OxigraphAdapterError> {
    BlankNode::new(node.as_str()).map_err(Into::into)
}

fn oxigraph_quad(quad: &RdfQuad) -> Result<OxQuad, OxigraphAdapterError> {
    Ok(OxQuad::new(
        oxigraph_named_or_blank(&quad.subject)?,
        oxigraph_iri(&quad.predicate)?,
        oxigraph_term(&quad.object)?,
        oxigraph_graph_name(&quad.graph_name)?,
    ))
}

fn oxigraph_graph_name(graph_name: &GraphName) -> Result<OxGraphName, OxigraphAdapterError> {
    match graph_name {
        GraphName::DefaultGraph => Ok(OxGraphName::DefaultGraph),
        GraphName::Iri(iri) => Ok(oxigraph_iri(iri)?.into()),
        GraphName::BlankNode(node) => Ok(oxigraph_blank_node(node)?.into()),
    }
}

fn oxigraph_named_or_blank(
    node: &NamedOrBlankNode,
) -> Result<OxNamedOrBlankNode, OxigraphAdapterError> {
    match node {
        NamedOrBlankNode::Iri(iri) => Ok(oxigraph_iri(iri)?.into()),
        NamedOrBlankNode::BlankNode(node) => Ok(oxigraph_blank_node(node)?.into()),
    }
}

fn oxigraph_term(term: &RdfTerm) -> Result<OxTerm, OxigraphAdapterError> {
    match term {
        RdfTerm::Iri(iri) => Ok(oxigraph_iri(iri)?.into()),
        RdfTerm::BlankNode(node) => Ok(oxigraph_blank_node(node)?.into()),
        RdfTerm::Literal(literal) => Ok(oxigraph_literal(literal)?.into()),
        RdfTerm::Triple(triple) => Ok(OxTerm::Triple(Box::new(oxigraph_triple(triple)?))),
    }
}

fn oxigraph_triple(triple: &RdfTriple) -> Result<OxTriple, OxigraphAdapterError> {
    Ok(OxTriple::new(
        oxigraph_named_or_blank(&triple.subject)?,
        oxigraph_iri(&triple.predicate)?,
        oxigraph_term(&triple.object)?,
    ))
}

fn oxigraph_literal(literal: &Literal) -> Result<OxLiteral, OxigraphAdapterError> {
    if let Some(direction) = literal.direction {
        let language = literal
            .language
            .as_deref()
            .ok_or_else(|| invalid_rdf("directional literal is missing its language tag"))?;
        let direction = match direction {
            BaseDirection::Ltr => OxBaseDirection::Ltr,
            BaseDirection::Rtl => OxBaseDirection::Rtl,
        };
        return OxLiteral::new_directional_language_tagged_literal(
            &literal.lexical,
            language,
            direction,
        )
        .map_err(|error| {
            invalid_rdf(format!(
                "invalid directional language-tagged literal: {error}"
            ))
        });
    }
    if let Some(language) = &literal.language {
        return OxLiteral::new_language_tagged_literal(&literal.lexical, language)
            .map_err(|error| invalid_rdf(format!("invalid language-tagged literal: {error}")));
    }
    if let Some(datatype) = &literal.datatype {
        Ok(OxLiteral::new_typed_literal(
            &literal.lexical,
            oxigraph_iri(datatype)?,
        ))
    } else {
        Ok(OxLiteral::new_simple_literal(&literal.lexical))
    }
}

fn oxigraph_iri(iri: &Iri) -> Result<OxNamedNode, OxigraphAdapterError> {
    OxNamedNode::new(iri.as_str())
        .map_err(|error| invalid_rdf(format!("invalid IRI for Oxigraph: {error}")))
}

fn oxigraph_blank_node(node: &BlankNode) -> Result<OxBlankNode, OxigraphAdapterError> {
    OxBlankNode::new(node.as_str())
        .map_err(|error| invalid_rdf(format!("invalid blank-node identifier: {error}")))
}

fn invalid_rdf(detail: impl Into<String>) -> OxigraphAdapterError {
    RdfAdapterError::new(detail).into()
}

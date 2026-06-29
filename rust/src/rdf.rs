// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optional native RDF dataset interop.
//!
//! This module is compiled only with `--features rdf`. It provides a small
//! dependency-free RDF 1.2 dataset model for callers that need structured RDF
//! rows without inheriting an external RDF toolkit or graph store.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use crate::model::{
    Graph, Quad, ReifierRow, Term as GtsTerm, TermKind, Triple3, RDF_LANG_STRING, XSD_STRING,
};
use crate::ulid::deterministic_label;
use crate::writer::Writer;
use crate::xsd::{
    ill_typed_literals_in_terms, ill_typed_literals_metadata, ILL_TYPED_LITERAL_META_KEY,
};

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Error raised by the optional RDF adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdfAdapterError {
    detail: String,
}

impl RdfAdapterError {
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

impl fmt::Display for RdfAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for RdfAdapterError {}

/// RDF 1.2 literal base direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BaseDirection {
    /// Left-to-right text direction.
    Ltr,
    /// Right-to-left text direction.
    Rtl,
}

impl BaseDirection {
    /// Return the RDF 1.2 lexical direction string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

impl fmt::Display for BaseDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// RDF IRI node.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iri {
    value: String,
}

impl Iri {
    /// Create an IRI node.
    ///
    /// This validates the minimal syntactic contract GTS needs for generated
    /// and imported rows: the value must be non-empty, contain a scheme
    /// separator, and avoid ASCII whitespace/control characters.
    pub fn new(value: impl Into<String>) -> Result<Self, RdfAdapterError> {
        let value = value.into();
        if value.is_empty()
            || !value.contains(':')
            || value.chars().any(|ch| {
                ch.is_ascii_control() || ch.is_ascii_whitespace() || ch == '<' || ch == '>'
            })
        {
            return Err(RdfAdapterError::new(format!("invalid IRI {value:?}")));
        }
        Ok(Self { value })
    }

    /// Borrow the IRI string.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Iri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// RDF blank node label.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlankNode {
    label: String,
}

impl BlankNode {
    /// Create a blank node label.
    pub fn new(label: impl Into<String>) -> Result<Self, RdfAdapterError> {
        let label = label.into();
        if !is_valid_blank_node_label(&label) {
            return Err(RdfAdapterError::new(format!(
                "invalid blank-node identifier {label:?}"
            )));
        }
        Ok(Self { label })
    }

    /// Borrow the blank node label.
    pub fn as_str(&self) -> &str {
        &self.label
    }
}

impl fmt::Display for BlankNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_:{}", self.as_str())
    }
}

/// RDF subject node: IRI or blank node.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamedOrBlankNode {
    /// IRI node.
    Iri(Iri),
    /// Blank node.
    BlankNode(BlankNode),
}

impl fmt::Display for NamedOrBlankNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Iri(iri) => write!(f, "<{iri}>"),
            Self::BlankNode(node) => node.fmt(f),
        }
    }
}

impl From<Iri> for NamedOrBlankNode {
    fn from(value: Iri) -> Self {
        Self::Iri(value)
    }
}

impl From<BlankNode> for NamedOrBlankNode {
    fn from(value: BlankNode) -> Self {
        Self::BlankNode(value)
    }
}

/// RDF graph name.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphName {
    /// Default graph.
    #[default]
    DefaultGraph,
    /// Named graph IRI.
    Iri(Iri),
    /// Named graph blank node.
    BlankNode(BlankNode),
}

impl GraphName {
    /// Whether this graph name is the default graph.
    pub fn is_default_graph(&self) -> bool {
        matches!(self, Self::DefaultGraph)
    }
}

impl fmt::Display for GraphName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefaultGraph => f.write_str("default graph"),
            Self::Iri(iri) => write!(f, "<{iri}>"),
            Self::BlankNode(node) => node.fmt(f),
        }
    }
}

impl From<Iri> for GraphName {
    fn from(value: Iri) -> Self {
        Self::Iri(value)
    }
}

impl From<BlankNode> for GraphName {
    fn from(value: BlankNode) -> Self {
        Self::BlankNode(value)
    }
}

/// RDF literal value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Literal {
    /// Literal lexical form.
    pub lexical: String,
    /// Explicit datatype for typed literals.
    pub datatype: Option<Iri>,
    /// Language tag for language-tagged literals.
    pub language: Option<String>,
    /// RDF 1.2 base direction for directional language-tagged literals.
    pub direction: Option<BaseDirection>,
}

impl Literal {
    /// Create a simple `xsd:string` literal.
    pub fn new_simple_literal(lexical: impl Into<String>) -> Self {
        Self {
            lexical: lexical.into(),
            datatype: None,
            language: None,
            direction: None,
        }
    }

    /// Create a typed literal.
    pub fn new_typed_literal(lexical: impl Into<String>, datatype: Iri) -> Self {
        Self {
            lexical: lexical.into(),
            datatype: Some(datatype),
            language: None,
            direction: None,
        }
    }

    /// Create a language-tagged literal.
    pub fn new_language_tagged_literal(
        lexical: impl Into<String>,
        language: impl Into<String>,
    ) -> Result<Self, RdfAdapterError> {
        let language = language.into();
        validate_language_tag(&language)?;
        Ok(Self {
            lexical: lexical.into(),
            datatype: None,
            language: Some(language),
            direction: None,
        })
    }

    /// Create a directional language-tagged literal.
    pub fn new_directional_language_tagged_literal(
        lexical: impl Into<String>,
        language: impl Into<String>,
        direction: BaseDirection,
    ) -> Result<Self, RdfAdapterError> {
        let language = language.into();
        validate_language_tag(&language)?;
        Ok(Self {
            lexical: lexical.into(),
            datatype: None,
            language: Some(language),
            direction: Some(direction),
        })
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.lexical)?;
        if let Some(language) = &self.language {
            write!(f, "@{language}")?;
            if let Some(direction) = self.direction {
                write!(f, "--{direction}")?;
            }
        } else if let Some(datatype) = &self.datatype {
            write!(f, "^^<{datatype}>")?;
        }
        Ok(())
    }
}

/// RDF object term.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RdfTerm {
    /// IRI node.
    Iri(Iri),
    /// Blank node.
    BlankNode(BlankNode),
    /// Literal node.
    Literal(Literal),
    /// RDF 1.2 quoted triple term.
    Triple(Box<RdfTriple>),
}

impl fmt::Display for RdfTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Iri(iri) => write!(f, "<{iri}>"),
            Self::BlankNode(node) => node.fmt(f),
            Self::Literal(literal) => literal.fmt(f),
            Self::Triple(triple) => write!(f, "<<( {triple} )>>"),
        }
    }
}

impl From<Iri> for RdfTerm {
    fn from(value: Iri) -> Self {
        Self::Iri(value)
    }
}

impl From<BlankNode> for RdfTerm {
    fn from(value: BlankNode) -> Self {
        Self::BlankNode(value)
    }
}

impl From<Literal> for RdfTerm {
    fn from(value: Literal) -> Self {
        Self::Literal(value)
    }
}

impl From<RdfTriple> for RdfTerm {
    fn from(value: RdfTriple) -> Self {
        Self::Triple(Box::new(value))
    }
}

/// RDF 1.2 quoted triple.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RdfTriple {
    /// Triple subject.
    pub subject: NamedOrBlankNode,
    /// Triple predicate.
    pub predicate: Iri,
    /// Triple object.
    pub object: RdfTerm,
}

impl RdfTriple {
    /// Create a quoted triple.
    pub fn new(
        subject: impl Into<NamedOrBlankNode>,
        predicate: Iri,
        object: impl Into<RdfTerm>,
    ) -> Self {
        Self {
            subject: subject.into(),
            predicate,
            object: object.into(),
        }
    }
}

impl fmt::Display for RdfTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}> {}", self.subject, self.predicate, self.object)
    }
}

/// RDF quad row.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RdfQuad {
    /// Quad subject.
    pub subject: NamedOrBlankNode,
    /// Quad predicate.
    pub predicate: Iri,
    /// Quad object.
    pub object: RdfTerm,
    /// Quad graph name.
    pub graph_name: GraphName,
}

impl RdfQuad {
    /// Create an RDF quad.
    pub fn new(
        subject: impl Into<NamedOrBlankNode>,
        predicate: Iri,
        object: impl Into<RdfTerm>,
        graph_name: impl Into<GraphName>,
    ) -> Self {
        Self {
            subject: subject.into(),
            predicate,
            object: object.into(),
            graph_name: graph_name.into(),
        }
    }
}

impl fmt::Display for RdfQuad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}> {}", self.subject, self.predicate, self.object)?;
        if !self.graph_name.is_default_graph() {
            write!(f, " {}", self.graph_name)?;
        }
        f.write_str(" .")
    }
}

/// Native RDF dataset.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dataset {
    quads: BTreeSet<RdfQuad>,
}

impl Dataset {
    /// Create an empty dataset.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a quad. Returns `true` if the quad was not already present.
    pub fn insert(&mut self, quad: RdfQuad) -> bool {
        self.quads.insert(quad)
    }

    /// Iterate over quads in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &RdfQuad> {
        self.quads.iter()
    }

    /// Number of unique quads.
    pub fn len(&self) -> usize {
        self.quads.len()
    }

    /// Whether the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.quads.is_empty()
    }
}

impl<'a> IntoIterator for &'a Dataset {
    type Item = &'a RdfQuad;
    type IntoIter = std::collections::btree_set::Iter<'a, RdfQuad>;

    fn into_iter(self) -> Self::IntoIter {
        self.quads.iter()
    }
}

/// Export options for converting folded GTS graphs into a native RDF dataset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Drop quads/reifier rows that use RDF 1.2 quoted triples in positions
    /// this dataset surface intentionally does not represent.
    ///
    /// Strict mode is the default. It returns [`RdfAdapterError`] instead of
    /// silently changing the graph.
    pub allow_rdf12_lossy: bool,
}

/// Convert a folded GTS graph into a native RDF dataset in strict mode.
pub fn to_rdf_dataset(graph: &Graph) -> Result<Dataset, RdfAdapterError> {
    to_rdf_dataset_with_options(graph, ExportOptions::default())
}

/// Convert a folded GTS graph into a native RDF dataset, dropping only RDF 1.2
/// quoted-triple rows that this dataset surface cannot represent.
pub fn to_rdf_dataset_lossy(graph: &Graph) -> Result<Dataset, RdfAdapterError> {
    to_rdf_dataset_with_options(
        graph,
        ExportOptions {
            allow_rdf12_lossy: true,
        },
    )
}

/// Convert a folded GTS graph into a native RDF dataset.
pub fn to_rdf_dataset_with_options(
    graph: &Graph,
    options: ExportOptions,
) -> Result<Dataset, RdfAdapterError> {
    let mut dataset = Dataset::new();
    for quad in to_rdf_quads_with_options(graph, options)? {
        dataset.insert(quad);
    }
    Ok(dataset)
}

/// Convert a folded GTS graph into native RDF quad rows in strict mode.
pub fn to_rdf_quads(graph: &Graph) -> Result<Vec<RdfQuad>, RdfAdapterError> {
    to_rdf_quads_with_options(graph, ExportOptions::default())
}

/// Convert a folded GTS graph into native RDF quad rows with export options.
pub fn to_rdf_quads_with_options(
    graph: &Graph,
    options: ExportOptions,
) -> Result<Vec<RdfQuad>, RdfAdapterError> {
    let mut quads = Vec::new();
    let bnode_labels = BnodeLabels::for_graph(graph);

    for &(s, p, o, graph_name) in &graph.quads {
        if let Some(quad) = graph_quad_to_rdf(graph, &bnode_labels, s, p, o, graph_name, options)? {
            quads.push(quad);
        }
    }

    let rdf_reifies = Iri::new(RDF_REIFIES)?;
    for &(rid, (s, p, o), graph_name) in &graph.reifiers {
        if is_internal_triple_self_binding(graph, rid) {
            continue;
        }
        let Some(subject) =
            named_or_blank_term(graph, &bnode_labels, rid, "reifier subject", options)?
        else {
            continue;
        };
        let Some(object) = quoted_triple(graph, &bnode_labels, s, p, o, "reified triple", options)?
        else {
            continue;
        };
        let Some(graph_name) = graph_name_term(graph, &bnode_labels, graph_name, options)? else {
            continue;
        };
        quads.push(RdfQuad::new(
            subject,
            rdf_reifies.clone(),
            RdfTerm::Triple(Box::new(object)),
            graph_name,
        ));
    }

    for &(s, p, o, graph_name) in &graph.annotations {
        if let Some(quad) = graph_quad_to_rdf(graph, &bnode_labels, s, p, o, graph_name, options)? {
            quads.push(quad);
        }
    }

    Ok(quads)
}

/// Alias for callers that do not need the concrete adapter name in their API.
pub fn to_dataset(graph: &Graph) -> Result<Dataset, RdfAdapterError> {
    to_rdf_dataset(graph)
}

/// Convert a native RDF dataset into a GTS file using the `dist` profile.
pub fn from_rdf_dataset(dataset: &Dataset) -> Result<Vec<u8>, RdfAdapterError> {
    from_rdf_dataset_with_profile(dataset, "dist")
}

/// Convert a native RDF dataset into a GTS file using the requested profile.
pub fn from_rdf_dataset_with_profile(
    dataset: &Dataset,
    profile: &str,
) -> Result<Vec<u8>, RdfAdapterError> {
    Ok(writer_from_rdf_dataset_with_profile(dataset, profile)?.to_bytes())
}

/// Build a [`Writer`] from a native RDF dataset using the `dist` profile.
pub fn writer_from_rdf_dataset(dataset: &Dataset) -> Result<Writer, RdfAdapterError> {
    writer_from_rdf_dataset_with_profile(dataset, "dist")
}

/// Build a [`Writer`] from a native RDF dataset using the requested profile.
pub fn writer_from_rdf_dataset_with_profile(
    dataset: &Dataset,
    profile: &str,
) -> Result<Writer, RdfAdapterError> {
    let mut interner = Interner::new();
    let mut quads: Vec<Quad> = Vec::new();
    let mut reifier_bindings: BTreeMap<usize, Triple3> = BTreeMap::new();
    let mut reifiers: Vec<ReifierRow> = Vec::new();

    for quad in dataset {
        if quad.predicate.as_str() == RDF_REIFIES && matches!(quad.object, RdfTerm::Triple(_)) {
            let rid = interner.named_or_blank(&quad.subject);
            let RdfTerm::Triple(triple) = &quad.object else {
                unreachable!("matched above")
            };
            let binding = interner.triple(triple, &mut reifier_bindings, &mut reifiers)?;
            insert_reifier(&mut reifier_bindings, rid, binding)?;
            let graph_name = graph_name_id(&quad.graph_name, &mut interner);
            let row = (rid, binding, graph_name);
            if !reifiers.contains(&row) {
                reifiers.push(row);
            }
            continue;
        }

        let s = interner.named_or_blank(&quad.subject);
        let p = interner.iri(&quad.predicate);
        let o = interner.term(&quad.object, &mut reifier_bindings, &mut reifiers)?;
        let g = graph_name_id(&quad.graph_name, &mut interner);
        quads.push((s, p, o, g));
    }

    let mut writer = Writer::new(profile);
    if !interner.terms.is_empty() {
        writer.add_terms(&interner.terms);
    }
    if !quads.is_empty() {
        writer.add_quads(&quads);
    }
    reifiers.sort_by_key(|&(rid, (s, p, o), graph_name)| (graph_name, rid, s, p, o));
    if !reifiers.is_empty() {
        writer.add_reifies(&reifiers);
    }
    let ill_typed = ill_typed_literals_in_terms(&interner.terms);
    if !ill_typed.is_empty() {
        writer.add_meta(ciborium::value::Value::Map(vec![(
            ILL_TYPED_LITERAL_META_KEY.into(),
            ill_typed_literals_metadata(&ill_typed),
        )]));
    }
    Ok(writer)
}

/// Alias for callers that do not need the concrete adapter name in their API.
pub fn from_dataset(dataset: &Dataset) -> Result<Vec<u8>, RdfAdapterError> {
    from_rdf_dataset(dataset)
}

fn graph_quad_to_rdf(
    graph: &Graph,
    bnode_labels: &BnodeLabels,
    s: usize,
    p: usize,
    o: usize,
    graph_name: Option<usize>,
    options: ExportOptions,
) -> Result<Option<RdfQuad>, RdfAdapterError> {
    let Some(subject) = named_or_blank_term(graph, bnode_labels, s, "quad subject", options)?
    else {
        return Ok(None);
    };
    let Some(predicate) = predicate_term(graph, p, "quad predicate", options)? else {
        return Ok(None);
    };
    let Some(object) = rdf_term(graph, bnode_labels, o, "quad object", options)? else {
        return Ok(None);
    };
    let Some(graph_name) = graph_name_term(graph, bnode_labels, graph_name, options)? else {
        return Ok(None);
    };
    Ok(Some(RdfQuad::new(subject, predicate, object, graph_name)))
}

fn graph_term<'a>(graph: &'a Graph, id: usize, role: &str) -> Result<&'a GtsTerm, RdfAdapterError> {
    graph
        .terms
        .get(id)
        .ok_or_else(|| RdfAdapterError::new(format!("{role} references missing term id {id}")))
}

fn term_value<'a>(term: &'a GtsTerm, role: &str, id: usize) -> Result<&'a str, RdfAdapterError> {
    term.value.as_deref().ok_or_else(|| {
        RdfAdapterError::new(format!("{role} term id {id} is missing its lexical value"))
    })
}

fn named_or_blank_term(
    graph: &Graph,
    bnode_labels: &BnodeLabels,
    id: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<NamedOrBlankNode>, RdfAdapterError> {
    let term = graph_term(graph, id, role)?;
    match term.kind {
        TermKind::Iri => Ok(Some(Iri::new(term_value(term, role, id)?)?.into())),
        TermKind::Bnode => {
            let label = bnode_labels.label(term, id);
            Ok(Some(BlankNode::new(label.as_ref())?.into()))
        }
        TermKind::Triple if options.allow_rdf12_lossy => Ok(None),
        TermKind::Triple => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is an RDF 1.2 quoted triple; this dataset surface does not represent quoted triples in this position"
        ))),
        TermKind::Literal => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is a literal, but RDF requires an IRI or blank node"
        ))),
    }
}

fn predicate_term(
    graph: &Graph,
    id: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<Iri>, RdfAdapterError> {
    let term = graph_term(graph, id, role)?;
    match term.kind {
        TermKind::Iri => Ok(Some(Iri::new(term_value(term, role, id)?)?)),
        TermKind::Triple if options.allow_rdf12_lossy => Ok(None),
        TermKind::Triple => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is an RDF 1.2 quoted triple; RDF predicates must be IRIs"
        ))),
        TermKind::Bnode | TermKind::Literal => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is not an IRI"
        ))),
    }
}

fn rdf_term(
    graph: &Graph,
    bnode_labels: &BnodeLabels,
    id: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<RdfTerm>, RdfAdapterError> {
    let term = graph_term(graph, id, role)?;
    match term.kind {
        TermKind::Iri => Ok(Some(Iri::new(term_value(term, role, id)?)?.into())),
        TermKind::Bnode => {
            let label = bnode_labels.label(term, id);
            Ok(Some(BlankNode::new(label.as_ref())?.into()))
        }
        TermKind::Literal => Ok(Some(literal_term(graph, term, id, role)?.into())),
        TermKind::Triple => {
            let Some((s, p, o)) = term.reifier.and_then(|rid| graph.reifier(rid)) else {
                if options.allow_rdf12_lossy {
                    return Ok(None);
                }
                return Err(RdfAdapterError::new(format!(
                    "{role} term id {id} is an unbound RDF 1.2 quoted triple"
                )));
            };
            Ok(quoted_triple(graph, bnode_labels, s, p, o, role, options)?
                .map(|triple| RdfTerm::Triple(Box::new(triple))))
        }
    }
}

fn graph_name_term(
    graph: &Graph,
    bnode_labels: &BnodeLabels,
    id: Option<usize>,
    options: ExportOptions,
) -> Result<Option<GraphName>, RdfAdapterError> {
    let Some(id) = id else {
        return Ok(Some(GraphName::DefaultGraph));
    };
    let term = graph_term(graph, id, "graph name")?;
    match term.kind {
        TermKind::Iri => Ok(Some(Iri::new(term_value(term, "graph name", id)?)?.into())),
        TermKind::Bnode => {
            let label = bnode_labels.label(term, id);
            Ok(Some(BlankNode::new(label.as_ref())?.into()))
        }
        TermKind::Triple if options.allow_rdf12_lossy => Ok(None),
        TermKind::Triple => Err(RdfAdapterError::new(format!(
            "graph name term id {id} is an RDF 1.2 quoted triple; this dataset surface does not represent quoted triples in this position"
        ))),
        TermKind::Literal => Err(RdfAdapterError::new(format!(
            "graph name term id {id} is a literal, but RDF graph names must be IRIs or blank nodes"
        ))),
    }
}

fn quoted_triple(
    graph: &Graph,
    bnode_labels: &BnodeLabels,
    s: usize,
    p: usize,
    o: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<RdfTriple>, RdfAdapterError> {
    let Some(subject) = named_or_blank_term(graph, bnode_labels, s, role, options)? else {
        return Ok(None);
    };
    let Some(predicate) = predicate_term(graph, p, role, options)? else {
        return Ok(None);
    };
    let Some(object) = rdf_term(graph, bnode_labels, o, role, options)? else {
        return Ok(None);
    };
    Ok(Some(RdfTriple::new(subject, predicate, object)))
}

fn literal_term(
    graph: &Graph,
    term: &GtsTerm,
    id: usize,
    role: &str,
) -> Result<Literal, RdfAdapterError> {
    let value = term_value(term, role, id)?;
    if let Some(direction) = &term.direction {
        let Some(lang) = &term.lang else {
            return Err(RdfAdapterError::new(format!(
                "{role} literal term id {id} has RDF 1.2 base direction {direction:?} without a language tag"
            )));
        };
        let direction = match direction.as_str() {
            "ltr" => BaseDirection::Ltr,
            "rtl" => BaseDirection::Rtl,
            _ => {
                return Err(RdfAdapterError::new(format!(
                    "{role} literal term id {id} has invalid RDF 1.2 base direction {direction:?}"
                )));
            }
        };
        return Literal::new_directional_language_tagged_literal(value, lang, direction);
    }
    if let Some(lang) = &term.lang {
        return Literal::new_language_tagged_literal(value, lang);
    }

    let datatype = graph.datatype_iri(term);
    if datatype == XSD_STRING {
        Ok(Literal::new_simple_literal(value))
    } else {
        Ok(Literal::new_typed_literal(value, Iri::new(datatype)?))
    }
}

fn is_internal_triple_self_binding(graph: &Graph, rid: usize) -> bool {
    matches!(
        graph.terms.get(rid),
        Some(GtsTerm {
            kind: TermKind::Triple,
            reifier: Some(reifier),
            ..
        }) if *reifier == rid
    )
}

fn validate_language_tag(language: &str) -> Result<(), RdfAdapterError> {
    if !is_valid_language_tag(language) {
        return Err(RdfAdapterError::new(format!(
            "invalid language tag {language:?}"
        )));
    }
    Ok(())
}

fn is_valid_blank_node_label(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() && first != '_' {
        return false;
    }
    let mut last = first;
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != '.' {
            return false;
        }
        last = ch;
    }
    last != '.'
}

fn is_valid_language_tag(language: &str) -> bool {
    if language.is_empty() {
        return false;
    }
    language
        .split('-')
        .all(|subtag| !subtag.is_empty() && subtag.chars().all(|ch| ch.is_ascii_alphanumeric()))
}

struct BnodeLabels {
    generated: HashMap<usize, String>,
}

impl BnodeLabels {
    fn for_graph(graph: &Graph) -> Self {
        let mut used: HashSet<String> = graph
            .terms
            .iter()
            .filter(|term| term.kind == TermKind::Bnode)
            .filter_map(|term| term.value.clone())
            .collect();
        let mut generated = HashMap::new();

        for (id, term) in graph.terms.iter().enumerate() {
            if term.kind != TermKind::Bnode || term.value.is_some() {
                continue;
            }
            let mut counter = id as u128;
            let mut label = deterministic_label("gts_", counter);
            let mut suffix = 0usize;
            while used.contains(&label) {
                suffix += 1;
                counter = graph.terms.len() as u128 + suffix as u128;
                label = deterministic_label("gts_", counter);
            }
            used.insert(label.clone());
            generated.insert(id, label);
        }

        Self { generated }
    }

    fn label<'a>(&'a self, term: &'a GtsTerm, id: usize) -> Cow<'a, str> {
        match &term.value {
            Some(value) => Cow::Borrowed(value),
            None => Cow::Borrowed(
                self.generated
                    .get(&id)
                    .expect("missing blank-node labels are allocated for every graph term"),
            ),
        }
    }
}

fn graph_name_id(graph_name: &GraphName, interner: &mut Interner) -> Option<usize> {
    match graph_name {
        GraphName::DefaultGraph => None,
        GraphName::Iri(iri) => Some(interner.iri(iri)),
        GraphName::BlankNode(node) => Some(interner.blank_node(node)),
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum TermKey {
    Iri(String),
    Bnode(String),
    Literal {
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<usize>,
    },
    Triple(usize, usize, usize),
}

struct Interner {
    ids: HashMap<TermKey, usize>,
    terms: Vec<GtsTerm>,
}

impl Interner {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            terms: Vec::new(),
        }
    }

    fn iri(&mut self, node: &Iri) -> usize {
        self.intern(TermKey::Iri(node.as_str().to_string()), || GtsTerm {
            kind: TermKind::Iri,
            value: Some(node.as_str().to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        })
    }

    fn blank_node(&mut self, node: &BlankNode) -> usize {
        self.intern(TermKey::Bnode(node.as_str().to_string()), || GtsTerm {
            kind: TermKind::Bnode,
            value: Some(node.as_str().to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        })
    }

    fn named_or_blank(&mut self, node: &NamedOrBlankNode) -> usize {
        match node {
            NamedOrBlankNode::Iri(node) => self.iri(node),
            NamedOrBlankNode::BlankNode(node) => self.blank_node(node),
        }
    }

    fn literal(&mut self, literal: &Literal) -> usize {
        let datatype_id = if literal.language.is_some() {
            None
        } else {
            let datatype = literal
                .datatype
                .as_ref()
                .map(|iri| iri.as_str())
                .unwrap_or(XSD_STRING);
            if datatype == XSD_STRING || datatype == RDF_LANG_STRING {
                None
            } else {
                let iri = datatype.to_string();
                Some(self.intern(TermKey::Iri(iri.clone()), || GtsTerm {
                    kind: TermKind::Iri,
                    value: Some(iri),
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: None,
                }))
            }
        };
        let direction = literal
            .direction
            .map(|direction| direction.as_str().to_string());
        let key = TermKey::Literal {
            value: literal.lexical.clone(),
            lang: literal.language.clone(),
            direction: direction.clone(),
            datatype: datatype_id,
        };
        self.intern(key, || GtsTerm {
            kind: TermKind::Literal,
            value: Some(literal.lexical.clone()),
            datatype: datatype_id,
            lang: literal.language.clone(),
            direction,
            reifier: None,
        })
    }

    fn term(
        &mut self,
        term: &RdfTerm,
        reifier_bindings: &mut BTreeMap<usize, Triple3>,
        reifiers: &mut Vec<ReifierRow>,
    ) -> Result<usize, RdfAdapterError> {
        match term {
            RdfTerm::Iri(node) => Ok(self.iri(node)),
            RdfTerm::BlankNode(node) => Ok(self.blank_node(node)),
            RdfTerm::Literal(literal) => Ok(self.literal(literal)),
            RdfTerm::Triple(triple) => {
                let (s, p, o) = self.triple(triple, reifier_bindings, reifiers)?;
                let key = TermKey::Triple(s, p, o);
                if let Some(id) = self.ids.get(&key) {
                    return Ok(*id);
                }
                let id = self.terms.len();
                self.terms.push(GtsTerm {
                    kind: TermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    direction: None,
                    reifier: Some(id),
                });
                self.ids.insert(key, id);
                insert_reifier(reifier_bindings, id, (s, p, o))?;
                let row = (id, (s, p, o), None);
                if !reifiers.contains(&row) {
                    reifiers.push(row);
                }
                Ok(id)
            }
        }
    }

    fn triple(
        &mut self,
        triple: &RdfTriple,
        reifier_bindings: &mut BTreeMap<usize, Triple3>,
        reifiers: &mut Vec<ReifierRow>,
    ) -> Result<Triple3, RdfAdapterError> {
        Ok((
            self.named_or_blank(&triple.subject),
            self.iri(&triple.predicate),
            self.term(&triple.object, reifier_bindings, reifiers)?,
        ))
    }

    fn intern(&mut self, key: TermKey, make: impl FnOnce() -> GtsTerm) -> usize {
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let id = self.terms.len();
        self.terms.push(make());
        self.ids.insert(key, id);
        id
    }
}

fn insert_reifier(
    reifiers: &mut BTreeMap<usize, Triple3>,
    rid: usize,
    spo: Triple3,
) -> Result<(), RdfAdapterError> {
    if let Some(existing) = reifiers.get(&rid) {
        if *existing != spo {
            return Err(RdfAdapterError::new(format!(
                "conflicting rdf:reifies binding for term id {rid}: existing {existing:?}, new {spo:?}"
            )));
        }
        return Ok(());
    }
    reifiers.insert(rid, spo);
    Ok(())
}

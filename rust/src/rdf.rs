// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Optional native RDF interop through `oxrdf`.
//!
//! This module is compiled only with `--features rdf`. It uses `oxrdf`'s
//! in-memory RDF data model, not the `oxigraph` store, so default transport
//! users do not inherit an RDF toolkit or embedded graph database dependency.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt;

use oxrdf::{
    BlankNode, BlankNodeRef, Dataset, GraphName, GraphNameRef, Literal, LiteralRef, NamedNode,
    NamedNodeRef, NamedOrBlankNode, NamedOrBlankNodeRef, Quad as OxQuad, Term as OxTerm,
    TermRef as OxTermRef, Triple as OxTriple, TripleRef as OxTripleRef,
};

use crate::model::{Graph, Quad, Term, TermKind, Triple3, RDF_LANG_STRING, XSD_STRING};
use crate::writer::Writer;

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// Error raised by the optional RDF adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdfAdapterError {
    detail: String,
}

impl RdfAdapterError {
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

impl fmt::Display for RdfAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for RdfAdapterError {}

/// Export options for converting folded GTS graphs into an `oxrdf::Dataset`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportOptions {
    /// Drop quads/reifier rows that use RDF 1.2 quoted triples in positions
    /// `oxrdf` cannot represent.
    ///
    /// Strict mode is the default. It returns [`RdfAdapterError`] instead of
    /// silently changing the graph.
    pub allow_rdf12_lossy: bool,
}

/// Convert a folded GTS graph into an `oxrdf::Dataset` in strict mode.
pub fn to_oxrdf_dataset(graph: &Graph) -> Result<Dataset, RdfAdapterError> {
    to_oxrdf_dataset_with_options(graph, ExportOptions::default())
}

/// Convert a folded GTS graph into an `oxrdf::Dataset`, dropping only RDF 1.2
/// quoted-triple rows that `oxrdf` cannot represent.
pub fn to_oxrdf_dataset_lossy(graph: &Graph) -> Result<Dataset, RdfAdapterError> {
    to_oxrdf_dataset_with_options(
        graph,
        ExportOptions {
            allow_rdf12_lossy: true,
        },
    )
}

/// Convert a folded GTS graph into an `oxrdf::Dataset`.
pub fn to_oxrdf_dataset_with_options(
    graph: &Graph,
    options: ExportOptions,
) -> Result<Dataset, RdfAdapterError> {
    let mut dataset = Dataset::new();

    for &(s, p, o, graph_name) in &graph.quads {
        if let Some(quad) = graph_quad_to_oxrdf(graph, s, p, o, graph_name, options)? {
            dataset.insert(quad.as_ref());
        }
    }

    let rdf_reifies = named_node(RDF_REIFIES, "rdf:reifies predicate")?;
    for &(rid, (s, p, o)) in &graph.reifiers {
        let Some(subject) = named_or_blank_term(graph, rid, "reifier subject", options)? else {
            continue;
        };
        let Some(object) = quoted_triple(graph, s, p, o, "reified triple", options)? else {
            continue;
        };
        dataset.insert(
            OxQuad::new(
                subject,
                rdf_reifies.clone(),
                OxTerm::Triple(Box::new(object)),
                GraphName::DefaultGraph,
            )
            .as_ref(),
        );
    }

    for &(s, p, o) in &graph.annotations {
        if let Some(quad) = graph_quad_to_oxrdf(graph, s, p, o, None, options)? {
            dataset.insert(quad.as_ref());
        }
    }

    Ok(dataset)
}

/// Alias for callers that do not need the concrete adapter name in their API.
pub fn to_dataset(graph: &Graph) -> Result<Dataset, RdfAdapterError> {
    to_oxrdf_dataset(graph)
}

/// Convert an `oxrdf::Dataset` into a GTS file using the `dist` profile.
pub fn from_oxrdf_dataset(dataset: &Dataset) -> Result<Vec<u8>, RdfAdapterError> {
    from_oxrdf_dataset_with_profile(dataset, "dist")
}

/// Convert an `oxrdf::Dataset` into a GTS file using the requested profile.
pub fn from_oxrdf_dataset_with_profile(
    dataset: &Dataset,
    profile: &str,
) -> Result<Vec<u8>, RdfAdapterError> {
    let mut interner = Interner::new();
    let mut quads: Vec<Quad> = Vec::new();
    let mut reifiers: BTreeMap<usize, Triple3> = BTreeMap::new();

    for quad in dataset {
        if quad.graph_name.is_default_graph()
            && quad.predicate.as_str() == RDF_REIFIES
            && matches!(quad.object, OxTermRef::Triple(_))
        {
            let rid = interner.named_or_blank_ref(quad.subject);
            let OxTermRef::Triple(triple) = quad.object else {
                unreachable!("matched above")
            };
            let binding = interner.triple_ref(triple.into(), &mut reifiers);
            reifiers.insert(rid, binding);
            continue;
        }

        let s = interner.named_or_blank_ref(quad.subject);
        let p = interner.named_node_ref(quad.predicate);
        let o = interner.term_ref(quad.object, &mut reifiers);
        let g = graph_name_id(quad.graph_name, &mut interner);
        quads.push((s, p, o, g));
    }

    let mut writer = Writer::new(profile);
    if !interner.terms.is_empty() {
        writer.add_terms(&interner.terms);
    }
    if !quads.is_empty() {
        writer.add_quads(&quads);
    }
    let reifiers: Vec<(usize, Triple3)> = reifiers.into_iter().collect();
    if !reifiers.is_empty() {
        writer.add_reifies(&reifiers);
    }
    Ok(writer.to_bytes())
}

/// Alias for callers that do not need the concrete adapter name in their API.
pub fn from_dataset(dataset: &Dataset) -> Result<Vec<u8>, RdfAdapterError> {
    from_oxrdf_dataset(dataset)
}

fn graph_quad_to_oxrdf(
    graph: &Graph,
    s: usize,
    p: usize,
    o: usize,
    graph_name: Option<usize>,
    options: ExportOptions,
) -> Result<Option<OxQuad>, RdfAdapterError> {
    let Some(subject) = named_or_blank_term(graph, s, "quad subject", options)? else {
        return Ok(None);
    };
    let Some(predicate) = predicate_term(graph, p, "quad predicate", options)? else {
        return Ok(None);
    };
    let Some(object) = oxrdf_term(graph, o, "quad object", options)? else {
        return Ok(None);
    };
    let Some(graph_name) = graph_name_term(graph, graph_name, options)? else {
        return Ok(None);
    };
    Ok(Some(OxQuad::new(subject, predicate, object, graph_name)))
}

fn graph_term<'a>(graph: &'a Graph, id: usize, role: &str) -> Result<&'a Term, RdfAdapterError> {
    graph
        .terms
        .get(id)
        .ok_or_else(|| RdfAdapterError::new(format!("{role} references missing term id {id}")))
}

fn term_value<'a>(term: &'a Term, role: &str, id: usize) -> Result<&'a str, RdfAdapterError> {
    term.value.as_deref().ok_or_else(|| {
        RdfAdapterError::new(format!("{role} term id {id} is missing its lexical value"))
    })
}

fn named_node(value: &str, role: &str) -> Result<NamedNode, RdfAdapterError> {
    NamedNode::new(value)
        .map_err(|err| RdfAdapterError::new(format!("{role} has invalid IRI {value:?}: {err}")))
}

fn blank_node(value: &str, role: &str) -> Result<BlankNode, RdfAdapterError> {
    BlankNode::new(value).map_err(|err| {
        RdfAdapterError::new(format!(
            "{role} has invalid blank-node identifier {value:?}: {err}"
        ))
    })
}

fn named_or_blank_term(
    graph: &Graph,
    id: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<NamedOrBlankNode>, RdfAdapterError> {
    let term = graph_term(graph, id, role)?;
    match term.kind {
        TermKind::Iri => Ok(Some(named_node(term_value(term, role, id)?, role)?.into())),
        TermKind::Bnode => {
            let label = bnode_label(term, id);
            Ok(Some(blank_node(&label, role)?.into()))
        }
        TermKind::Triple if options.allow_rdf12_lossy => Ok(None),
        TermKind::Triple => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is an RDF 1.2 quoted triple; oxrdf cannot represent quoted triples in this position"
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
) -> Result<Option<NamedNode>, RdfAdapterError> {
    let term = graph_term(graph, id, role)?;
    match term.kind {
        TermKind::Iri => Ok(Some(named_node(term_value(term, role, id)?, role)?)),
        TermKind::Triple if options.allow_rdf12_lossy => Ok(None),
        TermKind::Triple => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is an RDF 1.2 quoted triple; RDF predicates must be IRIs"
        ))),
        TermKind::Bnode | TermKind::Literal => Err(RdfAdapterError::new(format!(
            "{role} term id {id} is not an IRI"
        ))),
    }
}

fn oxrdf_term(
    graph: &Graph,
    id: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<OxTerm>, RdfAdapterError> {
    let term = graph_term(graph, id, role)?;
    match term.kind {
        TermKind::Iri => Ok(Some(named_node(term_value(term, role, id)?, role)?.into())),
        TermKind::Bnode => {
            let label = bnode_label(term, id);
            Ok(Some(blank_node(&label, role)?.into()))
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
            Ok(quoted_triple(graph, s, p, o, role, options)?
                .map(|triple| OxTerm::Triple(Box::new(triple))))
        }
    }
}

fn graph_name_term(
    graph: &Graph,
    id: Option<usize>,
    options: ExportOptions,
) -> Result<Option<GraphName>, RdfAdapterError> {
    let Some(id) = id else {
        return Ok(Some(GraphName::DefaultGraph));
    };
    let term = graph_term(graph, id, "graph name")?;
    match term.kind {
        TermKind::Iri => Ok(Some(named_node(term_value(term, "graph name", id)?, "graph name")?.into())),
        TermKind::Bnode => {
            let label = bnode_label(term, id);
            Ok(Some(blank_node(&label, "graph name")?.into()))
        }
        TermKind::Triple if options.allow_rdf12_lossy => Ok(None),
        TermKind::Triple => Err(RdfAdapterError::new(format!(
            "graph name term id {id} is an RDF 1.2 quoted triple; oxrdf cannot represent quoted triples in this position"
        ))),
        TermKind::Literal => Err(RdfAdapterError::new(format!(
            "graph name term id {id} is a literal, but RDF graph names must be IRIs or blank nodes"
        ))),
    }
}

fn quoted_triple(
    graph: &Graph,
    s: usize,
    p: usize,
    o: usize,
    role: &str,
    options: ExportOptions,
) -> Result<Option<OxTriple>, RdfAdapterError> {
    let Some(subject) = named_or_blank_term(graph, s, role, options)? else {
        return Ok(None);
    };
    let Some(predicate) = predicate_term(graph, p, role, options)? else {
        return Ok(None);
    };
    let Some(object) = oxrdf_term(graph, o, role, options)? else {
        return Ok(None);
    };
    Ok(Some(OxTriple::new(subject, predicate, object)))
}

fn literal_term(
    graph: &Graph,
    term: &Term,
    id: usize,
    role: &str,
) -> Result<Literal, RdfAdapterError> {
    let value = term_value(term, role, id)?;
    if let Some(lang) = &term.lang {
        return Literal::new_language_tagged_literal(value, lang).map_err(|err| {
            RdfAdapterError::new(format!(
                "{role} literal term id {id} has invalid language tag {lang:?}: {err}"
            ))
        });
    }

    let datatype = graph.datatype_iri(term);
    if datatype == XSD_STRING {
        Ok(Literal::new_simple_literal(value))
    } else {
        Ok(Literal::new_typed_literal(
            value,
            named_node(&datatype, "literal datatype")?,
        ))
    }
}

fn bnode_label(term: &Term, id: usize) -> Cow<'_, str> {
    match &term.value {
        Some(value) => Cow::Borrowed(value),
        None => Cow::Owned(format!("b{id}")),
    }
}

fn graph_name_id(graph_name: GraphNameRef<'_>, interner: &mut Interner) -> Option<usize> {
    match graph_name {
        GraphNameRef::DefaultGraph => None,
        GraphNameRef::NamedNode(node) => Some(interner.named_node_ref(node)),
        GraphNameRef::BlankNode(node) => Some(interner.blank_node_ref(node)),
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum TermKey {
    Iri(String),
    Bnode(String),
    Literal {
        value: String,
        lang: Option<String>,
        datatype: Option<usize>,
    },
    Triple(usize, usize, usize),
}

struct Interner {
    ids: HashMap<TermKey, usize>,
    terms: Vec<Term>,
}

impl Interner {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            terms: Vec::new(),
        }
    }

    fn named_node_ref(&mut self, node: NamedNodeRef<'_>) -> usize {
        self.intern(TermKey::Iri(node.as_str().to_string()), || Term {
            kind: TermKind::Iri,
            value: Some(node.as_str().to_string()),
            datatype: None,
            lang: None,
            reifier: None,
        })
    }

    fn blank_node_ref(&mut self, node: BlankNodeRef<'_>) -> usize {
        self.intern(TermKey::Bnode(node.as_str().to_string()), || Term {
            kind: TermKind::Bnode,
            value: Some(node.as_str().to_string()),
            datatype: None,
            lang: None,
            reifier: None,
        })
    }

    fn named_or_blank_ref(&mut self, node: NamedOrBlankNodeRef<'_>) -> usize {
        match node {
            NamedOrBlankNodeRef::NamedNode(node) => self.named_node_ref(node),
            NamedOrBlankNodeRef::BlankNode(node) => self.blank_node_ref(node),
        }
    }

    fn literal_ref(&mut self, literal: LiteralRef<'_>) -> usize {
        let value = literal.value().to_string();
        let lang = literal.language().map(str::to_string);
        let datatype_id = if lang.is_some() {
            None
        } else {
            let datatype = literal.datatype().as_str();
            if datatype == XSD_STRING || datatype == RDF_LANG_STRING {
                None
            } else {
                let iri = datatype.to_string();
                Some(self.intern(TermKey::Iri(iri.clone()), || Term {
                    kind: TermKind::Iri,
                    value: Some(iri),
                    datatype: None,
                    lang: None,
                    reifier: None,
                }))
            }
        };
        let key = TermKey::Literal {
            value: value.clone(),
            lang: lang.clone(),
            datatype: datatype_id,
        };
        self.intern(key, || Term {
            kind: TermKind::Literal,
            value: Some(value),
            datatype: datatype_id,
            lang,
            reifier: None,
        })
    }

    fn term_ref(&mut self, term: OxTermRef<'_>, reifiers: &mut BTreeMap<usize, Triple3>) -> usize {
        match term {
            OxTermRef::NamedNode(node) => self.named_node_ref(node),
            OxTermRef::BlankNode(node) => self.blank_node_ref(node),
            OxTermRef::Literal(literal) => self.literal_ref(literal),
            OxTermRef::Triple(triple) => {
                let (s, p, o) = self.triple_ref(triple.into(), reifiers);
                let key = TermKey::Triple(s, p, o);
                if let Some(id) = self.ids.get(&key) {
                    return *id;
                }
                let id = self.terms.len();
                self.terms.push(Term {
                    kind: TermKind::Triple,
                    value: None,
                    datatype: None,
                    lang: None,
                    reifier: Some(id),
                });
                self.ids.insert(key, id);
                reifiers.insert(id, (s, p, o));
                id
            }
        }
    }

    fn triple_ref(
        &mut self,
        triple: OxTripleRef<'_>,
        reifiers: &mut BTreeMap<usize, Triple3>,
    ) -> Triple3 {
        (
            self.named_or_blank_ref(triple.subject),
            self.named_node_ref(triple.predicate),
            self.term_ref(triple.object, reifiers),
        )
    }

    fn intern(&mut self, key: TermKey, make: impl FnOnce() -> Term) -> usize {
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let id = self.terms.len();
        self.terms.push(make());
        self.ids.insert(key, id);
        id
    }
}

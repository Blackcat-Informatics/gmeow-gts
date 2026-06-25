// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native RDF/XML mapper for the optional RDF text-codec surface.
//!
//! This module intentionally keeps XML tokenization separate from RDF
//! semantics: `quick-xml` supplies well-formed XML events, while the RDF/XML
//! production rules below lower the supported RDF/XML surface into the crate's
//! native RDF 1.2 dataset model.

use std::collections::{BTreeMap, HashMap};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::model::Graph;
use crate::rdf::{
    from_rdf_dataset, to_rdf_quads, BaseDirection, BlankNode, Dataset, GraphName, Iri, Literal,
    NamedOrBlankNode, RdfQuad, RdfTerm, RdfTriple,
};
use crate::rdf_codecs::RdfCodecError;
use crate::ulid::deterministic_label;

const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
const ITS_NS: &str = "http://www.w3.org/2005/11/its";
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

const RDF_DESCRIPTION: &str = "Description";
const RDF_ABOUT: &str = "about";
const RDF_ID: &str = "ID";
const RDF_NODE_ID: &str = "nodeID";
const RDF_RESOURCE: &str = "resource";
const RDF_DATATYPE: &str = "datatype";
const RDF_PARSE_TYPE: &str = "parseType";
const RDF_TYPE: &str = "type";
const RDF_VERSION: &str = "version";
const RDF_ANNOTATION: &str = "annotation";
const RDF_ANNOTATION_NODE_ID: &str = "annotationNodeID";
const RDF_REIFIES: &str = "reifies";
const RDF_FIRST: &str = "first";
const RDF_REST: &str = "rest";
const RDF_NIL: &str = "nil";
const RDF_STATEMENT: &str = "Statement";
const RDF_SUBJECT: &str = "subject";
const RDF_PREDICATE: &str = "predicate";
const RDF_OBJECT: &str = "object";
const RDF_XML_LITERAL: &str = "XMLLiteral";
const XML_BASE: &str = "base";
const XML_LANG: &str = "lang";
const ITS_DIR: &str = "dir";
const ITS_VERSION: &str = "version";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Name {
    raw: String,
    namespace: Option<String>,
    local: String,
}

impl Name {
    fn iri(&self) -> Result<Iri, RdfCodecError> {
        Iri::new(format!(
            "{}{}",
            self.namespace.as_deref().unwrap_or_default(),
            self.local
        ))
        .map_err(Into::into)
    }

    fn is_rdf(&self, local: &str) -> bool {
        self.namespace.as_deref() == Some(RDF_NS) && self.local == local
    }

    fn is_xml(&self, local: &str) -> bool {
        self.namespace.as_deref() == Some(XML_NS) && self.local == local
    }

    fn is_its(&self, local: &str) -> bool {
        self.namespace.as_deref() == Some(ITS_NS) && self.local == local
    }
}

#[derive(Clone, Debug)]
struct Attribute {
    name: Name,
    value: String,
}

#[derive(Clone, Debug)]
enum XmlNode {
    Element(Element),
    Text(String),
}

#[derive(Clone, Debug)]
struct Element {
    name: Name,
    attrs: Vec<Attribute>,
    children: Vec<XmlNode>,
    /// In-scope namespace declarations `(prefix, iri)` in source-declaration order
    /// (excluding the implicit `xml`), used to canonicalize `rdf:parseType="Literal"`
    /// XML literals (inherited namespaces are rendered on the literal's apex elements).
    ns_scope: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
struct ParseContext {
    base_iri: Option<String>,
    language: Option<String>,
    direction: Option<BaseDirection>,
    /// `rdf:version="1.2"` declared on this element or an ancestor: gates the RDF 1.2
    /// features (triple terms via `parseType="Triple"`, ITS base direction).
    rdf_version_12: bool,
    /// `its:version` declared (ITS 2.0 processing mode).
    its_version: bool,
}

impl ParseContext {
    fn for_child(&self, element: &Element) -> Result<Self, RdfCodecError> {
        let mut next = self.clone();
        // Version flags are sticky once declared on any ancestor.
        if element.attr_rdf(RDF_VERSION) == Some("1.2") {
            next.rdf_version_12 = true;
        }
        if element.attr_its(ITS_VERSION).is_some() {
            next.its_version = true;
        }
        if let Some(base) = element.attr_xml(XML_BASE) {
            next.base_iri = Some(match &self.base_iri {
                Some(parent) => resolve_relative_iri(parent, base),
                None => base.to_string(),
            });
        }
        if let Some(language) = element.attr_xml(XML_LANG) {
            next.language = (!language.is_empty()).then(|| language.to_string());
        }
        if let Some(direction) = element.attr_its(ITS_DIR) {
            let parsed = match direction {
                "ltr" => BaseDirection::Ltr,
                "rtl" => BaseDirection::Rtl,
                other => {
                    return Err(RdfCodecError::new(format!(
                        "RDF/XML parse error: invalid ITS direction {other:?}"
                    )))
                }
            };
            // RDF 1.2 base direction is suppressed in ITS 2.0 mode (`its:version`)
            // unless the document explicitly opts into RDF 1.2 via `rdf:version="1.2"`.
            next.direction = if next.its_version && !next.rdf_version_12 {
                None
            } else {
                Some(parsed)
            };
        }
        Ok(next)
    }
}

impl Element {
    fn attr_rdf(&self, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|attr| attr.name.is_rdf(local))
            .map(|attr| attr.value.as_str())
    }

    fn attr_xml(&self, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|attr| attr.name.is_xml(local))
            .map(|attr| attr.value.as_str())
    }

    fn attr_its(&self, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|attr| attr.name.is_its(local))
            .map(|attr| attr.value.as_str())
    }

    fn property_attrs(&self) -> impl Iterator<Item = &Attribute> {
        self.attrs
            .iter()
            .filter(|attr| attr.name.namespace.as_deref() != Some(XML_NS))
            .filter(|attr| attr.name.namespace.as_deref() != Some(ITS_NS))
            .filter(|attr| {
                !(attr.name.namespace.as_deref() == Some(RDF_NS)
                    && matches!(
                        attr.name.local.as_str(),
                        RDF_ABOUT
                            | RDF_ID
                            | RDF_NODE_ID
                            | RDF_RESOURCE
                            | RDF_DATATYPE
                            | RDF_PARSE_TYPE
                            | RDF_TYPE
                            | RDF_VERSION
                            | RDF_ANNOTATION
                            | RDF_ANNOTATION_NODE_ID
                    ))
            })
    }
}

/// Parse RDF/XML text into GTS bytes using the native RDF adapter.
pub(crate) fn from_rdf_xml(text: &str, base_iri: Option<&str>) -> Result<Vec<u8>, RdfCodecError> {
    let root = XmlDomParser::parse(text)?;
    let mut parser = RdfXmlParser {
        dataset: Dataset::new(),
        bnode_counter: 0,
        collection_counter: 0,
    };
    let context = ParseContext {
        base_iri: base_iri.map(str::to_string),
        ..Default::default()
    };
    parser.parse_document(&root, &context)?;
    from_rdf_dataset(&parser.dataset).map_err(Into::into)
}

/// Serialize a folded default graph to RDF/XML.
pub(crate) fn to_rdf_xml(graph: &Graph) -> Result<String, RdfCodecError> {
    let mut subjects: BTreeMap<String, Vec<(Iri, RdfTerm)>> = BTreeMap::new();
    let mut subject_nodes: BTreeMap<String, NamedOrBlankNode> = BTreeMap::new();
    for quad in to_rdf_quads(graph)? {
        if !quad.graph_name.is_default_graph() {
            return Err(RdfCodecError::new(format!(
                "RDF/XML cannot serialize named graph {}",
                quad.graph_name
            )));
        }
        let key = subject_key(&quad.subject);
        subject_nodes
            .entry(key.clone())
            .or_insert_with(|| quad.subject.clone());
        subjects
            .entry(key)
            .or_default()
            .push((quad.predicate, quad.object));
    }

    let namespaces = serializer_namespaces(&subjects);
    let mut out = String::from(
        "<?xml version=\"1.0\"?>\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema#\"",
    );
    for (namespace, prefix) in &namespaces {
        if prefix != "rdf" && prefix != "xsd" {
            out.push_str(&format!(
                " xmlns:{prefix}=\"{}\"",
                escape_xml_attr(namespace)
            ));
        }
    }
    // Declare RDF 1.2 so a round-trip preserves triple terms and base direction (their
    // parse is gated on `rdf:version="1.2"`).
    out.push_str(" rdf:version=\"1.2\">\n");

    for (key, properties) in subjects {
        let subject = subject_nodes
            .get(&key)
            .expect("subject node exists for every grouped subject");
        out.push_str("  <rdf:Description");
        match subject {
            NamedOrBlankNode::Iri(iri) => {
                out.push_str(&format!(" rdf:about=\"{}\"", escape_xml_attr(iri.as_str())));
            }
            NamedOrBlankNode::BlankNode(node) => {
                out.push_str(&format!(
                    " rdf:nodeID=\"{}\"",
                    escape_xml_attr(node.as_str())
                ));
            }
        }
        out.push_str(">\n");
        for (predicate, object) in properties {
            write_property(&mut out, "    ", &predicate, &object, &namespaces)?;
        }
        out.push_str("  </rdf:Description>\n");
    }

    out.push_str("</rdf:RDF>\n");
    Ok(out)
}

struct XmlDomParser;

impl XmlDomParser {
    fn parse(text: &str) -> Result<Element, RdfCodecError> {
        let mut reader = Reader::from_str(text);
        reader.config_mut().trim_text(false);
        let mut namespaces = vec![initial_namespaces()];
        let mut ns_order: Vec<Vec<(String, String)>> = vec![Vec::new()];
        let mut stack: Vec<Element> = Vec::new();
        let mut root: Option<Element> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(start)) => {
                    let element = parse_start(&start, &mut namespaces, &mut ns_order, &reader)?;
                    stack.push(element);
                }
                Ok(Event::Empty(start)) => {
                    let element = parse_start(&start, &mut namespaces, &mut ns_order, &reader)?;
                    namespaces.pop();
                    ns_order.pop();
                    attach_element(&mut stack, &mut root, element)?;
                }
                Ok(Event::Text(text)) => {
                    let text = text.unescape().map_err(xml_error)?.into_owned();
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlNode::Text(text));
                    } else if !text.trim().is_empty() {
                        return Err(RdfCodecError::new(
                            "RDF/XML parse error: text appears outside the document element",
                        ));
                    }
                }
                Ok(Event::CData(text)) => {
                    let text = String::from_utf8_lossy(text.as_ref()).into_owned();
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(XmlNode::Text(text));
                    }
                }
                Ok(Event::End(_)) => {
                    namespaces.pop();
                    ns_order.pop();
                    let element = stack.pop().ok_or_else(|| {
                        RdfCodecError::new("RDF/XML parse error: unmatched closing tag")
                    })?;
                    attach_element(&mut stack, &mut root, element)?;
                }
                Ok(Event::Decl(_) | Event::PI(_) | Event::Comment(_) | Event::DocType(_)) => {}
                Ok(Event::Eof) => break,
                Err(error) => return Err(xml_error(error)),
            }
        }

        if !stack.is_empty() {
            return Err(RdfCodecError::new(
                "RDF/XML parse error: document ended before all elements closed",
            ));
        }
        root.ok_or_else(|| RdfCodecError::new("RDF/XML parse error: missing document element"))
    }
}

fn parse_start(
    start: &BytesStart<'_>,
    namespaces: &mut Vec<HashMap<String, String>>,
    ns_order: &mut Vec<Vec<(String, String)>>,
    reader: &Reader<&[u8]>,
) -> Result<Element, RdfCodecError> {
    let raw_name = raw_xml_name(start.name().as_ref())?;
    let mut scope = namespaces
        .last()
        .cloned()
        .ok_or_else(|| RdfCodecError::new("RDF/XML parse error: missing namespace scope"))?;
    let mut order = ns_order.last().cloned().unwrap_or_default();

    let mut raw_attrs = Vec::new();
    for attr in start.attributes() {
        let attr = attr.map_err(xml_error)?;
        let raw = raw_xml_name(attr.key.as_ref())?;
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(xml_error)?
            .into_owned();
        if raw == "xmlns" {
            scope.insert(String::new(), value.clone());
            update_ns_order(&mut order, String::new(), value);
        } else if let Some(prefix) = raw.strip_prefix("xmlns:") {
            scope.insert(prefix.to_string(), value.clone());
            update_ns_order(&mut order, prefix.to_string(), value);
        } else {
            raw_attrs.push((raw, value));
        }
    }

    namespaces.push(scope.clone());
    ns_order.push(order.clone());
    let name = expand_name(&raw_name, &scope, true)?;
    let attrs = raw_attrs
        .into_iter()
        .map(|(raw, value)| {
            Ok(Attribute {
                name: expand_name(&raw, &scope, false)?,
                value,
            })
        })
        .collect::<Result<Vec<_>, RdfCodecError>>()?;

    Ok(Element {
        name,
        attrs,
        children: Vec::new(),
        ns_scope: order,
    })
}

/// Insert or update a namespace declaration in source-declaration order, keeping the
/// implicit `xml` prefix out (it is never rendered on canonicalized XML literals).
fn update_ns_order(order: &mut Vec<(String, String)>, prefix: String, iri: String) {
    if prefix == "xml" {
        return;
    }
    if let Some(slot) = order.iter_mut().find(|(p, _)| *p == prefix) {
        slot.1 = iri;
    } else {
        order.push((prefix, iri));
    }
}

fn initial_namespaces() -> HashMap<String, String> {
    HashMap::from([
        ("xml".to_string(), XML_NS.to_string()),
        ("rdf".to_string(), RDF_NS.to_string()),
    ])
}

fn raw_xml_name(bytes: &[u8]) -> Result<String, RdfCodecError> {
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|error| RdfCodecError::new(format!("RDF/XML parse error: {error}")))
}

fn expand_name(
    raw: &str,
    namespaces: &HashMap<String, String>,
    use_default: bool,
) -> Result<Name, RdfCodecError> {
    if let Some((prefix, local)) = raw.split_once(':') {
        let namespace = namespaces.get(prefix).ok_or_else(|| {
            RdfCodecError::new(format!(
                "RDF/XML parse error: unbound namespace prefix {prefix:?}"
            ))
        })?;
        return Ok(Name {
            raw: raw.to_string(),
            namespace: Some(namespace.clone()),
            local: local.to_string(),
        });
    }
    Ok(Name {
        raw: raw.to_string(),
        namespace: use_default
            .then(|| namespaces.get("").cloned())
            .flatten()
            .filter(|namespace| !namespace.is_empty()),
        local: raw.to_string(),
    })
}

fn attach_element(
    stack: &mut [Element],
    root: &mut Option<Element>,
    element: Element,
) -> Result<(), RdfCodecError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(XmlNode::Element(element));
    } else if root.replace(element).is_some() {
        return Err(RdfCodecError::new(
            "RDF/XML parse error: multiple document elements",
        ));
    }
    Ok(())
}

struct RdfXmlParser {
    dataset: Dataset,
    bnode_counter: usize,
    collection_counter: usize,
}

impl RdfXmlParser {
    fn parse_document(
        &mut self,
        root: &Element,
        context: &ParseContext,
    ) -> Result<(), RdfCodecError> {
        let context = context.for_child(root)?;
        if root.name.is_rdf("RDF") {
            for child in root.children.iter().filter_map(element_child) {
                self.parse_node_element(child, &context)?;
            }
        } else {
            self.parse_node_element(root, &context)?;
        }
        Ok(())
    }

    fn parse_node_element(
        &mut self,
        element: &Element,
        parent_context: &ParseContext,
    ) -> Result<NamedOrBlankNode, RdfCodecError> {
        let context = parent_context.for_child(element)?;
        let subject = self.subject_for_node(element, &context)?;

        if !element.name.is_rdf(RDF_DESCRIPTION) {
            self.insert_statement(
                subject.clone(),
                rdf_iri(RDF_TYPE)?,
                element.name.iri()?.into(),
                None,
                None,
            )?;
        }
        if let Some(type_iri) = element.attr_rdf(RDF_TYPE) {
            self.insert_statement(
                subject.clone(),
                rdf_iri(RDF_TYPE)?,
                self.iri_ref(type_iri, &context)?.into(),
                None,
                None,
            )?;
        }

        for attr in element.property_attrs() {
            let predicate = attr.name.iri()?;
            let literal = self.context_literal(&attr.value, None, &context)?;
            self.insert_statement(subject.clone(), predicate, literal.into(), None, None)?;
        }

        for child in element.children.iter().filter_map(element_child) {
            self.parse_property_element(&subject, child, &context)?;
        }
        Ok(subject)
    }

    fn parse_property_element(
        &mut self,
        subject: &NamedOrBlankNode,
        element: &Element,
        parent_context: &ParseContext,
    ) -> Result<(), RdfCodecError> {
        let context = parent_context.for_child(element)?;
        let predicate = element.name.iri()?;
        let reifier = element
            .attr_rdf(RDF_ID)
            .map(|id| self.rdf_id_iri(id, &context).map(NamedOrBlankNode::from))
            .transpose()?;
        // `rdf:annotation="IRI"` and `rdf:annotationNodeID="id"` both name the reifier
        // of the asserted triple; the former is an IRI, the latter a blank node.
        let annotation = match element.attr_rdf(RDF_ANNOTATION) {
            Some(annotation) => Some(self.iri_ref(annotation, &context)?.into()),
            None => match element.attr_rdf(RDF_ANNOTATION_NODE_ID) {
                Some(node_id) => Some(BlankNode::new(node_id)?.into()),
                None => None,
            },
        };

        if let Some(resource) = element.attr_rdf(RDF_RESOURCE) {
            let object: NamedOrBlankNode = self.iri_ref(resource, &context)?.into();
            self.insert_statement(
                subject.clone(),
                predicate,
                named_or_blank_term(&object),
                reifier,
                annotation,
            )?;
            self.insert_property_attribute_statements(&object, element, &context)?;
            return Ok(());
        }
        if let Some(node_id) = element.attr_rdf(RDF_NODE_ID) {
            let object: NamedOrBlankNode = BlankNode::new(node_id)?.into();
            self.insert_statement(
                subject.clone(),
                predicate,
                named_or_blank_term(&object),
                reifier,
                annotation,
            )?;
            self.insert_property_attribute_statements(&object, element, &context)?;
            return Ok(());
        }

        match element.attr_rdf(RDF_PARSE_TYPE) {
            Some("Resource") => {
                let object = self.fresh_bnode()?;
                self.insert_statement(
                    subject.clone(),
                    predicate,
                    named_or_blank_term(&object),
                    reifier,
                    annotation,
                )?;
                self.insert_property_attribute_statements(&object, element, &context)?;
                for child in element.children.iter().filter_map(element_child) {
                    self.parse_property_element(&object, child, &context)?;
                }
                return Ok(());
            }
            Some("Collection") => {
                let head = self.parse_collection(element, &context)?;
                return self.insert_statement(
                    subject.clone(),
                    predicate,
                    head,
                    reifier,
                    annotation,
                );
            }
            Some("Literal") => {
                let xml_literal = serialize_children_as_xml(element);
                let literal = Literal::new_typed_literal(xml_literal, rdf_iri(RDF_XML_LITERAL)?);
                return self.insert_statement(
                    subject.clone(),
                    predicate,
                    literal.into(),
                    reifier,
                    annotation,
                );
            }
            Some("Triple") => {
                // A triple term is an RDF 1.2 feature: without `rdf:version="1.2"` the
                // whole property is ignored (W3C `rdf12-xml-tt-01`, "Ignored triple term").
                if !context.rdf_version_12 {
                    return Ok(());
                }
                let triple = self.parse_triple_element(element, &context)?;
                return self.insert_statement(
                    subject.clone(),
                    predicate,
                    RdfTerm::Triple(Box::new(triple)),
                    reifier,
                    annotation,
                );
            }
            Some(other) => {
                return Err(RdfCodecError::new(format!(
                    "RDF/XML parse error: unsupported rdf:parseType {other:?}"
                )));
            }
            None => {}
        }

        let element_children: Vec<&Element> =
            element.children.iter().filter_map(element_child).collect();
        if let Some(datatype) = element.attr_rdf(RDF_DATATYPE) {
            if !element_children.is_empty() {
                return Err(RdfCodecError::new(
                    "RDF/XML parse error: rdf:datatype property cannot contain node elements",
                ));
            }
            let literal = Literal::new_typed_literal(
                element_text(element),
                self.iri_ref(datatype, &context)?,
            );
            return self.insert_statement(
                subject.clone(),
                predicate,
                literal.into(),
                reifier,
                annotation,
            );
        }

        if element_children.len() == 1 {
            let object = self.parse_node_element(element_children[0], &context)?;
            return self.insert_statement(
                subject.clone(),
                predicate,
                named_or_blank_term(&object),
                reifier,
                annotation,
            );
        }
        if element_children.len() > 1 {
            return Err(RdfCodecError::new(
                "RDF/XML parse error: property element contains more than one node element",
            ));
        }

        if element.property_attrs().next().is_some() {
            let object = self.fresh_bnode()?;
            self.insert_statement(
                subject.clone(),
                predicate,
                named_or_blank_term(&object),
                reifier,
                annotation,
            )?;
            self.insert_property_attribute_statements(&object, element, &context)?;
            return Ok(());
        }

        let literal = self.context_literal(&element_text(element), None, &context)?;
        self.insert_statement(
            subject.clone(),
            predicate,
            literal.into(),
            reifier,
            annotation,
        )
    }

    fn insert_property_attribute_statements(
        &mut self,
        subject: &NamedOrBlankNode,
        element: &Element,
        context: &ParseContext,
    ) -> Result<(), RdfCodecError> {
        for attr in element.property_attrs() {
            let literal = self.context_literal(&attr.value, None, context)?;
            self.insert_statement(
                subject.clone(),
                attr.name.iri()?,
                literal.into(),
                None,
                None,
            )?;
        }
        Ok(())
    }

    fn parse_collection(
        &mut self,
        element: &Element,
        context: &ParseContext,
    ) -> Result<RdfTerm, RdfCodecError> {
        let items: Vec<&Element> = element.children.iter().filter_map(element_child).collect();
        if items.is_empty() {
            return Ok(rdf_iri(RDF_NIL)?.into());
        }
        let nodes = (0..items.len())
            .map(|_| self.fresh_collection_bnode())
            .collect::<Result<Vec<_>, _>>()?;
        for (index, item) in items.iter().enumerate() {
            let object = self.parse_node_element(item, context)?;
            self.insert_statement(
                nodes[index].clone(),
                rdf_iri(RDF_FIRST)?,
                named_or_blank_term(&object),
                None,
                None,
            )?;
            let rest: RdfTerm = if let Some(next) = nodes.get(index + 1) {
                named_or_blank_term(next)
            } else {
                rdf_iri(RDF_NIL)?.into()
            };
            self.insert_statement(nodes[index].clone(), rdf_iri(RDF_REST)?, rest, None, None)?;
        }
        Ok(named_or_blank_term(
            nodes.first().expect("non-empty collection has a head node"),
        ))
    }

    fn parse_triple_element(
        &mut self,
        element: &Element,
        context: &ParseContext,
    ) -> Result<RdfTriple, RdfCodecError> {
        let nodes: Vec<&Element> = element.children.iter().filter_map(element_child).collect();
        if nodes.len() != 1 {
            return Err(RdfCodecError::new(
                "RDF/XML parse error: rdf:parseType=\"Triple\" requires one node element",
            ));
        }
        let node = nodes[0];
        let triple_subject = self.subject_for_node(node, context)?;
        let node_ctx = context.for_child(node)?;

        // The single predicate/object may come from a child property element, a
        // `rdf:type` attribute, or another property attribute (literal-valued).
        let type_attr = node.attr_rdf(RDF_TYPE);
        let prop_attrs: Vec<&Attribute> = node.property_attrs().collect();
        let child_props: Vec<&Element> = node.children.iter().filter_map(element_child).collect();
        if type_attr.is_some() as usize + prop_attrs.len() + child_props.len() != 1 {
            return Err(RdfCodecError::new(
                "RDF/XML parse error: rdf:parseType=\"Triple\" requires exactly one predicate/object",
            ));
        }
        let (predicate, object): (Iri, RdfTerm) = if let Some(type_iri) = type_attr {
            (
                rdf_iri(RDF_TYPE)?,
                self.iri_ref(type_iri, &node_ctx)?.into(),
            )
        } else if let Some(attr) = prop_attrs.first() {
            (
                attr.name.iri()?,
                self.context_literal(&attr.value, None, &node_ctx)?.into(),
            )
        } else {
            (
                child_props[0].name.iri()?,
                self.triple_object(child_props[0], context)?,
            )
        };
        Ok(RdfTriple::new(triple_subject, predicate, object))
    }

    fn triple_object(
        &mut self,
        property: &Element,
        context: &ParseContext,
    ) -> Result<RdfTerm, RdfCodecError> {
        let context = context.for_child(property)?;
        if let Some(resource) = property.attr_rdf(RDF_RESOURCE) {
            return Ok(self.iri_ref(resource, &context)?.into());
        }
        if let Some(node_id) = property.attr_rdf(RDF_NODE_ID) {
            return Ok(BlankNode::new(node_id)?.into());
        }
        if let Some("Triple") = property.attr_rdf(RDF_PARSE_TYPE) {
            return Ok(RdfTerm::Triple(Box::new(
                self.parse_triple_element(property, &context)?,
            )));
        }
        let nodes: Vec<&Element> = property.children.iter().filter_map(element_child).collect();
        if nodes.len() == 1 {
            let object = self.subject_for_node(nodes[0], &context)?;
            return Ok(named_or_blank_term(&object));
        }
        if nodes.len() > 1 {
            return Err(RdfCodecError::new(
                "RDF/XML parse error: rdf:parseType=\"Triple\" object has multiple node elements",
            ));
        }
        Ok(self
            .context_literal(
                &element_text(property),
                property.attr_rdf(RDF_DATATYPE),
                &context,
            )?
            .into())
    }

    fn subject_for_node(
        &mut self,
        element: &Element,
        context: &ParseContext,
    ) -> Result<NamedOrBlankNode, RdfCodecError> {
        if let Some(about) = element.attr_rdf(RDF_ABOUT) {
            return Ok(self.iri_ref(about, context)?.into());
        }
        if let Some(id) = element.attr_rdf(RDF_ID) {
            return Ok(self.rdf_id_iri(id, context)?.into());
        }
        if let Some(node_id) = element.attr_rdf(RDF_NODE_ID) {
            return Ok(BlankNode::new(node_id)?.into());
        }
        self.fresh_bnode()
    }

    fn insert_statement(
        &mut self,
        subject: NamedOrBlankNode,
        predicate: Iri,
        object: RdfTerm,
        reifier: Option<NamedOrBlankNode>,
        annotation: Option<NamedOrBlankNode>,
    ) -> Result<(), RdfCodecError> {
        self.dataset.insert(RdfQuad::new(
            subject.clone(),
            predicate.clone(),
            object.clone(),
            GraphName::DefaultGraph,
        ));
        // `rdf:ID` on a property element is RDF 1.0 reification (the classic
        // rdf:Statement/subject/predicate/object quads); `rdf:annotation` /
        // `rdf:annotationNodeID` is the RDF 1.2 reifier (rdf:reifies a triple term).
        if let Some(reifier) = reifier {
            self.insert_classic_reification(
                reifier,
                subject.clone(),
                predicate.clone(),
                object.clone(),
            )?;
        }
        if let Some(annotation) = annotation {
            self.insert_reifier(annotation, subject, predicate, object)?;
        }
        Ok(())
    }

    /// Emit the RDF 1.0 reification quads for a property element carrying `rdf:ID`.
    fn insert_classic_reification(
        &mut self,
        reifier: NamedOrBlankNode,
        subject: NamedOrBlankNode,
        predicate: Iri,
        object: RdfTerm,
    ) -> Result<(), RdfCodecError> {
        let g = GraphName::DefaultGraph;
        self.dataset.insert(RdfQuad::new(
            reifier.clone(),
            rdf_iri(RDF_TYPE)?,
            rdf_iri(RDF_STATEMENT)?,
            g.clone(),
        ));
        self.dataset.insert(RdfQuad::new(
            reifier.clone(),
            rdf_iri(RDF_SUBJECT)?,
            named_or_blank_term(&subject),
            g.clone(),
        ));
        self.dataset.insert(RdfQuad::new(
            reifier.clone(),
            rdf_iri(RDF_PREDICATE)?,
            predicate,
            g.clone(),
        ));
        self.dataset
            .insert(RdfQuad::new(reifier, rdf_iri(RDF_OBJECT)?, object, g));
        Ok(())
    }

    fn insert_reifier(
        &mut self,
        reifier: NamedOrBlankNode,
        subject: NamedOrBlankNode,
        predicate: Iri,
        object: RdfTerm,
    ) -> Result<(), RdfCodecError> {
        let quoted = RdfTerm::Triple(Box::new(RdfTriple::new(subject, predicate, object)));
        self.dataset.insert(RdfQuad::new(
            reifier,
            rdf_iri(RDF_REIFIES)?,
            quoted,
            GraphName::DefaultGraph,
        ));
        Ok(())
    }

    fn context_literal(
        &self,
        lexical: &str,
        datatype: Option<&str>,
        context: &ParseContext,
    ) -> Result<Literal, RdfCodecError> {
        if let Some(datatype) = datatype {
            return Ok(Literal::new_typed_literal(
                lexical,
                self.iri_ref(datatype, context)?,
            ));
        }
        if let Some(language) = &context.language {
            if let Some(direction) = context.direction {
                return Literal::new_directional_language_tagged_literal(
                    lexical, language, direction,
                )
                .map_err(Into::into);
            }
            return Literal::new_language_tagged_literal(lexical, language).map_err(Into::into);
        }
        Ok(Literal::new_simple_literal(lexical))
    }

    fn iri_ref(&self, value: &str, context: &ParseContext) -> Result<Iri, RdfCodecError> {
        let iri = if has_iri_scheme(value) {
            value.to_string()
        } else if let Some(base) = &context.base_iri {
            resolve_relative_iri(base, value)
        } else {
            value.to_string()
        };
        Iri::new(iri).map_err(Into::into)
    }

    fn rdf_id_iri(&self, value: &str, context: &ParseContext) -> Result<Iri, RdfCodecError> {
        if value.is_empty() {
            return Err(RdfCodecError::new("RDF/XML parse error: empty rdf:ID"));
        }
        let Some(base) = &context.base_iri else {
            return Iri::new(format!("#{value}")).map_err(Into::into);
        };
        let base_without_fragment = base
            .split_once('#')
            .map_or(base.as_str(), |(before, _)| before);
        Iri::new(format!("{base_without_fragment}#{value}")).map_err(Into::into)
    }

    fn fresh_bnode(&mut self) -> Result<NamedOrBlankNode, RdfCodecError> {
        let id = self.bnode_counter;
        self.bnode_counter += 1;
        Ok(BlankNode::new(deterministic_label("rdfxml_", id as u128))?.into())
    }

    fn fresh_collection_bnode(&mut self) -> Result<NamedOrBlankNode, RdfCodecError> {
        let id = self.collection_counter;
        self.collection_counter += 1;
        Ok(BlankNode::new(deterministic_label("rdfxml_list_", id as u128))?.into())
    }
}

fn element_child(node: &XmlNode) -> Option<&Element> {
    match node {
        XmlNode::Element(element) => Some(element),
        XmlNode::Text(_) => None,
    }
}

fn element_text(element: &Element) -> String {
    element
        .children
        .iter()
        .filter_map(|node| match node {
            XmlNode::Text(text) => Some(text.as_str()),
            XmlNode::Element(_) => None,
        })
        .collect()
}

fn rdf_iri(local: &str) -> Result<Iri, RdfCodecError> {
    Iri::new(format!("{RDF_NS}{local}")).map_err(Into::into)
}

fn xml_error(error: impl std::fmt::Display) -> RdfCodecError {
    RdfCodecError::new(format!("RDF/XML parse error: {error}"))
}

fn subject_key(subject: &NamedOrBlankNode) -> String {
    match subject {
        NamedOrBlankNode::Iri(iri) => format!("I{}", iri.as_str()),
        NamedOrBlankNode::BlankNode(node) => format!("B{}", node.as_str()),
    }
}

fn serializer_namespaces(
    subjects: &BTreeMap<String, Vec<(Iri, RdfTerm)>>,
) -> BTreeMap<String, String> {
    let mut namespaces = BTreeMap::from([
        (RDF_NS.to_string(), "rdf".to_string()),
        (XSD_NS.to_string(), "xsd".to_string()),
    ]);
    let mut next = 0usize;
    for properties in subjects.values() {
        for (predicate, _) in properties {
            let namespace = split_property_iri(predicate.as_str()).0;
            if namespaces.contains_key(namespace) {
                continue;
            }
            namespaces.insert(namespace.to_string(), format!("ns{next}"));
            next += 1;
        }
    }
    namespaces
}

fn write_property(
    out: &mut String,
    indent: &str,
    predicate: &Iri,
    object: &RdfTerm,
    namespaces: &BTreeMap<String, String>,
) -> Result<(), RdfCodecError> {
    let name = serializer_qname(predicate.as_str(), namespaces);
    match object {
        RdfTerm::Iri(iri) => {
            out.push_str(&format!(
                "{indent}<{name} rdf:resource=\"{}\"/>\n",
                escape_xml_attr(iri.as_str())
            ));
        }
        RdfTerm::BlankNode(node) => {
            out.push_str(&format!(
                "{indent}<{name} rdf:nodeID=\"{}\"/>\n",
                escape_xml_attr(node.as_str())
            ));
        }
        RdfTerm::Literal(literal) => {
            out.push_str(&format!("{indent}<{name}"));
            if let Some(language) = &literal.language {
                out.push_str(&format!(" xml:lang=\"{}\"", escape_xml_attr(language)));
            }
            if let Some(direction) = literal.direction {
                out.push_str(&format!(" xmlns:its=\"{ITS_NS}\" its:dir=\"{direction}\""));
            }
            if let Some(datatype) = &literal.datatype {
                out.push_str(&format!(
                    " rdf:datatype=\"{}\"",
                    escape_xml_attr(datatype.as_str())
                ));
            }
            out.push_str(&format!(
                ">{}</{name}>\n",
                escape_xml_text(&literal.lexical)
            ));
        }
        RdfTerm::Triple(triple) => {
            out.push_str(&format!("{indent}<{name} rdf:parseType=\"Triple\">\n"));
            write_triple_node(out, &format!("{indent}  "), triple, namespaces)?;
            out.push_str(&format!("{indent}</{name}>\n"));
        }
    }
    Ok(())
}

fn write_triple_node(
    out: &mut String,
    indent: &str,
    triple: &RdfTriple,
    namespaces: &BTreeMap<String, String>,
) -> Result<(), RdfCodecError> {
    out.push_str(&format!("{indent}<rdf:Description"));
    match &triple.subject {
        NamedOrBlankNode::Iri(iri) => {
            out.push_str(&format!(" rdf:about=\"{}\"", escape_xml_attr(iri.as_str())));
        }
        NamedOrBlankNode::BlankNode(node) => {
            out.push_str(&format!(
                " rdf:nodeID=\"{}\"",
                escape_xml_attr(node.as_str())
            ));
        }
    }
    out.push_str(">\n");
    write_property(
        out,
        &format!("{indent}  "),
        &triple.predicate,
        &triple.object,
        namespaces,
    )?;
    out.push_str(&format!("{indent}</rdf:Description>\n"));
    Ok(())
}

fn serializer_qname(iri: &str, namespaces: &BTreeMap<String, String>) -> String {
    let (namespace, local) = split_property_iri(iri);
    let prefix = namespaces
        .get(namespace)
        .map(String::as_str)
        .unwrap_or("ns");
    format!("{prefix}:{local}")
}

fn split_property_iri(iri: &str) -> (&str, &str) {
    let split = iri
        .rfind(['#', '/', ':'])
        .map(|index| index + 1)
        .unwrap_or(0);
    let (namespace, local) = iri.split_at(split);
    if local.is_empty() || !is_xml_name(local) {
        (iri, "property")
    } else {
        (namespace, local)
    }
}

fn is_xml_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_xml_name_start(first) {
        return false;
    }
    chars.all(is_xml_name_char)
}

fn is_xml_name_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_xml_name_char(ch: char) -> bool {
    is_xml_name_start(ch) || ch.is_numeric() || matches!(ch, '-' | '.')
}

fn serialize_children_as_xml(element: &Element) -> String {
    let mut out = String::new();
    for child in &element.children {
        // The literal's apex elements carry the in-scope namespace declarations
        // (inclusive canonicalization); descendants inherit them and add none.
        serialize_xml_node(child, Some(&element.ns_scope), &mut out);
    }
    out
}

fn serialize_xml_node(node: &XmlNode, apex_ns: Option<&[(String, String)]>, out: &mut String) {
    match node {
        XmlNode::Text(text) => out.push_str(&escape_xml_text(text)),
        XmlNode::Element(element) => {
            out.push('<');
            out.push_str(&element.name.raw);
            if let Some(namespaces) = apex_ns {
                for (prefix, iri) in namespaces {
                    if prefix.is_empty() {
                        out.push_str(&format!(" xmlns=\"{}\"", escape_xml_attr(iri)));
                    } else {
                        out.push_str(&format!(" xmlns:{prefix}=\"{}\"", escape_xml_attr(iri)));
                    }
                }
            }
            for attr in &element.attrs {
                out.push(' ');
                out.push_str(&attr.name.raw);
                out.push_str("=\"");
                out.push_str(&escape_xml_attr(&attr.value));
                out.push('"');
            }
            // Canonical XML has no self-closing form: always emit a start/end pair.
            out.push('>');
            for child in &element.children {
                serialize_xml_node(child, None, out);
            }
            out.push_str("</");
            out.push_str(&element.name.raw);
            out.push('>');
        }
    }
}

fn escape_xml_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_attr(value: &str) -> String {
    escape_xml_text(value).replace('"', "&quot;")
}

fn named_or_blank_term(node: &NamedOrBlankNode) -> RdfTerm {
    match node {
        NamedOrBlankNode::Iri(iri) => iri.clone().into(),
        NamedOrBlankNode::BlankNode(node) => node.clone().into(),
    }
}

fn has_iri_scheme(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    for ch in chars {
        if ch == ':' {
            return true;
        }
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

fn remove_dot_segments(path: &str) -> String {
    let absolute = path.starts_with('/');
    let keep_trailing_slash = path.ends_with('/')
        || path.ends_with("/.")
        || path.ends_with("/..")
        || path == "."
        || path == "..";
    let mut segments = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            segment => segments.push(segment),
        }
    }

    let mut normalized = String::new();
    if absolute {
        normalized.push('/');
    }
    normalized.push_str(&segments.join("/"));
    if keep_trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    if normalized.is_empty() && absolute {
        normalized.push('/');
    }
    normalized
}

fn split_raw_path_suffix(raw: &str) -> (&str, &str) {
    let split = raw.find(['?', '#']).unwrap_or(raw.len());
    (&raw[..split], &raw[split..])
}

fn split_base_for_path(base: &str) -> (String, &str) {
    let Some(scheme_end) = base.find(':') else {
        return (String::new(), base);
    };
    let scheme_prefix = &base[..=scheme_end];
    let rest = &base[scheme_end + 1..];
    if let Some(after_slashes) = rest.strip_prefix("//") {
        let authority_end = after_slashes.find('/').unwrap_or(after_slashes.len());
        let authority = &after_slashes[..authority_end];
        let path = &after_slashes[authority_end..];
        (format!("{scheme_prefix}//{authority}"), path)
    } else {
        (scheme_prefix.to_string(), rest)
    }
}

fn resolve_relative_iri(base: &str, raw: &str) -> String {
    if has_iri_scheme(raw) {
        return raw.to_string();
    }

    let base_without_fragment = base.split_once('#').map_or(base, |(before, _)| before);
    if raw.is_empty() {
        return base_without_fragment.to_string();
    }
    if raw.starts_with('#') {
        return format!("{base_without_fragment}{raw}");
    }

    let base_without_query = base_without_fragment
        .split_once('?')
        .map_or(base_without_fragment, |(before, _)| before);
    if raw.starts_with('?') {
        return format!("{base_without_query}{raw}");
    }

    if raw.starts_with("//") {
        if let Some(scheme_end) = base.find(':') {
            return format!("{}:{raw}", &base[..scheme_end]);
        }
        return raw.to_string();
    }

    let (prefix, base_path) = split_base_for_path(base_without_query);
    let (raw_path, suffix) = split_raw_path_suffix(raw);
    let merged_path = if raw_path.starts_with('/') {
        raw_path.to_string()
    } else {
        let base_dir = if base_path.is_empty() {
            "/"
        } else {
            base_path
                .rfind('/')
                .map(|index| &base_path[..=index])
                .unwrap_or("")
        };
        format!("{base_dir}{raw_path}")
    };
    format!("{prefix}{}{}", remove_dot_segments(&merged_path), suffix)
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `YAML-LD-star -> gts` transform: inverse of [`crate::yamlld`].
//!
//! The parser accepts the deterministic profile emitted by `yamlld`, plus the
//! compact context form used by downstream authoring tools. It builds a
//! canonical segment through [`crate::writer::Writer`] rather than preserving
//! source syntax.

use std::collections::HashMap;
use std::fmt;

use serde_json::{Map, Number, Value};

use crate::model::{Graph, Quad, Term, TermKind, Triple3};
use crate::writer::Writer;
use crate::yamlld::{
    ANNOTATION, GTS_GRAPH, GTS_REIFIERS, GTS_SUBJECT, GTS_TRIPLE, RDF_TYPE, XSD_BOOLEAN,
    XSD_DECIMAL, XSD_INTEGER,
};

/// Raised when YAML-LD-star or JSON-LD-star input is outside the GTS core profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlLdParseError {
    detail: String,
}

impl YamlLdParseError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for YamlLdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for YamlLdParseError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TermKey {
    Atom {
        kind: TermKind,
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<String>,
    },
    Triple(usize, usize, usize),
}

struct Interner {
    ids: HashMap<TermKey, usize>,
    terms: Vec<Term>,
    generated_bnodes: usize,
}

impl Interner {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            terms: Vec::new(),
            generated_bnodes: 0,
        }
    }

    fn atom(
        &mut self,
        kind: TermKind,
        value: String,
        lang: Option<String>,
        direction: Option<String>,
        datatype: Option<String>,
    ) -> usize {
        let key = TermKey::Atom {
            kind,
            value: value.clone(),
            lang: lang.clone(),
            direction: direction.clone(),
            datatype: datatype.clone(),
        };
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let datatype_id = if kind == TermKind::Literal {
            datatype
                .as_ref()
                .map(|iri| self.atom(TermKind::Iri, iri.clone(), None, None, None))
        } else {
            None
        };
        let id = self.terms.len();
        self.terms.push(Term {
            kind,
            value: Some(value),
            datatype: datatype_id,
            lang,
            direction,
            reifier: None,
        });
        self.ids.insert(key, id);
        id
    }

    fn triple(&mut self, statement: Triple3, reifiers: &mut Vec<(usize, Triple3)>) -> usize {
        let key = TermKey::Triple(statement.0, statement.1, statement.2);
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let id = self.terms.len();
        self.terms.push(Term {
            kind: TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(id),
        });
        self.ids.insert(key, id);
        set_reifier(reifiers, id, statement);
        id
    }

    fn generated_bnode(&mut self, prefix: &str) -> usize {
        loop {
            let label = format!("{prefix}{}", self.generated_bnodes);
            self.generated_bnodes += 1;
            let key = TermKey::Atom {
                kind: TermKind::Bnode,
                value: label.clone(),
                lang: None,
                direction: None,
                datatype: None,
            };
            if !self.ids.contains_key(&key) {
                return self.atom(TermKind::Bnode, label, None, None, None);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Context {
    prefixes: HashMap<String, String>,
}

impl Context {
    fn from_document(value: &Value) -> Self {
        let mut context = Self {
            prefixes: HashMap::from([
                (
                    "rdf".to_string(),
                    "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
                ),
                (
                    "xsd".to_string(),
                    "http://www.w3.org/2001/XMLSchema#".to_string(),
                ),
            ]),
        };
        if let Value::Object(map) = value {
            if let Some(raw) = map.get("@context") {
                context.merge(raw);
            }
        }
        context
    }

    fn merge(&mut self, value: &Value) {
        match value {
            Value::Array(items) => {
                for item in items {
                    self.merge(item);
                }
            }
            Value::Object(map) => {
                for (prefix, definition) in map {
                    match definition {
                        Value::String(iri) => {
                            self.prefixes.insert(prefix.clone(), iri.clone());
                        }
                        Value::Object(definition) => {
                            if let Some(Value::String(iri)) = definition.get("@id") {
                                self.prefixes.insert(prefix.clone(), iri.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn expand(&self, value: &str) -> String {
        if value.starts_with("_:") || value.starts_with('<') && value.ends_with('>') {
            return value
                .strip_prefix('<')
                .and_then(|inner| inner.strip_suffix('>'))
                .unwrap_or(value)
                .to_string();
        }
        let Some((prefix, suffix)) = value.split_once(':') else {
            return value.to_string();
        };
        match self.prefixes.get(prefix) {
            Some(base) => format!("{base}{suffix}"),
            None => value.to_string(),
        }
    }
}

/// Parse YAML-LD-star text into a canonical GTS file.
pub fn from_yaml_ld(text: &str) -> Result<Vec<u8>, YamlLdParseError> {
    let value: Value = serde_yaml::from_str(text)
        .map_err(|error| YamlLdParseError::new(format!("invalid YAML-LD: {error}")))?;
    from_json_ld_value(&value)
}

/// Parse JSON-LD-star text into a canonical GTS file.
pub fn from_json_ld(text: &str) -> Result<Vec<u8>, YamlLdParseError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| YamlLdParseError::new(format!("invalid JSON-LD: {error}")))?;
    from_json_ld_value(&value)
}

fn from_json_ld_value(value: &Value) -> Result<Vec<u8>, YamlLdParseError> {
    let context = Context::from_document(value);
    let mut interner = Interner::new();
    let mut quads: Vec<Quad> = Vec::new();
    let mut reifiers: Vec<(usize, Triple3)> = Vec::new();
    let mut annotations: Vec<Triple3> = Vec::new();

    for node in graph_nodes(value)? {
        parse_node(
            node,
            &context,
            &mut interner,
            &mut quads,
            &mut reifiers,
            &mut annotations,
        )?;
    }
    if let Value::Object(map) = value {
        if let Some(blocks) = map.get(GTS_REIFIERS) {
            parse_standalone_reifiers(
                blocks,
                &context,
                &mut interner,
                &mut reifiers,
                &mut annotations,
            )?;
        }
    }

    let graph = Graph {
        terms: interner.terms,
        quads,
        reifiers,
        annotations,
        ..Graph::default()
    };
    let writer = Writer::deterministic(&graph, "dist")
        .map_err(|error| YamlLdParseError::new(format!("cannot author GTS: {error}")))?;
    Ok(writer.to_bytes())
}

fn graph_nodes(value: &Value) -> Result<Vec<&Value>, YamlLdParseError> {
    match value {
        Value::Array(nodes) => Ok(nodes.iter().collect()),
        Value::Object(map) => match map.get("@graph") {
            Some(Value::Array(nodes)) => Ok(nodes.iter().collect()),
            Some(_) => Err(YamlLdParseError::new("@graph must be an array")),
            None => Ok(vec![value]),
        },
        _ => Err(YamlLdParseError::new(
            "YAML-LD document must be a node or graph",
        )),
    }
}

fn parse_node(
    value: &Value,
    context: &Context,
    interner: &mut Interner,
    quads: &mut Vec<Quad>,
    reifiers: &mut Vec<(usize, Triple3)>,
    annotations: &mut Vec<Triple3>,
) -> Result<(), YamlLdParseError> {
    let map = object(value, "graph node")?;
    let scoped_context = scoped_context(context, map);
    let subject = match (map.get("@id"), map.get(GTS_SUBJECT)) {
        (Some(id), None) => parse_id(id, &scoped_context, interner)?,
        (None, Some(subject)) => parse_term(subject, false, &scoped_context, interner, reifiers)?,
        (Some(_), Some(_)) => {
            return Err(YamlLdParseError::new(
                "graph node cannot contain both @id and gts:subject",
            ))
        }
        (None, None) => return Err(YamlLdParseError::new("graph node is missing @id")),
    };

    for (key, raw_values) in map {
        if matches!(
            key.as_str(),
            "@context" | "@id" | "@graph" | GTS_SUBJECT | GTS_REIFIERS
        ) {
            continue;
        }
        let type_position = key == "@type";
        if key.starts_with('@') && !type_position {
            return Err(YamlLdParseError::new(format!(
                "unsupported node keyword {key}"
            )));
        }
        let predicate = predicate_id(key, &scoped_context, interner);
        parse_property_values(
            raw_values,
            type_position,
            subject,
            predicate,
            &scoped_context,
            interner,
            quads,
            reifiers,
            annotations,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn parse_property_values(
    value: &Value,
    type_position: bool,
    subject: usize,
    predicate: usize,
    context: &Context,
    interner: &mut Interner,
    quads: &mut Vec<Quad>,
    reifiers: &mut Vec<(usize, Triple3)>,
    annotations: &mut Vec<Triple3>,
) -> Result<(), YamlLdParseError> {
    match value {
        Value::Array(items) => {
            for item in items {
                parse_property_values(
                    item,
                    type_position,
                    subject,
                    predicate,
                    context,
                    interner,
                    quads,
                    reifiers,
                    annotations,
                )?;
            }
        }
        item => {
            let active_context = match item {
                Value::Object(map) => scoped_context(context, map),
                _ => context.clone(),
            };
            let object_id = parse_term(item, type_position, &active_context, interner, reifiers)?;
            let graph_name = match item {
                Value::Object(map) => map
                    .get(GTS_GRAPH)
                    .map(|value| parse_term(value, false, &active_context, interner, reifiers))
                    .transpose()?,
                _ => None,
            };
            quads.push((subject, predicate, object_id, graph_name));
            if let Value::Object(map) = item {
                if let Some(blocks) = map.get(ANNOTATION) {
                    parse_annotation_blocks(
                        blocks,
                        (subject, predicate, object_id),
                        &active_context,
                        interner,
                        reifiers,
                        annotations,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn parse_standalone_reifiers(
    value: &Value,
    context: &Context,
    interner: &mut Interner,
    reifiers: &mut Vec<(usize, Triple3)>,
    annotations: &mut Vec<Triple3>,
) -> Result<(), YamlLdParseError> {
    match value {
        Value::Array(items) => {
            for item in items {
                parse_standalone_reifiers(item, context, interner, reifiers, annotations)?;
            }
        }
        item => {
            let map = object(item, "gts:reifiers entry")?;
            let scoped_context = scoped_context(context, map);
            let reifier = match map.get("@id") {
                Some(id) => parse_id(id, &scoped_context, interner)?,
                None => interner.generated_bnode("gts_reifier_"),
            };
            let triple = map
                .get(GTS_TRIPLE)
                .ok_or_else(|| YamlLdParseError::new("gts:reifiers entry is missing gts:triple"))
                .and_then(|value| parse_triple(value, &scoped_context, interner, reifiers))?;
            set_reifier(reifiers, reifier, triple);
            if let Some(block) = map.get(ANNOTATION) {
                parse_annotation_properties(
                    object(block, "@annotation")?,
                    reifier,
                    &scoped_context,
                    interner,
                    reifiers,
                    annotations,
                )?;
            }
        }
    }
    Ok(())
}

fn parse_annotation_blocks(
    value: &Value,
    statement: Triple3,
    context: &Context,
    interner: &mut Interner,
    reifiers: &mut Vec<(usize, Triple3)>,
    annotations: &mut Vec<Triple3>,
) -> Result<(), YamlLdParseError> {
    match value {
        Value::Array(items) => {
            for item in items {
                parse_annotation_blocks(item, statement, context, interner, reifiers, annotations)?;
            }
        }
        item => {
            let map = object(item, "@annotation")?;
            let scoped_context = scoped_context(context, map);
            let reifier = match map.get("@id") {
                Some(id) => parse_id(id, &scoped_context, interner)?,
                None => interner.generated_bnode("gts_annotation_"),
            };
            set_reifier(reifiers, reifier, statement);
            parse_annotation_properties(
                map,
                reifier,
                &scoped_context,
                interner,
                reifiers,
                annotations,
            )?;
        }
    }
    Ok(())
}

fn parse_annotation_properties(
    map: &Map<String, Value>,
    reifier: usize,
    context: &Context,
    interner: &mut Interner,
    reifiers: &mut Vec<(usize, Triple3)>,
    annotations: &mut Vec<Triple3>,
) -> Result<(), YamlLdParseError> {
    let scoped_context = scoped_context(context, map);
    for (key, value) in map {
        if matches!(key.as_str(), "@context" | "@id") {
            continue;
        }
        let type_position = key == "@type";
        if key.starts_with('@') && !type_position {
            return Err(YamlLdParseError::new(format!(
                "unsupported annotation keyword {key}"
            )));
        }
        let predicate = predicate_id(key, &scoped_context, interner);
        match value {
            Value::Array(items) => {
                for item in items {
                    let object =
                        parse_term(item, type_position, &scoped_context, interner, reifiers)?;
                    annotations.push((reifier, predicate, object));
                }
            }
            item => {
                let object = parse_term(item, type_position, &scoped_context, interner, reifiers)?;
                annotations.push((reifier, predicate, object));
            }
        }
    }
    Ok(())
}

fn parse_term(
    value: &Value,
    type_position: bool,
    context: &Context,
    interner: &mut Interner,
    reifiers: &mut Vec<(usize, Triple3)>,
) -> Result<usize, YamlLdParseError> {
    match value {
        Value::String(text) if type_position => {
            Ok(interner.atom(TermKind::Iri, context.expand(text), None, None, None))
        }
        Value::String(text) => Ok(interner.atom(TermKind::Literal, text.clone(), None, None, None)),
        Value::Bool(flag) => Ok(interner.atom(
            TermKind::Literal,
            flag.to_string(),
            None,
            None,
            Some(XSD_BOOLEAN.to_string()),
        )),
        Value::Number(number) => Ok(number_literal(number, interner)),
        Value::Object(map) => {
            let scoped_context = scoped_context(context, map);
            if let Some(id) = map.get("@id") {
                return parse_id(id, &scoped_context, interner);
            }
            if let Some(value) = map.get("@value") {
                return parse_literal_object(value, map, &scoped_context, interner);
            }
            if let Some(triple) = map.get(GTS_TRIPLE) {
                let statement = parse_triple(triple, &scoped_context, interner, reifiers)?;
                return Ok(interner.triple(statement, reifiers));
            }
            Err(YamlLdParseError::new(
                "term object must contain @id, @value, or gts:triple",
            ))
        }
        Value::Array(_) => Err(YamlLdParseError::new(
            "nested arrays are not valid term values in the GTS YAML-LD profile",
        )),
        Value::Null => Err(YamlLdParseError::new(
            "null is not a valid term value in the GTS YAML-LD profile",
        )),
    }
}

fn parse_literal_object(
    value: &Value,
    map: &Map<String, Value>,
    context: &Context,
    interner: &mut Interner,
) -> Result<usize, YamlLdParseError> {
    let lexical = scalar_lexical(value)?;
    let lang = match map.get("@language") {
        Some(Value::String(lang)) => Some(lang.clone()),
        Some(_) => return Err(YamlLdParseError::new("@language must be a string")),
        None => None,
    };
    let direction = match map.get("@direction") {
        Some(Value::String(direction)) if matches!(direction.as_str(), "ltr" | "rtl") => {
            Some(direction.clone())
        }
        Some(Value::String(_)) => {
            return Err(YamlLdParseError::new(
                "@direction must be \"ltr\" or \"rtl\"",
            ))
        }
        Some(_) => return Err(YamlLdParseError::new("@direction must be a string")),
        None => None,
    };
    if direction.is_some() && lang.is_none() {
        return Err(YamlLdParseError::new(
            "@direction requires a language-tagged literal",
        ));
    }
    let datatype = match map.get("@type") {
        Some(Value::String(datatype)) => Some(context.expand(datatype)),
        Some(_) => return Err(YamlLdParseError::new("@type must be a string")),
        None => inferred_datatype(value),
    };
    Ok(interner.atom(TermKind::Literal, lexical, lang, direction, datatype))
}

fn parse_triple(
    value: &Value,
    context: &Context,
    interner: &mut Interner,
    reifiers: &mut Vec<(usize, Triple3)>,
) -> Result<Triple3, YamlLdParseError> {
    let map = object(value, "gts:triple")?;
    let subject = map
        .get("subject")
        .ok_or_else(|| YamlLdParseError::new("gts:triple is missing subject"))
        .and_then(|value| parse_term(value, false, context, interner, reifiers))?;
    let predicate = map
        .get("predicate")
        .ok_or_else(|| YamlLdParseError::new("gts:triple is missing predicate"))
        .and_then(|value| parse_term(value, true, context, interner, reifiers))?;
    let object = map
        .get("object")
        .ok_or_else(|| YamlLdParseError::new("gts:triple is missing object"))
        .and_then(|value| parse_term(value, false, context, interner, reifiers))?;
    Ok((subject, predicate, object))
}

fn parse_id(
    value: &Value,
    context: &Context,
    interner: &mut Interner,
) -> Result<usize, YamlLdParseError> {
    let Value::String(id) = value else {
        return Err(YamlLdParseError::new("@id must be a string"));
    };
    if let Some(label) = id.strip_prefix("_:") {
        Ok(interner.atom(TermKind::Bnode, label.to_string(), None, None, None))
    } else {
        Ok(interner.atom(TermKind::Iri, context.expand(id), None, None, None))
    }
}

fn predicate_id(key: &str, context: &Context, interner: &mut Interner) -> usize {
    let iri = if key == "@type" {
        RDF_TYPE.to_string()
    } else {
        context.expand(key)
    };
    interner.atom(TermKind::Iri, iri, None, None, None)
}

fn number_literal(number: &Number, interner: &mut Interner) -> usize {
    let datatype = if number.is_i64() || number.is_u64() {
        XSD_INTEGER
    } else {
        XSD_DECIMAL
    };
    interner.atom(
        TermKind::Literal,
        number.to_string(),
        None,
        None,
        Some(datatype.to_string()),
    )
}

fn scalar_lexical(value: &Value) -> Result<String, YamlLdParseError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Bool(flag) => Ok(flag.to_string()),
        Value::Number(number) => Ok(number.to_string()),
        _ => Err(YamlLdParseError::new("@value must be a scalar")),
    }
}

fn inferred_datatype(value: &Value) -> Option<String> {
    match value {
        Value::Bool(_) => Some(XSD_BOOLEAN.to_string()),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(XSD_INTEGER.to_string())
        }
        Value::Number(_) => Some(XSD_DECIMAL.to_string()),
        _ => None,
    }
}

fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, YamlLdParseError> {
    match value {
        Value::Object(map) => Ok(map),
        _ => Err(YamlLdParseError::new(format!("{what} must be an object"))),
    }
}

fn scoped_context(parent: &Context, map: &Map<String, Value>) -> Context {
    let mut context = parent.clone();
    if let Some(local) = map.get("@context") {
        context.merge(local);
    }
    context
}

fn set_reifier(reifiers: &mut Vec<(usize, Triple3)>, rid: usize, statement: Triple3) {
    if let Some((_, existing)) = reifiers.iter_mut().find(|(candidate, _)| *candidate == rid) {
        *existing = statement;
    } else {
        reifiers.push((rid, statement));
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `gts -> YAML-LD-star` transform: a deterministic folded-graph projection.
//!
//! This module deliberately implements a narrow JSON-LD-star profile over the
//! folded [`crate::model::Graph`] tables instead of embedding a general JSON-LD
//! processor. YAML-LD is rendered from the same `serde_json::Value` tree.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::model::{Graph, TermKind, Triple3};

pub(crate) const ANNOTATION: &str = "@annotation";
pub(crate) const GTS_CONTEXT: &str = "https://blackcatinformatics.ca/projects/gts#";
pub(crate) const GTS_GRAPH: &str = "gts:graph";
pub(crate) const GTS_REIFIERS: &str = "gts:reifiers";
pub(crate) const GTS_SUBJECT: &str = "gts:subject";
pub(crate) const GTS_TRIPLE: &str = "gts:triple";
pub(crate) const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub(crate) const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
pub(crate) const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
pub(crate) const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// Project a folded graph into the repository's deterministic JSON-LD-star
/// core profile.
pub fn to_json_ld(graph: &Graph) -> Value {
    let term_sort_keys = term_sort_keys(graph);

    let mut annotations_by_reifier: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
    for &(reifier, predicate, value) in &graph.annotations {
        annotations_by_reifier
            .entry(reifier)
            .or_default()
            .push((predicate, value));
    }

    let mut reifiers_by_statement: BTreeMap<Triple3, Vec<usize>> = BTreeMap::new();
    for &(reifier, statement) in &graph.reifiers {
        reifiers_by_statement
            .entry(statement)
            .or_default()
            .push(reifier);
    }
    for reifiers in reifiers_by_statement.values_mut() {
        reifiers.sort_by_key(|reifier| term_sort_keys[*reifier].clone());
    }

    let mut quads = graph.quads.clone();
    quads.sort_by_key(|&(subject, predicate, object, graph_name)| {
        (
            term_sort_keys[subject].clone(),
            predicate_key(graph, predicate),
            term_sort_keys[object].clone(),
            graph_name.map(|term| term_sort_keys[term].clone()),
        )
    });

    let mut nodes: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    let mut attached_reifiers = BTreeSet::new();
    let mut attached_statements = BTreeSet::new();
    for (subject, predicate, object, graph_name) in quads {
        let key = term_sort_keys[subject].clone();
        let node = nodes
            .entry(key)
            .or_insert_with(|| node_object(graph, subject));

        let statement = (subject, predicate, object);
        let annotations = if attached_statements.insert(statement) {
            reifiers_by_statement.get(&statement)
        } else {
            None
        };
        let value = statement_value(
            graph,
            object,
            graph_name,
            annotations,
            &annotations_by_reifier,
            &mut attached_reifiers,
            &term_sort_keys,
        );
        append_value(node, predicate_key(graph, predicate), value);
    }

    let mut root = Map::new();
    root.insert("@context".to_string(), context_object());
    root.insert(
        "@graph".to_string(),
        Value::Array(nodes.into_values().map(Value::Object).collect()),
    );

    let mut standalone_rows: Vec<(usize, Triple3)> = graph
        .reifiers
        .iter()
        .filter(|(reifier, _)| !attached_reifiers.contains(reifier))
        .copied()
        .collect();
    standalone_rows.sort_by_key(|&(reifier, (subject, predicate, object))| {
        (
            term_sort_keys[reifier].clone(),
            term_sort_keys[subject].clone(),
            predicate_key(graph, predicate),
            term_sort_keys[object].clone(),
        )
    });
    let standalone_reifiers: Vec<Value> = standalone_rows
        .into_iter()
        .map(|(reifier, statement)| {
            standalone_reifier(
                graph,
                reifier,
                statement,
                &annotations_by_reifier,
                &term_sort_keys,
            )
        })
        .collect();
    if !standalone_reifiers.is_empty() {
        root.insert(GTS_REIFIERS.to_string(), Value::Array(standalone_reifiers));
    }

    Value::Object(root)
}

/// Render a folded graph as pretty JSON-LD-star.
pub fn to_json_ld_string(graph: &Graph) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&to_json_ld(graph))
}

/// Render a folded graph as YAML-LD-star.
pub fn to_yaml_ld(graph: &Graph) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(&to_json_ld(graph))
}

fn context_object() -> Value {
    let mut context = Map::new();
    context.insert("gts".to_string(), Value::String(GTS_CONTEXT.to_string()));
    context.insert(
        "rdf".to_string(),
        Value::String("http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string()),
    );
    context.insert(
        "xsd".to_string(),
        Value::String("http://www.w3.org/2001/XMLSchema#".to_string()),
    );
    Value::Object(context)
}

fn node_object(graph: &Graph, subject: usize) -> Map<String, Value> {
    let mut node = Map::new();
    match graph.terms[subject].kind {
        TermKind::Iri | TermKind::Bnode => {
            node.insert("@id".to_string(), Value::String(term_id(graph, subject)));
        }
        TermKind::Literal | TermKind::Triple => {
            node.insert(GTS_SUBJECT.to_string(), term_value(graph, subject));
        }
    }
    node
}

fn statement_value(
    graph: &Graph,
    object: usize,
    graph_name: Option<usize>,
    reifiers: Option<&Vec<usize>>,
    annotations_by_reifier: &BTreeMap<usize, Vec<(usize, usize)>>,
    attached_reifiers: &mut BTreeSet<usize>,
    term_sort_keys: &[String],
) -> Value {
    let mut value = expect_object(term_value(graph, object));
    if let Some(graph_name) = graph_name {
        value.insert(GTS_GRAPH.to_string(), term_value(graph, graph_name));
    }
    if let Some(reifiers) = reifiers {
        let mut blocks = Vec::new();
        for &reifier in reifiers {
            attached_reifiers.insert(reifier);
            blocks.push(annotation_block(
                graph,
                reifier,
                annotations_by_reifier.get(&reifier),
                term_sort_keys,
            ));
        }
        match blocks.as_slice() {
            [] => {}
            [single] => {
                value.insert(ANNOTATION.to_string(), single.clone());
            }
            _ => {
                value.insert(ANNOTATION.to_string(), Value::Array(blocks));
            }
        }
    }
    Value::Object(value)
}

fn standalone_reifier(
    graph: &Graph,
    reifier: usize,
    (subject, predicate, object): Triple3,
    annotations_by_reifier: &BTreeMap<usize, Vec<(usize, usize)>>,
    term_sort_keys: &[String],
) -> Value {
    let mut block = Map::new();
    block.insert("@id".to_string(), Value::String(term_id(graph, reifier)));
    block.insert(
        GTS_TRIPLE.to_string(),
        triple_value(graph, subject, predicate, object),
    );
    block.insert(
        ANNOTATION.to_string(),
        annotation_block(
            graph,
            reifier,
            annotations_by_reifier.get(&reifier),
            term_sort_keys,
        ),
    );
    Value::Object(block)
}

fn annotation_block(
    graph: &Graph,
    reifier: usize,
    annotations: Option<&Vec<(usize, usize)>>,
    term_sort_keys: &[String],
) -> Value {
    let mut block = Map::new();
    block.insert("@id".to_string(), Value::String(term_id(graph, reifier)));
    if let Some(annotations) = annotations {
        let mut rows = annotations.clone();
        rows.sort_by_key(|&(predicate, value)| {
            (
                predicate_key(graph, predicate),
                term_sort_keys[value].clone(),
            )
        });
        for (predicate, value) in rows {
            append_value(
                &mut block,
                predicate_key(graph, predicate),
                term_value(graph, value),
            );
        }
    }
    Value::Object(block)
}

fn triple_value(graph: &Graph, subject: usize, predicate: usize, object: usize) -> Value {
    let mut triple = Map::new();
    triple.insert("subject".to_string(), term_value(graph, subject));
    triple.insert("predicate".to_string(), term_value(graph, predicate));
    triple.insert("object".to_string(), term_value(graph, object));
    Value::Object(triple)
}

fn term_value(graph: &Graph, term: usize) -> Value {
    let t = &graph.terms[term];
    let mut value = Map::new();
    match t.kind {
        TermKind::Iri | TermKind::Bnode => {
            value.insert("@id".to_string(), Value::String(term_id(graph, term)));
        }
        TermKind::Literal => {
            value.insert(
                "@value".to_string(),
                Value::String(t.value.as_deref().unwrap_or("").to_string()),
            );
            if let Some(lang) = &t.lang {
                value.insert("@language".to_string(), Value::String(lang.clone()));
            } else if let Some(datatype) = t.datatype {
                value.insert("@type".to_string(), Value::String(term_id(graph, datatype)));
            }
        }
        TermKind::Triple => {
            let triple = t
                .reifier
                .and_then(|reifier| graph.reifier(reifier))
                .map(|(subject, predicate, object)| triple_value(graph, subject, predicate, object))
                .unwrap_or_else(|| {
                    let mut degraded = Map::new();
                    degraded.insert(
                        "@id".to_string(),
                        Value::String(format!("_:unbound_triple_{term}")),
                    );
                    Value::Object(degraded)
                });
            value.insert(GTS_TRIPLE.to_string(), triple);
        }
    }
    Value::Object(value)
}

fn term_id(graph: &Graph, term: usize) -> String {
    let t = &graph.terms[term];
    match t.kind {
        TermKind::Bnode => match &t.value {
            Some(value) => format!("_:{value}"),
            None => format!("_:b{term}"),
        },
        _ => t.value.as_deref().unwrap_or("").to_string(),
    }
}

fn predicate_key(graph: &Graph, predicate: usize) -> String {
    if graph.terms[predicate].kind == TermKind::Iri
        && graph.terms[predicate].value.as_deref() == Some(RDF_TYPE)
    {
        "@type".to_string()
    } else {
        term_id(graph, predicate)
    }
}

fn term_sort_key(graph: &Graph, term: usize) -> String {
    serde_json::to_string(&term_value(graph, term)).expect("serde_json::Value serializes")
}

fn term_sort_keys(graph: &Graph) -> Vec<String> {
    (0..graph.terms.len())
        .map(|term| term_sort_key(graph, term))
        .collect()
}

fn append_value(map: &mut Map<String, Value>, key: String, value: Value) {
    match map.remove(&key) {
        None => {
            map.insert(key, value);
        }
        Some(Value::Array(mut values)) => {
            values.push(value);
            map.insert(key, Value::Array(values));
        }
        Some(previous) => {
            map.insert(key, Value::Array(vec![previous, value]));
        }
    }
}

fn expect_object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => unreachable!("term_value always returns an object"),
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "rdf")]

use std::collections::HashSet;
use std::error::Error;

use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::rdf::{
    from_rdf_dataset, to_rdf_dataset, to_rdf_dataset_lossy, BaseDirection, BlankNode, Dataset,
    GraphName, Iri, Literal, NamedOrBlankNode, RdfQuad, RdfTerm, RdfTriple,
};
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

fn sorted_dataset(dataset: &Dataset) -> Vec<String> {
    let mut lines: Vec<String> = dataset.iter().map(|quad| quad.to_string()).collect();
    lines.sort();
    lines
}

#[test]
fn native_rdf_dataset_roundtrips_through_gts() -> Result<(), Box<dyn Error>> {
    let mut dataset = Dataset::new();
    let graph = Iri::new("https://example.org/graph")?;
    let cat = Iri::new("https://example.org/Cat")?;
    let friend = BlankNode::new("friend")?;
    let label = Iri::new(RDFS_LABEL)?;
    let lives = Iri::new("https://example.org/lives")?;
    let integer = Iri::new(XSD_INTEGER)?;

    dataset.insert(RdfQuad::new(
        cat.clone(),
        label.clone(),
        Literal::new_language_tagged_literal("Cat", "en")?,
        graph.clone(),
    ));
    dataset.insert(RdfQuad::new(
        cat,
        lives,
        Literal::new_typed_literal("9", integer),
        graph,
    ));
    dataset.insert(RdfQuad::new(
        friend,
        label,
        Literal::new_simple_literal("Friend"),
        GraphName::DefaultGraph,
    ));

    let bytes = from_rdf_dataset(&dataset)?;
    let folded = read(&bytes, true, None);
    assert!(folded.diagnostics.is_empty());
    let back = to_rdf_dataset(&folded)?;
    assert_eq!(sorted_dataset(&back), sorted_dataset(&dataset));
    Ok(())
}

#[test]
fn gts_reifier_projection_uses_native_rdf12_triple_terms() -> Result<(), Box<dyn Error>> {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/claim".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/subject".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("https://example.org/predicate".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Literal,
            value: Some("object".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
    ]);
    writer.add_reifies(&[(0, (1, 2, 3))]);
    let folded = read(&writer.to_bytes(), true, None);
    let dataset = to_rdf_dataset(&folded)?;

    let has_reifier_projection = dataset.iter().any(|quad| {
        if !quad.graph_name.is_default_graph() || quad.predicate.as_str() != RDF_REIFIES {
            return false;
        }
        let RdfTerm::Triple(triple) = &quad.object else {
            return false;
        };
        matches!(
            &triple.subject,
            NamedOrBlankNode::Iri(node)
                if node.as_str() == "https://example.org/subject"
        ) && triple.predicate.as_str() == "https://example.org/predicate"
            && matches!(&triple.object, RdfTerm::Literal(literal) if literal.lexical == "object")
    });
    assert!(has_reifier_projection, "reifier projection is exported");
    Ok(())
}

#[test]
fn native_rdf12_triple_terms_import_as_gts_reifiers() -> Result<(), Box<dyn Error>> {
    let mut dataset = Dataset::new();
    let claim = Iri::new("https://example.org/claim")?;
    let subject = Iri::new("https://example.org/subject")?;
    let predicate = Iri::new("https://example.org/predicate")?;
    let object = Literal::new_simple_literal("object");
    let rdf_reifies = Iri::new(RDF_REIFIES)?;
    let quoted = RdfTriple::new(subject, predicate, object);
    dataset.insert(RdfQuad::new(
        claim,
        rdf_reifies,
        quoted,
        GraphName::DefaultGraph,
    ));

    let bytes = from_rdf_dataset(&dataset)?;
    let folded = read(&bytes, true, None);
    assert_eq!(folded.reifiers.len(), 1);
    let back = to_rdf_dataset(&folded)?;
    assert_eq!(sorted_dataset(&back), sorted_dataset(&dataset));
    Ok(())
}

#[test]
fn strict_export_refuses_unrepresentable_quoted_triple_positions() {
    let graph = Graph {
        terms: vec![
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/subject".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/predicate".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("object".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(3),
            },
        ],
        quads: vec![(0, 1, 2, None), (3, 1, 2, None)],
        reifiers: vec![(3, (0, 1, 2))],
        ..Graph::default()
    };

    let err = to_rdf_dataset(&graph).expect_err("strict mode rejects triple subjects");
    assert!(err.detail().contains("quoted triple"));

    let lossy = to_rdf_dataset_lossy(&graph).expect("lossy mode drops unsupported row");
    assert_eq!(lossy.len(), 1);
    assert!(lossy
        .iter()
        .all(|quad| quad.predicate.as_str() != RDF_REIFIES));
}

#[test]
fn strict_export_rejects_quoted_triple_graph_names() {
    let graph = Graph {
        terms: vec![
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/subject".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/predicate".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("object".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(3),
            },
        ],
        quads: vec![(0, 1, 2, Some(3))],
        reifiers: vec![(3, (0, 1, 2))],
        ..Graph::default()
    };

    let err = to_rdf_dataset(&graph).expect_err("strict mode rejects triple graph names");
    assert!(err.detail().contains("graph name"));
    assert!(err.detail().contains("quoted triple"));
}

#[test]
fn native_rdf_dataset_roundtrips_directional_literals() -> Result<(), Box<dyn Error>> {
    let graph = Graph {
        terms: vec![
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/subject".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/label".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("Cat".to_string()),
                datatype: None,
                lang: Some("en".to_string()),
                direction: Some("ltr".to_string()),
                reifier: None,
            },
        ],
        quads: vec![(0, 1, 2, None)],
        ..Graph::default()
    };

    let dataset = to_rdf_dataset(&graph).expect("adapter preserves direction");
    let has_directional_literal = dataset.iter().any(|quad| {
        matches!(
            &quad.object,
            RdfTerm::Literal(literal)
                if literal.lexical == "Cat"
                    && literal.language.as_deref() == Some("en")
                    && literal.direction == Some(BaseDirection::Ltr)
        )
    });
    assert!(has_directional_literal);

    let bytes = from_rdf_dataset(&dataset)?;
    let folded = read(&bytes, true, None);
    let term = folded
        .terms
        .iter()
        .find(|term| term.kind == TermKind::Literal && term.value.as_deref() == Some("Cat"))
        .expect("literal survives import");
    assert_eq!(term.lang.as_deref(), Some("en"));
    assert_eq!(term.direction.as_deref(), Some("ltr"));
    Ok(())
}

#[test]
fn generated_blank_node_labels_do_not_collide_with_explicit_labels() {
    let graph = Graph {
        terms: vec![
            Term {
                kind: TermKind::Bnode,
                value: Some("b1".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Bnode,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/predicate".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/object".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
        ],
        quads: vec![(0, 2, 3, None), (1, 2, 3, None)],
        ..Graph::default()
    };

    let dataset = to_rdf_dataset(&graph).expect("missing blank node label is generated");
    assert_eq!(dataset.len(), 2);
    let subjects: HashSet<String> = dataset
        .iter()
        .map(|quad| quad.subject.to_string())
        .collect();
    assert_eq!(subjects.len(), 2);
    assert!(subjects.contains("_:b1"));
    assert!(subjects.contains("_:gts_00000000000000000000000001"));
}

#[test]
fn native_rdf_import_rejects_conflicting_reifier_bindings() -> Result<(), Box<dyn Error>> {
    let mut dataset = Dataset::new();
    let claim = Iri::new("https://example.org/claim")?;
    let rdf_reifies = Iri::new(RDF_REIFIES)?;
    let subject = Iri::new("https://example.org/subject")?;
    let predicate = Iri::new("https://example.org/predicate")?;

    dataset.insert(RdfQuad::new(
        claim.clone(),
        rdf_reifies.clone(),
        RdfTriple::new(
            subject.clone(),
            predicate.clone(),
            Literal::new_simple_literal("first"),
        ),
        GraphName::DefaultGraph,
    ));
    dataset.insert(RdfQuad::new(
        claim,
        rdf_reifies,
        RdfTriple::new(subject, predicate, Literal::new_simple_literal("second")),
        GraphName::DefaultGraph,
    ));

    let err = from_rdf_dataset(&dataset).expect_err("conflicting reifier binding is rejected");
    assert!(err.detail().contains("conflicting rdf:reifies"));
    Ok(())
}

#[test]
fn native_rdf_lexical_validators_reject_unsafe_values() {
    assert!(Iri::new("https://example.org/<bad>").is_err());
    assert!(Iri::new("https://example.org/>bad").is_err());
    assert!(Iri::new("https://example.org/good").is_ok());

    assert!(BlankNode::new("-bad").is_err());
    assert!(BlankNode::new("bad:label").is_err());
    assert!(BlankNode::new("bad.").is_err());
    assert!(BlankNode::new("good_label-1.2").is_ok());

    assert!(Literal::new_language_tagged_literal("Cat", "en_US").is_err());
    assert!(Literal::new_language_tagged_literal("Cat", "-en").is_err());
    assert!(Literal::new_language_tagged_literal("Cat", "en-").is_err());
    assert!(Literal::new_language_tagged_literal("Cat", "en--US").is_err());
    assert!(
        Literal::new_directional_language_tagged_literal("Cat", "en-US", BaseDirection::Ltr)
            .is_ok()
    );
}

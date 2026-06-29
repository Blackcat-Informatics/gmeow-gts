// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn literal(value: &str, datatype: Option<usize>) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_string()),
        datatype,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn triple(reifier: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(reifier),
    }
}

fn has_recursive_reifier_diagnostic(graph: &gmeow_gts::model::Graph) -> bool {
    graph.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "DamagedFrame" && diagnostic.detail.contains("recursive quoted-triple")
    })
}

fn read_without_panic(data: &[u8]) -> Graph {
    std::panic::catch_unwind(|| read(data, true, None)).expect("public reader must not panic")
}

fn diagnostic_codes(graph: &Graph) -> Vec<&str> {
    graph
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn multi_segment_union_preserves_literal_datatype_mapping() {
    let datatype_iri = "http://www.w3.org/2001/XMLSchema#integer";
    let mut first = Writer::new("dist");
    first.add_terms(&[
        iri("https://example.org/s"),
        iri("https://example.org/p"),
        iri(datatype_iri),
        literal("7", Some(2)),
    ]);
    first.add_quads(&[(0, 1, 3, None)]);

    let second = Writer::new("dist");
    let mut data = first.to_bytes();
    data.extend(second.to_bytes());

    let graph = read(&data, true, None);

    assert!(graph.diagnostics.is_empty());
    assert_eq!(graph.quads.len(), 1);
    let object = graph.quads[0].2;
    assert_eq!(graph.terms[object].kind, TermKind::Literal);
    assert_eq!(graph.terms[object].value.as_deref(), Some("7"));
    let datatype = graph.terms[object].datatype.expect("literal datatype");
    assert_eq!(graph.terms[datatype].value.as_deref(), Some(datatype_iri));
}

#[test]
fn public_reader_reports_malformed_input_diagnostics_without_panicking() {
    assert_eq!(
        diagnostic_codes(&read_without_panic(&[])),
        vec!["EmptyFile"]
    );
    assert_eq!(
        diagnostic_codes(&read_without_panic(&[0x01])),
        vec!["DamagedFrame"]
    );

    let writer = Writer::new("generic");
    let mut torn = writer.to_bytes();
    torn.push(0xa3);

    assert_eq!(
        diagnostic_codes(&read_without_panic(&torn)),
        vec!["TornAppendError"]
    );
}

#[test]
fn reader_rejects_direct_recursive_quoted_triple_reifier() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        triple(0),
        iri("https://example.org/predicate"),
        iri("https://example.org/object"),
    ]);
    writer.add_reifies(&[(0, (0, 1, 2), None)]);

    let graph = read(&writer.to_bytes(), true, None);

    assert!(has_recursive_reifier_diagnostic(&graph));
    assert!(graph.reifiers.is_empty());
    assert_eq!(to_nquads(&graph), "");
}

#[test]
fn reader_rejects_indirect_recursive_quoted_triple_reifier() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        triple(0),
        triple(1),
        iri("https://example.org/predicate"),
        iri("https://example.org/object"),
    ]);
    writer.add_reifies(&[(0, (1, 2, 3), None), (1, (0, 2, 3), None)]);

    let graph = read(&writer.to_bytes(), true, None);

    assert!(has_recursive_reifier_diagnostic(&graph));
    assert_eq!(graph.reifiers, vec![(0, (1, 2, 3), None)]);
    let _ = to_nquads(&graph);
}

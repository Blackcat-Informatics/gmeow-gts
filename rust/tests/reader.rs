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
    writer.add_reifies(&[(0, (0, 1, 2))]);

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
    writer.add_reifies(&[(0, (1, 2, 3)), (1, (0, 2, 3))]);

    let graph = read(&writer.to_bytes(), true, None);

    assert!(has_recursive_reifier_diagnostic(&graph));
    assert_eq!(graph.reifiers, vec![(0, (1, 2, 3))]);
    let _ = to_nquads(&graph);
}

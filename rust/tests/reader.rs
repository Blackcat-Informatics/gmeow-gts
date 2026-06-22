// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use gmeow_gts::model::{Term, TermKind};
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

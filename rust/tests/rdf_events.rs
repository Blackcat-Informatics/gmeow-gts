// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use gmeow_gts::model::{Diagnostic, Graph, Term, TermKind};
use gmeow_gts::rdf_events::{
    visit_dataset, EventError, EventQuad, EventScopeId, EventTerm, EventTriple,
    GraphRdfEventSource, RdfDatasetVisitor, RdfEventSink, RdfEventSource, ReaderRdfEventSource,
};
use gmeow_gts::writer::Writer;

#[derive(Default)]
struct RecordingSink {
    strict: bool,
    events: Vec<String>,
}

impl RecordingSink {
    fn strict() -> Self {
        Self {
            strict: true,
            events: Vec::new(),
        }
    }
}

impl RdfEventSink for RecordingSink {
    fn declares_before_reference(&self) -> bool {
        self.strict
    }

    fn start_scope(&mut self, scope: EventScopeId) -> Result<(), EventError> {
        self.events.push(format!("start:{scope}"));
        Ok(())
    }

    fn term(&mut self, term: EventTerm) -> Result<(), EventError> {
        self.events.push(format!("term:{}", term.id));
        Ok(())
    }

    fn quad(&mut self, quad: EventQuad) -> Result<(), EventError> {
        self.events.push(format!(
            "quad:{}:{}:{}:{:?}",
            quad.subject, quad.predicate, quad.object, quad.graph_name
        ));
        Ok(())
    }

    fn reifier(&mut self, reifier: u64, triple: EventTriple) -> Result<(), EventError> {
        self.events.push(format!(
            "reifier:{reifier}:{}:{}:{}",
            triple.subject, triple.predicate, triple.object
        ));
        Ok(())
    }

    fn annotation(&mut self, annotation: EventTriple) -> Result<(), EventError> {
        self.events.push(format!(
            "annotation:{}:{}:{}",
            annotation.subject, annotation.predicate, annotation.object
        ));
        Ok(())
    }

    fn diagnostic(
        &mut self,
        diagnostic: gmeow_gts::rdf_events::EventDiagnostic,
    ) -> Result<(), EventError> {
        self.events.push(format!("diagnostic:{}", diagnostic.code));
        Ok(())
    }

    fn end_scope(&mut self, scope: EventScopeId) -> Result<(), EventError> {
        self.events.push(format!("end:{scope}"));
        Ok(())
    }

    fn finish(&mut self) -> Result<(), EventError> {
        self.events.push("finish".to_string());
        Ok(())
    }
}

fn term(kind: TermKind, value: &str) -> Term {
    Term {
        kind,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn graph_with_forward_reifier() -> Graph {
    let mut graph = Graph {
        terms: vec![
            term(TermKind::Iri, "https://ex/reifier"),
            Term {
                kind: TermKind::Triple,
                value: None,
                datatype: None,
                lang: None,
                direction: None,
                reifier: Some(0),
            },
            term(TermKind::Iri, "https://ex/s"),
            term(TermKind::Iri, "https://ex/p"),
            term(TermKind::Iri, "https://ex/o"),
            term(TermKind::Iri, "https://ex/g"),
            term(TermKind::Iri, "https://ex/ann"),
            Term {
                kind: TermKind::Literal,
                value: Some("ok".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
        ],
        ..Default::default()
    };
    graph.set_reifier(0, (2, 3, 4));
    graph.quads.push((1, 3, 4, Some(5)));
    graph.annotations.push((0, 6, 7));
    graph.diagnostics.push(Diagnostic {
        code: "SyntheticDiagnostic".to_string(),
        detail: "test-only diagnostic".to_string(),
        frame_index: Some(42),
    });
    graph
}

#[test]
fn graph_source_drives_generic_and_erased_sinks() {
    let graph = graph_with_forward_reifier();
    let source = GraphRdfEventSource::with_scope(&graph, 7);

    let mut generic_sink = RecordingSink::default();
    source.drive(&mut generic_sink).expect("generic drive");
    assert!(generic_sink.events.starts_with(&[
        "start:7".to_string(),
        "term:0".to_string(),
        "term:1".to_string(),
    ]));
    assert!(generic_sink
        .events
        .contains(&"diagnostic:SyntheticDiagnostic".to_string()));
    assert_eq!(generic_sink.events.last(), Some(&"finish".to_string()));

    let mut erased_sink = RecordingSink::default();
    source
        .drive_erased(&mut erased_sink)
        .expect("trait-object drive");
    assert_eq!(erased_sink.events, generic_sink.events);

    let source_object: &dyn RdfEventSource = &source;
    let mut source_object_sink = RecordingSink::default();
    source_object
        .drive_erased(&mut source_object_sink)
        .expect("source trait-object drive");
    assert_eq!(source_object_sink.events, generic_sink.events);
}

#[test]
fn reader_source_drives_events_from_gts_bytes() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        term(TermKind::Iri, "https://ex/s"),
        term(TermKind::Iri, "https://ex/p"),
        term(TermKind::Iri, "https://ex/o"),
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);

    let mut sink = RecordingSink::default();
    ReaderRdfEventSource::new(&writer.to_bytes(), true, None)
        .with_scope(3)
        .drive(&mut sink)
        .expect("reader event source");

    assert_eq!(
        sink.events,
        vec![
            "start:3",
            "term:0",
            "term:1",
            "term:2",
            "quad:0:1:2:None",
            "end:3",
            "finish",
        ]
    );
}

#[test]
fn lenient_sink_accepts_forward_reifier_order() {
    let graph = graph_with_forward_reifier();
    let mut sink = RecordingSink::default();
    GraphRdfEventSource::new(&graph)
        .drive(&mut sink)
        .expect("lenient sink");

    let term_1 = sink
        .events
        .iter()
        .position(|event| event == "term:1")
        .unwrap();
    let reifier = sink
        .events
        .iter()
        .position(|event| event == "reifier:0:2:3:4")
        .unwrap();
    assert!(
        term_1 < reifier,
        "fold-order events may reference the reifier before its binding"
    );
}

#[test]
fn strict_sink_gets_declarations_before_references() {
    let graph = graph_with_forward_reifier();
    let mut sink = RecordingSink::strict();
    GraphRdfEventSource::new(&graph)
        .drive(&mut sink)
        .expect("strict sink");

    let term_0 = sink
        .events
        .iter()
        .position(|event| event == "term:0")
        .unwrap();
    let term_1 = sink
        .events
        .iter()
        .position(|event| event == "term:1")
        .unwrap();
    let term_2 = sink
        .events
        .iter()
        .position(|event| event == "term:2")
        .unwrap();
    let term_3 = sink
        .events
        .iter()
        .position(|event| event == "term:3")
        .unwrap();
    let term_4 = sink
        .events
        .iter()
        .position(|event| event == "term:4")
        .unwrap();
    let reifier = sink
        .events
        .iter()
        .position(|event| event == "reifier:0:2:3:4")
        .unwrap();
    let quad = sink
        .events
        .iter()
        .position(|event| event == "quad:1:3:4:Some(5)")
        .unwrap();
    let annotation = sink
        .events
        .iter()
        .position(|event| event == "annotation:0:6:7")
        .unwrap();

    assert!(term_0 < reifier);
    assert!(term_2 < reifier);
    assert!(term_3 < reifier);
    assert!(term_4 < reifier);
    assert!(reifier < term_1);
    assert!(term_1 < quad);
    assert!(quad < annotation);
}

#[derive(Default)]
struct VisitorCounts {
    terms: usize,
    quads: usize,
    reifiers: usize,
    annotations: usize,
    diagnostics: usize,
}

impl RdfDatasetVisitor for VisitorCounts {
    fn term(&mut self, _id: usize, _term: &Term) {
        self.terms += 1;
    }

    fn quad(&mut self, _quad: gmeow_gts::model::Quad) {
        self.quads += 1;
    }

    fn reifier(&mut self, _reifier: usize, _triple: gmeow_gts::model::Triple3) {
        self.reifiers += 1;
    }

    fn annotation(&mut self, _annotation: gmeow_gts::model::Triple3) {
        self.annotations += 1;
    }

    fn diagnostic(&mut self, _diagnostic: &Diagnostic) {
        self.diagnostics += 1;
    }
}

#[test]
fn dataset_visitor_is_infallible_folded_graph_visitor() {
    let graph = graph_with_forward_reifier();
    let mut visitor = VisitorCounts::default();
    visit_dataset(&graph, &mut visitor);

    assert_eq!(visitor.terms, graph.terms.len());
    assert_eq!(visitor.quads, graph.quads.len());
    assert_eq!(visitor.reifiers, graph.reifiers.len());
    assert_eq!(visitor.annotations, graph.annotations.len());
    assert_eq!(visitor.diagnostics, graph.diagnostics.len());
}

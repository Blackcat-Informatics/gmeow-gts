// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "rdf-codecs")]

//! RDF 1.2 Turtle-family codec tests.

use std::path::PathBuf;
use std::process::Command;

use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::rdf_codecs::{
    from_trig, from_turtle, to_trig_from_erased_source, to_turtle, to_turtle_from_source,
};
use gmeow_gts::rdf_events::{GraphRdfEventSource, RdfEventSource};
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;

fn sorted_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("@prefix"))
        .map(str::to_string)
        .collect();
    lines.sort();
    lines
}

fn nquads_from_gts(bytes: &[u8]) -> String {
    to_nquads(&read(bytes, true, None))
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

fn sample_graph(named_graph: bool) -> Graph {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        term(TermKind::Iri, "https://ex/s"),
        term(TermKind::Iri, "https://ex/p"),
        term(TermKind::Iri, "https://ex/o"),
        term(TermKind::Iri, "https://ex/g"),
        term(TermKind::Iri, "https://ex/confidence"),
        Term {
            kind: TermKind::Literal,
            value: Some("0.9".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
    ]);
    writer.add_quads(&[(0, 1, 2, named_graph.then_some(3))]);
    writer.add_reifies(&[(0, (0, 1, 2))]);
    writer.add_annot(&[(0, 4, 5)]);
    read(&writer.to_bytes(), true, None)
}

#[test]
fn turtle_parser_accepts_shared_turtle_grammar() {
    let turtle = r#"@base <https://ex/> .
@prefix ex: <https://ex/ns#> .

<s> a ex:Thing ;
    ex:label "Cat"@en ;
    ex:related ex:a, ex:b ;
    ex:nested [ ex:name "Kit" ] ;
    ex:list ( ex:a ex:b ) .
"#;

    let out = nquads_from_gts(&from_turtle(turtle).expect("Turtle imports"));
    assert!(out.contains("<https://ex/s>"));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"));
    assert!(out.contains("\"Cat\"@en"));
    assert!(out.contains("<https://ex/ns#related> <https://ex/ns#a>"));
    assert!(out.contains("<https://ex/ns#related> <https://ex/ns#b>"));
    assert!(out.contains("<https://ex/ns#name> \"Kit\""));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>"));
}

#[test]
fn trig_parser_accepts_named_graphs_and_rdf12_triple_forms() {
    let trig = r#"PREFIX ex: <https://ex/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

GRAPH ex:g {
  ex:s ex:p ex:o ;
       ex:label "Cat"@en .
}
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9" .
<< ex:s ex:p ex:o >> ex:source ex:doc .
"#;

    let out = nquads_from_gts(&from_trig(trig).expect("TriG imports"));
    assert!(out.contains("<https://ex/s> <https://ex/p> <https://ex/o> <https://ex/g> ."));
    assert!(out.contains("\"Cat\"@en <https://ex/g>"));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>"));
    assert!(out.contains("<<( <https://ex/s> <https://ex/p> <https://ex/o> )>>"));
    assert!(out.contains("<https://ex/confidence> \"0.9\""));
    assert!(out.contains("<https://ex/source> <https://ex/doc>"));
}

#[test]
fn trig_serialization_roundtrips_through_event_source() {
    let graph = sample_graph(true);
    let source: &dyn RdfEventSource = &GraphRdfEventSource::new(&graph);
    let trig = to_trig_from_erased_source(source).expect("TriG serializes from event source");
    let imported = from_trig(&trig).expect("serialized TriG imports");

    assert_eq!(
        sorted_lines(&to_nquads(&graph)),
        sorted_lines(&nquads_from_gts(&imported))
    );
}

#[test]
fn turtle_serialization_uses_event_source_and_rejects_named_graphs() {
    let default_graph = sample_graph(false);
    let source = GraphRdfEventSource::new(&default_graph);
    let turtle = to_turtle_from_source(&source).expect("Turtle serializes from event source");
    let imported = from_turtle(&turtle).expect("serialized Turtle imports");
    assert_eq!(
        sorted_lines(&to_nquads(&default_graph)),
        sorted_lines(&nquads_from_gts(&imported))
    );

    let err = to_turtle(&sample_graph(true)).expect_err("named graph is not Turtle");
    assert!(err
        .to_string()
        .contains("Turtle cannot serialize named graph"));
}

fn tmpdir() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-rdf-codecs-test-{}-{n}", std::process::id()))
}

fn gts(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

#[test]
fn cli_turtle_and_feature_trig_imports_roundtrip() {
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let ttl_path = tmp.join("in.ttl");
    let gts_path = tmp.join("out.gts");
    std::fs::write(
        &ttl_path,
        "@prefix ex: <https://ex/> .\nex:s ex:p ex:o ; ex:q ex:r .\n",
    )
    .unwrap();

    let out = gts(&[
        "from-turtle",
        ttl_path.to_str().unwrap(),
        "-o",
        gts_path.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let turtle = gts(&["to-turtle", gts_path.to_str().unwrap()]);
    assert!(
        turtle.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&turtle.stderr)
    );
    let rendered = String::from_utf8(turtle.stdout).unwrap();
    assert!(rendered.contains("@prefix"));
    assert!(rendered.contains("https://ex/"));

    let trig_path = tmp.join("in.trig");
    let trig_gts = tmp.join("trig.gts");
    std::fs::write(
        &trig_path,
        "PREFIX ex: <https://ex/>\nex:g { ex:s ex:p ex:o ; ex:q ex:r . }\n",
    )
    .unwrap();
    let out = gts(&[
        "from-trig",
        trig_path.to_str().unwrap(),
        "-o",
        trig_gts.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let folded = gts(&["fold", trig_gts.to_str().unwrap()]);
    assert!(
        folded.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&folded.stderr)
    );
    let folded = String::from_utf8(folded.stdout).unwrap();
    assert!(folded.contains("<https://ex/q> <https://ex/r> <https://ex/g>"));
}

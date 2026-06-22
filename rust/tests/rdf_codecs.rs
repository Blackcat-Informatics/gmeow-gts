// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "rdf-codecs")]

//! RDF 1.2 Turtle-family codec tests.

use std::path::PathBuf;
use std::process::Command;

use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::rdf_codecs::{
    from_ntriples, from_trig, from_turtle, to_ntriples, to_ntriples_from_erased_source,
    to_trig_from_erased_source, to_turtle, to_turtle_from_source,
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

const W3C_NTRIPLES_POSITIVE_SYNTAX: &[(&str, &str)] = &[
    (
        "ntriples12-syntax-01",
        "<http://example/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example/s> <http://example/p> <http://example/o> )>> .\n",
    ),
    (
        "ntriples12-syntax-02",
        "<http://example/s><http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies><<(<http://example/s2><http://example/p2><http://example/o2>)>>.\n",
    ),
    (
        "ntriples12-syntax-03",
        "<http://example/s><http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies><<(<http://example/s2><http://example/q2><<(<http://example/s3><http://example/p3><http://example/o3>)>>)>>.\n",
    ),
    (
        "ntriples12-bnode-1",
        "_:b0 <http://example/p> <http://example/o> .\n\
         _:b1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( _:b0 <http://example/p> <http://example/o> )>> .\n",
    ),
    (
        "ntriples12-nested-1",
        "<http://example/s> <http://example/p> <http://example/o> .\n\
         <http://example/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example/s1> <http://example/p1> <http://example/o1> )>> .\n\
         <http://example/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example/23> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example/s3> <http://example/p3> <http://example/o3> )>> )>> .\n",
    ),
    (
        "ntriples-langdir-1",
        "<http://example/a> <http://example/b> \"Hello\"@en--ltr .\n",
    ),
    (
        "ntriples-langdir-2",
        "<http://example/a> <http://example/b> \"Hello\"@en--rtl .\n",
    ),
];

const W3C_NTRIPLES_NEGATIVE_SYNTAX: &[(&str, &str)] = &[
    (
        "ntriples12-bad-syntax-01",
        "<http://example/a> <<( <http://example/s> <http://example/p> <http://example/o> )>>  <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-syntax-02",
        "<http://example/q> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( \"XYZ\" <http://example/p> <http://example/o> )>> .\n",
    ),
    (
        "ntriples12-bad-syntax-03",
        "<http://example/q> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <http://example/s> \"XYZ\" <http://example/o> )>> .\n",
    ),
    (
        "ntriples12-bad-syntax-04",
        "<http://example/q> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> << <http://example/s> _:label <http://example/o> >> .\n",
    ),
    (
        "ntriples12-bad-syntax-05",
        "<http://example/a> <<( <http://example/s> <http://example/p>  <http://example/o> )>> <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-syntax-06",
        "<<( \"XYZ\" <http://example/p> <http://example/o> )>> <http://example/q> <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-syntax-07",
        "<<( <http://example/s> \"XYZ\" <http://example/o> )>> <http://example/q> <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-syntax-08",
        "<<( <http://example/s> _:label <http://example/o> )>> <http://example/q> <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-syntax-09",
        "<http://example/a> <http://example/b> << <http://example/s> <http://example/p> <http://example/o> >> .\n",
    ),
    (
        "ntriples12-bad-syntax-10",
        "<<( <http://example/s> <http://example/p> <http://example/o> )>> <http://example/a> <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-iri-1",
        "<http://example/a> <http://example/b> <//example/missing-scheme> .\n",
    ),
    (
        "ntriples12-bad-reified-syntax-1",
        "<< <http://example/s> <http://example/p> <http://example/o> >> <http://example/q> <http://example/z> .\n",
    ),
    (
        "ntriples12-bad-reified-syntax-2",
        "<http://example/x> <http://example/p> << <http://example/s> <http://example/p> <http://example/o> >> .\n",
    ),
    (
        "ntriples12-bad-reified-syntax-3",
        "<< <http://example/s1> <http://example/p1> <http://example/o1> >> <http://example/q> << <http://example/s2> <http://example/p2> <http://example/o2> >> .\n",
    ),
    (
        "ntriples12-bad-reified-syntax-4",
        "<http://example/x> << <http://example/s> <http://example/p> <http://example/o> >> <http://example/z> .\n",
    ),
    (
        "ntriples12-bnode-bad-annotated-syntax-1",
        "_:b0 <http://example/p> <http://example/o> {| <http://example/q> \"ABC\" |} .\n",
    ),
    (
        "ntriples12-bnode-bad-annotated-syntax-2",
        "<http://example/s> <http://example/p> _:b1 {| <http://example/q> \"456\"^^<http://www.w3.org/2001/XMLSchema#integer> |} .\n",
    ),
    (
        "ntriples-langdir-bad-1",
        "<http://example/a> <http://example/b> \"Hello\"@en--unk .\n",
    ),
    (
        "ntriples-langdir-bad-2",
        "<http://example/a> <http://example/b> \"Hello\"@en--LTR .\n",
    ),
    (
        "ntriples-langdir-bad-3",
        "<http://example/a> <http://example/b> \"Hello\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#langString> .\n",
    ),
    (
        "ntriples-langdir-bad-4",
        "<http://example/a> <http://example/b> \"Hello\"@cantbethislong .\n",
    ),
    (
        "ntriples-langdir-bad-5",
        "<http://example/a> <http://example/b> \"Hello\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString> .\n",
    ),
];

#[test]
fn ntriples_parser_accepts_w3c_rdf12_positive_syntax_suite() {
    for (name, text) in W3C_NTRIPLES_POSITIVE_SYNTAX {
        from_ntriples(text).unwrap_or_else(|err| panic!("{name}: {err}"));
    }
}

#[test]
fn ntriples_parser_rejects_w3c_rdf12_negative_syntax_suite() {
    for (name, text) in W3C_NTRIPLES_NEGATIVE_SYNTAX {
        assert!(from_ntriples(text).is_err(), "{name} unexpectedly parsed");
    }
}

#[test]
fn ntriples_parser_accepts_w3c_syntax_cases_and_rdf12_triple_terms() {
    let ntriples = r#"# comments and blank lines are valid N-Triples

<https://ex/s> <https://ex/p> <https://ex/o> .
_:b0 <https://ex/name> "Kit\nTab\tQuote\""@en .
<https://ex/s> <https://ex/age> "12"^^<http://www.w3.org/2001/XMLSchema#integer> .
<https://ex/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .
<https://ex/r> <https://ex/source> <https://ex/doc> .
"#;

    let out = nquads_from_gts(&from_ntriples(ntriples).expect("N-Triples imports"));
    assert!(out.contains("<https://ex/s> <https://ex/p> <https://ex/o> ."));
    assert!(out.contains("_:b0 <https://ex/name> \"Kit\\nTab\\tQuote\\\"\"@en ."));
    assert!(out.contains(
        "<https://ex/s> <https://ex/age> \"12\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
    ));
    assert!(out.contains("<<( <https://ex/s> <https://ex/p> <https://ex/o> )>>"));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>"));
    assert!(out.contains("<https://ex/source> <https://ex/doc>"));
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
fn ntriples_serialization_roundtrips_through_event_source() {
    let graph = sample_graph(false);
    let source: &dyn RdfEventSource = &GraphRdfEventSource::new(&graph);
    let ntriples =
        to_ntriples_from_erased_source(source).expect("N-Triples serializes from event source");
    assert!(!ntriples.contains("@prefix"));
    assert!(ntriples.contains("<https://ex/s> <https://ex/p> <https://ex/o> ."));

    let imported = from_ntriples(&ntriples).expect("serialized N-Triples imports");
    assert_eq!(
        sorted_lines(&to_nquads(&graph)),
        sorted_lines(&nquads_from_gts(&imported))
    );

    let err = to_ntriples(&sample_graph(true)).expect_err("named graph is not N-Triples");
    assert!(err
        .to_string()
        .contains("N-Triples cannot serialize named graph"));
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

    let nt_path = tmp.join("in.nt");
    let nt_gts = tmp.join("nt.gts");
    std::fs::write(
        &nt_path,
        "<https://ex/s> <https://ex/p> <https://ex/o> .\n\
         <https://ex/s> <https://ex/q> \"Cat\"@en .\n",
    )
    .unwrap();
    let out = gts(&[
        "from-nt",
        nt_path.to_str().unwrap(),
        "-o",
        nt_gts.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ntriples = gts(&["to-nt", nt_gts.to_str().unwrap()]);
    assert!(
        ntriples.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ntriples.stderr)
    );
    let rendered = String::from_utf8(ntriples.stdout).unwrap();
    assert!(rendered.contains("<https://ex/s> <https://ex/p> <https://ex/o> ."));
    assert!(rendered.contains("\"Cat\"@en"));

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

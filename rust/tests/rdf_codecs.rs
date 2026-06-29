// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "rdf-codecs")]

//! RDF 1.2 text codec tests.

use std::path::PathBuf;
use std::process::Command;

use ciborium::value::Value;

use gmeow_gts::from_nquads::{
    from_nquads as native_from_nquads, from_ntriples as native_from_ntriples,
};
use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::rdf as native_rdf;
use gmeow_gts::rdf_codecs::{
    from_ntriples, from_rdf_xml, from_rdf_xml_with_base_iri, from_trig, from_turtle,
    graph_from_source, to_ntriples, to_ntriples_from_erased_source, to_rdf_xml,
    to_rdf_xml_from_erased_source, to_trig_from_erased_source, to_turtle, to_turtle_from_source,
};
use gmeow_gts::rdf_events::{GraphRdfEventSource, RdfEventSource};
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;
use gmeow_gts::xsd::{ILL_TYPED_LITERAL_CODE, ILL_TYPED_LITERAL_META_KEY};

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

fn canonical_native_lines(bytes: &[u8], name: &str, side: &str) -> Vec<String> {
    let graph = read(bytes, true, None);
    native_rdf::to_rdf_dataset(&graph)
        .unwrap_or_else(|err| panic!("{name}: {side} dataset projection failed: {err}"));
    canonicalize_blank_nodes(&to_nquads(&graph))
}

fn assert_nquads_isomorphic_to_ntriples(actual_nquads: &str, expected_ntriples: &str, name: &str) {
    let actual_gts = native_from_nquads(actual_nquads)
        .unwrap_or_else(|err| panic!("{name}: actual N-Quads did not parse: {err}"));
    let expected_gts = native_from_ntriples(expected_ntriples)
        .unwrap_or_else(|err| panic!("{name}: expected N-Triples did not parse: {err}"));
    let actual = canonical_native_lines(&actual_gts, name, "actual");
    let expected = canonical_native_lines(&expected_gts, name, "expected");
    assert_eq!(
        actual, expected,
        "{name}: RDF datasets differ\nactual:\n{actual_nquads}\nexpected:\n{expected_ntriples}"
    );
}

fn canonicalize_blank_nodes(text: &str) -> Vec<String> {
    let mut labels = std::collections::BTreeMap::new();
    let mut next = 0usize;
    let mut lines = sorted_lines(text)
        .into_iter()
        .map(|line| {
            let mut out = String::new();
            let mut index = 0usize;
            while index < line.len() {
                if line[index..].starts_with("_:") {
                    let start = index + 2;
                    let mut end = start;
                    while end < line.len() && is_blank_label_char(line.as_bytes()[end]) {
                        end += 1;
                    }
                    let label = &line[start..end];
                    let canonical = labels.entry(label.to_string()).or_insert_with(|| {
                        let label = format!("b{next}");
                        next += 1;
                        label
                    });
                    out.push_str("_:");
                    out.push_str(canonical);
                    index = end;
                } else {
                    let ch = line[index..]
                        .chars()
                        .next()
                        .expect("index is inside the string");
                    out.push(ch);
                    index += ch.len_utf8();
                }
            }
            out
        })
        .collect::<Vec<_>>();
    lines.sort();
    lines
}

fn is_blank_label_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
}

fn graph_meta<'a>(graph: &'a Graph, key: &str) -> Option<&'a Value> {
    graph
        .meta
        .iter()
        .find_map(|(stored, value)| (stored == key).then_some(value))
}

fn map_value<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Map(entries) = value else {
        return None;
    };
    entries.iter().find_map(|(stored, value)| match stored {
        Value::Text(stored) if stored == key => Some(value),
        _ => None,
    })
}

fn text_value<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    match map_value(value, key) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
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
    writer.add_reifies(&[(0, (0, 1, 2), None)]);
    writer.add_annot(&[(0, 4, 5, None)]);
    read(&writer.to_bytes(), true, None)
}

#[test]
fn ntriples_isomorphism_helper_accepts_renamed_blank_nodes() {
    assert_nquads_isomorphic_to_ntriples(
        "_:actual <https://ex/p> _:target .\n",
        "_:expected <https://ex/p> _:object .\n",
        "renamed blank nodes",
    );
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
fn ntriples_import_preserves_and_flags_ill_typed_xsd_literals() {
    let ntriples = r#"<https://ex/s> <https://ex/p> "maybe"^^<http://www.w3.org/2001/XMLSchema#boolean> .
<https://ex/s> <https://ex/n> "+0007"^^<http://www.w3.org/2001/XMLSchema#integer> .
"#;

    let bytes = from_ntriples(ntriples).expect("ill-typed RDF literal is imported");
    let graph = read(&bytes, true, None);
    let out = to_nquads(&graph);
    assert!(out.contains(
        "<https://ex/s> <https://ex/p> \"maybe\"^^<http://www.w3.org/2001/XMLSchema#boolean> ."
    ));
    assert!(out.contains(
        "<https://ex/s> <https://ex/n> \"+0007\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
    ));

    let meta = graph_meta(&graph, ILL_TYPED_LITERAL_META_KEY).expect("ill-typed metadata");
    let Some(Value::Array(items)) = map_value(meta, "items") else {
        panic!("ill-typed metadata has items array");
    };
    assert_eq!(items.len(), 1);
    assert_eq!(
        text_value(&items[0], "datatype"),
        Some("http://www.w3.org/2001/XMLSchema#boolean")
    );
    assert_eq!(text_value(&items[0], "lexical"), Some("maybe"));
}

#[test]
fn event_graph_materialization_surfaces_ill_typed_xsd_diagnostics() {
    let graph = Graph {
        terms: vec![
            Term {
                kind: TermKind::Iri,
                value: Some("https://ex/s".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://ex/p".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("http://www.w3.org/2001/XMLSchema#unsignedByte".to_string()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
            },
            Term {
                kind: TermKind::Literal,
                value: Some("256".to_string()),
                datatype: Some(2),
                lang: None,
                direction: None,
                reifier: None,
            },
        ],
        quads: vec![(0, 1, 3, None)],
        ..Default::default()
    };

    let materialized =
        graph_from_source(&GraphRdfEventSource::new(&graph)).expect("event graph materializes");
    assert!(materialized
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == ILL_TYPED_LITERAL_CODE));
    assert!(graph_meta(&materialized, ILL_TYPED_LITERAL_META_KEY).is_some());
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
fn turtle_parser_accepts_prefixed_names_with_internal_dots() {
    // Turtle PN_LOCAL admits internal dots (`repo:README.md`); only a TRAILING dot is
    // the statement terminator. The hand-rolled parser must not split the local name at
    // the first dot. Two statements exercise both the space-terminated and the
    // dot-terminated (no space) forms.
    let turtle = "PREFIX repo: <https://ex/repo/>\n\
                  <https://ex/a> <https://ex/p> repo:README.md .\n\
                  <https://ex/b> <https://ex/p> repo:docs.guide.v2.\n";
    let out = nquads_from_gts(&from_turtle(turtle).expect("Turtle imports dotted local names"));
    assert!(
        out.contains("<https://ex/p> <https://ex/repo/README.md>"),
        "internal-dot local name (space-terminated):\n{out}"
    );
    assert!(
        out.contains("<https://ex/p> <https://ex/repo/docs.guide.v2>"),
        "internal-dot local name (dot-terminated, no space):\n{out}"
    );
}

#[test]
fn turtle_parser_accepts_bare_numeric_boolean_and_long_string_literals() {
    // The literal forms the native parser previously lacked (which forced the oxttl
    // stopgap on the 909 branch): bare integer/decimal/double, boolean, single- and
    // triple-quoted strings. Lexical forms are preserved verbatim.
    let turtle = r#"@prefix ex: <https://ex/ns#> .
ex:s ex:int 42 ;
     ex:neg -7 ;
     ex:dec 0.70 ;
     ex:dotdec .5 ;
     ex:dbl 1.0e0 ;
     ex:expneg 6.022E23 ;
     ex:yes true ;
     ex:no false ;
     ex:apos 'single' ;
     ex:long """multi
line""" ;
     ex:longapos '''also
long''' .
"#;
    let out = nquads_from_gts(&from_turtle(turtle).expect("Turtle imports bare literals"));
    assert!(
        out.contains("\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"),
        "integer typed + verbatim:\n{out}"
    );
    assert!(
        out.contains("\"-7\"^^<http://www.w3.org/2001/XMLSchema#integer>"),
        "signed integer:\n{out}"
    );
    assert!(
        out.contains("\"0.70\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
        "decimal lexical form preserved (0.70, not 0.7):\n{out}"
    );
    assert!(
        out.contains("\".5\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
        "leading-dot decimal:\n{out}"
    );
    assert!(
        out.contains("\"1.0e0\"^^<http://www.w3.org/2001/XMLSchema#double>"),
        "double lexical form preserved (1.0e0):\n{out}"
    );
    assert!(
        out.contains("\"6.022E23\"^^<http://www.w3.org/2001/XMLSchema#double>"),
        "double with capital E and exponent:\n{out}"
    );
    assert!(
        out.contains("\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"),
        "boolean true:\n{out}"
    );
    assert!(
        out.contains("\"false\"^^<http://www.w3.org/2001/XMLSchema#boolean>"),
        "boolean false:\n{out}"
    );
    assert!(out.contains("\"single\""), "single-quoted string:\n{out}");
    assert!(
        out.contains("multi\\nline"),
        "triple-quoted string spans newlines (escaped in N-Triples):\n{out}"
    );
    assert!(
        out.contains("also\\nlong"),
        "triple single-quoted string:\n{out}"
    );
}

#[test]
fn turtle_parser_expands_rdf12_reifying_triples_and_annotations() {
    // RDF 1.2 reifier/annotation syntax must EXPAND (not be treated as triple terms):
    // `<< s p o >>`   -> fresh `_:r` + `_:r rdf:reifies <<( s p o )>>`, evaluates to `_:r`
    // `<< s p o ~ i >>` -> `i rdf:reifies <<( s p o )>>`, evaluates to `i`
    // `s p o {| a v |}` -> assert `s p o` + `_:r a v` + `_:r rdf:reifies <<( s p o )>>`
    let reifies = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>";

    // Subject reifying triple, anonymous reifier.
    let out = nquads_from_gts(
        &from_turtle("PREFIX : <http://example/>\n<<:s :p :o>> :q :z .\n").expect("rt-01"),
    );
    assert!(
        out.contains(&format!(
            "{reifies} <<( <http://example/s> <http://example/p> <http://example/o> )>>"
        )),
        "reifies a triple term:\n{out}"
    );
    assert!(
        out.contains("<http://example/q> <http://example/z>"),
        "the reifier is the subject of :q :z:\n{out}"
    );
    assert!(
        !out.contains(
            "<<( <http://example/s> <http://example/p> <http://example/o> )>> <http://example/q>"
        ),
        "the triple term must NOT be the asserted subject:\n{out}"
    );

    // Reifying triple with explicit IRI reifier identifier.
    let out = nquads_from_gts(
        &from_turtle("PREFIX : <http://example/>\n<< :s :p :o ~ :i >> :q :z .\n").expect("rt-03"),
    );
    assert!(
        out.contains(&format!(
            "<http://example/i> {reifies} <<( <http://example/s>"
        )),
        "explicit reifier id used:\n{out}"
    );
    assert!(
        out.contains("<http://example/i> <http://example/q> <http://example/z>"),
        "explicit reifier is the subject:\n{out}"
    );

    // Annotation syntax.
    let out = nquads_from_gts(
        &from_turtle("PREFIX : <http://example/>\n:s :p :o {| :r :z |} .\n")
            .expect("annotation-01"),
    );
    assert!(
        out.contains("<http://example/s> <http://example/p> <http://example/o>"),
        "base triple asserted:\n{out}"
    );
    assert!(
        out.contains(&format!(
            "{reifies} <<( <http://example/s> <http://example/p> <http://example/o> )>>"
        )),
        "annotation reifies the base triple:\n{out}"
    );
    assert!(
        out.contains("<http://example/r> <http://example/z>"),
        "annotation pair:\n{out}"
    );
}

#[test]
fn native_codecs_roundtrip_long_private_use_language_subtags() {
    // The driver behind gmeow-gts #358: GMEOW relies on long BCP-47 private-use
    // subtags like `x-gmeow-norwegiannynorsk` (>8 chars) which oxttl rejected without
    // `.lenient()`. The hand-rolled native parser/serializer do no BCP-47 length
    // validation, so the tag survives Turtle and N-Triples round-trips with no oxttl.
    let turtle =
        "@prefix ex: <https://ex/ns#> .\nex:s ex:greet \"hallo\"@x-gmeow-norwegiannynorsk .\n";
    let gts = from_turtle(turtle).expect("Turtle imports long private-use subtag");

    let turtle_out = to_turtle(&read(&gts, true, None)).expect("to_turtle");
    let reparsed = from_turtle(&turtle_out).expect("Turtle re-ingests long tag");
    let nq = to_nquads(&read(&reparsed, true, None));
    assert!(
        nq.contains("\"hallo\"@x-gmeow-norwegiannynorsk"),
        "turtle round-trip kept the long language tag:\n{nq}"
    );

    let ntriples_out = to_ntriples(&read(&gts, true, None)).expect("to_ntriples");
    let reparsed_nt = from_ntriples(&ntriples_out).expect("N-Triples re-ingests long tag");
    let nq_nt = to_nquads(&read(&reparsed_nt, true, None));
    assert!(
        nq_nt.contains("\"hallo\"@x-gmeow-norwegiannynorsk"),
        "ntriples round-trip kept the long language tag:\n{nq_nt}"
    );
}

#[test]
fn rdf_xml_parser_accepts_core_w3c_rdf_xml_shapes() {
    let cases = [
        (
            "rdf-element-not-mandatory",
            r#"<Book xmlns="http://example.org/terms#">
  <title>Dogs in Hats</title>
</Book>"#,
            [
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
                "<http://example.org/terms#Book>",
                "<http://example.org/terms#title> \"Dogs in Hats\"",
            ]
            .as_slice(),
        ),
        (
            "xml-base-language-direction-and-attribute-property",
            r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/"
         xmlns:its="http://www.w3.org/2005/11/its"
         xml:base="http://example.org/base/"
         xml:lang="en"
         its:dir="ltr"
         rdf:version="1.2">
  <rdf:Description rdf:about="item" ex:name="bar"/>
</rdf:RDF>"#,
            [
                "<http://example.org/base/item>",
                "<http://example.org/name> \"bar\"@en--ltr",
            ]
            .as_slice(),
        ),
        (
            "parse-type-resource-collection-and-literal",
            r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/eg#">
  <rdf:Description rdf:about="http://example.org/eg#eric">
    <rdf:type rdf:parseType="Resource">
      <eg:intersectionOf rdf:parseType="Collection">
        <rdf:Description rdf:about="http://example.org/eg#Person"/>
        <rdf:Description rdf:about="http://example.org/eg#Male"/>
      </eg:intersectionOf>
    </rdf:type>
  </rdf:Description>
  <rdf:Description rdf:about="http://example.org/doc">
    <eg:markup rdf:parseType="Literal"><span xmlns="http://www.w3.org/1999/xhtml">Hi</span></eg:markup>
  </rdf:Description>
</rdf:RDF>"#,
            [
                "<http://example.org/eg#eric>",
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:",
                "<http://example.org/eg#intersectionOf> _:",
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>",
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>",
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>",
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral>",
                "span",
            ]
            .as_slice(),
        ),
    ];

    for (name, rdf_xml, expected_fragments) in cases {
        let out =
            nquads_from_gts(&from_rdf_xml(rdf_xml).unwrap_or_else(|err| panic!("{name}: {err}")));
        for expected in expected_fragments {
            assert!(
                out.contains(expected),
                "{name}: expected fragment {expected:?}\n{out}"
            );
        }
    }
}

#[test]
fn rdf_xml_parser_resolves_text_entity_references() {
    let rdf_xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:label>A &amp; B &lt; C &#x21;</ex:label>
  </rdf:Description>
</rdf:RDF>"#;

    let out = nquads_from_gts(&from_rdf_xml(rdf_xml).expect("RDF/XML imports"));
    assert!(out.contains("\"A & B < C !\""), "{out}");
}

#[test]
fn rdf_xml_parser_preserves_empty_property_attributes_on_resource_objects() {
    let rdf_xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:related rdf:resource="http://example.org/o" ex:label="Object label"/>
    <ex:blank rdf:nodeID="target" ex:label="Blank label"/>
  </rdf:Description>
</rdf:RDF>"#;

    let out = nquads_from_gts(&from_rdf_xml(rdf_xml).expect("RDF/XML imports"));
    assert!(
        out.contains("<http://example.org/s> <http://example.org/related> <http://example.org/o>")
    );
    assert!(out.contains("<http://example.org/o> <http://example.org/label> \"Object label\""));
    assert!(out.contains("<http://example.org/s> <http://example.org/blank> _:target"));
    assert!(out.contains("_:target <http://example.org/label> \"Blank label\""));
}

#[test]
fn rdf_xml_parser_resolves_rdf_id_against_base_without_fragment() {
    let rdf_xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="">
    <ex:related rdf:ID="statement" rdf:resource="http://example.org/o"/>
  </rdf:Description>
</rdf:RDF>"#;

    let out = nquads_from_gts(
        &from_rdf_xml_with_base_iri(rdf_xml, "http://example.org/doc#old")
            .expect("RDF/XML imports"),
    );
    assert!(out
        .contains("<http://example.org/doc> <http://example.org/related> <http://example.org/o>"));
    // `rdf:ID` is RDF 1.0 reification: the reifier IRI is resolved against the base
    // WITHOUT the base fragment (`#statement`, not `#old#statement`) and carries the
    // classic rdf:Statement/subject/predicate/object quads.
    assert!(out.contains(
        "<http://example.org/doc#statement> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement>"
    ));
    assert!(out.contains(
        "<http://example.org/doc#statement> <http://www.w3.org/1999/02/22-rdf-syntax-ns#object> <http://example.org/o>"
    ));
    assert!(!out.contains("#old#statement"));
}

#[test]
fn rdf_xml_parser_accepts_w3c_rdf12_triple_terms_and_annotations() {
    let rdf_xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/stuff/1.0/"
         rdf:version="1.2">
  <rdf:Description rdf:about="http://example.org/">
    <ex:prop rdf:annotation="http://example.org/triple1">blah</ex:prop>
    <ex:triple rdf:parseType="Triple">
      <rdf:Description rdf:about="http://example.org/stuff/1.0/s">
        <ex:p rdf:resource="http://example.org/stuff/1.0/o"/>
      </rdf:Description>
    </ex:triple>
  </rdf:Description>
</rdf:RDF>"#;

    let out = nquads_from_gts(&from_rdf_xml(rdf_xml).expect("RDF/XML imports"));
    assert!(out.contains("<http://example.org/stuff/1.0/prop> \"blah\""));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>"));
    assert!(
        out.contains("<<( <http://example.org/> <http://example.org/stuff/1.0/prop> \"blah\" )>>")
    );
    assert!(out.contains(
        "<<( <http://example.org/stuff/1.0/s> <http://example.org/stuff/1.0/p> <http://example.org/stuff/1.0/o> )>>"
    ));
}

#[test]
fn rdf_xml_parser_rejects_w3c_rdf12_bad_triple_terms() {
    let missing_predicate_or_object = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/stuff/1.0/"
         rdf:version="1.2">
  <rdf:Description rdf:about="http://example.org/">
    <ex:prop rdf:parseType="Triple">
      <rdf:Description rdf:about="http://example.org/stuff/1.0/s"/>
    </ex:prop>
  </rdf:Description>
</rdf:RDF>"#;
    assert!(from_rdf_xml(missing_predicate_or_object).is_err());

    let two_objects = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/stuff/1.0/"
         rdf:version="1.2">
  <rdf:Description rdf:about="http://example.org/">
    <ex:prop rdf:parseType="Triple">
      <rdf:Description rdf:about="http://example.org/stuff/1.0/s">
        <ex:p rdf:resource="http://example.org/stuff/1.0/o1"/>
        <ex:p rdf:resource="http://example.org/stuff/1.0/o2"/>
      </rdf:Description>
    </ex:prop>
  </rdf:Description>
</rdf:RDF>"#;
    assert!(from_rdf_xml(two_objects).is_err());
}

#[test]
fn rdf_xml_parser_passes_w3c_suite_when_fixture_root_is_set() {
    let Some(root) = std::env::var_os("GTS_W3C_RDF_TESTS").map(PathBuf::from) else {
        return;
    };

    let mut cases = Vec::new();
    for manifest in [
        "rdf/rdf11/rdf-xml/manifest.ttl",
        "rdf/rdf12/rdf-xml/eval/manifest.ttl",
    ] {
        cases.extend(
            w3c_manifest_cases(&root, manifest).unwrap_or_else(|err| panic!("{manifest}: {err}")),
        );
    }
    cases.sort_by(|left, right| left.action.cmp(&right.action));

    let mut checked = 0usize;
    for case in cases {
        let relative = case
            .action
            .strip_prefix(&root)
            .unwrap_or(&case.action)
            .to_string_lossy()
            .replace('\\', "/");
        let base_iri = format!("https://w3c.github.io/rdf-tests/{relative}");
        let rdf_xml = std::fs::read_to_string(&case.action)
            .unwrap_or_else(|err| panic!("{relative}: cannot read RDF/XML fixture: {err}"));

        if let Some(expected_path) = case.result {
            let expected = std::fs::read_to_string(&expected_path)
                .unwrap_or_else(|err| panic!("{relative}: cannot read expected N-Triples: {err}"));
            let actual = from_rdf_xml_with_base_iri(&rdf_xml, &base_iri)
                .unwrap_or_else(|err| panic!("{relative}: {err}"));
            assert_nquads_isomorphic_to_ntriples(&nquads_from_gts(&actual), &expected, &relative);
        } else {
            assert!(
                from_rdf_xml_with_base_iri(&rdf_xml, &base_iri).is_err(),
                "{relative}: negative RDF/XML fixture parsed successfully"
            );
        }
        checked += 1;
    }

    assert!(
        checked > 150,
        "expected W3C RDF/XML suite, checked {checked}"
    );
}

#[derive(Debug)]
struct W3cRdfXmlCase {
    action: PathBuf,
    result: Option<PathBuf>,
}

fn w3c_manifest_cases(
    root: &std::path::Path,
    manifest: &str,
) -> std::io::Result<Vec<W3cRdfXmlCase>> {
    use std::collections::HashSet;

    let manifest_path = root.join(manifest);
    let manifest_text = std::fs::read_to_string(&manifest_path)?;
    let manifest_dir = manifest_path
        .parent()
        .expect("manifest path has a parent directory");
    let entries: HashSet<String> = manifest_entry_tokens(&manifest_text).into_iter().collect();
    let mut cases = Vec::new();
    let mut current: Option<String> = None;
    let mut action: Option<PathBuf> = None;
    let mut result: Option<PathBuf> = None;

    for line in manifest_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(subject) = trimmed.split_whitespace().next() {
            if entries.contains(subject) {
                push_w3c_manifest_case(&mut cases, manifest_dir, &mut action, &mut result);
                current = Some(subject.to_string());
            } else if current.is_some() && is_manifest_subject_token(subject) {
                push_w3c_manifest_case(&mut cases, manifest_dir, &mut action, &mut result);
                current = None;
            }
        }
        if current.is_some() {
            if let Some(path) = angle_value_after(trimmed, "mf:action") {
                action = Some(manifest_dir.join(path));
            }
            if let Some(path) = angle_value_after(trimmed, "mf:result") {
                result = Some(manifest_dir.join(path));
            }
        }
    }
    push_w3c_manifest_case(&mut cases, manifest_dir, &mut action, &mut result);
    Ok(cases)
}

fn is_manifest_subject_token(token: &str) -> bool {
    (token.starts_with("<#") && token.ends_with('>')) || token.starts_with("trs:")
}

fn manifest_entry_tokens(manifest_text: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut in_entries = false;
    for line in manifest_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.contains("mf:entries") {
            in_entries = true;
        }
        if !in_entries {
            continue;
        }
        if trimmed.starts_with(')') {
            break;
        }
        for token in trimmed.split_whitespace() {
            let token = token.trim_end_matches(';').trim_end_matches(',');
            if is_manifest_subject_token(token) {
                entries.push(token.to_string());
            }
        }
    }
    entries
}

fn angle_value_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    let after_marker = line.split_once(marker)?.1;
    let start = after_marker.find('<')? + 1;
    let rest = &after_marker[start..];
    let end = rest.find('>')?;
    Some(&rest[..end])
}

fn push_w3c_manifest_case(
    cases: &mut Vec<W3cRdfXmlCase>,
    manifest_dir: &std::path::Path,
    action: &mut Option<PathBuf>,
    result: &mut Option<PathBuf>,
) {
    if let Some(action) = action.take() {
        cases.push(W3cRdfXmlCase {
            action,
            result: result.take().map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    manifest_dir.join(path)
                }
            }),
        });
    }
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
fn trig_parser_allows_comment_only_empty_structures_inside_rdf12_triples() {
    let trig = r#"PREFIX ex: <https://ex/>
<< [
  # empty blank node subject
] ex:p (
  # rdf:nil object
) >> ex:source ex:doc .
"#;

    let out = nquads_from_gts(&from_trig(trig).expect("TriG imports"));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>"));
    assert!(out.contains("<<( _:"));
    assert!(out.contains("<https://ex/p> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"));
    assert!(out.contains("<https://ex/source> <https://ex/doc>"));
}

#[test]
fn trig_parser_rejects_non_empty_structures_inside_rdf12_triples_after_comments() {
    let blank_node_property_list = r#"PREFIX ex: <https://ex/>
<< [
  # non-empty property list
  ex:p ex:o
] ex:p ex:o >> ex:source ex:doc .
"#;
    assert!(from_trig(blank_node_property_list).is_err());

    let collection = r#"PREFIX ex: <https://ex/>
<< ex:s ex:p (
  # non-empty collection
  ex:o
) >> ex:source ex:doc .
"#;
    assert!(from_trig(collection).is_err());
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

#[test]
fn rdf_xml_serialization_uses_event_source_and_rejects_named_graphs() {
    let default_graph = sample_graph(false);
    let source: &dyn RdfEventSource = &GraphRdfEventSource::new(&default_graph);
    let rdf_xml = to_rdf_xml_from_erased_source(source).expect("RDF/XML serializes from source");
    assert!(rdf_xml.contains("<rdf:RDF"));

    let imported = from_rdf_xml(&rdf_xml).expect("serialized RDF/XML imports");
    assert_eq!(
        sorted_lines(&to_nquads(&default_graph)),
        sorted_lines(&nquads_from_gts(&imported))
    );

    let err = to_rdf_xml(&sample_graph(true)).expect_err("named graph is not RDF/XML");
    assert!(err
        .to_string()
        .contains("RDF/XML cannot serialize named graph"));
}

#[test]
fn rdf_xml_serialization_roundtrips_unicode_predicate_local_names() {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        term(TermKind::Iri, "https://ex/s"),
        term(TermKind::Iri, "https://ex/étiquette"),
        Term {
            kind: TermKind::Literal,
            value: Some("chat".to_string()),
            datatype: None,
            lang: Some("fr".to_string()),
            direction: None,
            reifier: None,
        },
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);

    let graph = read(&writer.to_bytes(), true, None);
    let rdf_xml = to_rdf_xml(&graph).expect("RDF/XML serializes");
    assert!(rdf_xml.contains("étiquette"));
    let imported = from_rdf_xml(&rdf_xml).expect("serialized RDF/XML imports");
    assert_eq!(
        sorted_lines(&to_nquads(&graph)),
        sorted_lines(&nquads_from_gts(&imported))
    );
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

    let rdfxml_path = tmp.join("in.rdf");
    let rdfxml_gts = tmp.join("rdfxml.gts");
    std::fs::write(
        &rdfxml_path,
        r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="https://ex/">
  <rdf:Description rdf:about="https://ex/s">
    <ex:p rdf:resource="https://ex/o"/>
  </rdf:Description>
</rdf:RDF>"#,
    )
    .unwrap();
    let out = gts(&[
        "from-rdfxml",
        rdfxml_path.to_str().unwrap(),
        "-o",
        rdfxml_gts.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rdfxml = gts(&["to-rdfxml", rdfxml_gts.to_str().unwrap()]);
    assert!(
        rdfxml.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&rdfxml.stderr)
    );
    let rendered = String::from_utf8(rdfxml.stdout).unwrap();
    assert!(rendered.contains("<rdf:RDF"));
    assert!(rendered.contains("https://ex/s"));

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

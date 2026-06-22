// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "rdf-codecs")]

//! RDF 1.2 text codec tests.

use std::path::PathBuf;
use std::process::Command;

use oxrdf::dataset::CanonicalizationAlgorithm;
use oxrdf::{Dataset, GraphNameRef};
use oxttl::{NQuadsParser, NTriplesParser};

use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::rdf_codecs::{
    from_ntriples, from_rdf_xml, from_rdf_xml_with_base_iri, from_trig, from_turtle, to_ntriples,
    to_ntriples_from_erased_source, to_rdf_xml, to_rdf_xml_from_erased_source,
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

fn canonical_dataset_from_nquads(text: &str) -> Dataset {
    let mut dataset = Dataset::new();
    for quad in NQuadsParser::new().for_slice(text.as_bytes()) {
        dataset.insert(quad.expect("N-Quads parser accepts codec output").as_ref());
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Unstable);
    dataset
}

fn canonical_dataset_from_ntriples(text: &str) -> Dataset {
    let mut dataset = Dataset::new();
    for triple in NTriplesParser::new().for_slice(text.as_bytes()) {
        dataset.insert(
            triple
                .expect("N-Triples parser accepts expected RDF")
                .as_ref()
                .in_graph(GraphNameRef::DefaultGraph),
        );
    }
    dataset.canonicalize(CanonicalizationAlgorithm::Unstable);
    dataset
}

fn assert_nquads_isomorphic_to_ntriples(actual_nquads: &str, expected_ntriples: &str, name: &str) {
    let actual = canonical_dataset_from_nquads(actual_nquads);
    let expected = canonical_dataset_from_ntriples(expected_ntriples);
    assert_eq!(
        actual, expected,
        "{name}: RDF datasets differ\nactual:\n{actual}\nexpected:\n{expected}"
    );
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

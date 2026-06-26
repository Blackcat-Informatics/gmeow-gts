// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `trig -> gts` inverse-of-fold tests.

use std::path::PathBuf;
use std::process::Command;

use gmeow_gts::from_trig::from_trig;
use gmeow_gts::model::{Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::trig::to_trig;
use gmeow_gts::writer::Writer;

const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors")
}

fn sorted_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("@prefix"))
        .map(str::to_string)
        .collect();
    lines.sort();
    lines
}

fn trig_roundtrip_nquads(data: &[u8]) -> bool {
    let folded = read(data, true, None);
    let nq1 = to_nquads(&folded);
    let trig = to_trig(&folded);
    let imported = from_trig(&trig).expect("TriG output parses");
    let nq2 = to_nquads(&read(&imported, true, None));
    sorted_lines(&nq1) == sorted_lines(&nq2)
}

#[test]
fn fold_roundtrips_through_trig_for_graph_vectors() {
    for entry in std::fs::read_dir(vectors()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("gts") {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let folded = read(&data, true, None);
        if to_trig(&folded).trim().is_empty() {
            continue;
        }
        assert!(
            trig_roundtrip_nquads(&data),
            "round-trip failed for {}",
            path.display()
        );
    }
}

#[test]
fn to_trig_groups_named_graphs_and_keeps_reifiers() {
    let mut w = Writer::new("dist");
    w.add_terms(&[
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
            value: Some("https://ex/o".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("https://ex/g".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("https://ex/conf".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Literal,
            value: Some("0.9".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
    ]);
    w.add_quads(&[(0, 1, 2, Some(3))]);
    w.add_reifies(&[(0, (0, 1, 2))]);
    w.add_annot(&[(0, 4, 5)]);

    let folded = read(&w.to_bytes(), true, None);
    let trig = to_trig(&folded);
    assert!(trig.contains("@prefix rdf:"));
    assert!(trig.contains("<https://ex/g> {\n  <https://ex/s> <https://ex/p> <https://ex/o> .\n}"));
    assert!(trig.contains("rdf:reifies <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> ."));
    assert!(trig_roundtrip_nquads(&w.to_bytes()));
}

#[test]
fn parses_prefixes_graph_blocks_reifiers_and_literals() {
    let trig = r#"@prefix ex: <https://ex/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:g {
  ex:s ex:label "Cat"@en .
  ex:s ex:n "42"^^xsd:integer .
}
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9" .
"#;
    let imported = from_trig(trig).expect("TriG parses");
    let out = to_nquads(&read(&imported, true, None));
    let expected = format!(
        "<https://ex/s> <https://ex/label> \"Cat\"@en <https://ex/g> .\n\
         <https://ex/s> <https://ex/n> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> <https://ex/g> .\n\
         <https://ex/r> <{RDF_REIFIES}> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n\
         <https://ex/r> <https://ex/confidence> \"0.9\" .\n"
    );
    assert_eq!(sorted_lines(&out), sorted_lines(&expected));
}

#[test]
fn trig_preserves_directional_language_literals() {
    let trig = r#"@prefix ex: <https://ex/> .

ex:s ex:label "Cat"@en--rtl .
"#;
    let imported = from_trig(trig).expect("TriG parses");
    let graph = read(&imported, true, None);
    let nq = to_nquads(&graph);
    assert!(nq.contains("\"Cat\"@en--rtl"));

    let rendered = to_trig(&graph);
    assert!(rendered.contains("\"Cat\"@en--rtl"));
    assert!(trig_roundtrip_nquads(&imported));
}

#[test]
fn parses_graph_keyword_and_a_predicate() {
    let trig = r#"PREFIX ex: <https://ex/>
GRAPH ex:g {
  ex:s a ex:Thing .
}
"#;
    let imported = from_trig(trig).expect("TriG parses");
    let out = to_nquads(&read(&imported, true, None));
    assert_eq!(
        sorted_lines(&out),
        vec![
            "<https://ex/s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ex/Thing> <https://ex/g> .".to_string()
        ]
    );
}

#[test]
fn prefixed_names_stop_before_quoted_triple_close() {
    let trig = r#"@prefix ex: <https://ex/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:r rdf:reifies <<( ex:s ex:p ex:o)>> .
"#;
    let imported = from_trig(trig).expect("adjacent quoted triple close parses");
    let out = to_nquads(&read(&imported, true, None));
    let expected = format!(
        "<https://ex/r> <{RDF_REIFIES}> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> ."
    );
    assert_eq!(sorted_lines(&out), vec![expected]);
}

#[test]
fn pn_local_esc_keeps_escaped_delimiters_in_local_name() {
    // Turtle PN_LOCAL_ESC: backslash-escaped delimiters (`\(`, `\)`, `\,`, `\.`) are
    // part of the prefixed name's local part, not statement/object delimiters. This
    // is the exact shape that appears in real DBpedia IRIs
    // (`dbr:Semantic_analysis_\(linguistics\)`), so an escaped `(` must NOT terminate
    // the scan.
    let trig = r#"@prefix ex: <https://ex/> .
@prefix dbr: <http://dbpedia.org/resource/> .
ex:s ex:p dbr:Semantic_analysis_\(linguistics\) , ex:o .
ex:s ex:q dbr:a\,b\.c .
"#;
    let imported = from_trig(trig).expect("PN_LOCAL_ESC escapes parse");
    let out = to_nquads(&read(&imported, true, None));
    let lines = sorted_lines(&out);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("<http://dbpedia.org/resource/Semantic_analysis_(linguistics)>")),
        "escaped parens resolve to literal `(`/`)` in the IRI: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("<http://dbpedia.org/resource/a,b.c>")),
        "escaped comma + dot resolve literally: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("<https://ex/o>")),
        "the object list continues past the escaped name: {lines:?}"
    );
}

#[test]
fn rejects_malformed_or_unsupported_trig() {
    let err = from_trig("@prefix ex: <https://ex/> .\nex:s ex:p ex:o\n")
        .expect_err("missing dot is malformed");
    assert!(err.to_string().contains("terminate statement"));
}

#[test]
fn parses_predicate_object_shorthand_and_lists() {
    let trig = r#"@base <https://ex/> .
@prefix ex: <https://ex/ns#> .

ex:s a ex:Thing ;
    ex:p ex:o, ex:o2 ;
    ex:nested [ ex:name "Kit" ] ;
    ex:list ( ex:a ex:b ) .
"#;
    let imported = from_trig(trig).expect("TriG shorthand parses");
    let out = to_nquads(&read(&imported, true, None));
    assert!(out.contains("<https://ex/ns#p> <https://ex/ns#o>"));
    assert!(out.contains("<https://ex/ns#p> <https://ex/ns#o2>"));
    assert!(out.contains("<https://ex/ns#name> \"Kit\""));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>"));
    assert!(out.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>"));
}

#[test]
fn resolves_base_iri_relative_references() {
    let trig = r#"@base <https://ex/dir/file?old#frag> .

<child> <#frag> <../up> .
</root> <?q> <> .
<./same> <https://ex/p> <a/./b/../c#frag> .
"#;
    let imported = from_trig(trig).expect("relative IRIs parse");
    let out = to_nquads(&read(&imported, true, None));
    let expected = "\
<https://ex/dir/child> <https://ex/dir/file?old#frag> <https://ex/up> .\n\
<https://ex/root> <https://ex/dir/file?q> <https://ex/dir/file?old> .\n\
<https://ex/dir/same> <https://ex/p> <https://ex/dir/a/c#frag> .\n";
    assert_eq!(sorted_lines(&out), sorted_lines(expected));
}

#[test]
fn rejects_forbidden_raw_iri_characters() {
    for ch in ['{', '}', '|', '\\', '^', '`'] {
        let trig = format!("<https://ex/{ch}> <https://ex/p> <https://ex/o> .\n");
        let err = match from_trig(&trig) {
            Ok(_) => panic!("forbidden IRI character {ch:?} should be rejected"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("invalid character in IRI"),
            "unexpected error for {ch:?}: {err}"
        );
    }
}

fn tmpdir() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-from-trig-test-{}-{n}", std::process::id()))
}

fn gts(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

#[test]
fn cli_from_trig_inverts_to_trig() {
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let src = vectors().join("11-datatype-defaulting.gts");
    let folded_src = gts(&["fold", src.to_str().unwrap()]);
    assert!(
        folded_src.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&folded_src.stderr)
    );
    let nq = String::from_utf8(folded_src.stdout).unwrap();

    let trig_src = gts(&["to-trig", src.to_str().unwrap()]);
    assert!(
        trig_src.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&trig_src.stderr)
    );
    let trig_path = tmp.join("in.trig");
    std::fs::write(&trig_path, trig_src.stdout).unwrap();
    let out_path = tmp.join("out.gts");

    let out = gts(&[
        "from-trig",
        trig_path.to_str().unwrap(),
        "-o",
        out_path.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let folded_out = gts(&["fold", out_path.to_str().unwrap()]);
    assert!(
        folded_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&folded_out.stderr)
    );
    let folded = String::from_utf8(folded_out.stdout).unwrap();
    assert_eq!(sorted_lines(&folded), sorted_lines(&nq));
}

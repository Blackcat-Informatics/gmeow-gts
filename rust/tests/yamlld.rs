// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "yaml-ld")]

//! YAML-LD-star / JSON-LD-star inverse-of-fold tests.

use std::path::PathBuf;
use std::process::{Command, Output};

use gmeow_gts::from_yamlld::{from_json_ld, from_yaml_ld};
use gmeow_gts::model::{Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;
use gmeow_gts::yamlld::{to_json_ld_string, to_yaml_ld};

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors")
}

fn sorted_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect();
    lines.sort();
    lines
}

fn graph_lines(data: &[u8]) -> Vec<String> {
    sorted_lines(&to_nquads(&read(data, true, None)))
}

fn gts(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

fn tmpdir() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-yamlld-test-{}-{n}", std::process::id()))
}

fn annotated_fixture() -> Vec<u8> {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
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
            kind: TermKind::Literal,
            value: Some("Cat".to_string()),
            datatype: None,
            lang: Some("en".to_string()),
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Bnode,
            value: Some("r".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("https://ex/confidence".to_string()),
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
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.add_reifies(&[(3, (0, 1, 2), None)]);
    writer.add_annot(&[(3, 4, 5, None)]);
    writer.to_bytes()
}

#[test]
fn yaml_ld_preserves_reifier_annotations_as_folded_state() {
    let source = annotated_fixture();
    let graph = read(&source, true, None);
    let yaml = to_yaml_ld(&graph).expect("YAML-LD renders");
    assert!(yaml.contains("@annotation"));
    assert!(yaml.contains("@language"));

    let imported = from_yaml_ld(&yaml).expect("YAML-LD imports");
    let folded = read(&imported, true, None);
    assert_eq!(folded.reifiers.len(), 1);
    assert_eq!(folded.annotations.len(), 1);
    assert_eq!(graph_lines(&source), graph_lines(&imported));
}

#[test]
fn json_ld_uses_the_same_intermediate_and_round_trips() {
    let source = annotated_fixture();
    let graph = read(&source, true, None);
    let json = to_json_ld_string(&graph).expect("JSON-LD renders");
    assert!(json.contains("\"@annotation\""));

    let imported = from_json_ld(&json).expect("JSON-LD imports");
    assert_eq!(graph_lines(&source), graph_lines(&imported));
}

#[test]
fn yaml_ld_roundtrips_the_graph_vector_corpus() {
    for entry in std::fs::read_dir(vectors()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("gts") {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let folded = read(&data, true, None);
        let nquads = to_nquads(&folded);
        if nquads.trim().is_empty() {
            continue;
        }
        let yaml = to_yaml_ld(&folded).expect("YAML-LD renders");
        let imported = from_yaml_ld(&yaml).unwrap_or_else(|error| {
            panic!("YAML-LD import failed for {}: {error}", path.display())
        });
        assert_eq!(
            sorted_lines(&nquads),
            graph_lines(&imported),
            "round-trip failed for {}",
            path.display()
        );
    }
}

#[test]
fn compact_context_authoring_shape_imports_annotations_and_scalars() {
    let yaml = r#"
"@context":
  ex: "https://ex/"
  gmeow: "https://blackcatinformatics.ca/gmeow/"
  xsd: "http://www.w3.org/2001/XMLSchema#"
"@id": "ex:claim"
"gmeow:claimModality":
  "@id": "gmeow:bullshit"
  "@annotation":
    "@id": "ex:r"
    "gmeow:confidence": 0.65
    "gmeow:assertedAt":
      "@value": "2026-06-05T00:00:00+00:00"
      "@type": "xsd:dateTime"
"#;
    let imported = from_yaml_ld(yaml).expect("compact YAML-LD imports");
    let nquads = to_nquads(&read(&imported, true, None));

    assert!(nquads.contains("<https://ex/r>"));
    assert!(nquads.contains("<https://blackcatinformatics.ca/gmeow/confidence>"));
    assert!(nquads.contains("\"0.65\"^^<http://www.w3.org/2001/XMLSchema#decimal>"));
    assert!(nquads
        .contains("\"2026-06-05T00:00:00+00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime>"));
}

#[test]
fn scoped_contexts_expand_node_value_and_annotation_terms() {
    let yaml = r#"
"@context":
  ex: "https://ex/"
"@graph":
  - "@context":
      local: "https://local/"
    "@id": "local:s"
    "local:p":
      "@context":
        obj: "https://obj/"
      "@id": "obj:o"
      "@annotation":
        "@context":
          ann: "https://ann/"
        "@id": "ann:r"
        "ann:confidence": 1
"#;
    let imported = from_yaml_ld(yaml).expect("scoped contexts import");
    let nquads = to_nquads(&read(&imported, true, None));

    assert!(nquads.contains("<https://local/s> <https://local/p> <https://obj/o> ."));
    assert!(nquads.contains("<https://ann/r>"));
    assert!(nquads.contains("<https://ann/confidence>"));
    assert!(nquads.contains("\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
}

#[test]
fn yaml_ld_preserves_directional_language_literals() {
    let yaml = r#"
"@id": "https://ex/s"
"https://ex/label":
  "@value": "Cat"
  "@language": "en"
  "@direction": "ltr"
"#;
    let imported = from_yaml_ld(yaml).expect("YAML-LD imports");
    let graph = read(&imported, true, None);
    let nquads = to_nquads(&graph);
    assert!(nquads.contains("\"Cat\"@en--ltr"));

    let rendered = to_yaml_ld(&graph).expect("YAML-LD renders");
    assert!(rendered.contains("@direction"));
    assert!(rendered.contains("ltr"));
}

#[test]
fn yaml_ld_rejects_language_literals_with_type_or_non_string_values() {
    let typed_language = r#"
"@id": "https://ex/s"
"https://ex/label":
  "@value": "Cat"
  "@language": "en"
  "@type": "xsd:string"
"#;
    let err = from_yaml_ld(typed_language).expect_err("mixed language/type is rejected");
    assert!(err.to_string().contains("@type cannot be combined"));

    let numeric_language = r#"
"@id": "https://ex/s"
"https://ex/label":
  "@value": 1
  "@language": "en"
"#;
    let err = from_yaml_ld(numeric_language).expect_err("language literal value must be string");
    assert!(err.to_string().contains("@value must be a string"));
}

#[test]
fn cli_yaml_ld_inverts_fold() {
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let src = vectors().join("11-datatype-defaulting.gts");
    let yaml_path = tmp.join("fold.yaml");
    let out_path = tmp.join("out.gts");

    let folded_src = gts(&["fold", src.to_str().unwrap()]);
    assert!(
        folded_src.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&folded_src.stderr)
    );
    let expected = String::from_utf8(folded_src.stdout).unwrap();

    let yaml = gts(&["to-yaml-ld", src.to_str().unwrap()]);
    assert!(
        yaml.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&yaml.stderr)
    );
    std::fs::write(&yaml_path, &yaml.stdout).unwrap();

    let out = gts(&[
        "from-yaml-ld",
        yaml_path.to_str().unwrap(),
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
    assert_eq!(
        sorted_lines(&expected),
        sorted_lines(&String::from_utf8(folded_out.stdout).unwrap())
    );
}

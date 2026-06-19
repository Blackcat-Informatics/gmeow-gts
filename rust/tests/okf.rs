// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "okf")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use gmeow_gts::from_okf::{from_okf, from_okf_with_options, FromOkfOptions};
use gmeow_gts::model::{Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::okf::{
    to_okf, OkfExportOptions, OKF_BODY, OKF_LINKS, OKF_LINK_OCCURRENCE, OKF_LINK_TEXT, OKF_PATH,
    OKF_TYPE,
};
use gmeow_gts::reader::read;
use gmeow_gts::wire::digest_str;

fn tmpdir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-okf-{name}-{}-{n}", std::process::id()))
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

fn sample_bundle(root: &Path) {
    write(
        &root.join("concepts/table.md"),
        r#"---
active: true
description: Event facts
producer:
  name: BigQuery
  version: 1
resource: https://data.example/tables/events
rows: 42
score: 12.5
tags:
  - warehouse
  - analytics
timestamp: "2026-06-19T00:00:00Z"
title: Events
type: BigQuery Table
---
# Events

See [Schema](schema.md).
"#,
    );
    write(
        &root.join("concepts/schema.md"),
        r#"---
type: Schema
title: Schema
tags:
  - zeta
  - alpha
---
Columns live here.
"#,
    );
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

fn body_after_frontmatter(path: &Path) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let mut offset = if text.starts_with("---\r\n") { 5 } else { 4 };
    loop {
        let line_end = offset + text[offset..].find('\n').unwrap();
        let line = text[offset..line_end].trim_end_matches('\r');
        let after = line_end + 1;
        if line == "---" {
            return bytes[after..].to_vec();
        }
        offset = after;
    }
}

fn gts(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

#[test]
fn from_okf_maps_frontmatter_body_and_links() {
    let root = tmpdir("import");
    let _ = std::fs::remove_dir_all(&root);
    sample_bundle(&root);

    let data = from_okf(&root).expect("OKF imports");
    let graph = read(&data, true, None);
    let nquads = to_nquads(&graph);
    let table_body = b"# Events\n\nSee [Schema](schema.md).\n";
    let digest = digest_str(table_body);

    assert!(nquads.contains(&format!(
        "<https://data.example/tables/events> <{OKF_PATH}> \"concepts/table.md\""
    )));
    assert!(nquads.contains(&format!(
        "<https://data.example/tables/events> <{OKF_TYPE}> \"BigQuery Table\""
    )));
    assert!(nquads.contains("\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
    assert!(nquads.contains("\"12.5\"^^<http://www.w3.org/2001/XMLSchema#decimal>"));
    assert!(nquads.contains("\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"));
    assert!(nquads.contains("\"{\\\"name\\\":\\\"BigQuery\\\",\\\"version\\\":1}\"^^<https://blackcatinformatics.ca/projects/gts/okf#json>"));
    assert!(nquads.contains(&format!(
        "<https://data.example/tables/events> <{OKF_BODY}> \"{digest}\""
    )));
    assert!(nquads.contains(OKF_LINKS));
    assert!(nquads.contains(OKF_LINK_TEXT));
    assert!(nquads.contains(OKF_LINK_OCCURRENCE));

    assert_eq!(
        graph.blob_entry(&digest).unwrap().cached_bytes(),
        Some(table_body.as_slice())
    );
}

#[test]
fn link_extraction_handles_utf8_link_text() {
    let root = tmpdir("utf8-link");
    let _ = std::fs::remove_dir_all(&root);
    write(
        &root.join("source.md"),
        r#"---
type: Concept
---
See [Schéma](target.md).
"#,
    );
    write(
        &root.join("target.md"),
        r#"---
type: Concept
---
Target.
"#,
    );

    let data = from_okf(&root).expect("UTF-8 OKF imports");
    let nquads = to_nquads(&read(&data, true, None));
    assert!(nquads.contains("\"Schéma\""));
    assert!(nquads.contains(OKF_LINKS));
}

#[test]
fn okf_forward_roundtrip_restores_body_bytes_and_normalizes_tags() {
    let root = tmpdir("forward-src");
    let out = tmpdir("forward-out");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
    sample_bundle(&root);

    let data = from_okf(&root).expect("OKF imports");
    let graph = read(&data, true, None);
    let report = to_okf(&graph, &out, &OkfExportOptions::default()).expect("OKF exports");

    assert_eq!(report.documents, 2);
    assert_eq!(report.unmapped_triples, 0);
    assert_eq!(
        body_after_frontmatter(&out.join("concepts/table.md")),
        b"# Events\n\nSee [Schema](schema.md).\n"
    );

    let schema = std::fs::read_to_string(out.join("concepts/schema.md")).unwrap();
    let alpha = schema.find("- alpha").unwrap();
    let zeta = schema.find("- zeta").unwrap();
    assert!(alpha < zeta, "tags are sorted in emitted frontmatter");
}

#[test]
fn okf_reverse_roundtrip_preserves_folded_profile_graph() {
    let root = tmpdir("reverse-src");
    let out = tmpdir("reverse-out");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
    sample_bundle(&root);

    let data = from_okf(&root).expect("OKF imports");
    let graph = read(&data, true, None);
    to_okf(&graph, &out, &OkfExportOptions::default()).expect("OKF exports");
    let imported = from_okf(&out).expect("exported OKF imports");

    assert_eq!(graph_lines(&data), graph_lines(&imported));
}

#[test]
fn inline_body_profile_variant_exports_only_when_requested() {
    let root = tmpdir("inline-src");
    let out = tmpdir("inline-out");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
    sample_bundle(&root);

    let data = from_okf_with_options(
        &root,
        &FromOkfOptions {
            inline_body: true,
            ..FromOkfOptions::default()
        },
    )
    .expect("inline OKF imports");
    let graph = read(&data, true, None);
    assert!(graph.blobs.is_empty());
    assert!(to_okf(&graph, &out, &OkfExportOptions::default()).is_err());
    to_okf(
        &graph,
        &out,
        &OkfExportOptions {
            inline_body: true,
            ..OkfExportOptions::default()
        },
    )
    .expect("inline body exports when requested");
    assert_eq!(
        body_after_frontmatter(&out.join("concepts/table.md")),
        b"# Events\n\nSee [Schema](schema.md).\n"
    );
}

#[test]
fn malformed_bundle_missing_type_is_rejected() {
    let root = tmpdir("bad");
    let _ = std::fs::remove_dir_all(&root);
    write(
        &root.join("bad.md"),
        r#"---
title: Missing Type
---
body
"#,
    );
    let err = from_okf(&root).expect_err("missing type is malformed");
    assert!(err.to_string().contains("type"));
}

#[test]
fn to_okf_writes_unmapped_sidecar_instead_of_dropping_rdf() {
    let root = tmpdir("unmapped-src");
    let out = tmpdir("unmapped-out");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
    sample_bundle(&root);

    let data = from_okf(&root).expect("OKF imports");
    let mut graph = read(&data, true, None);
    let subject = graph
        .terms
        .iter()
        .position(|term| {
            term.kind == TermKind::Iri
                && term.value.as_deref() == Some("https://data.example/tables/events")
        })
        .unwrap();
    let predicate = graph.terms.len();
    graph.terms.push(Term {
        kind: TermKind::Iri,
        value: Some("https://example.org/out-of-profile".to_string()),
        datatype: None,
        lang: None,
        reifier: None,
    });
    let object = graph.terms.len();
    graph.terms.push(Term {
        kind: TermKind::Literal,
        value: Some("kept".to_string()),
        datatype: None,
        lang: None,
        reifier: None,
    });
    graph.quads.push((subject, predicate, object, None));

    let report = to_okf(&graph, &out, &OkfExportOptions::default()).expect("OKF exports");
    assert_eq!(report.unmapped_triples, 1);
    let unmapped = std::fs::read_to_string(out.join("_unmapped.nq")).unwrap();
    assert!(unmapped.contains("https://example.org/out-of-profile"));
}

#[test]
fn cli_from_okf_inverts_to_okf() {
    let root = tmpdir("cli-src");
    let export = tmpdir("cli-export");
    let out_gts = tmpdir("cli-out").join("out.gts");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&export);
    if let Some(parent) = out_gts.parent() {
        let _ = std::fs::remove_dir_all(parent);
        std::fs::create_dir_all(parent).unwrap();
    }
    sample_bundle(&root);

    let from = gts(&[
        "from-okf",
        root.to_str().unwrap(),
        "-o",
        out_gts.to_str().unwrap(),
    ]);
    assert!(
        from.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&from.stderr)
    );

    let to = gts(&[
        "to-okf",
        out_gts.to_str().unwrap(),
        "--directory",
        export.to_str().unwrap(),
    ]);
    assert!(
        to.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&to.stderr)
    );

    let imported = from_okf(&export).expect("CLI export imports");
    assert_eq!(
        graph_lines(&std::fs::read(&out_gts).unwrap()),
        graph_lines(&imported)
    );
}

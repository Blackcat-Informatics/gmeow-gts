// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "okf")]

use std::fs;
use std::path::{Path, PathBuf};

use gmeow_gts::from_okf::from_okf;
use gmeow_gts::model::{Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::okf::{to_okf, OkfExportOptions};
use gmeow_gts::reader::read;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectors/okf")
}

fn tmpdir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-okf-corpus-{name}-{}-{n}", std::process::id()))
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

fn folded_lines(data: &[u8]) -> Vec<String> {
    sorted_lines(&to_nquads(&read(data, true, None)))
}

fn contains_markdown(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if contains_markdown(&entry.path()) {
                return true;
            }
        } else if file_type.is_file()
            && entry.path().extension().and_then(|ext| ext.to_str()) == Some("md")
        {
            return true;
        }
    }
    false
}

fn okf_bundles() -> Vec<(String, PathBuf)> {
    let dir = corpus_dir();
    let mut bundles = Vec::new();
    for entry in fs::read_dir(&dir).expect("OKF vector directory exists") {
        let entry = entry.expect("OKF vector dir entry");
        let file_type = entry.file_type().expect("OKF vector entry file type");
        if !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        if !contains_markdown(&path) {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .expect("OKF fixture names are UTF-8");
        bundles.push((name, path));
    }
    bundles.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        bundles.len() >= 6,
        "OKF corpus must cover the representative fixture set"
    );
    bundles
}

#[test]
fn okf_corpus_matches_folded_expectations() {
    let dir = corpus_dir();
    for (name, bundle) in okf_bundles() {
        let data = from_okf(&bundle).unwrap_or_else(|err| {
            panic!(
                "OKF fixture {name} must import cleanly from {}: {err}",
                bundle.display()
            )
        });
        let expected = fs::read_to_string(dir.join(format!("{name}.folded.nq")))
            .unwrap_or_else(|err| panic!("missing folded expectation for {name}: {err}"));
        assert_eq!(
            folded_lines(&data),
            sorted_lines(&expected),
            "OKF fixture {name} folded graph drifted from pinned expectation"
        );
    }
}

#[test]
fn okf_corpus_round_trips_through_export() {
    for (name, bundle) in okf_bundles() {
        let out = tmpdir(&name);
        let _ = fs::remove_dir_all(&out);
        let data = from_okf(&bundle).expect("OKF fixture imports");
        let graph = read(&data, true, None);
        let report = to_okf(&graph, &out, &OkfExportOptions::default())
            .unwrap_or_else(|err| panic!("OKF fixture {name} must export cleanly: {err}"));
        assert!(
            report.documents > 0,
            "OKF fixture {name} exported no documents"
        );
        assert_eq!(
            report.unmapped_triples, 0,
            "OKF fixture {name} should be profile-clean"
        );
        assert!(
            out.join(".gts-okf/manifest.json").is_file(),
            "OKF fixture {name} must emit an export manifest"
        );

        let imported = from_okf(&out).expect("exported OKF fixture imports");
        assert_eq!(
            folded_lines(&data),
            folded_lines(&imported),
            "OKF fixture {name} graph changed after from-okf -> to-okf -> from-okf"
        );
        let _ = fs::remove_dir_all(out);
    }
}

#[test]
fn okf_corpus_unmapped_sidecar_is_pinned() {
    let dir = corpus_dir();
    let bundle = dir.join("unmapped-sidecar");
    let data = from_okf(&bundle).expect("sidecar fixture imports");
    let mut graph = read(&data, true, None);
    let subject = graph
        .terms
        .iter()
        .position(|term| {
            term.kind == TermKind::Iri
                && term.value.as_deref() == Some("https://data.example/unmapped/source")
        })
        .expect("sidecar fixture subject exists");
    let predicate = graph.terms.len();
    graph.terms.push(Term {
        kind: TermKind::Iri,
        value: Some("https://example.org/out-of-profile".to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    });
    let object = graph.terms.len();
    graph.terms.push(Term {
        kind: TermKind::Literal,
        value: Some("kept in sidecar".to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    });
    graph.quads.push((subject, predicate, object, None));

    let out = tmpdir("unmapped-sidecar");
    let _ = fs::remove_dir_all(&out);
    let report = to_okf(&graph, &out, &OkfExportOptions::default())
        .expect("sidecar fixture exports with unmapped RDF");
    assert_eq!(report.unmapped_triples, 1);

    let expected = fs::read_to_string(dir.join("unmapped-sidecar.expected-unmapped.nq"))
        .expect("pinned sidecar expectation exists");
    let actual = fs::read_to_string(out.join("_unmapped.nq")).expect("sidecar is emitted");
    assert_eq!(
        sorted_lines(&actual),
        sorted_lines(&expected),
        "OKF unmapped sidecar drifted from pinned expectation"
    );
    let _ = fs::remove_dir_all(out);
}

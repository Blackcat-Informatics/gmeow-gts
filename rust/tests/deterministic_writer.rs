// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::fs;
use std::path::Path;

use ciborium::value::Value;
use ed25519_dalek::SigningKey;
use gmeow_gts::cose::verify_signatures;
use gmeow_gts::model::{Graph, Suppression, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::wire::{iter_items, map_get};
use gmeow_gts::writer::{
    choose_snapshot_transform, snapshot_from_graph, BlobRow, SnapshotOptions, SnapshotSigner,
    DEFAULT_RSYNCABLE_THRESHOLD,
};
use gmeow_gts::writer::{digest_string, Writer};

const CAT: &str = "https://example.org/Cat";
const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

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

fn literal(value: &str, lang: Option<&str>) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_string()),
        datatype: None,
        lang: lang.map(str::to_string),
        direction: None,
        reifier: None,
    }
}

fn blob_meta() -> Value {
    Value::Map(vec![("mt".into(), "text/plain".into())])
}

fn term_target(id: usize) -> Value {
    Value::Map(vec![
        ("kind".into(), "term".into()),
        ("id".into(), Value::from(id as u64)),
    ])
}

fn frame_types(data: &[u8]) -> Vec<String> {
    let (items, torn) = iter_items(data);
    assert_eq!(torn, None);
    items
        .iter()
        .skip(1)
        .map(|(_, item)| match item {
            Value::Map(entries) => match map_get(entries, "t") {
                Some(Value::Text(frame_type)) => frame_type.clone(),
                other => panic!("frame missing text type: {other:?}"),
            },
            other => panic!("frame is not a map: {other:?}"),
        })
        .collect()
}

fn frame_transform_ids(data: &[u8]) -> Vec<Vec<i128>> {
    let (items, torn) = iter_items(data);
    assert_eq!(torn, None);
    items
        .iter()
        .skip(1)
        .map(|(_, item)| match item {
            Value::Map(entries) => match map_get(entries, "x") {
                Some(Value::Array(ids)) => ids
                    .iter()
                    .map(|id| match id {
                        Value::Integer(value) => i128::from(*value),
                        other => panic!("transform id is not an integer: {other:?}"),
                    })
                    .collect(),
                None => Vec::new(),
                other => panic!("transform chain is not an array: {other:?}"),
            },
            other => panic!("frame is not a map: {other:?}"),
        })
        .collect()
}

fn blob_reps(graph: &Graph) -> Vec<String> {
    graph
        .blob_meta
        .iter()
        .filter_map(|(_, meta)| match meta {
            Value::Map(entries) => entries.iter().find_map(|(key, value)| match (key, value) {
                (Value::Text(key), Value::Text(value)) if key == "rep" => Some(value.clone()),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

fn deterministic_graphs() -> (Graph, Graph) {
    let payload = b"deterministic payload";
    let digest = digest_string(payload);

    let mut a = Graph {
        terms: vec![
            iri(LABEL),
            literal("Cat", Some("en")),
            iri(CAT),
            iri("https://example.org/graph"),
            iri("https://example.org/stmt1"),
            iri("https://example.org/confidence"),
            literal("0.9", None),
        ],
        quads: vec![(2, 0, 1, Some(3))],
        reifiers: vec![(4, (2, 0, 1))],
        annotations: vec![(4, 5, 6)],
        suppressions: vec![Suppression {
            targets: vec![term_target(6)],
            reason: Some("example suppression".to_string()),
            by: Some(4),
        }],
        ..Graph::default()
    };
    a.set_meta("generator".to_string(), "deterministic-writer".into());
    a.set_blob(digest.clone(), payload.to_vec());
    a.set_blob_meta(digest.clone(), blob_meta());

    let mut b = Graph {
        terms: vec![
            literal("0.9", None),
            iri("https://example.org/confidence"),
            iri("https://example.org/stmt1"),
            iri("https://example.org/graph"),
            iri(CAT),
            literal("Cat", Some("en")),
            iri(LABEL),
        ],
        quads: vec![(4, 6, 5, Some(3))],
        reifiers: vec![(2, (4, 6, 5))],
        annotations: vec![(2, 1, 0)],
        suppressions: vec![Suppression {
            targets: vec![term_target(0)],
            reason: Some("example suppression".to_string()),
            by: Some(2),
        }],
        ..Graph::default()
    };
    b.set_meta("generator".to_string(), "deterministic-writer".into());
    b.set_blob(digest.clone(), payload.to_vec());
    b.set_blob_meta(digest, blob_meta());

    (a, b)
}

#[test]
fn deterministic_writer_reorders_equivalent_graphs() {
    let (a, b) = deterministic_graphs();
    let first = Writer::deterministic(&a, "dist")
        .expect("graph writes")
        .to_bytes();
    let second = Writer::deterministic(&b, "dist")
        .expect("graph writes")
        .to_bytes();
    assert_eq!(first, second);
}

#[test]
fn deterministic_writer_matches_frozen_vector() {
    let (graph, _) = deterministic_graphs();
    let actual = Writer::deterministic(&graph, "dist")
        .expect("graph writes")
        .to_bytes();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectors");
    let frozen = fs::read(dir.join("29-deterministic-writer.gts")).expect("vector bytes");
    assert_eq!(actual, frozen);
}

#[test]
fn graph_snapshot_payload_folds_like_deterministic_writer() {
    let (graph, _) = deterministic_graphs();
    let snapshot = snapshot_from_graph(
        &graph,
        "dist",
        SnapshotOptions {
            transform: Vec::new(),
            ..SnapshotOptions::default()
        },
    )
    .expect("snapshot graph writes");
    assert_eq!(frame_types(&snapshot), vec!["snapshot"]);

    let deterministic = Writer::deterministic(&graph, "dist")
        .expect("graph writes")
        .to_bytes();
    let snapshot_graph = gmeow_gts::reader::read(&snapshot, true, None);
    let deterministic_graph = gmeow_gts::reader::read(&deterministic, true, None);
    assert!(snapshot_graph.diagnostics.is_empty());
    assert_eq!(to_nquads(&snapshot_graph), to_nquads(&deterministic_graph));
}

#[test]
fn snapshot_from_graph_emits_sorted_content_addressed_blobs_before_snapshot() {
    let (graph, _) = deterministic_graphs();
    let bytes = snapshot_from_graph(
        &graph,
        "dist",
        SnapshotOptions {
            transform: Vec::new(),
            doc_blobs: vec![BlobRow {
                data: b"second".to_vec(),
                media_type: "text/plain".to_string(),
                rep: "z-doc".to_string(),
            }],
            report_blobs: vec![BlobRow {
                data: b"first".to_vec(),
                media_type: "application/json".to_string(),
                rep: "a-report".to_string(),
            }],
            ..SnapshotOptions::default()
        },
    )
    .expect("snapshot graph writes");

    assert_eq!(frame_types(&bytes), vec!["blob", "blob", "snapshot"]);
    let folded = gmeow_gts::reader::read(&bytes, true, None);
    assert!(folded.diagnostics.is_empty());
    assert_eq!(
        blob_reps(&folded),
        vec!["a-report".to_string(), "z-doc".to_string()]
    );
}

#[test]
fn snapshot_transform_threshold_only_rewrites_default_zstd() {
    assert_eq!(
        choose_snapshot_transform(
            &["zstd".to_string()],
            DEFAULT_RSYNCABLE_THRESHOLD,
            DEFAULT_RSYNCABLE_THRESHOLD,
        ),
        vec!["zstd".to_string()]
    );
    assert_eq!(
        choose_snapshot_transform(&["zstd".to_string()], 10, 1),
        vec!["zstd-rsyncable".to_string()]
    );
    assert_eq!(
        choose_snapshot_transform(&["identity".to_string()], 10, 1),
        vec!["identity".to_string()]
    );

    let (graph, _) = deterministic_graphs();
    let bytes = snapshot_from_graph(
        &graph,
        "dist",
        SnapshotOptions {
            transform: vec!["zstd".to_string()],
            rsyncable_threshold: 1,
            doc_blobs: vec![BlobRow {
                data: b"large enough".to_vec(),
                media_type: "text/plain".to_string(),
                rep: "blob".to_string(),
            }],
            ..SnapshotOptions::default()
        },
    )
    .expect("snapshot graph writes");
    assert_eq!(frame_transform_ids(&bytes), vec![vec![3], vec![3]]);
}

#[test]
fn snapshot_from_graph_signs_transport_key_and_snapshot_frame() {
    let (graph, _) = deterministic_graphs();
    let secret = [9u8; 32];
    let kid = "did:example:gts-snapshot";
    let bytes = snapshot_from_graph(
        &graph,
        "dist",
        SnapshotOptions {
            transform: Vec::new(),
            signer: Some(SnapshotSigner {
                secret,
                kid: kid.to_string(),
                public_key_armor:
                    "-----BEGIN PGP PUBLIC KEY BLOCK-----\n...\n-----END PGP PUBLIC KEY BLOCK-----"
                        .to_string(),
            }),
            ..SnapshotOptions::default()
        },
    )
    .expect("snapshot graph writes");

    assert_eq!(frame_types(&bytes), vec!["meta", "snapshot"]);
    let mut folded = gmeow_gts::reader::read(&bytes, true, None);
    let transport =
        gmeow_gts::verify::extract_transport_key(&folded).expect("transport key metadata");
    assert_eq!(transport.kid, kid);
    assert_eq!(folded.signatures.len(), 2);

    let verifying_key = SigningKey::from_bytes(&secret).verifying_key();
    verify_signatures(&mut folded.signatures, |candidate| {
        (candidate == kid).then_some(verifying_key)
    });
    assert!(folded.signatures.iter().all(|sig| sig.status == "valid"));
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "native-store")]

use std::error::Error;

use ciborium::value::Value;
use gmeow_gts::model::{
    BlobEntry, Diagnostic, Graph, OpaqueNode, Signature, StreamableInfo, Suppression, Term,
    TermKind,
};
use gmeow_gts::native_store::{
    graph_into_quads_with_sidecar, graph_to_store, graph_to_store_with_sidecar, store_to_gts_bytes,
    GtsSidecar, NativeStore,
};
use gmeow_gts::rdf::{GraphName, Iri, Literal as RdfLiteral, RdfQuad};
use gmeow_gts::reader::read;
use gmeow_gts::writer::Writer;

fn iri_term(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn sorted_store(store: &NativeStore) -> Vec<String> {
    let mut rows = store.iter().map(ToString::to_string).collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn graph_to_store_preserves_named_graphs_and_sidecar() -> Result<(), Box<dyn Error>> {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        iri_term("https://example.org/s"),
        iri_term("https://example.org/p"),
        iri_term("https://example.org/o"),
        iri_term("https://example.org/g"),
    ]);
    writer.add_quads(&[(0, 1, 2, Some(3))]);
    writer.add_blob(b"payload", Some("text/plain"), None);

    let graph = read(&writer.to_bytes(), true, None);
    let projected = graph_to_store_with_sidecar(graph)?;
    let rows = projected.store.iter().collect::<Vec<_>>();

    assert_eq!(rows.len(), 1);
    match &rows[0].graph_name {
        GraphName::Iri(iri) => assert_eq!(iri.as_str(), "https://example.org/g"),
        other => panic!("expected named graph, got {other:?}"),
    }
    assert_eq!(projected.sidecar.blobs.len(), 1);
    assert_eq!(projected.sidecar.blob_meta.len(), 1);
    assert_eq!(projected.sidecar.segment_heads.len(), 1);
    Ok(())
}

#[test]
fn graph_into_quads_returns_gts_sidecar() -> Result<(), Box<dyn Error>> {
    let mut writer = Writer::new("dist");
    writer.add_terms(&[
        iri_term("https://example.org/s"),
        iri_term("https://example.org/p"),
        iri_term("https://example.org/o"),
    ]);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.add_meta(Value::Map(vec![("source".into(), "synthetic".into())]));

    let graph = read(&writer.to_bytes(), true, None);
    let (quads, sidecar) = graph_into_quads_with_sidecar(graph)?;

    assert_eq!(quads.count(), 1);
    assert_eq!(sidecar.meta.len(), 1);
    assert_eq!(sidecar.segment_profiles, vec!["dist".to_string()]);
    Ok(())
}

#[test]
fn sidecar_clones_all_gts_only_fields() {
    let mut graph = Graph::default();
    graph.set_blob_meta("blake3:abc".to_string(), "blob-meta".into());
    graph.set_blob_entry(
        "blake3:abc".to_string(),
        BlobEntry::bytes(b"payload".to_vec()),
    );
    graph.set_meta("name".to_string(), "fixture".into());
    graph.suppressions.push(Suppression {
        targets: vec!["target".into()],
        reason: Some("hidden".to_string()),
        by: Some(0),
    });
    graph.opaque.push(OpaqueNode {
        id: vec![1, 2, 3],
        frame_type: "future".to_string(),
        reason: "unknown-codec".to_string(),
        sigstat: "unverified".to_string(),
        pub_meta: Some("opaque-meta".into()),
        recipients: Some(vec!["recipient".into()]),
    });
    graph.signatures.push(Signature {
        frame_id: vec![4, 5, 6],
        kid: Some("kid".to_string()),
        status: "valid".to_string(),
        cose: Some(vec![7, 8, 9]),
    });
    graph.diagnostics.push(Diagnostic {
        code: "diagnostic".to_string(),
        detail: "detail".to_string(),
        frame_index: Some(3),
    });
    graph.segment_heads.push(vec![10, 11]);
    graph.segment_profiles.push("dist".to_string());
    graph
        .segment_meta
        .push(vec![("segment".to_string(), "meta".into())]);
    graph.segment_streamable.push(StreamableInfo {
        claimed: true,
        covered: 2,
        tail: 1,
        head: Some(vec![12, 13]),
    });

    let sidecar = GtsSidecar::from_graph(&graph);

    assert_eq!(sidecar.blob_meta.len(), 1);
    assert_eq!(sidecar.blobs.len(), 1);
    assert_eq!(sidecar.meta.len(), 1);
    assert_eq!(sidecar.suppressions[0].reason.as_deref(), Some("hidden"));
    assert_eq!(sidecar.opaque[0].frame_type, "future");
    assert_eq!(sidecar.signatures[0].status, "valid");
    assert_eq!(sidecar.diagnostics[0].code, "diagnostic");
    assert_eq!(sidecar.segment_heads, vec![vec![10, 11]]);
    assert_eq!(sidecar.segment_profiles, vec!["dist".to_string()]);
    assert_eq!(sidecar.segment_meta.len(), 1);
    assert!(sidecar.segment_streamable[0].claimed);
    assert_eq!(sidecar.segment_streamable[0].head, Some(vec![12, 13]));
}

#[test]
fn store_to_writer_roundtrips_native_quads() -> Result<(), Box<dyn Error>> {
    let mut store = NativeStore::new();
    let graph_name = Iri::new("https://example.org/g")?;
    let subject = Iri::new("https://example.org/s")?;
    let predicate = Iri::new("https://example.org/p")?;
    let object = RdfLiteral::new_simple_literal("object");
    store.insert(RdfQuad::new(
        subject.clone(),
        predicate.clone(),
        object,
        GraphName::from(graph_name.clone()),
    ));

    let bytes = Writer::from_store(&store, "dist")?.to_bytes();
    assert_eq!(bytes, store_to_gts_bytes(&store, "dist")?);
    let graph = read(&bytes, true, None);
    assert!(graph.diagnostics.is_empty());
    let roundtripped = graph_to_store(&graph)?;

    assert_eq!(sorted_store(&roundtripped), sorted_store(&store));
    Ok(())
}

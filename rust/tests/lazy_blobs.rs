// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use ciborium::value::Value;
use gmeow_gts::codec::Codec;
use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::reader::read;
use gmeow_gts::wire::{canonical, content_id, digest_str};
use gmeow_gts::writer::Writer;

const BLOB_MT: &str = "text/plain";
type TestQuad = (usize, usize, usize, Option<usize>);

fn iv(n: i64) -> Value {
    Value::Integer(ciborium::value::Integer::from(n))
}

fn term_triple() -> ([Term; 3], [TestQuad; 1]) {
    (
        [
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/s".into()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/p".into()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
            Term {
                kind: TermKind::Iri,
                value: Some("https://example.org/o".into()),
                datatype: None,
                lang: None,
                direction: None,
                reifier: None,
                triple: None,
            },
        ],
        [(0, 1, 2, None)],
    )
}

fn zstd_bytes(data: &[u8]) -> Vec<u8> {
    structured_zstd::encoding::compress_to_vec(
        data,
        structured_zstd::encoding::CompressionLevel::Uncompressed,
    )
}

fn blob_pub(digest: Option<&str>) -> Vec<(Value, Value)> {
    let mut pub_meta = vec![("mt".into(), BLOB_MT.into())];
    if let Some(digest) = digest {
        pub_meta.push(("digest".into(), digest.into()));
    }
    pub_meta
}

fn blob_frame(
    prev: &[u8],
    raw: Option<Vec<u8>>,
    codecs: &[i64],
    pub_meta: Vec<(Value, Value)>,
) -> Vec<u8> {
    let mut frame: Vec<(Value, Value)> = vec![
        ("t".into(), "blob".into()),
        ("prev".into(), Value::Bytes(prev.to_vec())),
    ];
    if let Some(raw) = raw {
        frame.push(("d".into(), Value::Bytes(raw)));
    }
    if !codecs.is_empty() {
        frame.push((
            "x".into(),
            Value::Array(codecs.iter().map(|codec| iv(*codec)).collect()),
        ));
    }
    if !pub_meta.is_empty() {
        frame.push(("pub".into(), Value::Map(pub_meta)));
    }
    frame.sort_by_key(|(key, _)| canonical(key));
    let id = content_id(&frame);
    frame.push(("id".into(), Value::Bytes(id)));
    frame.sort_by_key(|(key, _)| canonical(key));
    canonical(&Value::Map(frame))
}

fn zstd_blob_file(blob: &[u8]) -> Vec<u8> {
    let writer = Writer::new("generic");
    let digest = digest_str(blob);
    let mut out = writer.to_bytes();
    out.extend(blob_frame(
        writer.head(),
        Some(zstd_bytes(blob)),
        &[2],
        blob_pub(Some(&digest)),
    ));
    out
}

#[test]
fn blob_deferred_until_access() {
    let (terms, quads) = term_triple();
    let blob = b"hello world ".repeat(1024);
    let digest = digest_str(&blob);

    let mut writer = Writer::new("generic");
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    let mut data = writer.to_bytes();
    data.extend(blob_frame(
        writer.head(),
        Some(zstd_bytes(&blob)),
        &[2],
        blob_pub(Some(&digest)),
    ));

    let graph = read(&data, true, None);
    assert_eq!(graph.terms.len(), 3);
    assert_eq!(graph.quads.len(), 1);
    assert!(graph.diagnostics.is_empty());
    assert!(graph
        .blob_entry(&digest)
        .is_some_and(|entry| entry.is_lazy()));
}

/// A file with a term and a quad frame, then one zstd blob frame whose raw bytes are `payload`.
fn file_with_zstd_payload(payload: Vec<u8>, declared: &[u8]) -> Vec<u8> {
    let (terms, quads) = term_triple();
    let mut writer = Writer::new("generic");
    writer.add_terms(&terms);
    writer.add_quads(&quads);
    let mut data = writer.to_bytes();
    data.extend(blob_frame(
        writer.head(),
        Some(payload),
        &[2],
        blob_pub(Some(&digest_str(declared))),
    ));
    data
}

#[test]
fn blob_payload_that_is_not_whole_zstd_frames_is_one_opaque_node() {
    let content = b"the bytes it should have held ".repeat(30);
    let whole = zstd_bytes(&content);
    let cases = [
        ("not zstd", b"not zstd frame data".to_vec()),
        ("truncated", whole[..whole.len() / 2].to_vec()),
        ("missing last byte", whole[..whole.len() - 1].to_vec()),
    ];
    for (name, payload) in cases {
        let graph = read(&file_with_zstd_payload(payload, &content), true, None);
        let reasons: Vec<&str> = graph.opaque.iter().map(|o| o.reason.as_str()).collect();
        assert_eq!(reasons, ["damaged"], "{name}");
        let codes: Vec<&str> = graph.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, ["DamagedFrame"], "{name}");
        assert!(graph.blobs.is_empty(), "{name}");
        assert!(graph.blob_meta.is_empty(), "{name}");
        assert_eq!(graph.quads.len(), 1, "{name}: earlier frames still fold");
    }
}

#[test]
fn blob_payload_of_concatenated_zstd_frames_decodes_to_the_end() {
    let units: [&[u8]; 3] = [b"alpha ", b"beta ", b"gamma"];
    let content = units.concat();
    let payload: Vec<u8> = units.iter().flat_map(|unit| zstd_bytes(unit)).collect();
    let digest = digest_str(&content);
    let mut graph = read(&file_with_zstd_payload(payload, &content), true, None);
    assert!(graph.opaque.is_empty());
    assert_eq!(graph.blob_bytes(&digest).unwrap(), Some(content.as_slice()));
}

#[test]
fn blob_access_decompresses_and_caches() {
    let blob = b"hello world ".repeat(1024);
    let digest = digest_str(&blob);
    let mut graph = read(&zstd_blob_file(&blob), true, None);

    assert!(graph
        .blob_entry(&digest)
        .is_some_and(|entry| entry.is_lazy()));
    assert_eq!(graph.blob_bytes(&digest).unwrap(), Some(blob.as_slice()));
    assert!(graph
        .blob_entry(&digest)
        .is_some_and(|entry| !entry.is_lazy()));
    assert_eq!(graph.blob_bytes(&digest).unwrap(), Some(blob.as_slice()));
}

#[test]
fn lazy_blob_identity_codec_is_direct() {
    let blob = b"plain bytes";
    let mut writer = Writer::new("generic");
    writer.add_blob(blob, Some(BLOB_MT), None);

    let graph = read(&writer.to_bytes(), true, None);
    let digest = digest_str(blob);
    let entry = graph.blob_entry(&digest).expect("blob entry");
    assert!(!entry.is_lazy());
    assert_eq!(entry.cached_bytes(), Some(blob.as_slice()));
}

#[test]
fn lazy_blob_no_pub_digest_falls_back_to_eager_decode() {
    let blob = b"legacy blob";
    let writer = Writer::new("generic");
    let mut data = writer.to_bytes();
    data.extend(blob_frame(
        writer.head(),
        Some(zstd_bytes(blob)),
        &[2],
        blob_pub(None),
    ));

    let digest = digest_str(blob);
    let graph = read(&data, true, None);
    let entry = graph.blob_entry(&digest).expect("eager decoded blob");
    assert!(!entry.is_lazy());
    assert_eq!(entry.cached_bytes(), Some(blob.as_slice()));
    assert!(graph.blob_meta.iter().any(|(d, _)| d == &digest));
}

#[test]
fn lazy_blob_unknown_codec_degrades() {
    let writer = Writer::new("generic");
    let mut data = writer.to_bytes();
    data.extend(blob_frame(
        writer.head(),
        Some(b"x".to_vec()),
        &[99],
        blob_pub(None),
    ));

    let graph = read(&data, true, None);
    assert!(graph
        .opaque
        .iter()
        .any(|node| node.reason == "unknown-codec"));
    assert!(graph
        .diagnostics
        .iter()
        .any(|diag| diag.code == "UnknownCodec"));
}

#[test]
fn lazy_blob_meta_is_eager() {
    let blob = b"metadata test";
    let digest = digest_str(blob);
    let graph = read(&zstd_blob_file(blob), true, None);

    assert!(graph
        .blob_entry(&digest)
        .is_some_and(|entry| entry.is_lazy()));
    let meta = graph
        .blob_meta
        .iter()
        .find(|(d, _)| d == &digest)
        .map(|(_, meta)| meta)
        .expect("metadata");
    let Value::Map(entries) = meta else {
        panic!("blob metadata is a map");
    };
    assert!(entries.iter().any(|(key, value)| {
        matches!((key, value), (Value::Text(k), Value::Text(v)) if k == "mt" && v == BLOB_MT)
    }));
}

#[test]
fn lazy_blob_multi_segment_union_does_not_decode() {
    let blob_a = b"segment a ".repeat(128);
    let blob_b = b"segment b ".repeat(128);
    let mut data = zstd_blob_file(&blob_a);
    data.extend(zstd_blob_file(&blob_b));

    let graph = read(&data, true, None);
    assert_eq!(graph.blobs.len(), 2);
    assert!(graph.blobs.iter().all(|(_, entry)| entry.is_lazy()));
}

#[test]
fn lazy_blob_external_records_layout_iou() {
    let digest = format!("blake3:{}", "00".repeat(32));
    let writer = Writer::new("generic");
    let mut data = writer.to_bytes();
    data.extend(blob_frame(
        writer.head(),
        None,
        &[],
        blob_pub(Some(&digest)),
    ));

    let graph = read(&data, true, None);
    assert!(graph.blobs.is_empty());
    assert!(graph.blob_meta.iter().any(|(d, _)| d == &digest));
    assert!(graph.diagnostics.is_empty());
}

#[test]
fn blob_contract_preserves_inline_bytes_and_leaves_external_bytes_out_of_band() {
    let inline = b"inline payload bytes";
    let inline_digest = digest_str(inline);
    let external_digest = format!("blake3:{}", "11".repeat(32));

    let mut writer = Writer::new("generic");
    writer.add_blob(inline, Some(BLOB_MT), Some("inline"));
    let mut data = writer.to_bytes();
    data.extend(blob_frame(
        writer.head(),
        None,
        &[],
        blob_pub(Some(&external_digest)),
    ));

    let mut graph = read(&data, true, None);
    assert_eq!(
        graph.blob_bytes(&inline_digest).unwrap(),
        Some(inline.as_slice())
    );
    assert_eq!(graph.blob_bytes(&external_digest).unwrap(), None);
    assert!(graph.blob_meta.iter().any(|(d, _)| d == &inline_digest));
    assert!(graph.blob_meta.iter().any(|(d, _)| d == &external_digest));
}

#[test]
fn lazy_blob_lookup_and_iteration_helpers() {
    let mut graph = Graph::default();
    graph.set_blob("a".into(), b"A".to_vec());
    graph.set_lazy_blob(
        "b".into(),
        b"B".to_vec(),
        vec![Codec {
            name: "identity".into(),
            cls: "encode".into(),
        }],
    );

    assert_eq!(graph.blobs.len(), 2);
    assert_eq!(graph.blob_bytes("a").unwrap(), Some(&b"A"[..]));
    assert_eq!(graph.blob_bytes_cloned("b").unwrap(), Some(b"B".to_vec()));
    assert!(graph.blob_entry("b").is_some_and(|entry| !entry.is_lazy()));
    assert_eq!(
        graph.decoded_blobs().unwrap(),
        vec![("a".into(), b"A".to_vec()), ("b".into(), b"B".to_vec())]
    );
}

#[test]
fn lazy_blob_decode_failure_raises_on_access() {
    let mut graph = Graph::default();
    graph.set_lazy_blob(
        "a".into(),
        b"not zstd".to_vec(),
        vec![Codec {
            name: "zstd".into(),
            cls: "compress".into(),
        }],
    );

    assert!(graph.blob_entry("a").is_some_and(|entry| entry.is_lazy()));
    assert!(graph.blob_bytes("a").is_err());
    assert!(graph.blob_entry("a").is_some_and(|entry| entry.is_lazy()));
}

#[test]
fn damage_inside_a_block_with_valid_structure_fails_cleanly_on_first_read() {
    // One compressed block of three bytes that are not a valid block: the frame walk
    // passes, so the blob folds lazily, and the first read reports the failure.
    let content = b"declared";
    let payload = vec![
        0x28,
        0xb5,
        0x2f,
        0xfd,
        0x00,
        0x00,
        0x05 | (3 << 3),
        0x00,
        0x00,
        0xff,
        0xff,
        0xff,
    ];
    let digest = digest_str(content);
    let mut graph = read(&file_with_zstd_payload(payload, content), true, None);
    assert!(graph.opaque.is_empty());
    assert!(graph.diagnostics.is_empty());
    assert!(graph
        .blob_entry(&digest)
        .is_some_and(|entry| entry.is_lazy()));
    assert!(graph.blob_bytes(&digest).is_err());
}

#[test]
fn zstd_frame_declaring_a_window_the_decoder_refuses_is_one_opaque_node() {
    let content = b"declared window";
    let frame = |window_log: u32| {
        let block = 1 | ((content.len() as u32) << 3);
        let mut out = vec![0x28, 0xb5, 0x2f, 0xfd, 0x00, ((window_log - 10) << 3) as u8];
        out.extend(&block.to_le_bytes()[..3]);
        out.extend(content);
        out
    };
    let ok = read(&file_with_zstd_payload(frame(31), content), true, None);
    assert!(ok.opaque.is_empty());
    for window_log in [32, 35, 41] {
        let graph = read(
            &file_with_zstd_payload(frame(window_log), content),
            true,
            None,
        );
        let reasons: Vec<&str> = graph.opaque.iter().map(|o| o.reason.as_str()).collect();
        assert_eq!(reasons, ["damaged"], "window 2**{window_log}");
    }
}

#[test]
fn zstd_frame_naming_a_dictionary_is_one_opaque_node() {
    let mut payload = vec![0x28, 0xb5, 0x2f, 0xfd, 0x01, 0x00, 0x05];
    payload.extend([1 | (5 << 3), 0, 0]);
    payload.extend(b"hello");
    let graph = read(&file_with_zstd_payload(payload, b"hello"), true, None);
    let reasons: Vec<&str> = graph.opaque.iter().map(|o| o.reason.as_str()).collect();
    assert_eq!(reasons, ["damaged"]);
}

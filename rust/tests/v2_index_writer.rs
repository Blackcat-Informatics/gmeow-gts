// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, io::Read, path::Path};

use ciborium::Value;
use gmeow_gts::v2::{
    decode_header,
    frame::decode_frame,
    reader::{read_wire, PayloadState, ReadCapacity},
    writer::{Blob, IndexFold, IndexWriteError, SegmentWriter},
    Header, MalformedReason,
};
use structured_zstd::decoding::StreamingDecoder;

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn count(value: &Value) -> u64 {
    u64::try_from(value.as_integer().unwrap()).unwrap()
}

fn fold_from_published_index(payload: &[u8]) -> IndexFold {
    let record: Value = ciborium::from_reader(payload).expect("decode index");
    let fields = record.as_array().unwrap();
    let stats = fields[2].as_array().unwrap();
    let content_digest = fields[4]
        .as_array()
        .unwrap()
        .get(1)
        .map(|digest| digest.as_bytes().unwrap().as_slice().try_into().unwrap());
    IndexFold {
        lines: count(&stats[0]),
        records: count(&stats[1]),
        blank_nodes: count(&stats[2]),
        content_digest,
    }
}

#[test]
fn indexes_match_published_rows_and_pack_directories() {
    for name in ["P5/index-intact.bin", "P4/canon-regions-shared.bin"] {
        let reference = vector(name);
        let header = decode_header(&reference, 0).expect("published header");
        let read = read_wire(&reference, ReadCapacity::default());
        let mut writer = SegmentWriter::new(header.header).expect("write header");
        let mut observed_index = false;

        for frame in &read.segments[0].frames {
            let expected = frame.envelope.as_ref().expect("reference envelope");
            if expected.transparent {
                continue;
            }
            let PayloadState::Verified(decoded) = &frame.payload.state else {
                panic!("unverified payload in {name}");
            };
            let identifier = match expected.frame_type {
                14 => writer.commit().unwrap(),
                15 => {
                    let offset = writer.as_bytes().len();
                    let identifier = writer
                        .append_index(fold_from_published_index(decoded))
                        .unwrap_or_else(|error| panic!("{name}: {error}"));
                    let actual = decode_frame(writer.as_bytes(), offset, 1).unwrap();
                    assert_eq!(actual.content, expected.content, "{name}");
                    assert_eq!(actual.stored_payload, decoded.as_ref(), "{name}");
                    observed_index = true;
                    identifier
                }
                other => writer.append(other, expected.content, decoded).unwrap(),
            };
            assert_eq!(&identifier, expected.identifier, "{name}");
        }
        assert!(observed_index, "{name} has no index");
    }
}

#[test]
fn writes_the_complete_canonical_empty_blob_vector() {
    let expected = vector("P4/canon-empty-blob.bin");
    let read = read_wire(&expected, ReadCapacity::default());
    let index = read.segments[0]
        .frames
        .iter()
        .find(|frame| frame.frame_type == Some(15))
        .unwrap();
    let PayloadState::Verified(decoded) = &index.payload.state else {
        panic!("unverified published index");
    };
    let mut writer = SegmentWriter::new(Header {
        canonical: Some(1),
        ..Header::default()
    })
    .unwrap();
    writer
        .append_attachment_regions(
            [],
            [Blob {
                digest: blake3::derive_key("gts2 unit", b""),
                length: 0,
                bytes: Some(b""),
                media_types: &[],
            }],
        )
        .unwrap();
    writer.commit().unwrap();
    writer
        .append_index(fold_from_published_index(decoded))
        .unwrap();
    assert_eq!(writer.finish_indexed().unwrap(), expected);
}

#[test]
fn counts_written_predicates_when_the_fold_has_no_digest() {
    let reference = vector("P5/index-digest-absent.absent.bin");
    let header = decode_header(&reference, 0).unwrap();
    let terms = decode_frame(&reference, header.next_offset, 1).unwrap();
    let quad = decode_frame(&reference, terms.next_offset, 1).unwrap();
    let mut writer = SegmentWriter::new(header.header).unwrap();
    for frame in [terms, quad] {
        writer
            .append(frame.frame_type, frame.content, frame.stored_payload)
            .unwrap();
    }
    let index_at = writer.as_bytes().len();
    writer.append_index(IndexFold::default()).unwrap();
    let index = decode_frame(writer.as_bytes(), index_at, 1).unwrap();
    let payload: Value = ciborium::from_reader(index.stored_payload).unwrap();
    let fields = payload.as_array().unwrap();
    assert_eq!(fields[4], Value::Array(vec![Value::from(1)]));
    assert_eq!(
        fields[2].as_array().unwrap()[3],
        Value::Array(vec![Value::Array(vec![Value::from(0), Value::from(1)])]),
    );
}

#[test]
fn cannot_index_a_row_payload_with_inconsistent_counts() {
    let mut writer = SegmentWriter::new(Header::default()).unwrap();
    // A one-row quad claims a graph but provides an empty predicate column.
    let content = [0x82, 1, 1];
    let rows = [0x84, 0x81, 0x82, 0x81, 1, 1, 0x80, 0x81, 1, 0x81, 0];
    let head = writer.append(2, &content, &rows).unwrap();
    let len = writer.as_bytes().len();
    assert!(matches!(
        writer.append_index(IndexFold::default()),
        Err(IndexWriteError::FrameFields { position: 0, error })
            if error.reason == MalformedReason::InconsistentCounts
    ));
    assert_eq!(writer.as_bytes().len(), len);
    assert_eq!(*writer.head(), head);
}

#[test]
fn transparent_frames_are_absent_from_index_claims() {
    let compressed = vector("P5/parity-placement.exact.bin.zst");
    let mut reference = Vec::new();
    StreamingDecoder::new(compressed.as_slice())
        .read_to_end(&mut reference)
        .unwrap();
    let header = decode_header(&reference, 0).unwrap();
    let chunk = decode_frame(&reference, header.next_offset, 1).unwrap();
    let parity = decode_frame(&reference, chunk.next_offset, 1).unwrap();
    let mut protected = SegmentWriter::new(header.header).unwrap();
    protected
        .append(8, chunk.content, chunk.stored_payload)
        .unwrap();
    protected
        .append(16, parity.content, parity.stored_payload)
        .unwrap();
    assert_eq!(protected.as_bytes(), reference);

    let mut unprotected = SegmentWriter::new(header.header).unwrap();
    unprotected
        .append(8, chunk.content, chunk.stored_payload)
        .unwrap();
    let fold = IndexFold::default();
    assert_eq!(
        protected.append_index(fold).unwrap(),
        unprotected.append_index(fold).unwrap()
    );
}

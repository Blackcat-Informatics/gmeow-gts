// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, io::Read, path::Path};

use ciborium::Value;
use gmeow_gts::{
    v2::{
        decode_header,
        frame::{decode_frame, Storage},
        reader::{read_wire, ItemKind, ReadCapacity},
        writer::{SegmentFinishError, SegmentWriter},
        Header,
    },
    wire,
};
use structured_zstd::decoding::StreamingDecoder;

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2")
        .join(name);
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    if name.ends_with(".zst") {
        let mut decoded = Vec::new();
        StreamingDecoder::new(bytes.as_slice())
            .read_to_end(&mut decoded)
            .expect("decompress fixture");
        decoded
    } else {
        bytes
    }
}

fn ordinary_prefix(reference: &[u8]) -> SegmentWriter {
    let header = decode_header(reference, 0).expect("published header");
    let mut writer = SegmentWriter::new(header.header).expect("write header");
    let mut offset = header.next_offset;
    while reference[offset..].starts_with(&[0x8b, 0x02]) {
        let frame = decode_frame(reference, offset, 1).expect("published frame");
        assert!(matches!(frame.storage, Storage::Plain));
        if !frame.transparent {
            let identifier = if frame.frame_type == 14 {
                writer.commit().expect("commit the reconstructed prefix")
            } else {
                writer
                    .append(frame.frame_type, frame.content, frame.stored_payload)
                    .expect("write ordinary frame")
            };
            assert_eq!(&identifier, frame.identifier);
        }
        offset = frame.next_offset;
    }
    writer
}

#[test]
fn closing_a_buffered_indexed_segment_reproduces_the_corpus() {
    for name in [
        "P4/canon-empty-blob.bin",
        "P5/parity-span-4096.bin",
        "P5/parity-span-65536.bin",
        "P5/parity-span-262143.bin",
        "P5/parity-span-262144.bin",
    ] {
        let reference = vector(name);
        let writer = ordinary_prefix(&reference);
        let written = writer.finish_indexed().expect("finish indexed segment");
        assert!(written == reference, "stored bytes differ from {name}");
    }
}

fn add_index_for_one_chunk(writer: &mut SegmentWriter) {
    let fixture = vector("P5/parity-span-4096.bin");
    let opener = decode_header(&fixture, 0).expect("fixture header");
    let chunk = decode_frame(&fixture, opener.next_offset, 1).expect("fixture chunk");
    let index = decode_frame(&fixture, chunk.next_offset, 1).expect("fixture index");
    let mut payload: Value =
        ciborium::from_reader(index.stored_payload).expect("decode fixture index payload");

    // A chunk frame declares no blob, so the folded index fields are the same
    // for this prefix. The head and range root are specific to its bytes.
    let h0 = decode_header(writer.as_bytes(), 0)
        .unwrap()
        .identifier
        .unwrap();
    let hash = |context, record| {
        Value::Bytes(blake3::derive_key(context, &wire::encode(&record)).to_vec())
    };
    let leaf = |id: &[u8; 32]| {
        hash(
            "gts2 range leaf",
            Value::Array(vec![Value::Bytes(id.to_vec())]),
        )
    };
    let pair = hash(
        "gts2 range node",
        Value::Array(vec![leaf(&h0), leaf(writer.head())]),
    );
    payload.as_array_mut().unwrap()[3] = hash(
        "gts2 range root",
        Value::Array(vec![Value::from(2), Value::Array(vec![pair])]),
    );
    let content = wire::encode(&Value::Array(vec![
        Value::from(1),
        Value::Bytes(writer.head().to_vec()),
    ]));
    writer
        .append(15, &content, &wire::encode(&payload))
        .expect("write the closing index");
}

#[test]
fn reached_full_spans_follow_the_frame_that_crosses_them() {
    for name in [
        "P5/parity-placement.exact.bin.zst",
        "P5/parity-placement.crossing.bin.zst",
    ] {
        let reference = vector(name);
        let mut writer = ordinary_prefix(&reference);
        add_index_for_one_chunk(&mut writer);
        let closed = writer.finish_indexed().expect("finish indexed segment");

        assert!(
            closed.starts_with(&reference),
            "published full spans differ in {name}"
        );
        let read = read_wire(&closed, ReadCapacity::default());
        let segment = &read.segments[0];
        let index = segment
            .frames
            .iter()
            .find(|f| f.frame_type == Some(15))
            .unwrap();
        assert_eq!(segment.items[index.position].offset, reference.len());
        assert_eq!(segment.frames.last().unwrap().frame_type, Some(16));
        assert_eq!(segment.items.last().unwrap().kind, ItemKind::Locator);
        assert!(read.stop.is_none());
    }
}

#[test]
fn finishing_requires_a_closing_index_and_refuses_existing_parity() {
    let header = Header::default();
    let mut writer = SegmentWriter::new(header).expect("header");
    writer.commit().expect("unindexed commit");
    assert_eq!(
        writer.finish_indexed(),
        Err(SegmentFinishError::MissingClosingIndex)
    );

    let reference = vector("P5/parity-span-4096.bin");
    let read = read_wire(&reference, ReadCapacity::default());
    let parity = read.segments[0]
        .frames
        .last()
        .unwrap()
        .envelope
        .as_ref()
        .unwrap();
    let mut writer = ordinary_prefix(&reference);
    let offset = writer.as_bytes().len();
    writer
        .append(16, parity.content, parity.stored_payload)
        .expect("add the existing parity");
    assert_eq!(
        writer.finish_indexed(),
        Err(SegmentFinishError::ExistingParity { offset }),
    );
}

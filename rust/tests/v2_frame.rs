// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The v2 frame writer and reader: identifiers, payload boundaries, stored checks and registries.

use std::{fs, path::Path};

use ciborium::Value;
use gmeow_gts::{
    v2::{
        decode_header,
        frame::{decode_frame, encode_plain_frame, Storage},
        writer::{SegmentWriteError, SegmentWriter},
        MalformedReason,
    },
    wire,
};

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn frame_identifiers_and_plain_payloads_match_the_published_chain() {
    let data = vector("P5/commit-valid.bin");
    let header = decode_header(&data, 0).expect("header");
    let mut predecessor = header.identifier.expect("header ID");
    let mut offset = header.next_offset;
    let mut count = 0;
    while offset < data.len() {
        let frame = decode_frame(&data, offset, 1).expect("frame");
        assert_eq!(&frame.computed_identifier(&predecessor), frame.identifier);
        assert!(frame.check_decoded_payload(frame.stored_payload));
        assert_eq!(frame.check_storage(), None);
        assert_eq!(
            encode_plain_frame(
                &predecessor,
                frame.frame_type,
                frame.content,
                frame.stored_payload
            )
            .expect("re-encode published frame"),
            data[offset..frame.next_offset],
        );
        if !frame.transparent {
            predecessor = *frame.identifier;
        }
        offset = frame.next_offset;
        count += 1;
    }
    assert!(count >= 2);
}

#[test]
fn segment_writer_reproduces_commit_prefixes_in_the_corpus() {
    for name in [
        "P5/commit-valid.bin",
        "P5/range-transparent.no-parity.bin",
        "P5/range-transparent.with-parity.bin",
        "P3/state-prefix-fold.bin",
    ] {
        let data = vector(name);
        let mut offset = 0;
        while offset < data.len() {
            let start = offset;
            let header = decode_header(&data, offset).expect("header");
            let mut writer = SegmentWriter::new(header.header).expect("write header");
            offset = header.next_offset;
            assert_eq!(
                writer.head(),
                &header.identifier.expect("header identifier")
            );
            assert_eq!(
                writer.append(14, &[], &[]),
                Err(SegmentWriteError::CommitRequired)
            );
            assert!(matches!(
                writer.append(8, &[0x80], b"data"),
                Err(SegmentWriteError::Frame(_))
            ));

            while data[offset..].starts_with(&[0x8b, 0x02]) {
                let frame = decode_frame(&data, offset, 1).expect("frame");
                let previous_head = *writer.head();
                let identifier = if frame.frame_type == 14 {
                    writer.commit().expect("write computed commit")
                } else {
                    writer
                        .append(frame.frame_type, frame.content, frame.stored_payload)
                        .expect("write frame")
                };
                assert_eq!(&identifier, frame.identifier);
                if frame.transparent {
                    assert_eq!(writer.head(), &previous_head);
                }
                offset = frame.next_offset;
            }
            assert_eq!(writer.as_bytes(), &data[start..offset], "{name}");
        }
    }
}

#[test]
fn plain_writer_preserves_payload_length_boundaries() {
    let predecessor = [0; 32];
    for length in [23, 24, 255, 256] {
        let payload = vec![0xaf; length];
        let content = wire::encode(&Value::Array(vec![Value::Bytes(
            blake3::derive_key("gts2 unit", &payload).to_vec(),
        )]));
        let encoded = encode_plain_frame(&predecessor, 8, &content, &payload).expect("chunk");
        let frame = decode_frame(&encoded, 0, 1).expect("decode written frame");
        assert_eq!(frame.stored_payload, payload);
        assert_eq!(&frame.computed_identifier(&predecessor), frame.identifier);
        assert!(frame.check_decoded_payload(&payload));
    }
    let content = wire::encode(&Value::Array(vec![Value::Bytes(
        blake3::derive_key("gts2 unit", &[]).to_vec(),
    )]));
    assert_eq!(
        encode_plain_frame(&predecessor, 8, &content, &[])
            .expect_err("empty chunk")
            .reason,
        MalformedReason::OutOfRange,
    );
}

#[test]
fn plain_writer_requires_one_deterministic_content_record() {
    let predecessor = [0; 32];
    for (content, reason) in [
        (&[0x81, 0, 0][..], MalformedReason::WrongLength),
        (&[0x81, 0x18, 0][..], MalformedReason::NotDeterministic),
        (&[0x81, 0x81, 0][..], MalformedReason::WrongType),
    ] {
        assert_eq!(
            encode_plain_frame(&predecessor, 7, content, &[])
                .expect_err("invalid content")
                .reason,
            reason
        );
    }
}

#[test]
fn stored_checks_judge_compressed_bytes_independently() {
    for (name, matches) in [
        ("P1/stored-matches.bin", true),
        ("P1/stored-mismatch.bin", false),
    ] {
        let data = vector(name);
        let header = decode_header(&data, 0).expect("header");
        let frame = decode_frame(&data, header.next_offset, 1).expect("compressed frame");
        assert!(matches!(
            frame.storage,
            Storage::Compressed { codec: 1, .. }
        ));
        assert_eq!(frame.check_storage(), Some(matches));
    }

    let data = vector("P1/payload-unknown-codec.bin");
    let header = decode_header(&data, 0).expect("later header");
    let frame = decode_frame(&data, header.next_offset, header.header.registry_version)
        .expect("compressed form with later codec");
    assert!(matches!(
        frame.storage,
        Storage::Compressed { codec: 2, .. }
    ));
    assert_eq!(frame.check_storage(), Some(true));
}

#[test]
fn later_registry_keeps_unknown_content_and_storage() {
    let data = vector("P1/frame-storage-tag-newer.bin");
    let header = decode_header(&data, 0).expect("later header");
    let frame = decode_frame(&data, header.next_offset, header.header.registry_version)
        .expect("frame with later storage");
    assert!(matches!(frame.storage, Storage::Unknown { tag: 3, .. }));
    assert_eq!(
        &frame.computed_identifier(&header.identifier.expect("header ID")),
        frame.identifier
    );
    assert_eq!(
        decode_frame(&data, header.next_offset, 1)
            .expect_err("unknown storage under registry 1")
            .reason,
        MalformedReason::UnregisteredValue,
    );

    let data = vector("P1/frame-unknown-noncritical.bin");
    let header = decode_header(&data, 0).expect("later header");
    let regular = decode_frame(&data, header.next_offset, 2).expect("first frame");
    let future = decode_frame(&data, regular.next_offset, 2).expect("later frame type");
    assert_eq!(future.frame_type, 99);
    assert!(!future.critical);
    assert_eq!(
        &future.computed_identifier(regular.identifier),
        future.identifier
    );
}

#[test]
fn report_the_first_envelope_violation() {
    for (name, reason) in [
        (
            "P1/frame-flags-order.bin",
            MalformedReason::ForbiddenCombination,
        ),
        (
            "P1/frame-signatures-unordered.bin",
            MalformedReason::UnorderedOrRepeated,
        ),
        (
            "P1/frame-unknown-content-bad-utf8.bin",
            MalformedReason::WrongType,
        ),
        ("P1/stored-check-short.bin", MalformedReason::WrongLength),
        (
            "P1/frame-content-wrong-type.bin",
            MalformedReason::WrongType,
        ),
        ("P4/chunk-empty.bin", MalformedReason::OutOfRange),
    ] {
        let data = vector(name);
        let header = decode_header(&data, 0).expect("fixture header");
        let error = decode_frame(&data, header.next_offset, header.header.registry_version)
            .expect_err(name);
        assert_eq!(error.reason, reason, "{name}");
        assert!(error.offset >= header.next_offset && error.offset < data.len());
    }
}

#[test]
fn truncated_frame_is_bounded_by_the_input() {
    let data = vector("P1/frame-overhead-compressed.bin");
    let header = decode_header(&data, 0).expect("header");
    let start = header.next_offset;
    let frame = decode_frame(&data, start, 1).expect("complete frame");
    for end in start..frame.next_offset {
        assert!(
            decode_frame(&data[..end], start, 1).is_err(),
            "prefix end {end}"
        );
    }
}

#[test]
fn unknown_content_judges_the_head_before_the_type() {
    let prefix = [0x8b, 2, 0x18, 99, 0xf4, 0xf4];
    for (head, reason) in [
        (0xa0, MalformedReason::WrongType),
        (0xbc, MalformedReason::WrongLength),
        (0xd8, MalformedReason::WrongLength),
        (0xdf, MalformedReason::WrongLength),
    ] {
        let mut frame = prefix.to_vec();
        frame.push(head);
        assert_eq!(
            decode_frame(&frame, 0, 2)
                .expect_err("bad content head")
                .reason,
            reason
        );
    }
}

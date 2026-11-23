// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, path::Path};

use gmeow_gts::v2::{
    decode_header,
    frame::{decode_frame, Storage},
    MalformedReason,
};

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn frame_identifiers_and_plain_payloads_match_the_published_chain() {
    let data = vector("P1/link-ok.bin");
    let header = decode_header(&data, 0).expect("header");
    let mut predecessor = header.identifier.expect("header ID");
    let mut offset = header.next_offset;
    let mut count = 0;
    while offset < data.len() {
        let frame = decode_frame(&data, offset, 1).expect("frame");
        assert_eq!(&frame.computed_identifier(&predecessor), frame.identifier);
        assert!(frame.check_decoded_payload(frame.stored_payload));
        assert_eq!(frame.check_storage(), None);
        predecessor = *frame.identifier;
        offset = frame.next_offset;
        count += 1;
    }
    assert!(count >= 2);
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

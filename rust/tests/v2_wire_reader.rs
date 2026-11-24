// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, path::Path};

use ciborium::Value;
use gmeow_gts::{
    v2::{
        reader::{
            read_wire, Dispatch, FramingFailure, FramingReason, ItemKind, LinkState, Opener,
            PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity, ReadStop,
        },
        MalformedReason,
    },
    wire,
};

fn vector(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2/P1")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn dispatches_by_the_typed_generation_field() {
    for (name, expected) in [
        (
            "append-generation.first-generation-header.bin",
            Dispatch::FirstGeneration,
        ),
        (
            "append-generation.first-generation-map-and-frame.bin",
            Dispatch::FirstGeneration,
        ),
        (
            "append-generation.header-generation-3.bin",
            Dispatch::UnsupportedGeneration(3),
        ),
    ] {
        let input = vector(name);
        let result = read_wire(&input, ReadCapacity::default());
        assert_eq!(result.dispatch, Some(expected), "{name}");
        assert!(result.segments.is_empty());
        assert!(result.stop.is_none());
    }
    let input = [0x87, 0x18, 1, 2, 1, 1, 0x81, 1, 0x81, 1, 0x81, 1];
    let result = read_wire(&input, ReadCapacity::default());
    assert_eq!(result.dispatch, Some(Dispatch::SecondGeneration));
    assert!(matches!(result.segments[0].opener, Opener::Header(Err(_))));
}

#[test]
fn framing_distinguishes_torn_and_undelimitable_tails() {
    let mut input = vector("link-ok.bin");
    let end = input.len();
    for (tail, reason) in [
        (&[0x58][..], FramingReason::TornTail),
        (&[0xbf, 0, 0xff], FramingReason::UndelimitableInput),
        (&[0x5f, 0x78], FramingReason::UndelimitableInput),
    ] {
        input.truncate(end);
        input.extend_from_slice(tail);
        let result = read_wire(&input, ReadCapacity::default());
        assert_eq!(
            result.stop,
            Some(ReadStop::Framing(FramingFailure {
                reason,
                offset: end,
                length: tail.len()
            })),
        );
        assert_eq!(result.segments[0].items.last().unwrap().end, end);
    }
}

#[test]
fn repairs_a_link_only_with_the_direct_admitted_witness() {
    let input = vector("link-healed-before-transparent.bin");
    let read = read_wire(&input, ReadCapacity::default());
    let frames = &read.segments[0].frames;
    assert_eq!(frames[0].link.state, LinkState::Healed);
    assert_eq!(frames[1].link.state, LinkState::Linked);
    assert!(frames[1].transparent);
    assert_eq!(read.segments[0].head, frames[0].link.computed_identifier);

    let cap = ReadCapacity {
        items_per_read: 2,
        ..ReadCapacity::default()
    };
    let bounded = read_wire(&input, cap);
    assert_eq!(bounded.segments[0].frames[0].link.state, LinkState::Broken);
    assert!(matches!(bounded.stop, Some(ReadStop::ItemLimit { .. })));

    let opaque = vector("link-heal-opaque-between.bin");
    let read = read_wire(&opaque, ReadCapacity::default());
    let segment = &read.segments[0];
    assert!(matches!(segment.items[3].kind, ItemKind::Malformed(_)));
    assert_eq!(
        segment
            .frames
            .iter()
            .map(|frame| frame.link.state)
            .collect::<Vec<_>>(),
        [
            LinkState::Linked,
            LinkState::Broken,
            LinkState::Broken,
            LinkState::Linked
        ],
    );
}

#[test]
fn a_walked_resync_opens_a_segment_using_the_computed_identifier() {
    let input = vector("resync-displaced-join.bin");
    let read = read_wire(&input, ReadCapacity::default());
    let [_, segment] = read.segments.as_slice() else {
        panic!("expected two segments");
    };
    let Opener::Resync(Ok(resync)) = segment.opener else {
        panic!("expected a readable resync opener");
    };
    assert_ne!(resync.claimed_offset, segment.items[0].offset as u64);
    assert!(segment
        .frames
        .iter()
        .all(|frame| frame.link.state == LinkState::Linked));
    assert!(read.stop.is_none());
}

#[test]
fn a_walked_resync_reports_the_byte_cap_where_it_is_reached() {
    let input = vector("resync-too-long.walked.bin");
    let result = read_wire(&input, ReadCapacity::default());
    let segment = result.segments.last().unwrap();
    let Opener::Resync(Err(error)) = segment.opener else {
        panic!("expected an unreadable resync opener");
    };
    assert_eq!(error.reason, MalformedReason::OutOfRange);
    assert_eq!(error.offset, segment.items[0].offset + 4096);

    let input = vector("resync-too-long.exactly-4096.bin");
    let result = read_wire(&input, ReadCapacity::default());
    assert!(matches!(
        result.segments.last().unwrap().opener,
        Opener::Resync(Ok(_))
    ));
}

#[test]
fn malformed_mark_shape_precedes_a_later_mark_limit() {
    let marker = blake3::derive_key("gts2 header", &[0x81, 5])[..16].to_vec();
    let absent = Value::Array(vec![Value::from(1)]);
    let mut marks = vec![Value::Array(vec![])];
    marks.extend((1..=64).map(|tag| Value::Array(vec![Value::from(tag)])));
    let input = wire::encode(&Value::Array(vec![
        Value::from(5),
        Value::from(2),
        Value::Bytes(marker),
        Value::from(1),
        Value::from(1),
        absent.clone(),
        absent.clone(),
        Value::from(0),
        absent,
        Value::Array(marks),
        Value::Bytes(vec![0; 32]),
    ]));
    let result = read_wire(&input, ReadCapacity::default());
    let Opener::Resync(Err(error)) = result.segments[0].opener else {
        panic!("expected malformed mark");
    };
    assert_eq!(error.reason, MalformedReason::WrongType);
    assert_eq!(input[error.offset], 0x80);
}

#[test]
fn checks_payload_independently_of_storage_and_link_state() {
    let input = vector("stored-mismatch.bin");
    let result = read_wire(&input, ReadCapacity::default());
    let frame = &result.segments[0].frames[0];
    assert_eq!(frame.link.state, LinkState::Linked);
    assert_eq!(frame.payload.stored_matches, Some(false));
    assert!(matches!(frame.payload.state, PayloadState::Damaged(_)));

    for (name, damage) in [
        ("payload-decoder-error.bin", PayloadDamage::DecodeFailed),
        ("payload-output-longer.bin", PayloadDamage::CodecConstraint),
        ("zstd-two-frames.bin", PayloadDamage::DecodeFailed),
        ("zstd-trailing-byte.bin", PayloadDamage::DecodeFailed),
    ] {
        let input = vector(name);
        let result = read_wire(&input, ReadCapacity::default());
        let frame = &result.segments[0].frames[0];
        assert_eq!(frame.payload.stored_matches, Some(true));
        assert_eq!(frame.payload.state, PayloadState::Damaged(damage), "{name}");
    }

    let input = vector("payload-unknown-codec.bin");
    let result = read_wire(&input, ReadCapacity::default());
    let frame = &result.segments[0].frames[0];
    assert_eq!(frame.payload.stored_matches, Some(true));
    assert_eq!(
        frame.payload.state,
        PayloadState::Unchecked(PayloadRefusal::UnknownCodec)
    );
}

#[test]
fn a_malformed_payload_head_does_not_hide_the_stored_checksum() {
    let input = vector("zstd-ok.bin");
    let good = read_wire(&input, ReadCapacity::default());
    let good = &good.segments[0].frames[0];
    assert!(matches!(good.payload.state, PayloadState::Verified(_)));
    assert_eq!(good.payload.stored_matches, Some(true));
    let payload = good.envelope.as_ref().unwrap().stored_payload;
    let encoded = wire::encode(&Value::Bytes(payload.to_vec()));
    let offset = input
        .windows(encoded.len())
        .position(|bytes| bytes == encoded)
        .unwrap();
    let mut second_form = vec![0x5b];
    second_form.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    second_form.extend_from_slice(payload);
    let mut chunked = vec![0x5f];
    chunked.extend_from_slice(&encoded);
    chunked.push(0xff);
    for malformed in [second_form, chunked] {
        let mut changed = input.clone();
        changed.splice(offset..offset + encoded.len(), malformed);
        let result = read_wire(&changed, ReadCapacity::default());
        let frame = &result.segments[0].frames[0];
        assert_eq!(frame.link.state, LinkState::Linked);
        assert_eq!(
            frame.envelope.as_ref().unwrap_err().reason,
            MalformedReason::NotDeterministic
        );
        assert_eq!(frame.payload.stored_matches, Some(true));
        assert_eq!(
            frame.payload.state,
            PayloadState::Damaged(PayloadDamage::MalformedFields)
        );
    }
}

#[test]
fn a_total_decoded_refusal_latches_even_for_a_later_empty_payload() {
    let input = vector("payload-total-after-refusal.bin");
    let capacity = ReadCapacity {
        decoded_bytes_per_read: 8,
        ..ReadCapacity::default()
    };
    let result = read_wire(&input, capacity);
    let [first, second, third, commit] = result.segments[0].frames.as_slice() else {
        panic!("expected four frames");
    };
    assert!(matches!(first.payload.state, PayloadState::Verified(_)));
    assert!(matches!(second.payload.state, PayloadState::Verified(_)));
    assert_eq!(
        third.payload.state,
        PayloadState::Unchecked(PayloadRefusal::TotalDecoded(9))
    );
    assert_eq!(
        commit.payload.state,
        PayloadState::Unchecked(PayloadRefusal::TotalDecoded(8))
    );
}

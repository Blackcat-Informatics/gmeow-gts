// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, path::Path};

use ciborium::Value;
use gmeow_gts::{
    v2::{
        reader::{
            read_wire, Dispatch, FramingFailure, FramingReason, ItemKind, LinkState, Opener,
            PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity, ReadStop, ResyncRefusal,
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
fn resync_distinguishes_a_damaged_region_from_a_torn_prefix() {
    for (name, reason, damaged_at, recovered_at, old_end) in [
        (
            "resync-after-undelimitable.whole.bin",
            FramingReason::UndelimitableInput,
            209,
            210,
            210,
        ),
        (
            "resync-after-damaged-length-head.whole.bin",
            FramingReason::TornTail,
            114,
            535,
            114,
        ),
    ] {
        let input = vector(name);
        let result = read_wire(&input, ReadCapacity::default());
        let [before, resumed] = result.segments.as_slice() else {
            panic!("{name}: expected two segments");
        };
        assert_eq!(before.end_offset, old_end, "{name}");
        assert_eq!(
            before.framing_failure,
            Some(FramingFailure {
                reason,
                offset: damaged_at,
                length: recovered_at - damaged_at,
            })
        );
        assert_eq!(
            before.items.last().unwrap().kind == ItemKind::DamagedRegion,
            reason == FramingReason::UndelimitableInput,
        );
        assert!(resumed.scanned);
        assert_eq!(resumed.items[0].offset, recovered_at);
        assert!(resumed
            .frames
            .iter()
            .all(|frame| frame.link.state == LinkState::Linked));
        assert!(result.stop.is_none());
    }
}

#[test]
fn the_complete_item_before_lost_framing_keeps_its_wire_extent() {
    let input = vector("resync-after-torn-swallowed.passing-whole.bin");
    let result = read_wire(&input, ReadCapacity::default());
    let [before, resumed] = result.segments.as_slice() else {
        panic!("expected two segments");
    };
    assert_eq!(before.end_offset, 306);
    let locator = before.items.last().unwrap();
    assert_eq!(locator.kind, ItemKind::Locator);
    assert_eq!(locator.end, 311);
    assert!(before.framing_failure.is_none());
    assert_eq!(before.frames.len(), 1);
    assert!(before.closing_grammar.is_empty());
    assert!(resumed.scanned);
    assert_eq!(resumed.items[0].offset, 306);
}

#[test]
fn a_scanned_opener_can_establish_the_generation_after_foreign_bytes() {
    let input = vector("resync-first-byte-foreign.whole.bin");
    let result = read_wire(&input, ReadCapacity::default());
    assert_eq!(result.dispatch, Some(Dispatch::SecondGeneration));
    assert_eq!(
        result.prefix_failure,
        Some(FramingFailure {
            reason: FramingReason::UndelimitableInput,
            offset: 0,
            length: 2,
        })
    );
    assert!(result.segments[0].scanned);
    assert_eq!(result.segments[0].items[0].offset, 2);
    assert!(result.stop.is_none());
}

#[test]
fn a_removed_witness_does_not_heal_the_preceding_frame() {
    let ample = vector("resync-heal-witness-removed.ample-whole.bin");
    let result = read_wire(&ample, ReadCapacity::default());
    let [prior, resumed] = result.segments.as_slice() else {
        panic!("expected a recovered segment");
    };
    assert_eq!(prior.frames[1].link.state, LinkState::Broken);
    assert_eq!(prior.head, prior.frames[1].link.effective_identifier);
    assert_eq!(resumed.items[0].offset, 316);

    let low = vector("resync-heal-witness-removed.low-whole.bin");
    let capacity = ReadCapacity {
        decoded_bytes_per_read: 4114,
        ..ReadCapacity::default()
    };
    let result = read_wire(&low, capacity);
    let [segment] = result.segments.as_slice() else {
        panic!("expected refusal to retain the witness");
    };
    assert_eq!(segment.frames[1].link.state, LinkState::Healed);
    assert_eq!(segment.frames[2].link.state, LinkState::Linked);
    assert_eq!(
        segment.resync_refusal,
        Some(ResyncRefusal {
            position: 3,
            offset: 316,
            length: 4096,
            count: 4184,
        })
    );
}

#[test]
fn charges_scan_candidates_after_place_and_before_tail_grammar() {
    let capacity = ReadCapacity {
        decoded_bytes_per_read: 4000,
        ..ReadCapacity::default()
    };
    let input = vector("resync-charge-before-read.whole.bin");
    let result = read_wire(&input, capacity);
    assert_eq!(
        result.segments[0].resync_refusal,
        Some(ResyncRefusal {
            position: 3,
            offset: 210,
            length: 4096,
            count: 4127,
        })
    );
    let foreign = vector("resync-offset-false.whole.bin");
    let result = read_wire(&foreign, capacity);
    assert_eq!(result.segments.len(), 1);
    assert!(result.segments[0].resync_refusal.is_none());
}

#[test]
fn items_held_before_the_scan_decision_count_toward_the_limit() {
    let mut input = vector("resync-candidate-untested-at-limit.ends-at-third.bin");
    let cap = ReadCapacity {
        items_per_read: 3,
        ..ReadCapacity::default()
    };
    let read = read_wire(&input, cap);
    let [prior] = read.segments.as_slice() else {
        panic!("the item limit should exclude the candidate");
    };
    assert_eq!(prior.end_offset, 299);
    assert!(prior.resync_refusal.is_none());
    assert!(read.stop.is_none());
    input.push(0);
    let read = read_wire(&input, cap);
    assert_eq!(read.segments.len(), 1);
    assert_eq!(read.stop, Some(ReadStop::ItemLimit { offset: 299 }));
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
fn a_walked_resync_decides_each_nested_field_at_its_start() {
    let marker = blake3::derive_key("gts2 header", &[0x81, 5]);
    for (size, past, reason, error_offset) in [
        (
            4061_u16,
            &[0x82, 2, 0][..],
            MalformedReason::WrongType,
            4095,
        ),
        (4062, &[0x82, 2, 0], MalformedReason::OutOfRange, 4096),
        (4062, &[0x81, 0], MalformedReason::UnregisteredValue, 4095),
        (4063, &[0x81, 0], MalformedReason::OutOfRange, 4096),
        (
            4061,
            &[0x82, 2, 0x18, 0],
            MalformedReason::NotDeterministic,
            4095,
        ),
    ] {
        let mut input = vec![0x8b, 5, 2, 0x50];
        input.extend_from_slice(&marker[..16]);
        // A later registry and unknown hash permit the long lineage digest.
        input.extend_from_slice(&[2, 9, 0x81, 1, 0x82, 2, 0x82, 0x59]);
        input.extend_from_slice(&size.to_be_bytes());
        input.resize(input.len() + usize::from(size), 0);
        input.extend_from_slice(&[1, 0]);
        input.extend_from_slice(past);
        input.extend_from_slice(&[0x80, 0x40]);

        let read = read_wire(&input, ReadCapacity::default());
        let Opener::Resync(Err(error)) = read.segments[0].opener else {
            panic!("expected an unreadable resync for lineage size {size} and past {past:?}");
        };
        assert_eq!((error.reason, error.offset), (reason, error_offset));
    }
}

fn bare_resync(marker: Vec<u8>, marks: Vec<Value>) -> Vec<u8> {
    let absent = Value::Array(vec![Value::from(1)]);
    wire::encode(&Value::Array(vec![
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
    ]))
}

#[test]
fn malformed_mark_shape_precedes_a_later_mark_limit() {
    let marker = blake3::derive_key("gts2 header", &[0x81, 5])[..16].to_vec();
    let mut marks = vec![Value::Array(vec![])];
    marks.extend((1..=64).map(|tag| Value::Array(vec![Value::from(tag)])));
    let input = bare_resync(marker, marks);
    let result = read_wire(&input, ReadCapacity::default());
    let Opener::Resync(Err(error)) = result.segments[0].opener else {
        panic!("expected malformed mark");
    };
    assert_eq!(error.reason, MalformedReason::WrongType);
    assert_eq!(input[error.offset], 0x80);
}

#[test]
fn violations_before_a_resync_byte_limit_keep_their_first_offset() {
    let oversized_marker = bare_resync(vec![0; 5000], vec![]);
    let result = read_wire(&oversized_marker, ReadCapacity::default());
    let Opener::Resync(Err(error)) = result.segments[0].opener else {
        panic!("expected malformed marker");
    };
    assert_eq!(error.reason, MalformedReason::OutOfRange);
    assert_eq!(error.offset, 3);

    let marker = blake3::derive_key("gts2 header", &[0x81, 5])[..16].to_vec();
    let text = Value::Text("abcd".repeat(1250));
    let mut input = bare_resync(marker, vec![Value::Array(vec![Value::from(9), text])]);
    let body = input.windows(4).position(|part| part == b"abcd").unwrap();
    input[body] = 0xff;
    let result = read_wire(&input, ReadCapacity::default());
    let Opener::Resync(Err(error)) = result.segments[0].opener else {
        panic!("expected invalid text inside the mark");
    };
    assert_eq!(error.reason, MalformedReason::WrongType);
    assert_eq!(error.offset, body - 3);
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

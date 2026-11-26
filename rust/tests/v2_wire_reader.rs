// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The v2 wire reader: dispatch, framing tails, link repair and resync against the published vectors.

use std::{fs, path::Path};

use ciborium::Value;
use gmeow_gts::{
    v2::{
        reader::{
            read_wire, read_wire_with_keys, ClosingFinding, CommitCheck, Dispatch, FramingFailure,
            FramingReason, IndexOutcome, IndexWireClaims, ItemKind, LinkState, Opener,
            PayloadDamage, PayloadRefusal, PayloadState, ReadCapacity, ReadStop, ResyncRefusal,
            UncommittedTail,
        },
        signature::{HeldKey, HeldKeys, SignatureReason, SignatureStatus},
        writer::SegmentWriter,
        Header, MalformedReason,
    },
    wire,
};

fn vector(name: &str) -> Vec<u8> {
    vector_in("P1", name)
}

fn vector_in(part: &str, name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2")
        .join(part)
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
        assert_eq!(
            result.segments,
            [] as [gmeow_gts::v2::reader::WireSegment<'_>; 0]
        );
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
    assert_eq!(
        before.closing_grammar,
        [] as [(usize, gmeow_gts::v2::HeaderError); 0]
    );
    assert_eq!(
        before.closing_findings,
        [ClosingFinding::IndexLocator { position: 2 }]
    );
    assert!(resumed.scanned);
    assert_eq!(resumed.items[0].offset, 306);
}

#[test]
fn offsets_check_every_retained_item_and_report_count_or_length_disagreement() {
    use ClosingFinding::IndexOffsets;
    for (name, mismatch) in [
        ("offsets-holds.bin", false),
        ("offsets-counts-opaque.bin", false),
        ("offsets-length-differs.bin", true),
        ("offsets-count-short.bin", true),
        ("offsets-count-long.bin", true),
    ] {
        let data = vector(name);
        let read = read_wire(&data, ReadCapacity::default());
        let findings = &read.segments[0].closing_findings;
        if mismatch {
            assert_eq!(findings, &[IndexOffsets { position: 3 }], "{name}");
        } else {
            assert!(findings.is_empty(), "{name}");
        }
    }
}

#[test]
fn locator_claim_names_the_last_well_typed_index_frame_type() {
    use ClosingFinding::IndexLocator;
    for (part, name, expected) in [
        ("P1", "locator-last-index.first.bin", Some(6)),
        ("P1", "locator-last-index.second.bin", None),
        ("P1", "locator-index-type-long-form.bin", Some(8)),
        ("P5", "locator-closing-index.bin", Some(6)),
    ] {
        let data = vector_in(part, name);
        let read = read_wire(&data, ReadCapacity::default());
        let findings = &read.segments[0].closing_findings;
        assert_eq!(
            findings,
            &expected
                .into_iter()
                .map(|position| IndexLocator { position })
                .collect::<Vec<_>>(),
            "{name}",
        );
    }
}

#[test]
fn misplaced_closing_items_report_placement_before_grammar_or_claims() {
    for (name, expected) in [
        ("segment-two-offsets.bin", &[1, 2][..]),
        ("segment-two-locators.bin", &[1, 2][..]),
        ("segment-locator-before-offsets.bin", &[1, 2][..]),
        ("segment-offsets-then-frame.bin", &[1][..]),
        ("segment-locator-then-frame.bin", &[1][..]),
    ] {
        let data = vector(name);
        let read = read_wire(&data, ReadCapacity::default());
        let findings = &read.segments[0].closing_findings;
        let positions: Vec<_> = findings
            .iter()
            .map(|finding| match finding {
                ClosingFinding::Malformed { position, error } => {
                    assert_eq!(error.reason, MalformedReason::UnorderedOrRepeated, "{name}");
                    *position
                }
                _ => panic!("{name}: expected placement finding, got {finding:?}"),
            })
            .collect();
        assert_eq!(positions, expected, "{name}");
    }
    let data = vector("locator-short-form.bin");
    let read = read_wire(&data, ReadCapacity::default());
    assert!(matches!(
        read.segments[0].closing_findings.as_slice(),
        [ClosingFinding::Malformed { position: 2, error }] if error.reason == MalformedReason::NotDeterministic
    ));
}

#[test]
fn commit_prefix_checks_count_and_root_independently_of_transparent_frames() {
    for (name, count_matches, root_matches) in [
        ("commit-valid.bin", true, true),
        ("commit-count-mismatch.bin", false, true),
        ("commit-root-mismatch.bin", true, false),
    ] {
        let data = vector_in("P5", name);
        let read = read_wire(&data, ReadCapacity::default());
        let report = read.segments[0].commits.as_ref().unwrap();
        let valid = count_matches && root_matches;
        assert_eq!(
            report.checks,
            [CommitCheck {
                position: 4,
                ordinary_position: 2,
                count_matches,
                root_matches,
                valid,
            }],
            "{name}"
        );
        assert_eq!(report.last_valid_position, valid.then_some(4), "{name}");
        assert_eq!(report.committed_frames, if valid { 3 } else { 0 }, "{name}");
        assert_eq!(
            report.uncommitted_tail,
            Some(if valid {
                UncommittedTail {
                    position: 5,
                    count: 1,
                }
            } else {
                UncommittedTail {
                    position: 1,
                    count: 4,
                }
            }),
            "{name}"
        );
    }
    let data = vector_in("P5", "commit-canonical-tail.bin");
    let read = read_wire(&data, ReadCapacity::default());
    let report = read.segments[0].commits.as_ref().unwrap();
    assert_eq!(report.committed_frames, 2);
    assert!(report.uncommitted_tail.is_none());
}

#[test]
fn malformed_or_unread_commit_content_checks_no_claims() {
    let bytes = vector_in("P5", "commit-payload-nonempty.bin");
    let read = read_wire(&bytes, ReadCapacity::default());
    let report = read.segments[0].commits.as_ref().unwrap();
    assert_eq!(report.checks, [] as [gmeow_gts::v2::reader::CommitCheck; 0]);
    assert_eq!(
        report.uncommitted_tail,
        Some(UncommittedTail {
            position: 1,
            count: 2
        })
    );

    let bytes = vector_in("P5", "commit-valid.bin");
    let capacity = ReadCapacity {
        stored_bytes_per_frame: 0,
        ..ReadCapacity::default()
    };
    let read = read_wire(&bytes, capacity);
    assert_eq!(
        read.segments[0].commits.as_ref().unwrap().checks,
        [] as [gmeow_gts::v2::reader::CommitCheck; 0]
    );

    let capacity = ReadCapacity {
        decoded_bytes_per_read: 0,
        ..ReadCapacity::default()
    };
    let read = read_wire(&bytes, capacity);
    assert_eq!(
        read.segments[0].commits.as_ref().unwrap().committed_frames,
        3
    );
}

#[test]
fn a_frame_without_an_effective_identifier_creates_no_commit_leaf() {
    let mut writer = SegmentWriter::new(Header::default()).unwrap();
    let offset = writer.as_bytes().len();
    writer.commit().unwrap();
    let mut bytes = writer.into_bytes();
    let malformed = wire::encode(&Value::Array(vec![
        Value::from(2),
        Value::from(8),
        Value::Bool(true),
        Value::Bool(false),
        Value::Array(vec![Value::from(0)]),
        Value::from(0),
        Value::Bytes(vec![0; 32]),
        Value::Array(vec![Value::from(1)]),
        Value::Bytes(Vec::new()),
        Value::from(0),
        Value::Array(Vec::new()),
    ]));
    bytes.splice(offset..offset, malformed);
    let read = read_wire(&bytes, ReadCapacity::default());
    let segment = &read.segments[0];
    assert_eq!(segment.frames[0].link.effective_identifier, None);
    let report = segment.commits.as_ref().unwrap();
    assert_eq!(report.checks[0].ordinary_position, 0);
    assert_eq!(report.last_valid_position, Some(2));
    assert_eq!(report.committed_frames, 1);
    assert!(report.uncommitted_tail.is_none());
}

#[test]
fn a_commit_needs_a_link_even_when_both_claims_hold() {
    let mut writer = SegmentWriter::new(Header::default()).unwrap();
    writer.commit().unwrap();
    let first_identifier_end = writer.as_bytes().len() - 2;
    writer.commit().unwrap();
    let second_identifier_end = writer.as_bytes().len() - 2;
    let bytes = writer.into_bytes();

    let mut healed = bytes.clone();
    healed[first_identifier_end] ^= 1;
    let read = read_wire(&healed, ReadCapacity::default());
    let segment = &read.segments[0];
    assert_eq!(segment.frames[0].link.state, LinkState::Healed);
    assert_eq!(segment.commits.as_ref().unwrap().committed_frames, 2);

    let mut broken = bytes;
    broken[second_identifier_end] ^= 1;
    let read = read_wire(&broken, ReadCapacity::default());
    let segment = &read.segments[0];
    assert_eq!(segment.frames[1].link.state, LinkState::Broken);
    let report = segment.commits.as_ref().unwrap();
    let last = report.checks[1];
    assert!(last.count_matches && last.root_matches);
    assert!(!last.valid);
    assert_eq!(report.last_valid_position, Some(1));
    assert_eq!(
        report.uncommitted_tail,
        Some(UncommittedTail {
            position: 2,
            count: 1
        })
    );
}

#[test]
fn index_wire_claims_are_checked_independently_at_each_index() {
    for (name, head_matches, root_matches, type_runs_match) in [
        ("index-each-claim.head.bin", false, true, true),
        ("index-each-claim.range_root.bin", true, false, true),
        ("index-each-claim.type_runs.bin", true, true, false),
        (
            "index-not-last.structural-claim-false.bin",
            false,
            true,
            true,
        ),
        ("index-nonterminal-claims.bin", true, true, true),
        ("index-not-last.digest-claims-false.bin", true, true, true),
    ] {
        let data = vector_in("P5", name);
        let read = read_wire(&data, ReadCapacity::default());
        let [index] = read.segments[0].indexes.as_slice() else {
            panic!("{name}: expected one checked index");
        };
        assert_eq!(
            index.outcome,
            IndexOutcome::Checked(IndexWireClaims {
                head_matches,
                root_matches,
                type_runs_match
            }),
            "{name}",
        );
    }
}

#[test]
fn covering_a_frame_without_a_type_rejects_the_type_runs() {
    for name in [
        "index-covers-frame-of-no-type.long-form-15-typed.bin",
        "index-covers-frame-of-no-type.long-form-15-omitted.bin",
    ] {
        let data = vector_in("P5", name);
        let read = read_wire(&data, ReadCapacity::default());
        let segment = &read.segments[0];
        assert!(segment.frames.iter().any(|frame| {
            frame.frame_type.is_none() && frame.link.effective_identifier.is_some()
        }));
        assert_eq!(
            segment.indexes[0].outcome,
            IndexOutcome::Checked(IndexWireClaims {
                head_matches: true,
                root_matches: true,
                type_runs_match: false,
            }),
            "{name}",
        );
    }
}

#[test]
fn an_index_after_an_unknown_critical_frame_still_checks_the_wire_prefix() {
    for (name, type_runs_match) in [
        ("index-after-stop.exact.bin", true),
        ("index-after-stop.wrong-type-runs.bin", false),
    ] {
        let data = vector_in("P5", name);
        let read = read_wire(&data, ReadCapacity::default());
        let segment = &read.segments[0];
        assert!(segment
            .frames
            .iter()
            .any(|frame| frame.unknown_type_stops_fold));
        assert_eq!(
            segment.indexes[0].outcome,
            IndexOutcome::Checked(IndexWireClaims {
                head_matches: true,
                root_matches: true,
                type_runs_match,
            }),
            "{name}",
        );
    }
}

#[test]
fn an_index_with_bad_payload_grammar_is_not_compared() {
    let mut writer = SegmentWriter::new(Header::default()).unwrap();
    let content = wire::encode(&Value::Array(vec![
        Value::from(99),
        Value::Bytes(vec![0; 32]),
    ]));
    writer.append(15, &content, &[0x80]).unwrap();
    let data = writer.into_bytes();
    let read = read_wire(&data, ReadCapacity::default());
    let segment = &read.segments[0];
    assert!(matches!(
        segment.frames[0].payload.state,
        PayloadState::Verified(_)
    ));
    assert!(matches!(
        segment.indexes[0].outcome,
        IndexOutcome::MalformedPayload(error) if error.reason == MalformedReason::WrongLength
    ));

    let low = ReadCapacity {
        decoded_bytes_per_frame: 0,
        ..ReadCapacity::default()
    };
    let read = read_wire(&data, low);
    assert_eq!(
        read.segments[0].indexes,
        [] as [gmeow_gts::v2::reader::IndexCheck; 0]
    );
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

fn unhex(encoded: &str) -> Vec<u8> {
    encoded
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|byte| {
            u8::from_str_radix(std::str::from_utf8(byte).expect("corpus hex"), 16)
                .expect("corpus hex byte")
        })
        .collect()
}

fn items(value: &Value) -> &[Value] {
    value.as_array().expect("corpus array")
}

fn uint(value: &Value) -> u64 {
    u64::try_from(value.as_integer().expect("corpus uint")).expect("corpus uint in range")
}

fn bytes(value: &Value) -> &[u8] {
    value.as_bytes().expect("corpus bytes")
}

/// One signature outcome as a vector records it: status, reason, algorithm code and `key_id`.
type Outcome = (SignatureStatus, Option<SignatureReason>, u64, Vec<u8>);

/// A signature vector case: its input, the held keys of its recorded read and the recorded
/// outcomes of each signed frame, keyed by the byte offset of the frame.
struct SignatureCase {
    input_file: String,
    input: Vec<u8>,
    held: Vec<HeldKey>,
    expected: Vec<(usize, Vec<Outcome>)>,
}

fn recorded_status(code: u64) -> SignatureStatus {
    match code {
        1 => SignatureStatus::Valid,
        2 => SignatureStatus::Invalid,
        3 => SignatureStatus::Unverified,
        _ => panic!("unexpected signature status {code}"),
    }
}

fn recorded_reason(code: u64) -> SignatureReason {
    match code {
        53 => SignatureReason::UnknownSignatureAlgorithm,
        54 => SignatureReason::KeyNotHeld,
        57 => SignatureReason::KeyMismatch,
        58 => SignatureReason::ValueLength,
        59 => SignatureReason::NonCanonicalScalar,
        60 => SignatureReason::SmallOrderKey,
        61 => SignatureReason::SmallOrderNonce,
        62 => SignatureReason::EquationFailed,
        63 => SignatureReason::CompositeClassicalFailed,
        64 => SignatureReason::CompositePostQuantumFailed,
        _ => panic!("unexpected signature reason {code}"),
    }
}

/// Read the cases of a P6 vector: the held keys come from the `inputs` of the recorded read and
/// the expected outcomes from the frame records of its second-generation file.
fn signature_cases(name: &str) -> Vec<SignatureCase> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors/v2/P6")
        .join(format!("{name}.json"));
    let description: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read vector description"))
            .expect("vector description is JSON");
    description["cases"]
        .as_array()
        .expect("vector cases")
        .iter()
        .map(|case| {
            let input_file = case["input_file"].as_str().expect("input file").to_owned();
            let recorded: Value = ciborium::from_reader(
                unhex(case["read_result"].as_str().expect("typed read result")).as_slice(),
            )
            .expect("recorded read is CBOR");
            let record = items(&recorded);
            let held = items(&items(&record[5])[1])
                .iter()
                .map(|key| {
                    let key = items(key);
                    HeldKey {
                        key_id: bytes(&key[0]).try_into().expect("32-byte key identifier"),
                        algorithm: uint(&key[1]),
                        public_key: bytes(&key[2]).to_vec(),
                    }
                })
                .collect();
            let expected = items(&items(&record[6])[1])
                .iter()
                .flat_map(|segment| items(&items(segment)[16]))
                .filter(|frame| !items(&items(frame)[7]).is_empty())
                .map(|frame| {
                    let frame = items(frame);
                    let outcomes = items(&frame[7])
                        .iter()
                        .map(|result| {
                            let result = items(result);
                            (
                                recorded_status(uint(&result[0])),
                                items(&result[1])
                                    .get(1)
                                    .map(|code| recorded_reason(uint(code))),
                                uint(&result[2]),
                                bytes(&result[3]).to_vec(),
                            )
                        })
                        .collect();
                    (
                        usize::try_from(uint(&frame[1])).expect("frame offset"),
                        outcomes,
                    )
                })
                .collect();
            SignatureCase {
                input: vector_in("P6", &input_file),
                input_file,
                held,
                expected,
            }
        })
        .collect()
}

/// The outcomes the reader gives, keyed by frame offset, for the frames that have any.
fn read_outcomes(input: &[u8], held: Vec<HeldKey>) -> Vec<(usize, Vec<Outcome>)> {
    let read = read_wire_with_keys(input, ReadCapacity::default(), &HeldKeys::new(held));
    read.segments
        .iter()
        .flat_map(|segment| {
            segment
                .frames
                .iter()
                .filter(|frame| !frame.signatures.is_empty())
                .map(|frame| {
                    let outcomes = frame
                        .signatures
                        .iter()
                        .map(|result| {
                            (
                                result.status,
                                result.reason,
                                result.algorithm,
                                result.key_id.to_vec(),
                            )
                        })
                        .collect();
                    (segment.items[frame.position].offset, outcomes)
                })
        })
        .collect()
}

fn assert_signature_vectors(names: &[&str]) {
    for name in names {
        for case in signature_cases(name) {
            assert!(
                !case.expected.is_empty(),
                "{} records a signature result",
                case.input_file
            );
            assert_eq!(
                read_outcomes(&case.input, case.held),
                case.expected,
                "{}",
                case.input_file
            );
        }
    }
}

/// P6 6.2 step 6: honest signatures of each algorithm are valid, including an ML-DSA half made
/// with hedged signing, which the same verification accepts (P6 5).
#[test]
fn honest_signatures_of_both_algorithms_are_valid() {
    assert_signature_vectors(&[
        "sig-record-valid",
        "ed25519-honest",
        "ed25519-honest-reproduce",
        "composite-honest",
        "composite-hedged",
        "cosign-two-algorithms",
    ]);
}

/// P6 4.1 and P6-17: the first failing check of SCALAR, KEY, NONCE, EQUATION is the reason, over
/// the whole input.
#[test]
fn ed25519_strict_reports_the_first_failing_check_in_policy_order() {
    let mut names: Vec<String> = [
        "ed25519-scalar-above-l",
        "ed25519-scalar-equals-l",
        "ed25519-scalar-max",
        "ed25519-scalar-below-l",
        "ed25519-order-scalar-key",
        "ed25519-order-key-nonce",
        "proc-case-5",
        "proc-case-6",
        "proc-case-7",
    ]
    .map(String::from)
    .into();
    for order in 1..=8 {
        names.push(format!("ed25519-key-small-order-{order}"));
        names.push(format!("ed25519-nonce-small-order-{order}"));
    }
    assert_signature_vectors(&names.iter().map(String::as_str).collect::<Vec<_>>());
}

/// P6 4.1 and P6-20: a key decodes only by the stated decoding, the equation is cofactorless, and
/// a non-canonical or off-curve `R` never satisfies it.
#[test]
fn ed25519_equation_decodes_the_key_strictly_and_is_cofactorless() {
    assert_signature_vectors(&[
        "ed25519-key-off-curve",
        "ed25519-key-alias",
        "ed25519-key-non-canonical-sign",
        "ed25519-key-non-canonical-y",
        "ed25519-nonce-non-canonical",
        "ed25519-nonce-off-curve",
        "ed25519-equation-message",
        "ed25519-mixed-order-valid",
        "ed25519-mixed-order-cofactored-only",
        "proc-case-8",
        "proc-case-9",
    ]);
}

/// P6 4.2 and P6-26: the classical half over `M'` is judged before the post-quantum half, and a
/// composite whose both halves fail is `composite_classical_failed`.
#[test]
fn composite_reports_the_classical_half_over_the_representative_before_the_post_quantum_half() {
    assert_signature_vectors(&[
        "composite-both",
        "composite-classical-nonce",
        "composite-classical-scalar",
        "composite-classical-small-order",
        "composite-halves-two-subjects",
        "composite-label-wrong",
        "composite-prehash-wrong",
        "composite-context-nonempty",
        "proc-case-11",
        "proc-case-13",
    ]);
}

/// P6 4.2 and P6-53: the ML-DSA half is `ML-DSA.Verify` with the label as its context, and the
/// four refusals of the hint decoding are `composite_post_quantum_failed`.
#[test]
fn composite_post_quantum_half_is_ml_dsa_verify_with_the_label_context() {
    assert_signature_vectors(&[
        "composite-stripped-half",
        "composite-post-quantum",
        "composite-mldsa-context",
        "composite-hint-count-above-omega",
        "composite-hint-count-decreasing",
        "composite-hint-indices-unordered",
        "composite-hint-padding-nonzero",
        "proc-case-12",
    ]);
}

/// P6 6.2 step 4 and P6-34: a key or value of the wrong length is `value_length` before any
/// check of the algorithm, never a failure of a half.
#[test]
fn a_key_or_value_of_the_wrong_length_is_value_length_before_any_algorithm_check() {
    assert_signature_vectors(&[
        "ed25519-length-63",
        "ed25519-length-65",
        "composite-length-3372",
        "composite-length-3374",
        "composite-key-1983",
        "composite-key-1985",
        "key-length-short",
        "proc-lengths-key-first",
        "proc-length-before-policy",
        "proc-value-length",
        "proc-case-14",
    ]);
}

/// P6 6.2 steps 1 to 3 and P6-33: an unknown algorithm is unverified, an absent key is
/// `key_not_held`, and a held key that is inconsistent is `key_mismatch`, all before the lengths.
#[test]
fn algorithm_and_held_key_steps_come_before_the_lengths() {
    assert_signature_vectors(&[
        "proc-unknown-algorithm",
        "proc-unknown-algorithm-long",
        "reg-unknown-algorithm-version",
        "proc-key-not-held",
        "key-trusted-not-held",
        "proc-key-mismatch",
        "key-mismatch-algorithm",
        "key-mismatch-bytes",
        "subject-relabel-algorithm",
        "proc-mismatch-before-length",
        "key-mismatch-two-entries",
    ]);
}

/// P6 2 and P6-6: the subject binds the opener of the segment and the effective identifier of the
/// frame, so a signature moved to another frame or segment, relabelled, or made before a link was
/// healed gives the outcome the vector records.
#[test]
fn the_subject_binds_the_opener_and_the_effective_identifier_of_the_frame() {
    assert_signature_vectors(&[
        "subject-healed-frame",
        "subject-other-frame",
        "subject-other-segment",
        "subject-relabel-key",
        "cov-healed",
        "cov-broken-signed-frame",
        "sig-list-independence",
        "sig-same-key-two-values",
    ]);
}

/// P6-3 and P6-31: a signed frame whose envelope is malformed outside its `signatures` field, a
/// commit with malformed content and a frame whose `transparent` is not a bool are ordinary and
/// carry results.
#[test]
fn a_frame_with_a_malformed_envelope_but_signatures_in_grammar_has_results() {
    assert_signature_vectors(&[
        "auth-commit-malformed",
        "parity-transparent-not-bool-signed",
        "proc-opaque-frame",
    ]);
}

/// P6-13 and P6-37: the held keys are a set, so the order and repetition the caller gives change
/// no outcome.
#[test]
fn held_keys_are_a_set_whatever_order_or_repetition_the_caller_gives() {
    for name in [
        "key-held-order",
        "key-mismatch-two-entries",
        "sig-record-valid",
    ] {
        for case in signature_cases(name) {
            let mut scrambled: Vec<HeldKey> = case.held.iter().rev().cloned().collect();
            scrambled.extend(case.held.iter().cloned());
            assert_eq!(
                read_outcomes(&case.input, scrambled),
                case.expected,
                "{}",
                case.input_file
            );
        }
    }
}

/// P6 6.2 precondition and P6-3: no result exists for a frame with a `signatures` field outside
/// the grammar, for a transparent frame, or in a segment whose opener has no identifier.
#[test]
fn frames_without_a_subject_or_in_grammar_carry_no_signature_result() {
    let names = [
        "sig-grammar-not-array",
        "sig-grammar-element-not-array",
        "sig-grammar-two-fields",
        "sig-grammar-four-fields",
        "sig-grammar-algorithm-text",
        "sig-grammar-algorithm-zero",
        "sig-grammar-algorithm-unregistered",
        "sig-grammar-key-id-text",
        "sig-grammar-key-id-short",
        "sig-grammar-value-text",
        "sig-grammar-long-head",
        "sig-list-unordered",
        "sig-list-grammar-first",
        "sig-duplicate-records",
        "sig-transparent-signature",
        "proc-no-hash",
    ];
    for name in names {
        for case in signature_cases(name) {
            assert!(case.expected.is_empty(), "{}", case.input_file);
            assert_eq!(
                read_outcomes(&case.input, case.held),
                [],
                "{}",
                case.input_file
            );
        }
    }
}

/// P6 6.2 step 2: a reader given no held keys reports each signature of a frame within the
/// stored-bytes capacity as `key_not_held`.
#[test]
fn a_read_without_keys_reports_key_not_held_within_the_stored_bytes_capacity() {
    let input = vector_in("P6", "sig-record-valid.bin");
    let read = read_wire(&input, ReadCapacity::default());
    let results: Vec<_> = read.segments[0]
        .frames
        .iter()
        .flat_map(|frame| frame.signatures.iter())
        .collect();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| {
        result.status == SignatureStatus::Unverified
            && result.reason == Some(SignatureReason::KeyNotHeld)
    }));
    let small = ReadCapacity {
        stored_bytes_per_frame: 100,
        ..ReadCapacity::default()
    };
    let read = read_wire(&input, small);
    assert!(read.segments[0]
        .frames
        .iter()
        .all(|frame| frame.signatures.is_empty()));
}

/// P6 6.2 step 2 and P6-26: a composite signature is refused when only one of
/// its component keys is held. Each half is held under its own identifier, so the composite's
/// identifier matches no held key and the signature is `key_not_held`, never `valid`.
#[test]
fn a_composite_with_only_one_component_key_held_is_refused() {
    let case = signature_cases("composite-honest")
        .into_iter()
        .next()
        .expect("composite-honest has a case");
    let composite = case
        .held
        .iter()
        .find(|key| key.algorithm == 2)
        .expect("the composite key is held in the recorded read");
    let (mldsa_half, ed25519_half) = composite.public_key.split_at(1952);
    let only_ed25519 = HeldKey {
        key_id: HeldKey::identifier(1, ed25519_half),
        algorithm: 1,
        public_key: ed25519_half.to_vec(),
    };
    let only_mldsa = HeldKey {
        key_id: HeldKey::identifier(2, mldsa_half),
        algorithm: 2,
        public_key: mldsa_half.to_vec(),
    };
    for held in [only_ed25519, only_mldsa] {
        let outcomes = read_outcomes(&case.input, vec![held]);
        let composite_outcomes: Vec<_> = outcomes
            .iter()
            .flat_map(|(_, outcomes)| outcomes.iter())
            .filter(|outcome| outcome.2 == 2)
            .collect();
        assert!(
            !composite_outcomes.is_empty(),
            "the composite signature has a result"
        );
        for outcome in composite_outcomes {
            assert_eq!(outcome.0, SignatureStatus::Unverified);
            assert_eq!(outcome.1, Some(SignatureReason::KeyNotHeld));
        }
    }
}

/// P5 7.2 step 6 and 7.2A, P1 3.4 step 4: reading resumes at the parity frame after a damaged
/// region and the region is one item, so the frames after it are read and signed results exist.
/// The frame that takes the `anchor` is detached unless the `anchor` is the identifier of the
/// ordinary frame before the region.
#[test]
fn a_parity_frame_after_a_damaged_region_resumes_the_read_and_detaches_by_its_anchor() {
    assert_signature_vectors(&[
        "sig-detached-frame",
        "sig-two-after-detached-frame",
        "sig-commit-after-detached-frame",
        "sig-resumed-frame-anchor-true",
    ]);
    for (name, detached) in [
        ("sig-detached-frame.bin", true),
        ("sig-resumed-frame-anchor-true.bin", false),
    ] {
        let input = vector_in("P6", name);
        let read = read_wire(&input, ReadCapacity::default());
        assert!(read.stop.is_none(), "{name}");
        let [segment] = read.segments.as_slice() else {
            panic!("{name}: expected one segment");
        };
        let [region] = segment.resumptions.as_slice() else {
            panic!("{name}: expected one resumption");
        };
        assert_eq!(region.reason, FramingReason::UndelimitableInput, "{name}");
        assert_eq!(region.length, 1, "{name}");
        let damaged: Vec<_> = segment
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.kind == ItemKind::DamagedRegion)
            .collect();
        assert_eq!(damaged.len(), 1, "{name}");
        assert_eq!(
            (damaged[0].1.offset, damaged[0].0),
            (region.offset, 2),
            "{name}"
        );
        let after: Vec<_> = segment
            .frames
            .iter()
            .filter(|frame| !frame.transparent && frame.position > damaged[0].0)
            .collect();
        assert_eq!(after.len(), 1, "{name}");
        assert_eq!(after[0].detached, detached, "{name}");
    }
}

/// P5 6.6 condition 6 judges an earlier parity frame whatever its own state, and only a frame
/// whose envelope is as the type states can be set aside as a later-version frame. The input is
/// `H T Q P0 <undelimitable byte> P` in a segment of registry version 2, `P0` with a `geometry`
/// of a later version and `P` a frame that passes every test but condition 6 when `P0` has a
/// `first`. An unsigned `P0` has none, so reading resumes at `P`; a `P0` with a signature is
/// damaged by condition 2, has the `first` of `P`, and nothing resumes.
#[test]
fn a_signed_earlier_parity_frame_in_a_later_version_segment_still_counts_for_condition_six() {
    let base = vector_in(
        "P5",
        "parity-condition-six-first-by-case.later-version-geometry.bin",
    );
    let read = read_wire(&base, ReadCapacity::default());
    let end = read.segments[0].items[3].end;
    assert_eq!(base[end - 1], 0x80, "P0 ends with an empty signatures list");
    // One signature record: algorithm 1, a 32-byte key identifier and an empty value.
    let mut signature = vec![0x81, 0x83, 0x01, 0x58, 0x20];
    signature.extend([0; 32]);
    signature.push(0x40);

    let mut unsigned = base[..end].to_vec();
    unsigned.push(0x1c);
    unsigned.extend_from_slice(&base[end..]);
    let mut signed = base[..end - 1].to_vec();
    signed.extend_from_slice(&signature);
    signed.push(0x1c);
    signed.extend_from_slice(&base[end..]);

    let control = read_wire(&unsigned, ReadCapacity::default());
    assert!(control.stop.is_none());
    assert_eq!(
        control.segments[0].resumptions,
        [FramingFailure {
            reason: FramingReason::UndelimitableInput,
            offset: end,
            length: 1,
        }]
    );

    let result = read_wire(&signed, ReadCapacity::default());
    let signed_end = end - 1 + signature.len();
    assert_eq!(result.segments[0].resumptions.len(), 0);
    assert_eq!(
        result.stop,
        Some(ReadStop::Framing(FramingFailure {
            reason: FramingReason::UndelimitableInput,
            offset: signed_end,
            length: signed.len() - signed_end,
        }))
    );
}

/// P5 6.6, the paragraph on a later version, and condition 6: the later-version case has five
/// conditions, and a second-form `signatures` head (`98 00` for the empty list) is not among them
/// (it is a condition 7 failure that the case does not judge). So an earlier parity frame in a
/// segment of a later version, with a `geometry` of a later version, whose envelope fails only
/// there still has no `first` and matches no span, and reading resumes at the next parity frame.
#[test]
fn an_earlier_parity_frame_in_a_later_version_segment_with_a_second_form_signatures_head_still_counts_as_the_later_version_case(
) {
    let base = vector_in(
        "P5",
        "parity-condition-six-first-by-case.later-version-geometry.bin",
    );
    let read = read_wire(&base, ReadCapacity::default());
    let end = read.segments[0].items[3].end;
    assert_eq!(base[end - 1], 0x80, "P0 ends with an empty signatures list");

    let mut second_form = base[..end - 1].to_vec();
    second_form.extend_from_slice(&[0x98, 0x00, 0x1c]);
    second_form.extend_from_slice(&base[end..]);

    let result = read_wire(&second_form, ReadCapacity::default());
    assert!(result.stop.is_none());
    assert_eq!(
        result.segments[0].resumptions,
        [FramingFailure {
            reason: FramingReason::UndelimitableInput,
            offset: end + 1,
            length: 1,
        }]
    );
}

/// P5 6.6, the paragraph on a later version: `storage` is as the type states when it is
/// `stored_plain`, whatever head form encodes it. A second-form storage tag (`81 18 01` for
/// `[1]`) is a condition 7 failure that the later-version case does not judge, so an earlier
/// parity frame in a segment of a later version, with a `geometry` of a later version, whose
/// envelope fails only there still has no `first`, and reading resumes at the next parity frame.
#[test]
fn an_earlier_parity_frame_in_a_later_version_segment_with_a_second_form_storage_tag_still_counts_as_the_later_version_case(
) {
    let base = vector_in(
        "P5",
        "parity-condition-six-first-by-case.later-version-geometry.bin",
    );
    let read = read_wire(&base, ReadCapacity::default());
    let parity = read.segments[0].items[3];
    let frame = &base[parity.offset..parity.end];
    let found: Vec<usize> = frame
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| *pair == [0x81, 0x01])
        .map(|(index, _)| parity.offset + index)
        .collect();
    let [storage] = found.as_slice() else {
        panic!("P0 holds `81 01` once, its storage field: {found:?}");
    };

    let mut second_form = base[..storage + 1].to_vec();
    second_form.push(0x18);
    second_form.extend_from_slice(&base[storage + 1..parity.end]);
    second_form.push(0x1c);
    second_form.extend_from_slice(&base[parity.end..]);

    let result = read_wire(&second_form, ReadCapacity::default());
    assert!(result.stop.is_none());
    assert_eq!(
        result.segments[0].resumptions,
        [FramingFailure {
            reason: FramingReason::UndelimitableInput,
            offset: parity.end + 1,
            length: 1,
        }]
    );
}

/// P5 7.2 step 6 part 3 and P5-38: the first parity candidate that the total decoded length
/// refuses is recorded with the position the next item would have, its offset, the length of its
/// item and the total at the refusal, and no later candidate passes.
#[test]
fn the_total_decoded_length_refusing_a_parity_candidate_is_recorded_on_the_segment() {
    let input = vector_in("P5", "recover-scan-charge.bin");
    let capacity = ReadCapacity {
        decoded_bytes_per_read: 1739,
        ..ReadCapacity::default()
    };
    let read = read_wire(&input, capacity);
    let [segment] = read.segments.as_slice() else {
        panic!("expected one segment");
    };
    assert_eq!(
        segment.parity_refusal,
        Some(ResyncRefusal {
            position: 2,
            offset: 2634,
            length: 501,
            count: 2086,
        })
    );
    assert_eq!(segment.resumptions.len(), 0);
    assert!(matches!(
        read.stop,
        Some(ReadStop::Framing(FramingFailure {
            reason: FramingReason::UndelimitableInput,
            ..
        }))
    ));
}

/// P5 7.2A: an item that is incomplete and followed by a parity resumption point is the damaged
/// region of an `undelimitable_input` finding, not a `torn_tail`.
#[test]
fn an_incomplete_item_before_a_parity_frame_is_a_damaged_region_and_not_a_torn_tail() {
    let input = vector_in(
        "P5",
        "recovery-parity-in-resync-segment.incomplete-item.bin",
    );
    let read = read_wire(&input, ReadCapacity::default());
    assert!(read.stop.is_none());
    let [_, segment] = read.segments.as_slice() else {
        panic!("expected two segments");
    };
    assert_eq!(
        segment.resumptions,
        [FramingFailure {
            reason: FramingReason::UndelimitableInput,
            offset: 274,
            length: 312,
        }]
    );
    assert_eq!(segment.items[1].kind, ItemKind::DamagedRegion);
    assert_ne!(
        segment.framing_failure.map(|failure| failure.reason),
        Some(FramingReason::TornTail)
    );
}

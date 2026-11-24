// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, path::Path};

use ciborium::Value;
use gmeow_gts::{
    v2::{
        reader::{
            read_wire, ClosingFinding, CommitCheck, Dispatch, FramingFailure, FramingReason,
            IndexOutcome, IndexWireClaims, ItemKind, LinkState, Opener, PayloadDamage,
            PayloadRefusal, PayloadState, ReadCapacity, ReadStop, ResyncRefusal, UncommittedTail,
        },
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
    assert!(report.checks.is_empty());
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
    assert!(read.segments[0].commits.as_ref().unwrap().checks.is_empty());

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
    assert!(read.segments[0].indexes.is_empty());
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

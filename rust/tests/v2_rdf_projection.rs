// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Projection of v2 content into RDF rows: nested objects, deduplication, inline typed literals and value lookup order.

use std::{path::Path, process::Command};

use ciborium::{cbor, Value};
use gmeow_gts::v2::{
    frame::Storage,
    rdf::{FrameIssue, ProjectionError, RdfProjection, Term, ValueIssue},
    reader::{read_wire, PayloadState, ReadCapacity},
    writer::SegmentWriter,
    Header, MalformedReason,
};
use gmeow_gts::wire::encode;

const ROUND_TRIP: &[u8] = include_bytes!("../../vectors/v2/P3/project__round-trip-rdf.bin");

fn project(bytes: &[u8]) -> RdfProjection {
    read_wire(bytes, ReadCapacity::default())
        .whole_rdf_projection()
        .expect("complete RDF projection")
}

#[test]
fn projection_keeps_nested_object_and_empty_graph() {
    let rdf = project(ROUND_TRIP);
    assert_eq!(rdf.quads().len(), 5);
    let name = *rdf.empty_graphs().iter().next().expect("empty graph");
    assert_eq!(rdf.empty_graphs().len(), 1);
    assert!(matches!(&rdf.terms()[name], Term::Iri(iri) if iri.as_ref() == "http://e/g3"));
    let text = rdf.to_nquads().unwrap();
    assert!(text.contains("<<( <http://e/s> <http://e/p> <<( <http://e/a> <http://e/b> <http://e/c> )>> )>> <http://e/g2> ."));
    assert!(text.contains("\"v\"^^<http://e/dt> <http://e/g> ."));
    assert!(text.contains("\"w\"@en ."));
    assert_eq!(text.lines().count(), 5);
}

#[test]
fn projection_deduplicates_equivalent_lines_and_scopes_blanks() {
    let collision = project(include_bytes!(
        "../../vectors/v2/P3/project__binding-quad-collision.bin"
    ));
    assert_eq!(collision.quads().len(), 1);
    assert!(collision.to_nquads().unwrap().contains("#reifies> <<("));

    let annotation = project(include_bytes!(
        "../../vectors/v2/P3/project__annotation-equals-quad.bin"
    ));
    assert_eq!(annotation.quads().len(), 1);

    let blank = project(include_bytes!("../../vectors/v2/P3/fold-blank-scope.bin"));
    let subjects: Vec<_> = blank
        .quads()
        .iter()
        .map(|&(subject, _, _, _)| &blank.terms()[subject])
        .collect();
    assert_eq!(subjects.len(), 2);
    assert!(matches!(subjects.as_slice(), [
        Term::Blank { segment: 0, label: first },
        Term::Blank { segment: 1, label: second },
    ] if first == second));
}

#[test]
fn projection_generates_inline_typed_lexical_forms() {
    let rdf = project(include_bytes!(
        "../../vectors/v2/P3/terms-value-each-kind.bin"
    ));
    let projected: Vec<_> = rdf
        .terms()
        .iter()
        .filter_map(|term| {
            let Term::Literal { lexical, datatype } = term else {
                return None;
            };
            let Term::Iri(iri) = &rdf.terms()[*datatype] else {
                return None;
            };
            Some((
                lexical.as_ref(),
                iri.split_once('#').expect("datatype fragment").1,
            ))
        })
        .collect();
    assert_eq!(
        projected,
        [
            ("false", "boolean"),
            ("0", "nonNegativeInteger"),
            ("-1", "integer"),
            ("0001-01-01T00:00:00Z", "dateTime"),
            ("9999-12-31T23:59:59.999999999Z", "dateTime"),
            ("1970-01-01T00:00:00.12Z", "dateTime"),
            ("", "hexBinary"),
        ]
    );
}

#[test]
fn projection_checks_inline_types_before_column_counts() {
    let mut segment = SegmentWriter::new(Header::default()).unwrap();
    let content = encode(&cbor!([0, [[6, 2]], 2, 1]).unwrap());
    // The column has one value instead of two. Its Boolean is a half-float
    // with a CBOR argument that would otherwise match the code for false.
    let payload = [0x82, 0x81, 0x82, 6, 0x81, 0x82, 1, 0xf9, 0, 20, 0x80];
    segment.append(1, &content, &payload).unwrap();
    let wire = read_wire(segment.as_bytes(), ReadCapacity::default());
    assert!(matches!(wire.whole_rdf_projection(),
        Err(ProjectionError::Frame { issue: FrameIssue::Content(error), .. })
            if error.reason == MalformedReason::WrongType
    ));
}

#[test]
fn reference_columns_and_language_table_precede_value_lookup() {
    let digest = Value::Bytes(vec![0; 32]);
    let content = encode(&cbor!([0, [[5, 1]], 2, 1]).unwrap());
    let unused = encode(&cbor!([[[5, [digest], [1], [2], [], [0]]], ["en", "unused"]]).unwrap());
    let malformed = [0x82, 0x81, 0x86, 5, 0x84, 0xf5];
    for (payload, expected) in [
        (unused.as_slice(), MalformedReason::NotDeterministic),
        (malformed.as_slice(), MalformedReason::WrongType),
    ] {
        let mut segment = SegmentWriter::new(Header::default()).unwrap();
        segment.append(1, &content, payload).unwrap();
        let wire = read_wire(segment.as_bytes(), ReadCapacity::default());
        assert!(matches!(wire.whole_rdf_projection(),
            Err(ProjectionError::Frame { issue: FrameIssue::Content(error), .. })
                if error.reason == expected
        ));
    }
}

#[test]
fn projection_uses_first_verified_earlier_value() {
    for fixture in [
        include_bytes!("../../vectors/v2/P3/terms-literal-ref-first-candidate.failing-first.bin")
            .as_slice(),
        include_bytes!("../../vectors/v2/P4/rebuild-value-first-verified.first-fails.bin")
            .as_slice(),
        include_bytes!("../../vectors/v2/P4/rebuild-value-other-segment.value-resolves.bin")
            .as_slice(),
    ] {
        assert_eq!(
            project(fixture).to_nquads().unwrap(),
            "<http://e/s> <http://e/p> \"abc\"^^<http://e/dt> .\n"
        );
    }
    for (fixture, expected) in [
        (
            include_bytes!(
                "../../vectors/v2/P3/terms-literal-ref-value-after.same-segment-after.bin"
            )
            .as_slice(),
            ValueIssue::MissingValue,
        ),
        (
            include_bytes!("../../vectors/v2/P3/terms-literal-ref-length-pair.length-4.bin")
                .as_slice(),
            ValueIssue::MissingValue,
        ),
        (
            include_bytes!(
                "../../vectors/v2/P4/rebuild-value-other-segment.unit-in-term-segment.bin"
            )
            .as_slice(),
            ValueIssue::MissingUnit,
        ),
        (
            include_bytes!("../../vectors/v2/P3/terms-literal-ref-not-utf8.bin").as_slice(),
            ValueIssue::InvalidUtf8,
        ),
    ] {
        let wire = read_wire(fixture, ReadCapacity::default());
        assert!(
            matches!(wire.whole_rdf_projection(), Err(ProjectionError::Frame {
            issue: FrameIssue::Value(issue), ..
        }) if issue == expected)
        );
    }
    let compressed = include_bytes!("../../vectors/v2/P4/pack-compressed.one-frame.bin");
    assert!(project(compressed).quads().is_empty());
    let malformed = include_bytes!("../../vectors/v2/P4/pack-recipe-trailing-count.bin");
    let wire = read_wire(malformed, ReadCapacity::default());
    assert!(
        matches!(wire.whole_rdf_projection(), Err(ProjectionError::Frame {
        issue: FrameIssue::Content(error), ..
    }) if error.reason == MalformedReason::WrongLength)
    );
}

fn append_referenced_literals(segment: &mut SegmentWriter, digest: &[u8; 32]) {
    let iris = Value::Bytes([b"http://e/s".as_slice(), b"http://e/dt".as_slice()].concat());
    let name = || Value::Bytes(digest.to_vec());
    let first = name();
    let duplicate = name();
    let octets = name();
    let content = encode(&cbor!([0, [[1, 2], [5, 2], [7, 1]], 2, 1]).unwrap());
    let body = encode(
        &cbor!([
            [
                [1, [0, 0], [10, 11], iris],
                [5, [first, duplicate], [2, 2], [1, 1], [1, 1], []],
                [7, [octets], [2]]
            ],
            []
        ])
        .unwrap(),
    );
    segment.append(1, &content, &body).unwrap();
    let content = encode(&cbor!([2, 3]).unwrap());
    let body = encode(&cbor!([[[[1], 3]], [[0, 3]], [2, 1, 1], [0, 0, 0]]).unwrap());
    segment.append(2, &content, &body).unwrap();
}

#[test]
fn pack_value_can_use_later_members_and_share_reference_text() {
    let mut segment = SegmentWriter::new(Header::default()).unwrap();
    let digest = blake3::derive_key("gts2 unit", "é".as_bytes());
    let first = Value::Bytes(blake3::derive_key("gts2 unit", &[0xc3]).to_vec());
    let last = Value::Bytes(blake3::derive_key("gts2 unit", &[0xa9]).to_vec());
    let value = Value::Bytes(digest.to_vec());
    let entries = encode(&cbor!([[first.clone(), 1], [last.clone(), 1]]).unwrap());
    let size = entries.len() as u64;
    let mut bodies = entries;
    bodies.extend([0, 0xc3, 0xa9]);
    let bodies = Value::Bytes(bodies);
    let payload = encode(
        &cbor!([
            [
                [4, [value, 2, 2], size],
                [1, [first.clone()], 1],
                [1, [first], 1],
                [2, [last, [1]], 1]
            ],
            bodies
        ])
        .unwrap(),
    );
    segment
        .append(13, &encode(&cbor!([4, 3]).unwrap()), &payload)
        .unwrap();
    append_referenced_literals(&mut segment, &digest);

    let wire = read_wire(segment.as_bytes(), ReadCapacity::default());
    assert!(matches!(
        wire.whole_rdf_projection_with_value_limit(5),
        Err(ProjectionError::Frame {
            issue: FrameIssue::Value(ValueIssue::MaterializedCapacity),
            ..
        })
    ));
    let rdf = wire.whole_rdf_projection_with_value_limit(6).unwrap();
    assert_eq!(rdf.quads().len(), 2);
    let nq = rdf.to_nquads().unwrap();
    assert!(nq.contains("\"é\"^^<http://e/dt>"));
    assert!(nq.contains("\"C3A9\"^^<http://www.w3.org/2001/XMLSchema#hexBinary>"));
}

#[test]
fn recipe_charges_can_refuse_later_literal_values() {
    let mut segment = SegmentWriter::new(Header::default()).unwrap();
    let digest = blake3::derive_key("gts2 unit", b"ab");
    let name = || Value::Bytes(digest.to_vec());
    segment
        .append(8, &encode(&cbor!([name()]).unwrap()), b"ab")
        .unwrap();
    let too_big = (1_u64 << 37) + 1;
    let large_unit = too_big - 1;
    segment
        .append(
            10,
            &encode(&cbor!([name(), too_big, [1], 2]).unwrap()),
            &encode(&cbor!([[name(), large_unit], [name(), 1]]).unwrap()),
        )
        .unwrap();
    segment
        .append(
            11,
            &encode(&cbor!([name(), 2, 1]).unwrap()),
            &encode(&cbor!([[name(), 2]]).unwrap()),
        )
        .unwrap();
    append_referenced_literals(&mut segment, &digest);
    let wire = read_wire(segment.as_bytes(), ReadCapacity::default());
    assert!(matches!(
        wire.whole_rdf_projection(),
        Err(ProjectionError::Frame {
            issue: FrameIssue::Value(ValueIssue::RebuiltCapacity),
            ..
        })
    ));
}

#[test]
fn annotation_duplicates_use_storage_identity_inside_triples() {
    for (first, second, duplicate) in [(2, 3, false), (8, 9, false), (3, 4, true), (9, 10, true)] {
        let mut segment = SegmentWriter::new(Header::default()).unwrap();
        let datatype = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
        let iris = Value::Bytes(format!("http://e/s{datatype}").into_bytes());
        let dt_length = datatype.len() as u64;
        let lexical = Value::Bytes(vec![b'7']);
        segment
            .append(
                1,
                &encode(&cbor!([0, [[1, 2], [2, 1], [6, 2], [4, 6]], 2, 1]).unwrap()),
                &encode(
                    &cbor!([
                        [
                            [1, [0, 0], [10, dt_length], iris],
                            [2, [0], [1], lexical, [1], [1], []],
                            [6, [[2, 7], [2, 7]]],
                            [
                                4,
                                [0, 0, 0, 0, 0, 0],
                                [0, 0, 0, 0, 0, 0],
                                [2, 3, 4, 5, 6, 7]
                            ]
                        ],
                        []
                    ])
                    .unwrap(),
                ),
            )
            .unwrap();
        let delta = second - first;
        segment
            .append(
                4,
                &encode(&cbor!([2, 2]).unwrap()),
                &encode(&cbor!([[[[1], 2]], [0, 0], [0, 0], [first, delta], [1, 0]]).unwrap()),
            )
            .unwrap();
        let wire = read_wire(segment.as_bytes(), ReadCapacity::default());
        let result = wire.whole_rdf_projection();
        if duplicate {
            assert!(matches!(result,
                Err(ProjectionError::Frame { issue: FrameIssue::Content(error), .. })
                    if error.reason == MalformedReason::UnorderedOrRepeated
            ));
        } else {
            assert_eq!(result.unwrap().quads().len(), 1);
        }
    }
}

#[test]
fn projection_preserves_text_and_refuses_unreadable_content() {
    let invalid = project(include_bytes!(
        "../../vectors/v2/P3/project__malformed-iri.bin"
    ));
    assert_eq!(invalid.quads().len(), 1);
    assert!(invalid.to_nquads().is_err());

    let languages = project(include_bytes!(
        "../../vectors/v2/P3/project__language-case.bin"
    ));
    let text = languages.to_nquads().unwrap();
    assert!(text.contains("\"x\"@EN ."));
    assert!(text.contains("\"x\"@en ."));

    let prefix =
        include_bytes!("../../vectors/v2/P3/terms-front-term-zero-prefix.prefix-5-at-term-0.bin");
    assert!(matches!(
        read_wire(prefix, ReadCapacity::default()).whole_rdf_projection(),
        Err(ProjectionError::Frame { issue: FrameIssue::Content(error), .. })
            if error.reason == MalformedReason::ForbiddenCombination
    ));

    let suppress = include_bytes!("../../vectors/v2/P3/suppress-unmatched-term.bin");
    assert!(matches!(
        read_wire(suppress, ReadCapacity::default()).whole_rdf_projection(),
        Err(ProjectionError::Frame {
            issue: FrameIssue::UnsupportedFrameType(7),
            ..
        })
    ));
}

#[test]
fn projection_refuses_bad_storage_check_with_verified_bytes() {
    let input = include_bytes!("../../vectors/v2/P1/zstd-ok.bin");
    let wire = read_wire(input, ReadCapacity::default());
    let frame = wire.segments[0].frames[0].envelope.as_ref().unwrap();
    let Storage::Compressed { stored_check, .. } = frame.storage else {
        panic!("expected a compressed fixture");
    };
    let offset = input
        .windows(stored_check.len())
        .position(|bytes| bytes == stored_check)
        .unwrap();
    let mut damaged = input.to_vec();
    damaged[offset] ^= 1;
    let wire = read_wire(&damaged, ReadCapacity::default());
    assert!(matches!(
        wire.segments[0].frames[0].payload.state,
        PayloadState::Verified(_)
    ));
    assert!(matches!(
        wire.whole_rdf_projection(),
        Err(ProjectionError::Frame {
            issue: FrameIssue::StoredCheck,
            ..
        })
    ));
}

#[test]
fn fold_cli_exports_v2_and_continues_reading_v1() {
    let vectors = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectors");
    let fold = |path| {
        Command::new(env!("CARGO_BIN_EXE_gts"))
            .arg("fold")
            .arg(vectors.join(path))
            .output()
            .unwrap()
    };
    let output = fold("v2/P3/project__round-trip-rdf.bin");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        project(ROUND_TRIP).to_nquads().unwrap().as_bytes()
    );
    let value = fold("v2/P3/project__value-pair.bin");
    assert!(
        value.status.success(),
        "{}",
        String::from_utf8_lossy(&value.stderr)
    );
    assert_eq!(value.stdout, b"<http://e/s> <http://e/p> \"7\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> .\n");

    let legacy = fold("11-datatype-defaulting.gts");
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert_ne!(legacy.stdout, [] as [u8; 0]);

    let withheld = fold("v2/P3/suppress-unmatched-term.bin");
    assert!(!withheld.status.success());
    assert_eq!(withheld.stdout, [] as [u8; 0]);
}

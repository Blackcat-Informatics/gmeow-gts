// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{path::Path, process::Command};

use gmeow_gts::v2::{
    frame::Storage,
    rdf::{FrameIssue, ProjectionError, RdfProjection, Term},
    reader::{read_wire, PayloadState, ReadCapacity},
    MalformedReason,
};

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

    let legacy = fold("11-datatype-defaulting.gts");
    assert!(
        legacy.status.success(),
        "{}",
        String::from_utf8_lossy(&legacy.stderr)
    );
    assert!(!legacy.stdout.is_empty());

    let withheld = fold("v2/P3/suppress-unmatched-term.bin");
    assert!(!withheld.status.success());
    assert!(withheld.stdout.is_empty());
}

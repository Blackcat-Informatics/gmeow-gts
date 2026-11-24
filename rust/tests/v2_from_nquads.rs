// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::fmt::Write;

use ciborium::Value;
use gmeow_gts::v2::{
    from_nquads,
    reader::{read_wire, Dispatch, LinkState, PayloadState, ReadCapacity, WireFrame},
    ImportError,
};

fn parse(encoded: &[u8]) -> Value {
    ciborium::from_reader(encoded).expect("deterministic CBOR")
}

fn decode(frame: &WireFrame<'_>) -> Value {
    let PayloadState::Verified(bytes) = &frame.payload.state else {
        panic!("payload did not verify");
    };
    parse(bytes)
}

fn uint(value: &Value) -> usize {
    usize::try_from(i128::from(value.as_integer().unwrap())).unwrap()
}

fn array(value: &Value) -> &[Value] {
    value.as_array().unwrap()
}

#[test]
fn imports_rdf_projection_as_v2_quads_with_typed_terms() {
    let source = concat!(
        "<https://e/s> <https://e/p> \"v\" .\n",
        "<https://e/s> <https://e/p> \"v\"^^<http://www.w3.org/2001/XMLSchema#string> .\n",
        "<https://e/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ",
        "<<( <https://e/s> <https://e/p> \"v\" )>> _:g .\n",
        "_:s <https://e/p> \"abé\"@FR--ltr _:g .\n",
        "_:s <https://e/p> \"abè\"@fr--rtl _:g .\n",
    );
    let bytes = from_nquads(source).expect("import N-Quads");
    let read = read_wire(&bytes, ReadCapacity::default());
    assert_eq!(read.dispatch, Some(Dispatch::SecondGeneration));
    assert!(read.stop.is_none());
    let [terms, quads, commit] = read.segments[0].frames.as_slice() else {
        panic!("expected terms, quads and a commit");
    };
    for (frame, kind) in [(terms, 1), (quads, 2), (commit, 14)] {
        assert_eq!(frame.frame_type, Some(kind));
        assert_eq!(frame.link.state, LinkState::Linked);
    }
    let dictionary = decode(terms);
    let blocks = array(&array(&dictionary)[0]);
    let typed = array(&blocks[1]);
    assert_eq!(
        uint(&array(&typed[5])[0]),
        2,
        "plain string uses its datatype IRI"
    );
    let triple = array(&blocks[3]);
    assert_eq!(
        uint(&array(&triple[3])[0]),
        3,
        "quoted literal uses the same term ID"
    );
    let language_block = array(&blocks[5]);
    assert_eq!(
        uint(&array(&language_block[1])[1]),
        3,
        "common prefix includes a UTF-8 lead byte"
    );
    assert_eq!(array(&language_block[4]), [Value::from(3), Value::from(4)]);
    assert_eq!(
        array(&array(&dictionary)[1]),
        [Value::Text("FR".into()), Value::Text("fr".into())]
    );

    let quad_content = parse(quads.envelope.as_ref().unwrap().content);
    assert_eq!(array(&quad_content), [Value::from(2), Value::from(4)]);
    let quad_payload = decode(quads);
    let graph_groups = array(&array(&quad_payload)[0]);
    assert_eq!(uint(&array(&graph_groups[0])[1]), 1);
    assert_eq!(uint(&array(&graph_groups[1])[1]), 3);
    let commit_content = parse(commit.envelope.as_ref().unwrap().content);
    assert_eq!(uint(&array(&commit_content)[0]), 2);
}

#[test]
fn keeps_dictionary_ids_across_frames_and_resets_front_coding_at_restarts() {
    let mut input = String::new();
    for index in 0..4_100 {
        writeln!(input, "<https://e/s{index}> <https://e/p> <https://e/o> .").unwrap();
    }
    let bytes = from_nquads(&input).unwrap();
    let read = read_wire(&bytes, ReadCapacity::default());
    let frames = &read.segments[0].frames;
    let [first, later, quads, commit] = frames.as_slice() else {
        panic!("expected two dictionary frames, quads and a commit");
    };
    assert!(frames
        .iter()
        .all(|frame| frame.link.state == LinkState::Linked));
    let first_payload = decode(first);
    let iri_block = array(&array(&array(&first_payload)[0])[0]);
    let prefixes = array(&iri_block[1]);
    assert_eq!(uint(&prefixes[0]), 0);
    assert!(uint(&prefixes[63]) > 0);
    assert_eq!(uint(&prefixes[64]), 0);
    assert!(uint(&prefixes[65]) > 0);
    let later_content = parse(later.envelope.as_ref().unwrap().content);
    assert_eq!(uint(&array(&later_content)[0]), prefixes.len());
    assert_eq!(quads.frame_type, Some(2));
    assert_eq!(commit.frame_type, Some(14));
}

#[test]
fn refuses_triple_subjects_outside_the_v2_position_rules() {
    let source = "<<( <https://e/s> <https://e/p> <https://e/o> )>> <https://e/p> <https://e/o> .";
    assert!(matches!(from_nquads(source), Err(ImportError::Syntax(_))));
}

#[test]
fn preserves_carriage_return_separated_statements_and_escaped_terms() {
    let source = concat!(
        "VERSION \"1.2\"\r",
        "_:猫 <https://e/\\u0070\\u00a0> \"it\\'s\" . # comment\r",
        "_:猫 <https://e/p\u{a0}> \"other\" _:猫.\r\n",
    );
    let bytes = from_nquads(source).unwrap();
    let read = read_wire(&bytes, ReadCapacity::default());
    let frames = &read.segments[0].frames;
    let dictionary = decode(&frames[0]);
    let blocks = array(&array(&dictionary)[0]);
    assert_eq!(
        array(&array(&blocks[0])[1])[0].as_bytes().unwrap(),
        "猫".as_bytes()
    );
    assert!(array(&blocks[1])[3]
        .as_bytes()
        .unwrap()
        .starts_with("https://e/p\u{a0}".as_bytes()));
    assert_eq!(array(&blocks[2])[3].as_bytes().unwrap(), b"it'sother");

    let quad_content = parse(frames[1].envelope.as_ref().unwrap().content);
    assert_eq!(uint(&array(&quad_content)[1]), 2);
    for bad in [
        "<https://e/s> <https://e/p> <https://e/o>",
        "<https://e/s> <https://e/p> <https://e/o> . undiscarded",
        "<https://e/%ZZ> <https://e/p> <https://e/o> .",
        "<https://[]> <https://e/p> <https://e/o> .",
        "<https://e/s> <https://e/p> \"value\"@x .",
    ] {
        assert!(matches!(from_nquads(bad), Err(ImportError::Syntax(_))));
    }
}

#[test]
fn deep_triple_objects_use_earlier_ids_without_recursive_parsing() {
    const DEPTH: usize = 16_384;
    let mut source = String::from("<https://e/s> <https://e/p> ");
    source.push_str(&"<<( <https://e/s> <https://e/p> ".repeat(DEPTH));
    source.push_str("<https://e/o>");
    source.push_str(&" )>>".repeat(DEPTH));
    source.push_str(" .");
    let bytes = from_nquads(&source).unwrap();
    let read = read_wire(&bytes, ReadCapacity::default());
    let mut terms = 0;
    for frame in &read.segments[0].frames {
        if frame.frame_type != Some(1) {
            continue;
        }
        let dictionary = decode(frame);
        for block in array(&array(&dictionary)[0]) {
            let fields = array(block);
            if uint(&fields[0]) == 4 {
                for object in array(&fields[3]) {
                    assert_eq!(uint(object), terms + 2);
                    terms += 1;
                }
            }
        }
    }
    assert_eq!(terms, DEPTH);
    let rdf = read.whole_rdf_projection().unwrap();
    assert_eq!(rdf.to_nquads().unwrap().trim_end(), source);
}

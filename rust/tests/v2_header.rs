// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::{fs, path::PathBuf};

use ciborium::value::Value;
use gmeow_gts::v2::{decode_header, encode_header, Header, HeaderWriteError, MalformedReason};

fn array(value: &Value) -> &[Value] {
    value.as_array().expect("corpus array")
}

fn uint(value: &Value) -> u64 {
    value
        .as_integer()
        .expect("corpus integer")
        .try_into()
        .expect("corpus uint")
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

fn reason(code: u64) -> MalformedReason {
    match code {
        3 => MalformedReason::NotDeterministic,
        4 => MalformedReason::WrongType,
        5 => MalformedReason::WrongLength,
        6 => MalformedReason::OutOfRange,
        7 => MalformedReason::UnregisteredValue,
        _ => panic!("unexpected header finding reason {code}"),
    }
}

#[test]
fn header_outcomes_and_identifiers_match_p1() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors/v2/P1");
    for name in [
        "header-minimal",
        "header-lineage",
        "header-hash-newer",
        "header-registry-newer",
        "header-profile-unknown",
        "header-previous-head-unheld-algorithm",
        "header-registry-zero",
        "header-profile-zero",
        "header-hash-unregistered",
        "header-lineage-digest-short",
        "header-lineage-ordinal-zero",
        "header-length-six",
        "header-length-eight",
        "header-long-form-field",
        "header-long-form-first-item",
        "header-element0-long-form-later",
        "header-two-violations-byte-order",
    ] {
        let fixture: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join(format!("{name}.json"))).expect("P1 header fixture"),
        )
        .expect("fixture JSON");
        for case in fixture["cases"].as_array().expect("fixture cases") {
            let data = fs::read(root.join(case["input_file"].as_str().expect("fixture input")))
                .expect("fixture bytes");
            let expected = unhex(case["read_result"].as_str().expect("typed read result"));
            let report: Value = ciborium::de::from_reader(expected.as_slice()).expect("P7 CBOR");
            let segments = array(
                &array(&report)
                    .last()
                    .expect("file result")
                    .as_array()
                    .expect("file record")[1],
            );
            for segment in segments {
                let fields = array(segment);
                let start = usize::try_from(uint(&fields[1])).expect("header offset");
                let registry = array(&fields[5]);
                let decoded = decode_header(&data, start);
                if uint(&registry[0]) == 1 {
                    let finding = array(fields.last().expect("segment findings"))
                        .iter()
                        .map(array)
                        .find(|finding| uint(&finding[0]) == 7)
                        .expect("malformed header finding");
                    assert_eq!(
                        decoded.expect_err(name).reason,
                        reason(uint(&array(&finding[1])[1])),
                        "{name} at {start}"
                    );
                } else {
                    let decoded = decoded.expect(name);
                    assert_eq!(
                        decoded.header.registry_version,
                        uint(&registry[1]),
                        "{name}"
                    );
                    let identifier = array(&fields[4]);
                    if uint(&identifier[0]) == 1 {
                        assert_eq!(decoded.identifier, None, "{name}");
                    } else {
                        assert_eq!(
                            decoded
                                .identifier
                                .as_ref()
                                .expect("readable header")
                                .as_slice(),
                            identifier[1].as_bytes().expect("P7 digest"),
                            "{name}"
                        );
                    }
                    if matches!(name, "header-minimal" | "header-lineage") {
                        assert_eq!(
                            encode_header(decoded.header).expect("writable header"),
                            data[start..decoded.next_offset],
                            "{name}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn truncated_prefixes_produce_header_errors() {
    let header = encode_header(Header::default()).expect("minimal header");
    for prefix_length in 0..header.len() {
        assert_eq!(
            decode_header(&header[..prefix_length], 0)
                .expect_err("truncated header")
                .reason,
            MalformedReason::WrongLength
        );
    }
}

#[test]
fn writer_refuses_a_registry_it_does_not_implement() {
    let header = Header {
        registry_version: 2,
        ..Header::default()
    };
    assert_eq!(
        encode_header(header),
        Err(HeaderWriteError::RegistryVersion(2))
    );
}

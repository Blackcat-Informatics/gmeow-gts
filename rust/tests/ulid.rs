// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::str::FromStr;

use gmeow_gts::ulid::{DeterministicUlidGenerator, Ulid};

#[test]
fn ulid_renders_canonical_crockford_base32() {
    let zero = Ulid::from_parts(0, [0; 10]).expect("zero ULID is valid");
    assert_eq!(zero.to_string(), "00000000000000000000000000");
    assert_eq!(format!("{zero:?}"), "Ulid(00000000000000000000000000)");

    let max = Ulid::from_parts(Ulid::MAX_TIMESTAMP_MS, [0xff; 10]).expect("max ULID is valid");
    assert_eq!(max.to_string(), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");
}

#[test]
fn deterministic_ulids_order_by_timestamp_then_counter() {
    let first = Ulid::from_counter(0, 1).expect("counter fits");
    let second = Ulid::from_counter(0, 2).expect("counter fits");
    let later_timestamp = Ulid::from_counter(1, 0).expect("timestamp fits");

    assert_eq!(first.to_string(), "00000000000000000000000001");
    assert!(first < second);
    assert!(second < later_timestamp);

    let mut generator = DeterministicUlidGenerator::with_counter(0, 1).expect("generator starts");
    assert_eq!(
        generator.next_ulid().expect("first").to_string(),
        first.to_string()
    );
    assert_eq!(
        generator.next_ulid().expect("second").to_string(),
        second.to_string()
    );
}

#[test]
fn ulid_parse_roundtrips_and_rejects_invalid_input() {
    let parsed = Ulid::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").expect("canonical ULID parses");
    assert_eq!(parsed.to_string(), "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    assert_eq!(parsed.timestamp_ms(), 1_469_922_850_259);

    assert!(Ulid::from_str("01ARZ3NDEKTSV4RRFFQ69G5FA").is_err());
    assert!(Ulid::from_str("81ARZ3NDEKTSV4RRFFQ69G5FAV").is_err());
    assert!(Ulid::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAI").is_err());
}

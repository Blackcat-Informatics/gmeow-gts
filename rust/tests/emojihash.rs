// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-engine emojihash conformance (`vectors/emojihash/*.json`).

use std::path::Path;

use gmeow_gts::emojihash::{emoji_indices, emojihash, emojihash_labels};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn emojihash_vectors() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectors/emojihash");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("vectors/emojihash must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let data = unhex(json["data"].as_str().unwrap());
        let length = json["length"].as_u64().unwrap() as usize;

        let want_indices: Vec<usize> = json["indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(emoji_indices(&data, length), want_indices, "{path:?}");
        assert_eq!(emojihash(&data, length), json["emoji"].as_str().unwrap());
        assert_eq!(
            emojihash_labels(&data, length),
            json["labels"].as_str().unwrap()
        );
        count += 1;
    }
    assert!(count >= 4, "expected emojihash vectors, found {count}");
}

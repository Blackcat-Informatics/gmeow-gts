// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Conformance against the frozen vectors (the Python reference is the oracle).
//! While `visual-hashing` lives in the gmeow-gts monorepo, the vectors are
//! shared at `../vectors`; they travel with the crate if it breaks out.

use std::path::{Path, PathBuf};

use visual_hashing::{emoji_indices, emojihash, emojihash_labels, randomart};

/// The shared conformance corpus lives in the gmeow-gts monorepo at `../vectors`
/// and does not ship inside the published crate tarball. Returns `None` (so the
/// test skips) when it is absent — e.g. when someone runs `cargo test` on the
/// crate downloaded from crates.io. In the source repo it is always present.
fn vectors_dir(kind: &str) -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectors")
        .join(kind);
    dir.is_dir().then_some(dir)
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

#[test]
fn emojihash_vectors() {
    let Some(dir) = vectors_dir("emojihash") else {
        eprintln!("skipping: shared vectors/emojihash corpus not present");
        return;
    };
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

#[test]
fn randomart_vectors() {
    let Some(dir) = vectors_dir("randomart") else {
        eprintln!("skipping: shared vectors/randomart corpus not present");
        return;
    };
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("vectors/randomart must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let data = unhex(json["data"].as_str().unwrap());
        let label = json["label"].as_str().unwrap();
        assert_eq!(
            randomart(&data, label),
            json["art"].as_str().unwrap(),
            "{path:?}"
        );
        count += 1;
    }
    assert!(count >= 5, "expected randomart vectors, found {count}");
}

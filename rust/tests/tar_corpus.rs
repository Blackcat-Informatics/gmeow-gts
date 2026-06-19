// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "tar")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gmeow_gts::files::{
    read_entries, unpack_with_options, FileEntry, FileEntryKind, UnpackOptions,
};
use gmeow_gts::from_tar::{from_tar_bytes, FromTarOptions};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::tar::{to_tar_vec, ToTarOptions};
use serde_json::Value;

fn vectors() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../vectors")
}

fn tmpdir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-tar-corpus-{name}-{}-{n}", std::process::id()))
}

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn new(name: &str) -> Self {
        let path = tmpdir(name);
        let _ = std::fs::remove_dir_all(&path);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn tar_manifest_entries() -> Vec<Value> {
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(vectors().join("manifest.json")).unwrap()).unwrap();
    let mut entries: Vec<Value> = manifest["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["subsets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|subset| subset.as_str() == Some("tar-archive"))
        })
        .cloned()
        .collect();
    entries.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    entries
}

fn tar_expectation_names() -> BTreeSet<String> {
    std::fs::read_dir(vectors().join("tar"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter_map(|name| {
            name.strip_suffix(".expected.json")
                .map(std::string::ToString::to_string)
        })
        .collect()
}

fn fixture_name(entry: &Value) -> &str {
    entry["id"].as_str().unwrap().strip_prefix("tar-").unwrap()
}

fn load_expectation(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(vectors().join("tar").join(format!("{name}.expected.json"))).unwrap(),
    )
    .unwrap()
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn from_tar_options(expectation: &Value) -> FromTarOptions {
    let mut options = FromTarOptions {
        source_name: Some(expectation["input"].as_str().unwrap().to_string()),
        ..FromTarOptions::default()
    };
    for arg in string_array(expectation, "from_tar_args") {
        match arg.as_str() {
            "--allow-symlinks" => options.allow_symlinks = true,
            "--allow-special" => options.allow_special = true,
            "--owner" => options.owner = true,
            other => panic!("unsupported from-tar corpus argument: {other}"),
        }
    }
    options
}

fn unpack_options(expectation: &Value) -> UnpackOptions {
    let mut options = UnpackOptions::default();
    for arg in string_array(expectation, "extract_args") {
        match arg.as_str() {
            "--allow-symlinks" => options.allow_symlinks = true,
            "--allow-special" => options.allow_special = true,
            "--same-owner" | "--numeric-owner" => options.same_owner = true,
            "--include-suppressed" => options.include_suppressed = true,
            "--preserve-setid" => options.preserve_setid = true,
            other => panic!("unsupported extract corpus argument: {other}"),
        }
    }
    options
}

fn entry_kind(kind: FileEntryKind) -> &'static str {
    match kind {
        FileEntryKind::File => "file",
        FileEntryKind::Directory => "directory",
        FileEntryKind::Symlink => "symlink",
        FileEntryKind::Hardlink => "hardlink",
        FileEntryKind::Fifo => "fifo",
        FileEntryKind::CharDev => "chardev",
        FileEntryKind::BlockDev => "blockdev",
        FileEntryKind::Socket => "socket",
    }
}

fn optional_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn optional_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn assert_entry_matches(expected: &Value, actual: &FileEntry) {
    assert_eq!(
        entry_kind(actual.kind),
        expected["kind"].as_str().unwrap(),
        "entry kind for {}",
        expected["path"].as_str().unwrap()
    );
    if let Some(size) = optional_u64(expected, "size") {
        assert_eq!(actual.size, Some(size), "size for {}", actual.path);
    }
    if let Some(mode) = optional_u64(expected, "mode") {
        assert_eq!(actual.mode, Some(mode as u32), "mode for {}", actual.path);
    }
    if let Some(uid) = optional_u64(expected, "uid") {
        assert_eq!(actual.uid, Some(uid), "uid for {}", actual.path);
    }
    if let Some(gid) = optional_u64(expected, "gid") {
        assert_eq!(actual.gid, Some(gid), "gid for {}", actual.path);
    }
    if let Some(name) = optional_string(expected, "user_name") {
        assert_eq!(
            actual.user_name.as_deref(),
            Some(name),
            "user for {}",
            actual.path
        );
    }
    if let Some(name) = optional_string(expected, "group_name") {
        assert_eq!(
            actual.group_name.as_deref(),
            Some(name),
            "group for {}",
            actual.path
        );
    }
    if let Some(target) = optional_string(expected, "link_target") {
        assert_eq!(
            actual.link_target.as_deref(),
            Some(target),
            "link target for {}",
            actual.path
        );
    }
    if let Some(dev_major) = optional_u64(expected, "dev_major") {
        assert_eq!(
            actual.dev_major,
            Some(dev_major),
            "dev major for {}",
            actual.path
        );
    }
    if let Some(dev_minor) = optional_u64(expected, "dev_minor") {
        assert_eq!(
            actual.dev_minor,
            Some(dev_minor),
            "dev minor for {}",
            actual.path
        );
    }
    if let Some(expected_pax) = expected.get("pax_records") {
        let actual_pax: Vec<Value> = actual
            .pax_records
            .iter()
            .map(|record| {
                serde_json::json!({"key": record.key.as_str(), "value": record.value.as_str()})
            })
            .collect();
        assert_eq!(
            actual_pax,
            expected_pax.as_array().unwrap().clone(),
            "PAX records for {}",
            actual.path
        );
    }
}

fn assert_expected_entries(expectation: &Value, archive: &[u8]) {
    let graph = read(archive, true, None);
    let entries = read_entries(&graph).expect("files-profile entries read");
    let expected_entries = expectation["expected_entries"].as_array().unwrap();
    let expected_paths: BTreeSet<&str> = expected_entries
        .iter()
        .map(|entry| entry["path"].as_str().unwrap())
        .collect();
    let actual_paths: BTreeSet<&str> = entries.keys().map(String::as_str).collect();
    assert_eq!(
        actual_paths,
        expected_paths,
        "entry path set for {}",
        expectation["fixture"].as_str().unwrap()
    );
    for expected in expected_entries {
        let path = expected["path"].as_str().unwrap();
        assert_entry_matches(expected, entries.get(path).unwrap());
    }
}

fn assert_folded_graph(expectation: &Value, archive: &[u8]) {
    let expected_graph = expectation["folded_nq"].as_str().unwrap();
    let expected = std::fs::read_to_string(vectors().join("tar").join(expected_graph)).unwrap();
    let actual = to_nquads(&read(archive, true, None));
    assert_eq!(
        actual,
        expected,
        "folded graph drift for {}",
        expectation["fixture"].as_str().unwrap()
    );
}

fn assert_expected_error(err: &str, expectation: &Value) {
    for needle in string_array(expectation, "expected_error_contains") {
        assert!(
            err.contains(&needle),
            "{} error {err:?} did not contain {needle:?}",
            expectation["fixture"].as_str().unwrap()
        );
    }
}

#[test]
fn tar_corpus_matches_pinned_expectations() {
    let entries = tar_manifest_entries();
    let manifest_fixtures: BTreeSet<String> = entries
        .iter()
        .map(|entry| fixture_name(entry).to_string())
        .collect();
    assert!(
        !manifest_fixtures.is_empty(),
        "tar corpus manifest entries must not be empty"
    );
    assert_eq!(
        manifest_fixtures,
        tar_expectation_names(),
        "tar manifest entries must cover every expectation sidecar"
    );

    for manifest_entry in entries {
        let fixture = fixture_name(&manifest_entry);
        let expectation = load_expectation(fixture);
        let input_path = vectors()
            .join("tar")
            .join(expectation["input"].as_str().unwrap());
        assert_eq!(
            manifest_entry["expected"]["fixture_kind"], expectation["kind"],
            "manifest kind for {fixture}"
        );
        assert_eq!(
            manifest_entry["expected"]["entries"].as_u64().unwrap(),
            expectation["expected_entries"].as_array().unwrap().len() as u64,
            "manifest entry count for {fixture}"
        );

        let input = std::fs::read(&input_path).unwrap();
        let options = from_tar_options(&expectation);
        match expectation["kind"].as_str().unwrap() {
            "from-tar-refusal" => {
                let err = from_tar_bytes(&input, &options).expect_err("fixture must be refused");
                assert_expected_error(&err.to_string(), &expectation);
            }
            "extract-refusal" => {
                let archive = from_tar_bytes(&input, &options).expect("tar fixture imports");
                assert_folded_graph(&expectation, &archive);
                assert_expected_entries(&expectation, &archive);
                let graph = read(&archive, true, None);
                let dest = TestTempDir::new(fixture);
                let err = unpack_with_options(&graph, dest.path(), &unpack_options(&expectation))
                    .expect_err("fixture extraction must be refused");
                assert_expected_error(&err, &expectation);
            }
            "roundtrip" => {
                let archive = from_tar_bytes(&input, &options).expect("tar fixture imports");
                assert_folded_graph(&expectation, &archive);
                assert_expected_entries(&expectation, &archive);

                let graph = read(&archive, true, None);
                let roundtrip_tar =
                    to_tar_vec(&graph, &ToTarOptions::default()).expect("fixture exports to tar");
                let mut roundtrip_options = options.clone();
                roundtrip_options.source_name = Some("roundtrip.tar".to_string());
                let roundtrip_archive =
                    from_tar_bytes(&roundtrip_tar, &roundtrip_options).expect("roundtrip imports");
                assert_folded_graph(&expectation, &roundtrip_archive);
                assert_expected_entries(&expectation, &roundtrip_archive);
            }
            other => panic!("unsupported tar fixture kind: {other}"),
        }
    }
}

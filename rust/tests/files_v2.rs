// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::PathBuf;
use std::process::{Command, Output};

use ciborium::value::Value;
use gmeow_gts::files::{
    pack, pack_entries_v2, read_entries, FileEntry, FileEntryKind, FilePaxRecord, FileXattr,
};
use gmeow_gts::model::{Graph, Term, TermKind};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::writer::digest_string;

fn tmpdir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-files-v2-{name}-{}-{n}", std::process::id()))
}

fn gts(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        reifier: None,
    }
}

fn literal(value: &str) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        reifier: None,
    }
}

fn bnode(value: &str) -> Term {
    Term {
        kind: TermKind::Bnode,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        reifier: None,
    }
}

fn meta_u64(graph: &gmeow_gts::model::Graph, key: &str) -> Option<u64> {
    graph.meta.iter().find_map(|(stored, value)| {
        if stored != key {
            return None;
        }
        match value {
            Value::Integer(value) => u64::try_from(*value).ok(),
            _ => None,
        }
    })
}

fn sample_v2_entries() -> Vec<FileEntry> {
    vec![
        FileEntry {
            path: "tables".to_string(),
            kind: FileEntryKind::Directory,
            mode: Some(0o755),
            modified: Some("2026-06-19T12:00:00.123456789Z".to_string()),
            uid: Some(1000),
            gid: Some(100),
            user_name: Some("alice".to_string()),
            group_name: Some("staff".to_string()),
            pax_records: vec![FilePaxRecord {
                key: "comment".to_string(),
                value: "empty directory survives".to_string(),
            }],
            ..FileEntry::default()
        },
        FileEntry {
            path: "tables/events.csv".to_string(),
            kind: FileEntryKind::File,
            mode: Some(0o640),
            modified: Some("2026-06-19T12:00:01.000000123Z".to_string()),
            media_type: Some("text/csv".to_string()),
            uid: Some(1000),
            gid: Some(100),
            xattrs: vec![
                FileXattr {
                    name: "user.zeta".to_string(),
                    value: "eg==".to_string(),
                },
                FileXattr {
                    name: "user.alpha".to_string(),
                    value: "YQ==".to_string(),
                },
            ],
            pax_records: vec![FilePaxRecord {
                key: "SCHILY.dev".to_string(),
                value: "opaque".to_string(),
            }],
            data: Some(b"id,name\n1,cat\n".to_vec()),
            ..FileEntry::default()
        },
        FileEntry {
            path: "events-link".to_string(),
            kind: FileEntryKind::Symlink,
            link_target: Some("tables/events.csv".to_string()),
            mode: Some(0o777),
            ..FileEntry::default()
        },
        FileEntry {
            path: "events-hardlink".to_string(),
            kind: FileEntryKind::Hardlink,
            link_target: Some("tables/events.csv".to_string()),
            ..FileEntry::default()
        },
        FileEntry {
            path: "pipe".to_string(),
            kind: FileEntryKind::Fifo,
            ..FileEntry::default()
        },
        FileEntry {
            path: "tty0".to_string(),
            kind: FileEntryKind::CharDev,
            dev_major: Some(4),
            dev_minor: Some(0),
            ..FileEntry::default()
        },
        FileEntry {
            path: "disk0".to_string(),
            kind: FileEntryKind::BlockDev,
            dev_major: Some(8),
            dev_minor: Some(0),
            ..FileEntry::default()
        },
        FileEntry {
            path: "daemon.sock".to_string(),
            kind: FileEntryKind::Socket,
            ..FileEntry::default()
        },
    ]
}

#[test]
fn v1_archive_reads_as_default_file_entries() {
    let tmp = tmpdir("v1");
    let _ = std::fs::remove_dir_all(&tmp);
    let src = tmp.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("a.txt"), "hello").unwrap();

    let data = pack(&[src.as_path()]).expect("v1 pack succeeds");
    let graph = read(&data, true, None);
    assert_eq!(meta_u64(&graph, "profileVersion"), None);
    let entries = read_entries(&graph).expect("v1 entries read through v2 reader");
    let entry = entries.get("a.txt").expect("entry exists");
    assert_eq!(entry.kind, FileEntryKind::File);
    assert_eq!(entry.size, Some(5));
    assert!(entry.digest.as_deref().unwrap_or("").starts_with("blake3:"));
}

#[test]
fn v2_entries_round_trip_all_metadata() {
    let data = pack_entries_v2(&sample_v2_entries()).expect("v2 pack succeeds");
    let graph = read(&data, true, None);
    assert_eq!(meta_u64(&graph, "profileVersion"), Some(2));
    let entries = read_entries(&graph).expect("v2 entries read");

    let file = entries.get("tables/events.csv").expect("file entry");
    assert_eq!(file.kind, FileEntryKind::File);
    assert_eq!(file.size, Some(14));
    assert_eq!(file.mode, Some(0o640));
    assert_eq!(file.media_type.as_deref(), Some("text/csv"));
    assert_eq!(file.uid, Some(1000));
    assert_eq!(file.gid, Some(100));
    let expected_digest = digest_string(b"id,name\n1,cat\n");
    assert_eq!(file.digest.as_deref(), Some(expected_digest.as_str()));
    assert_eq!(file.xattrs[0].name, "user.alpha");
    assert_eq!(file.pax_records[0].key, "SCHILY.dev");

    assert_eq!(
        entries.get("tables").expect("directory").kind,
        FileEntryKind::Directory
    );
    assert_eq!(
        entries
            .get("events-link")
            .expect("symlink")
            .link_target
            .as_deref(),
        Some("tables/events.csv")
    );
    assert_eq!(
        entries.get("events-hardlink").expect("hardlink").kind,
        FileEntryKind::Hardlink
    );
    assert_eq!(entries.get("pipe").expect("fifo").kind, FileEntryKind::Fifo);
    assert_eq!(entries.get("tty0").expect("chardev").dev_major, Some(4));
    assert_eq!(entries.get("disk0").expect("blockdev").dev_minor, Some(0));
    assert_eq!(
        entries.get("daemon.sock").expect("socket").kind,
        FileEntryKind::Socket
    );

    let nquads = to_nquads(&graph);
    assert!(nquads.contains("https://w3id.org/gts/files#xattrName"));
    assert!(nquads.contains("https://w3id.org/gts/files#paxKey"));
}

#[test]
fn v2_reader_accepts_duplicate_iri_term_ids() {
    let graph = Graph {
        terms: vec![
            bnode("entry"),
            iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            iri("https://w3id.org/gts/files#FileEntry"),
            iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            iri("https://w3id.org/gts/files#FileEntry"),
            iri("https://w3id.org/gts/files#path"),
            literal("fixture.txt"),
            iri("https://w3id.org/gts/files#path"),
        ],
        quads: vec![(0, 1, 2, None), (0, 5, 6, None)],
        ..Graph::default()
    };

    let entries = read_entries(&graph).expect("duplicate term ids are read");
    assert_eq!(
        entries.get("fixture.txt").expect("entry").kind,
        FileEntryKind::File
    );
}

#[test]
fn v2_emission_is_deterministic_after_sorting() {
    let mut entries = sample_v2_entries();
    let first = pack_entries_v2(&entries).expect("first pack");
    entries.reverse();
    let second = pack_entries_v2(&entries).expect("second pack");
    assert_eq!(first, second);
}

#[test]
fn v2_unpack_materializes_directories_and_regular_files() {
    let tmp = tmpdir("unpack");
    let _ = std::fs::remove_dir_all(&tmp);
    let archive = pack_entries_v2(&[
        FileEntry {
            path: "empty".to_string(),
            kind: FileEntryKind::Directory,
            mode: Some(0o755),
            ..FileEntry::default()
        },
        FileEntry {
            path: "empty/file.txt".to_string(),
            kind: FileEntryKind::File,
            data: Some(b"payload".to_vec()),
            mode: Some(0o644),
            ..FileEntry::default()
        },
    ])
    .expect("v2 pack succeeds");
    let graph = read(&archive, true, None);
    let dest = tmp.join("dest");
    gmeow_gts::files::unpack(&graph, &dest, false).expect("safe v2 entries unpack");
    assert!(dest.join("empty").is_dir());
    assert_eq!(
        std::fs::read_to_string(dest.join("empty").join("file.txt")).unwrap(),
        "payload"
    );
}

#[test]
fn default_unpack_refuses_v2_links_and_special_files() {
    for (idx, entry) in [
        FileEntry {
            path: "link".to_string(),
            kind: FileEntryKind::Symlink,
            link_target: Some("target".to_string()),
            ..FileEntry::default()
        },
        FileEntry {
            path: "pipe".to_string(),
            kind: FileEntryKind::Fifo,
            ..FileEntry::default()
        },
    ]
    .into_iter()
    .enumerate()
    {
        let archive = pack_entries_v2(&[entry]).expect("v2 archive authors");
        let graph = read(&archive, true, None);
        let dest = tmpdir(&format!("refuse-{idx}"));
        let err = gmeow_gts::files::unpack(&graph, &dest, false).expect_err("entry refused");
        assert!(err.contains("use --allow-"), "error: {err}");
    }
}

#[cfg(unix)]
#[test]
fn opt_in_unpack_materializes_in_destination_symlinks() {
    let tmp = tmpdir("symlink");
    let _ = std::fs::remove_dir_all(&tmp);
    let archive = pack_entries_v2(&[
        FileEntry {
            path: "docs/a.txt".to_string(),
            kind: FileEntryKind::File,
            data: Some(b"payload".to_vec()),
            ..FileEntry::default()
        },
        FileEntry {
            path: "docs/latest.txt".to_string(),
            kind: FileEntryKind::Symlink,
            link_target: Some("a.txt".to_string()),
            ..FileEntry::default()
        },
    ])
    .expect("v2 pack succeeds");
    let graph = read(&archive, true, None);
    let dest = tmp.join("dest");
    let options = gmeow_gts::files::UnpackOptions {
        allow_symlinks: true,
        ..gmeow_gts::files::UnpackOptions::default()
    };

    gmeow_gts::files::unpack_with_options(&graph, &dest, &options)
        .expect("allowed symlink extracts");

    let link_target = std::fs::read_link(dest.join("docs/latest.txt")).unwrap();
    assert_eq!(link_target, std::path::PathBuf::from("a.txt"));
}

#[test]
fn symlink_opt_in_still_refuses_destination_escape() {
    let archive = pack_entries_v2(&[FileEntry {
        path: "docs/latest.txt".to_string(),
        kind: FileEntryKind::Symlink,
        link_target: Some("../../outside".to_string()),
        ..FileEntry::default()
    }])
    .expect("v2 archive authors");
    let graph = read(&archive, true, None);
    let dest = tmpdir("symlink-escape");
    let options = gmeow_gts::files::UnpackOptions {
        allow_symlinks: true,
        ..gmeow_gts::files::UnpackOptions::default()
    };

    let err = gmeow_gts::files::unpack_with_options(&graph, &dest, &options)
        .expect_err("escaping symlink is refused");

    assert!(err.contains("escapes destination"), "error: {err}");
}

#[cfg(unix)]
#[test]
fn unpack_strips_setid_bits_by_default() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tmpdir("setid");
    let _ = std::fs::remove_dir_all(&tmp);
    let archive = pack_entries_v2(&[FileEntry {
        path: "tool".to_string(),
        kind: FileEntryKind::File,
        data: Some(b"payload".to_vec()),
        mode: Some(0o4755),
        ..FileEntry::default()
    }])
    .expect("v2 archive authors");
    let graph = read(&archive, true, None);
    let dest = tmp.join("dest");

    gmeow_gts::files::unpack(&graph, &dest, false).expect("safe file extracts");

    let mode = std::fs::metadata(dest.join("tool"))
        .unwrap()
        .permissions()
        .mode()
        & 0o7777;
    assert_eq!(mode, 0o755);
}

#[test]
fn dumpdir_surfaces_v2_fields_in_entries_jsonl() {
    let tmp = tmpdir("dump");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let archive = tmp.join("files-v2.gts");
    std::fs::write(&archive, pack_entries_v2(&sample_v2_entries()).unwrap()).unwrap();
    let dump = tmp.join("dump");
    let out = gts(&[
        "dump",
        archive.to_str().unwrap(),
        "--directory",
        dump.to_str().unwrap(),
        "--metadata-only",
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let entries = std::fs::read_to_string(dump.join("files").join("entries.jsonl")).unwrap();
    assert!(entries.contains("\"type\":\"symlink\""));
    assert!(entries.contains("\"link_target\":\"tables/events.csv\""));
    assert!(entries.contains("\"uid\":1000"));
    assert!(entries.contains("\"xattrs\":[{\"name\":\"user.alpha\""));
    assert!(entries.contains("\"pax_records\":[{\"key\":\"SCHILY.dev\""));
}

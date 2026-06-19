// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "tar")]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use gmeow_gts::codec::encode_chain;
use gmeow_gts::files::{read_entries, FileEntryKind, FilePaxRecord};
use gmeow_gts::from_nquads::from_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::tar as gts_tar;

fn tmpdir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-tar-{name}-{}-{n}", std::process::id()))
}

fn gts(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

fn gts_with_stdin(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("gts binary starts");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(input)
        .expect("stdin write succeeds");
    child.wait_with_output().expect("gts output is collected")
}

fn make_tree(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("a.txt"), "hello").unwrap();
    std::fs::write(root.join("docs").join("b.txt"), "world").unwrap();
}

fn header(entry_type: tar::EntryType, size: u64) -> tar::Header {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(entry_type);
    header.set_mode(0o640);
    header.set_mtime(1_787_000_000);
    header.set_uid(1000);
    header.set_gid(100);
    header.set_size(size);
    header.set_username("alice").unwrap();
    header.set_groupname("staff").unwrap();
    header
}

fn fixture_tar(include_symlink: bool) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut out);

        let mut dir = header(tar::EntryType::Directory, 0);
        dir.set_mode(0o750);
        builder
            .append_data(&mut dir, "docs", std::io::empty())
            .unwrap();

        builder
            .append_pax_extensions([("comment", b"kept for experts".as_slice())])
            .unwrap();
        let payload = b"same payload\n";
        let mut file = header(tar::EntryType::Regular, payload.len() as u64);
        builder
            .append_data(&mut file, "docs/a.txt", payload.as_slice())
            .unwrap();

        let mut duplicate = header(tar::EntryType::Regular, payload.len() as u64);
        builder
            .append_data(&mut duplicate, "docs/b.txt", payload.as_slice())
            .unwrap();

        if include_symlink {
            let mut link = header(tar::EntryType::Symlink, 0);
            link.set_mode(0o777);
            builder
                .append_link(&mut link, "docs/latest.txt", "a.txt")
                .unwrap();
        }

        builder.finish().unwrap();
    }
    out
}

fn fifo_tar() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut out);
        let mut fifo = header(tar::EntryType::Fifo, 0);
        builder
            .append_data(&mut fifo, "run/pipe", std::io::empty())
            .unwrap();
        builder.finish().unwrap();
    }
    out
}

fn traversal_tar() -> Vec<u8> {
    let mut out = Vec::new();
    {
        let data = b"bad";
        let mut builder = tar::Builder::new(&mut out);
        let mut bad = header(tar::EntryType::Regular, data.len() as u64);
        bad.set_path("safe.txt").unwrap();
        let raw = bad.as_mut_bytes();
        raw[..100].fill(0);
        raw[..11].copy_from_slice(b"../evil.txt");
        bad.set_cksum();
        builder.append(&bad, data.as_slice()).unwrap();
        builder.finish().unwrap();
    }
    out
}

#[derive(Debug)]
struct TarEntry {
    kind: &'static str,
    link_target: Option<String>,
    data: Vec<u8>,
    pax_records: Vec<FilePaxRecord>,
}

fn inspect_tar(data: &[u8]) -> BTreeMap<String, TarEntry> {
    let mut archive = tar::Archive::new(data);
    let mut out = BTreeMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().replace('\\', "/");
        let entry_type = entry.header().entry_type();
        let kind = if entry_type.is_file() {
            "file"
        } else if entry_type.is_dir() {
            "directory"
        } else if entry_type.is_symlink() {
            "symlink"
        } else if entry_type.is_hard_link() {
            "hardlink"
        } else if entry_type.is_fifo() {
            "fifo"
        } else {
            "other"
        };
        let link_target = entry
            .link_name()
            .unwrap()
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        let mut pax_records = Vec::new();
        if let Some(extensions) = entry.pax_extensions().unwrap() {
            for extension in extensions {
                let extension = extension.unwrap();
                pax_records.push(FilePaxRecord {
                    key: extension.key().unwrap().to_string(),
                    value: extension.value().unwrap().to_string(),
                });
            }
        }
        let mut body = Vec::new();
        entry.read_to_end(&mut body).unwrap();
        out.insert(
            path,
            TarEntry {
                kind,
                link_target,
                data: body,
                pax_records,
            },
        );
    }
    out
}

fn graph_from_nquads(text: &str) -> gmeow_gts::model::Graph {
    let archive = from_nquads(text).expect("N-Quads author a GTS archive");
    read(&archive, true, None)
}

#[test]
fn tar_verb_creates_lists_extracts_and_diffs_gts_archives() {
    let tmp = tmpdir("verb-gts");
    let _ = std::fs::remove_dir_all(&tmp);
    let input = tmp.join("input");
    let archive = tmp.join("tree.gts");
    let restore = tmp.join("restore");
    make_tree(&input);

    let create = gts(&[
        "tar",
        "-cf",
        archive.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        create.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create.stderr)
    );

    let list = gts(&["tar", "-tf", archive.to_str().unwrap()]);
    assert!(
        list.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let listing = String::from_utf8(list.stdout).unwrap();
    assert!(listing.contains("file\t5\ta.txt"), "listing: {listing}");
    assert!(
        listing.contains("file\t5\tdocs/b.txt"),
        "listing: {listing}"
    );

    let extract = gts(&[
        "tar",
        "-xf",
        archive.to_str().unwrap(),
        "-C",
        restore.to_str().unwrap(),
    ]);
    assert!(
        extract.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&extract.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(restore.join("a.txt")).unwrap(),
        "hello"
    );
    assert_eq!(
        std::fs::read_to_string(restore.join("docs").join("b.txt")).unwrap(),
        "world"
    );

    let clean = gts(&[
        "tar",
        "-df",
        archive.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        clean.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&clean.stderr)
    );

    std::fs::write(input.join("a.txt"), "changed").unwrap();
    let changed = gts(&[
        "tar",
        "-df",
        archive.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert_eq!(changed.status.code(), Some(1));
    let diff = String::from_utf8(changed.stdout).unwrap();
    assert!(diff.contains("modified: a.txt"), "diff: {diff}");
}

#[test]
fn tar_verb_creates_and_extracts_compressed_tar_archives() {
    let tmp = tmpdir("verb-tar");
    let _ = std::fs::remove_dir_all(&tmp);
    let input = tmp.join("input");
    let gzip_archive = tmp.join("tree.tar.gz");
    let zstd_archive = tmp.join("tree.tar.zst");
    let restore = tmp.join("restore");
    make_tree(&input);

    let gzip = gts(&[
        "tar",
        "-czf",
        gzip_archive.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        gzip.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&gzip.stderr)
    );
    assert!(std::fs::read(&gzip_archive)
        .unwrap()
        .starts_with(&[0x1f, 0x8b]));

    let zstd = gts(&[
        "tar",
        "--zstd",
        "-cf",
        zstd_archive.to_str().unwrap(),
        input.to_str().unwrap(),
    ]);
    assert!(
        zstd.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&zstd.stderr)
    );
    assert!(std::fs::read(&zstd_archive)
        .unwrap()
        .starts_with(&[0x28, 0xb5, 0x2f, 0xfd]));

    let extract = gts(&[
        "tar",
        "-xf",
        gzip_archive.to_str().unwrap(),
        "-C",
        restore.to_str().unwrap(),
    ]);
    assert!(
        extract.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&extract.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(restore.join("docs").join("b.txt")).unwrap(),
        "world"
    );
}

#[cfg(unix)]
#[test]
fn tar_verb_enforces_symlink_opt_in_on_extract() {
    let tmp = tmpdir("verb-symlink");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let input = tmp.join("links.tar");
    let restore = tmp.join("restore");
    std::fs::write(&input, fixture_tar(true)).unwrap();

    let refused = gts(&[
        "tar",
        "-xf",
        input.to_str().unwrap(),
        "-C",
        restore.to_str().unwrap(),
    ]);
    assert_eq!(refused.status.code(), Some(1));
    let err = String::from_utf8_lossy(&refused.stderr);
    assert!(err.contains("--allow-symlinks"), "stderr: {err}");

    let allowed = gts(&[
        "tar",
        "-xf",
        input.to_str().unwrap(),
        "--allow-symlinks",
        "-C",
        restore.to_str().unwrap(),
    ]);
    assert!(
        allowed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&allowed.stderr)
    );
    assert_eq!(
        std::fs::read_link(restore.join("docs/latest.txt")).unwrap(),
        std::path::PathBuf::from("a.txt")
    );
}

#[test]
fn cli_round_trips_tar_with_owner_pax_links_and_dedup() {
    let tmp = tmpdir("roundtrip");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let input = tmp.join("input.tar");
    let archive = tmp.join("archive.gts");
    let output = tmp.join("output.tar");
    std::fs::write(&input, fixture_tar(true)).unwrap();

    let from = gts(&[
        "from-tar",
        input.to_str().unwrap(),
        "--allow-symlinks",
        "--owner",
        "-o",
        archive.to_str().unwrap(),
    ]);
    assert!(
        from.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&from.stderr)
    );

    let graph = read(&std::fs::read(&archive).unwrap(), true, None);
    let entries = read_entries(&graph).expect("files-profile-v2 entries read");
    assert_eq!(entries.get("docs").unwrap().kind, FileEntryKind::Directory);
    assert_eq!(
        entries
            .get("docs/latest.txt")
            .unwrap()
            .link_target
            .as_deref(),
        Some("a.txt")
    );
    assert_eq!(entries.get("docs/a.txt").unwrap().uid, Some(1000));
    assert_eq!(
        entries.get("docs/a.txt").unwrap().user_name.as_deref(),
        Some("alice")
    );
    assert_eq!(
        entries.get("docs/a.txt").unwrap().pax_records,
        vec![FilePaxRecord {
            key: "comment".to_string(),
            value: "kept for experts".to_string(),
        }]
    );
    assert_eq!(graph.blobs.len(), 1, "identical file payloads deduplicate");

    let to = gts(&[
        "to-tar",
        archive.to_str().unwrap(),
        "--numeric-owner",
        "-o",
        output.to_str().unwrap(),
    ]);
    assert!(
        to.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&to.stderr)
    );

    let tar = inspect_tar(&std::fs::read(&output).unwrap());
    assert_eq!(tar.get("docs").unwrap().kind, "directory");
    assert_eq!(tar.get("docs/a.txt").unwrap().data, b"same payload\n");
    assert_eq!(tar.get("docs/b.txt").unwrap().data, b"same payload\n");
    assert_eq!(
        tar.get("docs/latest.txt").unwrap().link_target.as_deref(),
        Some("a.txt")
    );
    assert_eq!(
        tar.get("docs/a.txt").unwrap().pax_records,
        vec![FilePaxRecord {
            key: "comment".to_string(),
            value: "kept for experts".to_string(),
        }]
    );
}

#[test]
fn from_tar_and_to_tar_are_pipe_friendly() {
    let tmp = tmpdir("pipes");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let archive = tmp.join("stdin.gts");

    let from = gts_with_stdin(
        &["from-tar", "-", "-o", archive.to_str().unwrap()],
        &fixture_tar(false),
    );
    assert!(
        from.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&from.stderr)
    );

    let to = gts(&["to-tar", archive.to_str().unwrap()]);
    assert!(
        to.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&to.stderr)
    );
    let tar = inspect_tar(&to.stdout);
    assert_eq!(tar.get("docs/a.txt").unwrap().data, b"same payload\n");
}

#[test]
fn compression_variants_round_trip_through_the_cli() {
    let tmp = tmpdir("compression");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let raw = fixture_tar(false);
    let gzip_tar = tmp.join("fixture.tar.gz");
    let zstd_tar = tmp.join("fixture.tar.zst");
    std::fs::write(
        &gzip_tar,
        encode_chain(&["gzip".to_string()], &raw).expect("gzip encodes"),
    )
    .unwrap();
    std::fs::write(
        &zstd_tar,
        encode_chain(&["zstd".to_string()], &raw).expect("zstd encodes"),
    )
    .unwrap();

    for (input, flag, magic) in [
        (gzip_tar.as_path(), "--gzip", &[0x1f, 0x8b][..]),
        (zstd_tar.as_path(), "--zstd", &[0x28, 0xb5, 0x2f, 0xfd][..]),
    ] {
        let archive = tmp.join(format!("{flag}.gts"));
        let compressed_out = tmp.join(format!("{flag}.tar"));
        let from = gts(&[
            "from-tar",
            input.to_str().unwrap(),
            "-o",
            archive.to_str().unwrap(),
        ]);
        assert!(
            from.status.success(),
            "{flag} stderr: {}",
            String::from_utf8_lossy(&from.stderr)
        );
        let to = gts(&[
            "to-tar",
            archive.to_str().unwrap(),
            flag,
            "-o",
            compressed_out.to_str().unwrap(),
        ]);
        assert!(
            to.status.success(),
            "{flag} stderr: {}",
            String::from_utf8_lossy(&to.stderr)
        );
        let encoded = std::fs::read(&compressed_out).unwrap();
        assert!(encoded.starts_with(magic));

        let again = tmp.join(format!("{flag}.again.gts"));
        let from_again = gts(&[
            "from-tar",
            compressed_out.to_str().unwrap(),
            "-o",
            again.to_str().unwrap(),
        ]);
        assert!(
            from_again.status.success(),
            "{flag} reread stderr: {}",
            String::from_utf8_lossy(&from_again.stderr)
        );
    }
}

#[test]
fn from_tar_refuses_links_special_files_and_traversal_by_default() {
    let tmp = tmpdir("safety");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let links = tmp.join("links.tar");
    std::fs::write(&links, fixture_tar(true)).unwrap();
    let out = gts(&["from-tar", links.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--allow-symlinks"), "stderr: {err}");

    let special = tmp.join("special.tar");
    std::fs::write(&special, fifo_tar()).unwrap();
    let out = gts(&["from-tar", special.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("--allow-special"), "stderr: {err}");

    let traversal = tmp.join("traversal.tar");
    std::fs::write(&traversal, traversal_tar()).unwrap();
    let out = gts(&["from-tar", traversal.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("path traversal"), "stderr: {err}");
}

#[test]
fn allow_special_preserves_special_entry_metadata() {
    let tmp = tmpdir("special");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let input = tmp.join("special.tar");
    let archive = tmp.join("special.gts");
    std::fs::write(&input, fifo_tar()).unwrap();

    let out = gts(&[
        "from-tar",
        input.to_str().unwrap(),
        "--allow-special",
        "-o",
        archive.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let graph = read(&std::fs::read(&archive).unwrap(), true, None);
    let entries = read_entries(&graph).expect("entries read");
    assert_eq!(entries.get("run/pipe").unwrap().kind, FileEntryKind::Fifo);
}

#[test]
fn to_tar_refuses_link_entries_without_targets() {
    let graph = graph_from_nquads(
        r#"
_:f0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://w3id.org/gts/files#FileEntry> .
_:f0 <https://w3id.org/gts/files#path> "link" .
_:f0 <https://w3id.org/gts/files#type> "symlink" .
"#,
    );

    let err = gts_tar::to_tar(&graph, Vec::<u8>::new(), &gts_tar::ToTarOptions::default())
        .expect_err("missing link target is rejected");

    assert!(
        err.to_string()
            .contains("symlink entry link has no link target"),
        "error: {err}"
    );
}

#[test]
fn to_tar_refuses_device_numbers_outside_tar_range() {
    let graph = graph_from_nquads(
        r#"
_:f0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://w3id.org/gts/files#FileEntry> .
_:f0 <https://w3id.org/gts/files#path> "dev/null" .
_:f0 <https://w3id.org/gts/files#type> "chardev" .
_:f0 <https://w3id.org/gts/files#devMajor> "4294967296"^^<http://www.w3.org/2001/XMLSchema#integer> .
_:f0 <https://w3id.org/gts/files#devMinor> "1"^^<http://www.w3.org/2001/XMLSchema#integer> .
"#,
    );

    let err = gts_tar::to_tar(&graph, Vec::<u8>::new(), &gts_tar::ToTarOptions::default())
        .expect_err("out-of-range device major is rejected");

    assert!(
        err.to_string()
            .contains("dev/null devMajor exceeds u32 range"),
        "error: {err}"
    );
}

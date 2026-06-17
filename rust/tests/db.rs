// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relational export CLI tests.

use std::path::PathBuf;
use std::process::{Command, Output};

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors")
}

fn tmpdir() -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("gts-db-test-{}-{n}", std::process::id()))
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

fn gts(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gts"))
        .args(args)
        .output()
        .expect("gts binary runs")
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn sqlite_query(db: &str, sql: &str) -> String {
    stdout(
        Command::new("sqlite3")
            .args(["-batch", "-noheader", db, sql])
            .output()
            .expect("sqlite3 runs"),
    )
}

fn duckdb_query(db: &str, sql: &str) -> String {
    stdout(
        Command::new("duckdb")
            .args(["-csv", "-noheader", db, "-c", sql])
            .output()
            .expect("duckdb runs"),
    )
}

#[test]
fn to_sqlite_exports_schema_and_rows() {
    if !command_available("sqlite3") {
        eprintln!("skipping: sqlite3 is not installed");
        return;
    }
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("minimal.sqlite");
    let out = gts(&[
        "to-sqlite",
        vectors().join("01-minimal.gts").to_str().unwrap(),
        db.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert_eq!(
        sqlite_query(db.to_str().unwrap(), "SELECT count(*) FROM terms;").trim(),
        "3"
    );
    assert_eq!(
        sqlite_query(db.to_str().unwrap(), "SELECT count(*) FROM quads;").trim(),
        "1"
    );
    assert_eq!(
        sqlite_query(db.to_str().unwrap(), "SELECT lex FROM terms WHERE id = 0;").trim(),
        "https://example.org/Cat"
    );
}

#[test]
fn to_sqlite_refuses_damaged_input_before_writing() {
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("damaged.sqlite");
    let out = gts(&[
        "to-sqlite",
        vectors().join("04-damaged-frame.gts").to_str().unwrap(),
        db.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("DamagedFrame"), "stderr lists diagnostics");
    assert!(err.contains("refusing export"), "stderr names refusal");
    assert!(!db.exists(), "no database should be written");
}

#[test]
fn to_duckdb_exports_schema_and_rows() {
    if !command_available("duckdb") {
        eprintln!("skipping: duckdb is not installed");
        return;
    }
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let db = tmp.join("minimal.duckdb");
    let out = gts(&[
        "to-duckdb",
        vectors().join("01-minimal.gts").to_str().unwrap(),
        db.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert_eq!(
        duckdb_query(db.to_str().unwrap(), "SELECT count(*) FROM terms;").trim(),
        "3"
    );
    assert_eq!(
        duckdb_query(db.to_str().unwrap(), "SELECT count(*) FROM quads;").trim(),
        "1"
    );
    assert_eq!(
        duckdb_query(db.to_str().unwrap(), "SELECT lex FROM terms WHERE id = 0;").trim(),
        "https://example.org/Cat"
    );
}

#[test]
fn to_parquet_writes_non_empty_tables() {
    if !command_available("duckdb") {
        eprintln!("skipping: duckdb is not installed");
        return;
    }
    let tmp = tmpdir();
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let out_dir = tmp.join("parquet");
    let out = gts(&[
        "to-parquet",
        vectors().join("22-inline-blob.gts").to_str().unwrap(),
        out_dir.to_str().unwrap(),
    ]);
    let text = stdout(out);

    for name in ["terms.parquet", "quads.parquet", "blobs.parquet"] {
        let path = out_dir.join(name);
        assert!(path.exists(), "{name} was written");
        assert!(
            text.contains(path.to_str().unwrap()),
            "{name} listed on stdout"
        );
    }
    assert!(!out_dir.join("reifiers.parquet").exists());
    assert!(!out_dir.join("annotations.parquet").exists());

    let blobs = out_dir.join("blobs.parquet");
    assert_eq!(
        duckdb_query(
            ":memory:",
            &format!(
                "SELECT count(*) FROM read_parquet('{}');",
                blobs.to_string_lossy().replace('\'', "''")
            ),
        )
        .trim(),
        "1"
    );
}

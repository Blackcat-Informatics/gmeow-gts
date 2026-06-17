// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Relational exports for the folded graph (§14).
//!
//! The schema mirrors the Python `gts.db` module: five dictionary-encoded
//! tables (`terms`, `quads`, `reifiers`, `annotations`, `blobs`) whose rows keep
//! GTS integer term ids intact so query engines can resolve labels lazily.
//!
//! To avoid pulling database engines into the core Rust crate, these helpers use
//! command-line database tools at runtime: `sqlite3` for SQLite, plus `duckdb`
//! for DuckDB/Parquet when the optional `duckdb` feature is enabled.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{Graph, TermKind};
use crate::wire::hex;

const SCHEMA: &[&str] = &[
    "CREATE TABLE terms (id INTEGER PRIMARY KEY, kind INTEGER, lex TEXT, datatype INTEGER, lang TEXT, reifier INTEGER)",
    "CREATE TABLE quads (s INTEGER, p INTEGER, o INTEGER, g INTEGER)",
    "CREATE TABLE reifiers (reifier INTEGER, s INTEGER, p INTEGER, o INTEGER)",
    "CREATE TABLE annotations (reifier INTEGER, predicate INTEGER, value INTEGER)",
    "CREATE TABLE blobs (digest TEXT PRIMARY KEY, bytes BLOB)",
];

const INDEXES: &[&str] = &[
    "CREATE INDEX quads_s ON quads (s)",
    "CREATE INDEX quads_p ON quads (p)",
    "CREATE INDEX quads_o ON quads (o)",
    "CREATE INDEX annot_reifier ON annotations (reifier)",
];

#[cfg(feature = "duckdb")]
const TABLES: &[&str] = &["terms", "quads", "reifiers", "annotations", "blobs"];

/// Export failure with a displayable message suitable for CLI output.
#[derive(Debug)]
pub struct DbExportError {
    detail: String,
}

impl DbExportError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for DbExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for DbExportError {}

#[derive(Clone, Copy)]
enum SqlDialect {
    Sqlite,
    #[cfg(feature = "duckdb")]
    Duckdb,
}

struct TableSql {
    terms: Vec<String>,
    quads: Vec<String>,
    reifiers: Vec<String>,
    annotations: Vec<String>,
    blobs: Vec<String>,
}

impl TableSql {
    #[cfg(feature = "duckdb")]
    fn count(&self, table: &str) -> usize {
        match table {
            "terms" => self.terms.len(),
            "quads" => self.quads.len(),
            "reifiers" => self.reifiers.len(),
            "annotations" => self.annotations.len(),
            "blobs" => self.blobs.len(),
            _ => 0,
        }
    }

    fn append_inserts(&self, script: &mut String) {
        for row in &self.terms {
            script.push_str("INSERT INTO terms VALUES ");
            script.push_str(row);
            script.push_str(";\n");
        }
        for row in &self.quads {
            script.push_str("INSERT INTO quads VALUES ");
            script.push_str(row);
            script.push_str(";\n");
        }
        for row in &self.reifiers {
            script.push_str("INSERT INTO reifiers VALUES ");
            script.push_str(row);
            script.push_str(";\n");
        }
        for row in &self.annotations {
            script.push_str("INSERT INTO annotations VALUES ");
            script.push_str(row);
            script.push_str(";\n");
        }
        for row in &self.blobs {
            script.push_str("INSERT INTO blobs VALUES ");
            script.push_str(row);
            script.push_str(";\n");
        }
    }
}

fn term_kind_int(kind: TermKind) -> u8 {
    match kind {
        TermKind::Iri => 0,
        TermKind::Literal => 1,
        TermKind::Bnode => 2,
        TermKind::Triple => 3,
    }
}

fn sql_text(value: Option<&str>) -> String {
    match value {
        Some(text) => format!("'{}'", text.replace('\'', "''")),
        None => "NULL".to_string(),
    }
}

fn sql_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "NULL".to_string(), |v| v.to_string())
}

fn sql_blob(bytes: &[u8], dialect: SqlDialect) -> String {
    match dialect {
        SqlDialect::Sqlite => format!("X'{}'", hex(bytes)),
        #[cfg(feature = "duckdb")]
        SqlDialect::Duckdb => format!("from_hex('{}')", hex(bytes)),
    }
}

fn table_sql(graph: &Graph, dialect: SqlDialect) -> Result<TableSql, DbExportError> {
    let terms = graph
        .terms
        .iter()
        .enumerate()
        .map(|(id, term)| {
            format!(
                "({id},{},{},{},{},{})",
                term_kind_int(term.kind),
                sql_text(term.value.as_deref()),
                sql_usize(term.datatype),
                sql_text(term.lang.as_deref()),
                sql_usize(term.reifier)
            )
        })
        .collect();
    let quads = graph
        .quads
        .iter()
        .map(|(s, p, o, g)| format!("({s},{p},{o},{})", sql_usize(*g)))
        .collect();
    let reifiers = graph
        .reifiers
        .iter()
        .map(|(r, (s, p, o))| format!("({r},{s},{p},{o})"))
        .collect();
    let annotations = graph
        .annotations
        .iter()
        .map(|(r, p, v)| format!("({r},{p},{v})"))
        .collect();
    let blobs = graph
        .blobs
        .iter()
        .map(|(digest, entry)| {
            let bytes = entry.decoded_bytes().map_err(|err| {
                DbExportError::new(format!("cannot decode blob {digest}: {err:?}"))
            })?;
            Ok(format!(
                "({},{})",
                sql_text(Some(digest.as_str())),
                sql_blob(bytes.as_ref(), dialect)
            ))
        })
        .collect::<Result<Vec<_>, DbExportError>>()?;
    Ok(TableSql {
        terms,
        quads,
        reifiers,
        annotations,
        blobs,
    })
}

fn build_load_script(rows: &TableSql) -> String {
    let mut script = String::new();
    for ddl in SCHEMA {
        script.push_str(ddl);
        script.push_str(";\n");
    }
    script.push_str("BEGIN TRANSACTION;\n");
    rows.append_inserts(&mut script);
    script.push_str("COMMIT;\n");
    for ddl in INDEXES {
        script.push_str(ddl);
        script.push_str(";\n");
    }
    script
}

fn run_sql_tool(program: &str, args: &[&str], script: &str) -> Result<(), DbExportError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            DbExportError::new(format!(
                "{program} is required for this export and could not be started: {e}"
            ))
        })?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| DbExportError::new(format!("{program}: stdin unavailable")))?
        .write_all(script.as_bytes())
        .map_err(|e| DbExportError::new(format!("{program}: could not write SQL script: {e}")))?;
    let output = child
        .wait_with_output()
        .map_err(|e| DbExportError::new(format!("{program}: could not collect status: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(DbExportError::new(format!(
        "{program} failed with status {}: {}{}{}",
        output.status,
        stderr.trim(),
        if stderr.trim().is_empty() || stdout.trim().is_empty() {
            ""
        } else {
            "\n"
        },
        stdout.trim()
    )))
}

#[cfg(feature = "duckdb")]
fn sql_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn temp_sibling_path(path: &Path, suffix: &str) -> PathBuf {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "export".into());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(
        ".{name}.{}.{}.{}",
        std::process::id(),
        nanos,
        suffix
    ))
}

fn replace_file(staged: &Path, out: &Path) -> Result<(), DbExportError> {
    if out.is_dir() {
        return Err(DbExportError::new(format!(
            "cannot replace directory {} with exported file",
            out.display()
        )));
    }
    if out.exists() {
        let backup = temp_sibling_path(out, "bak");
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(out, &backup).map_err(|e| {
            DbExportError::new(format!(
                "cannot stage replacement for {}: {e}",
                out.display()
            ))
        })?;
        if let Err(err) = std::fs::rename(staged, out) {
            let _ = std::fs::rename(&backup, out);
            return Err(DbExportError::new(format!(
                "cannot replace {}: {err}",
                out.display()
            )));
        }
        let _ = std::fs::remove_file(&backup);
        return Ok(());
    }
    std::fs::rename(staged, out)
        .map_err(|e| DbExportError::new(format!("cannot write {}: {e}", out.display())))
}

fn export_database_file(program: &str, out: &Path, script: &str) -> Result<(), DbExportError> {
    let staged = temp_sibling_path(out, "tmp");
    let _ = std::fs::remove_file(&staged);
    let staged_arg = staged.to_string_lossy().into_owned();
    if let Err(err) = run_sql_tool(program, &[staged_arg.as_str()], script) {
        let _ = std::fs::remove_file(&staged);
        return Err(err);
    }
    replace_file(&staged, out)
}

/// Write a folded graph to a SQLite database.
pub fn to_sqlite(graph: &Graph, path: impl AsRef<Path>) -> Result<PathBuf, DbExportError> {
    let out = path.as_ref();
    let rows = table_sql(graph, SqlDialect::Sqlite)?;
    let script = build_load_script(&rows);
    export_database_file("sqlite3", out, &script)?;
    Ok(out.to_path_buf())
}

/// Write a folded graph to a DuckDB database.
#[cfg(feature = "duckdb")]
pub fn to_duckdb(graph: &Graph, path: impl AsRef<Path>) -> Result<PathBuf, DbExportError> {
    let out = path.as_ref();
    let rows = table_sql(graph, SqlDialect::Duckdb)?;
    let script = build_load_script(&rows);
    export_database_file("duckdb", out, &script)?;
    Ok(out.to_path_buf())
}

/// Write one Parquet file per non-empty relational table.
#[cfg(feature = "duckdb")]
pub fn to_parquet(graph: &Graph, out_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>, DbExportError> {
    let target = out_dir.as_ref();
    std::fs::create_dir_all(target)
        .map_err(|e| DbExportError::new(format!("cannot create {}: {e}", target.display())))?;
    let staged_dir = temp_sibling_path(target, "tmpdir");
    let _ = std::fs::remove_dir_all(&staged_dir);
    std::fs::create_dir_all(&staged_dir).map_err(|e| {
        DbExportError::new(format!(
            "cannot create staging directory {}: {e}",
            staged_dir.display()
        ))
    })?;
    let rows = table_sql(graph, SqlDialect::Duckdb)?;
    let mut script = build_load_script(&rows);
    let mut written = Vec::new();
    for table in TABLES {
        if rows.count(table) == 0 {
            continue;
        }
        let staged = staged_dir.join(format!("{table}.parquet"));
        let out = target.join(format!("{table}.parquet"));
        script.push_str(&format!(
            "COPY {table} TO '{}' (FORMAT parquet);\n",
            sql_path(&staged)
        ));
        written.push((staged, out));
    }
    if let Err(err) = run_sql_tool("duckdb", &[], &script) {
        let _ = std::fs::remove_dir_all(&staged_dir);
        return Err(err);
    }
    let mut final_paths = Vec::new();
    let mut replace_err = None;
    for (staged, out) in &written {
        if let Err(err) = replace_file(staged, out) {
            replace_err = Some(err);
            break;
        }
        final_paths.push(out.clone());
    }
    if replace_err.is_none() {
        for table in TABLES {
            if rows.count(table) == 0 {
                let _ = std::fs::remove_file(target.join(format!("{table}.parquet")));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&staged_dir);
    if let Some(err) = replace_err {
        return Err(err);
    }
    Ok(final_paths)
}

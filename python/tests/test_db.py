# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""`gts → {sqlite,duckdb,parquet}` relational export (#13)."""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from gts import read
from gts.cli import main
from gts.db import to_duckdb, to_parquet, to_sqlite

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors"


def _graph(name: str):
    return read((VECTORS_DIR / name).read_bytes())


def _counts(graph: object) -> dict[str, int]:
    return {
        "terms": len(graph.terms),  # type: ignore[attr-defined]
        "quads": len(graph.quads),  # type: ignore[attr-defined]
        "reifiers": len(graph.reifiers),  # type: ignore[attr-defined]
        "annotations": len(graph.annotations),  # type: ignore[attr-defined]
        "blobs": len(graph.blobs),  # type: ignore[attr-defined]
    }


def test_to_sqlite_loads_the_dictionary_tables(tmp_path: Path) -> None:
    g = _graph("12-conflicting-reifier.gts")
    out = to_sqlite(g, tmp_path / "out.sqlite")
    assert out.exists()
    conn = sqlite3.connect(out)
    try:
        for table, n in _counts(g).items():
            got = conn.execute(f"SELECT count(*) FROM {table}").fetchone()[0]
            assert got == n, f"{table}: {got} != {n}"
        # terms are the dictionary; the engine resolves quad ids by join.
        rows = conn.execute(
            "SELECT t.lex FROM quads q JOIN terms t ON t.id = q.p"
        ).fetchall()
        assert all(isinstance(r[0], str) for r in rows)
    finally:
        conn.close()


def test_to_sqlite_overwrites_existing(tmp_path: Path) -> None:
    out = tmp_path / "out.sqlite"
    out.write_bytes(b"stale")
    to_sqlite(_graph("01-minimal.gts"), out)
    conn = sqlite3.connect(out)
    try:
        assert conn.execute("SELECT count(*) FROM terms").fetchone()[0] >= 1
    finally:
        conn.close()


def test_to_duckdb_roundtrips() -> None:
    duckdb = pytest.importorskip("duckdb")
    import tempfile

    g = _graph("11-datatype-defaulting.gts")
    with tempfile.TemporaryDirectory() as d:
        out = to_duckdb(g, Path(d) / "out.duckdb")
        con = duckdb.connect(str(out))
        try:
            assert con.execute("SELECT count(*) FROM terms").fetchone()[0] == len(
                g.terms
            )
            assert con.execute("SELECT count(*) FROM quads").fetchone()[0] == len(
                g.quads
            )
        finally:
            con.close()


def test_to_parquet_writes_only_nonempty_tables(tmp_path: Path) -> None:
    duckdb = pytest.importorskip("duckdb")
    g = _graph("11-datatype-defaulting.gts")
    written = to_parquet(g, tmp_path / "pq")
    names = {p.name for p in written}
    assert "terms.parquet" in names
    assert "quads.parquet" in names
    assert "blobs.parquet" not in names  # empty table is skipped
    # the columnar export is readable back via duckdb
    n = duckdb.connect(":memory:").execute(
        f"SELECT count(*) FROM read_parquet('{tmp_path / 'pq' / 'terms.parquet'}')"
    ).fetchone()[0]
    assert n == len(g.terms)


def test_cli_to_sqlite(tmp_path: Path) -> None:
    src = VECTORS_DIR / "01-minimal.gts"
    out = tmp_path / "out.sqlite"
    assert main(["to-sqlite", str(src), str(out)]) == 0
    assert out.exists()
    conn = sqlite3.connect(out)
    try:
        assert conn.execute("SELECT count(*) FROM quads").fetchone()[0] == 1
    finally:
        conn.close()


def test_cli_to_parquet(tmp_path: Path) -> None:
    pytest.importorskip("duckdb")
    src = VECTORS_DIR / "11-datatype-defaulting.gts"
    out_dir = tmp_path / "pq"
    assert main(["to-parquet", str(src), str(out_dir)]) == 0
    assert (out_dir / "terms.parquet").exists()

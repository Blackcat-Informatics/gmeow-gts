# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Conversion and export commands for the internal Python CLI."""

from __future__ import annotations

import sys

from gts.cli_common import _load, _write_out
from gts.reader import read


def _cmd_to_trig(path: str) -> int:
    from gts.trig import to_trig

    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    sys.stdout.write(to_trig(g))
    return 1 if g.diagnostics or not g.segment_heads else 0


def _cmd_from_nq(path: str, out: str | None) -> int:
    """Build a GTS from N-Quads text — the inverse of ``fold`` (§14).

    Reads ``path`` (or stdin when ``path`` is ``-``); writes GTS to ``out`` (or
    stdout). Lets RDF producers delegate the GTS encoding to the binary.
    """
    from gts.from_nquads import NQuadsParseError, from_nquads

    try:
        text = sys.stdin.read() if path == "-" else _load(path).decode("utf-8")
    except OSError as exc:
        print(f"gts from-nq: cannot read {path}: {exc}", file=sys.stderr)
        return 2
    except UnicodeDecodeError as exc:
        print(f"gts from-nq: invalid UTF-8 in {path}: {exc}", file=sys.stderr)
        return 1
    try:
        data = from_nquads(text)
    except NQuadsParseError as exc:
        print(f"gts from-nq: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


def _cmd_from_trig(path: str, out: str | None) -> int:
    """Build a GTS from TriG text."""
    from gts.trig import TriGParseError, from_trig

    try:
        text = sys.stdin.read() if path == "-" else _load(path).decode("utf-8")
    except OSError as exc:
        print(f"gts from-trig: cannot read {path}: {exc}", file=sys.stderr)
        return 2
    except UnicodeDecodeError as exc:
        print(f"gts from-trig: invalid UTF-8 in {path}: {exc}", file=sys.stderr)
        return 1
    try:
        data = from_trig(text)
    except TriGParseError as exc:
        print(f"gts from-trig: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


_DB_EXTRA_HINT = "requires the [db] extra: pip install 'gmeow-gts[db]'"


def _cmd_to_sqlite(path: str, out: str) -> int:
    """Export a folded graph to a SQLite database (stdlib, §14)."""
    from gts.db import to_sqlite

    to_sqlite(read(_load(path)), out)
    return 0


def _cmd_to_duckdb(path: str, out: str) -> int:
    """Export a folded graph to a DuckDB database (needs the [db] extra)."""
    from gts.db import to_duckdb

    try:
        to_duckdb(read(_load(path)), out)
    except ImportError:
        print(f"gts to-duckdb: {_DB_EXTRA_HINT}", file=sys.stderr)
        return 2
    return 0


def _cmd_to_parquet(path: str, out_dir: str) -> int:
    """Export a folded graph to Parquet, one file per table (needs [db])."""
    from gts.db import to_parquet

    try:
        for written in to_parquet(read(_load(path)), out_dir):
            print(written)
    except ImportError:
        print(f"gts to-parquet: {_DB_EXTRA_HINT}", file=sys.stderr)
        return 2
    return 0

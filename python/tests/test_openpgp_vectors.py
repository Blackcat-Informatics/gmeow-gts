# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Cross-engine OpenPGP `extract-key` vectors (§9.2): the Python oracle gates
against the same frozen ``vectors/openpgp/*.json`` that the Rust/Go/TS engines
do, and proves the committed corpus is reproducible byte-for-byte."""

from __future__ import annotations

import json
from pathlib import Path

from cryptography.hazmat.primitives import serialization

from gts.emojihash import emojihash
from gts.openpgp import load_public_key, public_key_fingerprint
from gts.verify import format_fingerprint

OPENPGP_DIR = Path(__file__).resolve().parents[2] / "vectors" / "openpgp"


def test_test_key_vector_round_trips() -> None:
    case = json.loads((OPENPGP_DIR / "test-key.json").read_text(encoding="utf-8"))
    raw = load_public_key(case["armored"]).public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    assert raw.hex() == case["raw_pub"]
    fp = public_key_fingerprint(case["armored"])
    assert fp == case["fingerprint"]
    assert format_fingerprint(fp) == case["fingerprint_grouped"]
    assert emojihash(raw) == case["emojihash"]


def test_extract_key_vector_matches_cli() -> None:
    """The frozen end-to-end stdout is exactly what the CLI emits for the file."""
    import io
    from contextlib import redirect_stdout

    from gts.cli import main

    case = json.loads((OPENPGP_DIR / "extract-key.json").read_text(encoding="utf-8"))

    import tempfile

    with tempfile.TemporaryDirectory() as td:
        path = Path(td) / "signed.gts"
        path.write_bytes(bytes.fromhex(case["gts"]))
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = main(["extract-key", str(path)])
    assert rc == 0
    assert buf.getvalue() == case["stdout"]


def test_openpgp_vectors_reproducible() -> None:
    """The committed vectors are exactly what the generator emits."""
    import subprocess
    import sys

    gen = Path(__file__).parents[1] / "scripts" / "gen_openpgp_vectors.py"
    before = {p.name: p.read_text() for p in OPENPGP_DIR.glob("*.json")}
    subprocess.run([sys.executable, str(gen)], check=True, capture_output=True)
    after = {p.name: p.read_text() for p in OPENPGP_DIR.glob("*.json")}
    assert before == after

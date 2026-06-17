# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""randomart conformance vectors: the Python oracle gates against the same frozen
``vectors/randomart/*.json`` that the standalone ``visual-hashing`` Rust crate does."""

from __future__ import annotations

import json
from pathlib import Path

from gts.emojihash import randomart

RANDOMART_DIR = Path(__file__).resolve().parents[2] / "vectors" / "randomart"


def test_randomart_vectors_reproduce() -> None:
    cases = sorted(RANDOMART_DIR.glob("*.json"))
    assert cases, "vectors/randomart must exist"
    for path in cases:
        case = json.loads(path.read_text(encoding="utf-8"))
        data = bytes.fromhex(case["data"])
        assert randomart(data, case["label"]) == case["art"], path.name


def test_randomart_vectors_reproducible() -> None:
    """The committed vectors are exactly what the generator emits."""
    import subprocess
    import sys

    gen = Path(__file__).parents[1] / "scripts" / "gen_randomart_vectors.py"
    before = {
        p.name: p.read_text(encoding="utf-8") for p in RANDOMART_DIR.glob("*.json")
    }
    subprocess.run([sys.executable, str(gen)], check=True, capture_output=True)
    after = {
        p.name: p.read_text(encoding="utf-8") for p in RANDOMART_DIR.glob("*.json")
    }
    assert before == after

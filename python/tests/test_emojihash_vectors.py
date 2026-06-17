# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Cross-engine emojihash vectors: the Python oracle gates against the same
frozen ``vectors/emojihash/*.json`` that the Rust/Go/TS engines do."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from gts.emojihash import _emoji_indices, emojihash, emojihash_labels

EMOJI_DIR = Path(__file__).resolve().parents[2] / "vectors" / "emojihash"


@pytest.mark.parametrize("vector", sorted(p.name for p in EMOJI_DIR.glob("*.json")))
def test_emojihash_vector(vector: str) -> None:
    case = json.loads((EMOJI_DIR / vector).read_text(encoding="utf-8"))
    data = bytes.fromhex(case["data"])
    length = case["length"]
    assert _emoji_indices(data, length) == case["indices"]
    assert emojihash(data, length) == case["emoji"]
    assert emojihash_labels(data, length) == case["labels"]


def test_emojihash_vectors_reproducible() -> None:
    import subprocess
    import sys

    gen = Path(__file__).parents[1] / "scripts" / "gen_emojihash_vectors.py"
    before = {p.name: p.read_text(encoding="utf-8") for p in EMOJI_DIR.glob("*.json")}
    subprocess.run([sys.executable, str(gen)], check=True, capture_output=True)
    after = {p.name: p.read_text(encoding="utf-8") for p in EMOJI_DIR.glob("*.json")}
    assert before == after

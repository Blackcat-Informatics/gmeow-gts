# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Cross-engine COSE_Encrypt0 vector (§9.3): the Python oracle gates against the
same frozen ``vectors/encrypt0/basic.json`` that the Rust/Go/TS engines do.

AES-256-GCM is deterministic for a fixed IV, so the sealed transform can be
frozen; the random-IV production path is covered by the round-trip in
``test_gts_crypto.py``."""

from __future__ import annotations

import json
from pathlib import Path

from gts.crypto import InMemoryKeys, _encrypt0_with_iv, decrypt0

ENCRYPT0_DIR = Path(__file__).resolve().parents[2] / "vectors" / "encrypt0"


def test_encrypt0_vector_seals_and_opens() -> None:
    case = json.loads((ENCRYPT0_DIR / "basic.json").read_text(encoding="utf-8"))
    key = bytes.fromhex(case["key"])
    iv = bytes.fromhex(case["iv"])
    plaintext = bytes.fromhex(case["plaintext"])

    # Fixed IV -> the sealed bytes reproduce the frozen vector exactly.
    sealed = _encrypt0_with_iv(plaintext, case["kid"], key, iv)
    assert sealed.hex() == case["cose"]

    # The frozen COSE opens back to the plaintext under the content key.
    holder = InMemoryKeys(content={case["kid"]: key})
    assert decrypt0(bytes.fromhex(case["cose"]), holder) == plaintext


def test_encrypt0_vectors_reproducible() -> None:
    """The committed vector is exactly what the generator emits."""
    import subprocess
    import sys

    gen = Path(__file__).parents[1] / "scripts" / "gen_encrypt0_vectors.py"
    before = {
        p.name: p.read_text(encoding="utf-8") for p in ENCRYPT0_DIR.glob("*.json")
    }
    subprocess.run([sys.executable, str(gen)], check=True, capture_output=True)
    after = {p.name: p.read_text(encoding="utf-8") for p in ENCRYPT0_DIR.glob("*.json")}
    assert before == after

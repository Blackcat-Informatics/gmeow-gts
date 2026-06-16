# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regenerate the cross-engine emojihash conformance vectors.

emojihash is a deterministic BLAKE3-XOF → 6-bit → 64-emoji mapping, so every
engine (Python is the oracle; Rust/Go/TS gate against the same JSON) must
reproduce the indices, emoji string, and labels for each input. Reproducible:

    uv run python scripts/gen_emojihash_vectors.py
    git diff --exit-code ../vectors/emojihash
"""

from __future__ import annotations

import json
from pathlib import Path

from gts.emojihash import _emoji_indices, emojihash, emojihash_labels

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "emojihash"

# (name, data-hex). The empty input and a 32-byte key are the load-bearing ones.
_CASES = [
    ("empty", ""),
    ("ascii-gts", "677473"),
    ("seq32", "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"),
    ("zeros32", "00" * 32),
]
_LENGTH = 11


def main() -> None:
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    for name, data_hex in _CASES:
        data = bytes.fromhex(data_hex)
        case = {
            "data": data_hex,
            "length": _LENGTH,
            "indices": _emoji_indices(data, _LENGTH),
            "emoji": emojihash(data, _LENGTH),
            "labels": emojihash_labels(data, _LENGTH),
        }
        (VECTORS_DIR / f"{name}.json").write_text(
            json.dumps(case, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
    print(f"wrote {len(_CASES)} emojihash vectors to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

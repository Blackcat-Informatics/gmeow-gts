# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regenerate the randomart conformance vectors.

randomart is the OpenSSH-style "Drunken Bishop" walk: a deterministic function
of the input bytes (and an optional label, which only affects the header line).
Python is the oracle; the `visual-hashing` Rust crate gates against this same
JSON so its port reproduces the art byte-for-byte. Reproducible:

    uv run python scripts/gen_randomart_vectors.py
    git diff --exit-code ../vectors/randomart
"""

from __future__ import annotations

import json
from pathlib import Path

from gts.emojihash import randomart

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "randomart"

# (name, data-hex, label). The empty input, a 32-byte key, and a labeled case
# exercise the start/end markers, the visit-count ramp, and the header.
_CASES = [
    ("empty", "", ""),
    ("ascii-gts", "677473", ""),
    ("seq32", "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f", ""),
    ("zeros32", "00" * 32, ""),
    ("labeled-ed25519", "000102030405060708090a0b0c0d0e0f", "ED25519 256"),
]


def main() -> None:
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    for name, data_hex, label in _CASES:
        data = bytes.fromhex(data_hex)
        case = {
            "data": data_hex,
            "label": label,
            "art": randomart(data, label),
        }
        (VECTORS_DIR / f"{name}.json").write_text(
            json.dumps(case, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
    print(f"wrote {len(_CASES)} randomart vectors to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

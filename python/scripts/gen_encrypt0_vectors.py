# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regenerate the cross-engine COSE_Encrypt0 vector (§9.3).

AES-256-GCM is deterministic once the IV is fixed, so the sealed bytes are
reproducible across every engine. The IV here is a fixed test constant (NOT
from a CSPRNG) precisely so the transform can be frozen; production sealing
(:func:`gts.crypto.encrypt0`) always draws a fresh random IV. Reproducible:

    uv run python scripts/gen_encrypt0_vectors.py
    git diff --exit-code ../vectors/encrypt0
"""

from __future__ import annotations

import json
from pathlib import Path

from gts.crypto import _encrypt0_with_iv

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "encrypt0"

# Fixed test inputs — deterministic, never used for real sealing.
KEY = bytes.fromhex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
IV = bytes.fromhex("a0a1a2a3a4a5a6a7a8a9aaab")
KID = "did:court"
PLAINTEXT = b"verified id record"


def main() -> None:
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    cose = _encrypt0_with_iv(PLAINTEXT, KID, KEY, IV)
    case = {
        "key": KEY.hex(),
        "iv": IV.hex(),
        "kid": KID,
        "plaintext": PLAINTEXT.hex(),
        "cose": cose.hex(),
    }
    (VECTORS_DIR / "basic.json").write_text(
        json.dumps(case, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote COSE_Encrypt0 vector to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

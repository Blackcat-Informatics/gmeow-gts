# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regenerate the cross-engine COSE_Sign1 conformance vectors (GTS-SPEC §9.2).

Ed25519 is deterministic (RFC 8032), so a fixed seed + payload yields a fixed
COSE_Sign1. Every engine (Python is the oracle; Rust/Go/TS gate against the same
JSON) must reproduce ``cose`` byte-for-byte from ``seed`` + ``frame_id`` and
verify it against ``pub``. Reproducible:

    uv run python scripts/gen_cose_vectors.py
    git diff --exit-code ../vectors/cose
"""

from __future__ import annotations

import json
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from gts.crypto import Signer, sign_id

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "cose"

# (name, seed-hex, kid, frame_id-hex)
_CASES = [
    (
        "sign1-basic",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        "test-kid",
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
    ),
    (
        "sign1-empty-id",
        "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20",
        "k2",
        "",
    ),
]


def main() -> None:
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    for name, seed_hex, kid, fid_hex in _CASES:
        seed = bytes.fromhex(seed_hex)
        sk = Ed25519PrivateKey.from_private_bytes(seed)
        pub = sk.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw
        )
        frame_id = bytes.fromhex(fid_hex)
        cose = sign_id(frame_id, Signer(kid, sk))
        case = {
            "alg": "Ed25519",
            "seed": seed_hex,
            "pub": pub.hex(),
            "kid": kid,
            "frame_id": fid_hex,
            "cose": cose.hex(),
        }
        (VECTORS_DIR / f"{name}.json").write_text(
            json.dumps(case, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    print(f"wrote {len(_CASES)} COSE vectors to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

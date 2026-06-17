# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regenerate the cross-engine signed-GTS conformance vector (§9.2).

A small GTS file signed with a deterministic Ed25519 key: every frame carries a
COSE_Sign1 over its content id. Because both the canonical CBOR encoding and
Ed25519 are deterministic, every engine must (a) reproduce these exact bytes when
signing the same content and (b) verify every frame against ``pub``. Reproducible:

    uv run python scripts/gen_signed_vectors.py
    git diff --exit-code ../vectors/signed
"""

from __future__ import annotations

import json
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from gts import Term, TermKind, Writer
from gts.crypto import Signer

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "signed"

CAT = "https://example.org/Cat"
LABEL = "http://www.w3.org/2000/01/rdf-schema#label"


def main() -> None:
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    seed = bytes(range(32))
    sk = Ed25519PrivateKey.from_private_bytes(seed)
    pub = sk.public_key().public_bytes(
        serialization.Encoding.Raw, serialization.PublicFormat.Raw
    )
    kid = "test-kid"

    w = Writer(profile="dist", signer=Signer(kid, sk))
    w.add_terms(
        [
            Term(TermKind.IRI, CAT),
            Term(TermKind.IRI, LABEL),
            Term(TermKind.LITERAL, "Cat", lang="en"),
        ]
    )
    w.add_quads([(0, 1, 2, None)])
    data = w.to_bytes()

    case = {
        "alg": "Ed25519",
        "seed": seed.hex(),
        "pub": pub.hex(),
        "kid": kid,
        "gts": data.hex(),
    }
    (VECTORS_DIR / "basic.json").write_text(
        json.dumps(case, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote signed vector ({len(data)} bytes) to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

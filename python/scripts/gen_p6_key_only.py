# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Derive the P6 case whose composite signature is checked with only its Ed25519 key.

`composite-ed25519-key-only.json` reads `composite-honest.bin` with `held_keys` holding
the Ed25519 half of the composite public key alone. The key identifier of the signature
is the composite key identifier, so no held key has it: the verifier reports
`unverified` with `key_not_held` (P6 step 2, P6-33).

The read record is built from `composite-honest.json`: its readers, registry version,
profiles, capacity and marks are kept; the trusted key is the composite key identifier;
the held key is the Ed25519 key identifier over the last 32 bytes of the composite
public key. Its `read_result` file element is recomputed by `gen_p6_read_results.py`.

Usage, from `python/`:

    # rewrite the derived case if it differs
    uv run python scripts/gen_p6_key_only.py
    # rewrite nothing; exit 1 if it differs
    uv run python scripts/gen_p6_key_only.py --check
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import cbor2

from gen_p6_read_results import rebuild
from gts.v2.signatures import key_identifier

P6_DIR = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P6"
NAME = "composite-ed25519-key-only"
ALGORITHM_COMPOSITE = 2
ALGORITHM_ED25519 = 1
MLDSA65_KEY_LENGTH = 1952
COMPOSITE_KEY_LENGTH = 1984


def _honest_case() -> dict:
    document = json.loads(
        (P6_DIR / "composite-honest.json").read_text(encoding="utf-8")
    )
    return document["cases"][0]


def build_case() -> dict:
    """Return the derived case with its `read_result` rebuilt from the honest input."""
    honest = _honest_case()
    record = cbor2.loads(bytes.fromhex(honest["read_result"]))
    _, _, _, _, _, inputs, _ = record
    trusted, held = inputs
    [(composite_id, algorithm, composite_key)] = held
    if algorithm != ALGORITHM_COMPOSITE or len(composite_key) != COMPOSITE_KEY_LENGTH:
        raise ValueError("composite-honest.json does not hold one composite key")
    if trusted != [composite_id]:
        raise ValueError("composite-honest.json does not trust its composite key")
    ed25519_key = composite_key[MLDSA65_KEY_LENGTH:]
    ed25519_id = key_identifier(ALGORITHM_ED25519, ed25519_key)
    case = {
        "label": "",
        "input_file": "composite-honest.bin",
        "capacity": honest["capacity"],
        "read": {
            "trusted_keys": [composite_id.hex()],
            "held_keys": [[ed25519_id.hex(), ALGORITHM_ED25519, ed25519_key.hex()]],
        },
        "read_result": cbor2.dumps(
            [
                record[0],
                record[1],
                record[2],
                record[3],
                record[4],
                [[composite_id], [[ed25519_id, ALGORITHM_ED25519, ed25519_key]]],
                record[6],
            ],
            canonical=True,
        ).hex(),
        "result": [
            "unverified",
            "key_not_held",
            ALGORITHM_COMPOSITE,
            composite_id.hex(),
            1,
            1,
            False,
        ],
    }
    case["read_result"] = rebuild(P6_DIR, case)
    return case


def build() -> str:
    """Return the text of the derived case document."""
    document = {
        "name": NAME,
        "status": "built and matches",
        "note": (
            "the composite signature of composite-honest.bin checked with only the "
            "Ed25519 half of the composite key held; derived by "
            "python/scripts/gen_p6_key_only.py"
        ),
        "cases": [build_case()],
    }
    return json.dumps(document, indent=1, ensure_ascii=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Derive the P6 composite case checked with only its Ed25519 key."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="rewrite nothing; exit 1 if the derived case differs",
    )
    args = parser.parse_args(argv)
    path = P6_DIR / f"{NAME}.json"
    text = build()
    if path.exists() and path.read_text(encoding="utf-8") == text:
        print(f"{NAME}.json matches")
        return 0
    if args.check:
        print(f"differs: {NAME}.json")
        return 1
    path.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote: {NAME}.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())

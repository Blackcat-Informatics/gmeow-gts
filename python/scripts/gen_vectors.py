# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Regenerate the frozen GTS conformance corpus (GTS-SPEC §18).

The Python reference implementation (``gts.vectors``) is the single source of
truth for the language-neutral corpus that all four engines (Rust, Python, Go,
TypeScript) gate against. This script writes one ``<name>.gts`` (canonical bytes)
and one ``<name>.expected.json`` (oracle-folded expectation) per case into the
repository's ``vectors/`` directory.

The committed corpus is reproducible byte-for-byte:

    uv run python scripts/gen_vectors.py
    git diff --exit-code ../vectors        # no changes => reproducible

Run from anywhere; output location is resolved relative to this file.
"""

from __future__ import annotations

import json
from pathlib import Path

from gts.vectors import corpus, expected_for

# scripts/ -> python/ -> <repo root>; corpus lives at <repo root>/vectors
VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors"


def main() -> None:
    """Write corpus bytes and oracle-computed expectations into ``vectors/``."""
    VECTORS_DIR.mkdir(parents=True, exist_ok=True)
    count = 0
    for case in corpus():
        (VECTORS_DIR / f"{case.name}.gts").write_bytes(case.data)
        (VECTORS_DIR / f"{case.name}.expected.json").write_text(
            json.dumps(expected_for(case), indent=1, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        count += 1
    print(f"wrote {count} vector cases to {VECTORS_DIR}")


if __name__ == "__main__":
    main()

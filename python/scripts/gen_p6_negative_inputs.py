# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Derive the P6 negative composite input files from the corpus files they alter.

Each derived file is the input of one recorded case. Its producer is this program, so
the bytes of the file can be rebuilt from the file it alters.

`composite-ed25519-zeroed.bin` is `composite-honest.bin` with the Ed25519 half of its
composite signature value set to zero. The value is the only byte string of 3373 bytes
in the file, written with the CBOR header `59 0d 2d` and preceded by the 32-byte key
identifier `58 20`. The program refuses an input whose layout it does not find exactly
once.

Usage, from `python/`:

    # rewrite each derived file that differs
    uv run python scripts/gen_p6_negative_inputs.py
    # rewrite nothing; exit 1 if any differs
    uv run python scripts/gen_p6_negative_inputs.py --check
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

P6_DIR = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P6"

COMPOSITE_VALUE_LENGTH = 3373
ED25519_HALF_LENGTH = 64
_KEY_ID_HEADER = b"\x58\x20"
_VALUE_HEADER = b"\x59\x0d\x2d"
_KEY_ID_LENGTH = 32


def _value_offset(honest: bytes) -> int:
    """Return the offset of the composite signature value in `honest`."""
    offsets = []
    start = honest.find(_VALUE_HEADER)
    while start != -1:
        offsets.append(start)
        start = honest.find(_VALUE_HEADER, start + 1)
    if len(offsets) != 1:
        raise ValueError(f"expected one composite value header, found {len(offsets)}")
    offset = offsets[0]
    key_start = offset - 2 - _KEY_ID_LENGTH
    if key_start < 0 or honest[key_start : key_start + 2] != _KEY_ID_HEADER:
        raise ValueError(
            "the composite value header is not preceded by a 32-byte key identifier"
        )
    if len(honest) < offset + 3 + COMPOSITE_VALUE_LENGTH:
        raise ValueError("the composite value is truncated")
    return offset + 3


def zeroed_ed25519_half(honest: bytes) -> bytes:
    """Return `honest` with the Ed25519 half of its composite value set to zero."""
    value = _value_offset(honest)
    end = value + COMPOSITE_VALUE_LENGTH
    half = end - ED25519_HALF_LENGTH
    return honest[:half] + bytes(ED25519_HALF_LENGTH) + honest[end:]


def build(folder: Path) -> dict[str, bytes]:
    """Return every derived input file of the P6 corpus by name."""
    honest = (folder / "composite-honest.bin").read_bytes()
    return {"composite-ed25519-zeroed.bin": zeroed_ed25519_half(honest)}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Derive the P6 negative composite input files from the corpus."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="rewrite nothing; exit 1 if any derived file differs",
    )
    args = parser.parse_args(argv)
    differing = []
    for name, data in build(P6_DIR).items():
        path = P6_DIR / name
        if not path.exists() or path.read_bytes() != data:
            differing.append(name)
            if not args.check:
                path.write_bytes(data)
    for name in differing:
        print(("differs: " if args.check else "wrote: ") + name)
    print(f"{len(build(P6_DIR))} derived input files, {len(differing)} differ")
    return 1 if args.check and differing else 0


if __name__ == "__main__":
    sys.exit(main())

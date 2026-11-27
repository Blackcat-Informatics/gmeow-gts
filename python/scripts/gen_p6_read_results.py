# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Rebuild the recorded `read_result` of every P6 signature case.

Each case of `vectors/v2/P6/*.json` that records a `read_result` names its input file
and its capacity. The read record is
`[readers, registry_version, profiles, capacity, marks, inputs, file]`. The rebuild
keeps the record's declared inputs (`readers`, `registry_version`, `profiles`, `marks`,
`inputs`) and its `capacity`, which must equal the case's `capacity`. It recomputes only
the `file` element with the Python reader and its encoder. Trusted and held keys are
read from `inputs`, as the signature verifier of the read takes them.

The reader runs with no profiles, no marks and whole-read access. A case that declares
anything else, or that declares a reader holding the reference encoder, is refused: the
rebuild does not implement those inputs.

Usage, from `python/`:

    # rewrite each recorded result that differs
    uv run python scripts/gen_p6_read_results.py
    # rewrite nothing; exit 1 if any differs
    uv run python scripts/gen_p6_read_results.py --check
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import cbor2

from gts.v2.content import ContentCapacity, read_content
from gts.v2.read_encoding import encode_file
from gts.v2.signatures import HeldKey, SignatureVerifier
from gts.v2.typed_file import file_result

P6_DIR = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P6"

# The `capacity` element of the record lists the nine capacity counts in this order,
# then the access enumeration. The case's `capacity` object names the same counts.
_CAPACITY_ORDER = (
    "stored_bytes_per_frame",
    "decoded_bytes_per_frame",
    "labelling_units",
    "labelled_lines",
    "decoded_bytes_per_read",
    "items_per_read",
    "rebuilt_bytes_per_read",
    "formed_bytes_per_read",
    "labelled_bytes_per_read",
)


def _capacity(case: dict) -> ContentCapacity:
    counts = case["capacity"]
    return ContentCapacity(
        counts["stored_bytes_per_frame"],
        counts["decoded_bytes_per_frame"],
        counts["decoded_bytes_per_read"],
        counts["items_per_read"],
        counts["rebuilt_bytes_per_read"],
        counts["formed_bytes_per_read"],
        counts["labelled_lines"],
        counts["labelled_bytes_per_read"],
        counts["labelling_units"],
    )


_ABSENT_OPTION = [1]  # option(limits): `[1]` is absent, `[2, value]` is present
_READ_KEYS = frozenset(
    (
        "profiles",
        "trusted_keys",
        "held_keys",
        "sealing_input",
        "marks",
        "holds_reference_encoder",
    )
)


def _refuse_undeclared_inputs(case: dict, record: list) -> None:
    """Refuse a case whose read declares or records an input this rebuild does not run.

    The reader runs with the record's trusted and held keys, no profiles, no marks, no
    sealing input, whole-read access, and no reference encoder. Each declaration that
    the case makes must say the same.
    """
    label = case.get("input_file")
    readers, _, profiles, _, marks, inputs, _ = record
    if readers not in (1, 2):
        raise ValueError(
            f"{label}: readers {readers} is not a generation this rebuild reads"
        )
    if profiles != [] or marks != _ABSENT_OPTION:
        raise ValueError(f"{label}: the record declares profiles or marks, not rebuilt")
    if case["capacity"].get("access") != "whole":
        raise ValueError(f"{label}: the capacity access is not whole, not rebuilt")
    declared = case.get("read", {})
    unknown = set(declared) - _READ_KEYS
    if unknown:
        raise ValueError(f"{label}: the read declares {sorted(unknown)}, not rebuilt")
    if declared.get("holds_reference_encoder", False):
        raise ValueError(
            f"{label}: the reader holds the reference encoder, not rebuilt"
        )
    if declared.get("profiles", []) != [] or declared.get("sealing_input", []) not in (
        [],
        None,
    ):
        raise ValueError(
            f"{label}: the read declares profiles or a sealing input, not rebuilt"
        )
    if declared.get("marks") is not None:
        raise ValueError(f"{label}: the read declares marks, not rebuilt")
    trusted, held = inputs
    if "trusted_keys" in declared and declared["trusted_keys"] != [
        key.hex() for key in trusted
    ]:
        raise ValueError(f"{label}: the declared trusted keys differ from the record's")
    if "held_keys" in declared and declared["held_keys"] != [
        [key.hex(), algorithm, public.hex()] for key, algorithm, public in held
    ]:
        raise ValueError(f"{label}: the declared held keys differ from the record's")


def rebuild(folder: Path, case: dict) -> str:
    """Return the hex of the read record of `case`, rebuilt from its input file."""
    record = cbor2.loads(bytes.fromhex(case["read_result"]))
    _refuse_undeclared_inputs(case, record)
    readers, registry_version, profiles, recorded_capacity, marks, inputs, _ = record
    counts = case["capacity"]
    if list(recorded_capacity[:9]) != [counts[name] for name in _CAPACITY_ORDER]:
        raise ValueError(
            f"{case.get('input_file')}: the record's capacity differs from the case's"
        )
    capacity = _capacity(case)
    trusted, held = inputs
    verifier = SignatureVerifier(
        trusted_keys=trusted, held_keys=[HeldKey(*key) for key in held]
    )
    raw = (folder / case["input_file"]).read_bytes()
    read = read_content(raw, capacity, verifier=verifier, first_and_second=readers == 2)
    file = encode_file(file_result(read, raw, capacity))
    rebuilt = [
        readers,
        registry_version,
        profiles,
        recorded_capacity,
        marks,
        inputs,
        file,
    ]
    return cbor2.dumps(rebuilt, canonical=True).hex()


def _cases(folder: Path):
    for path in sorted(folder.glob("*.json")):
        document = json.loads(path.read_text(encoding="utf-8"))
        for index, case in enumerate(document.get("cases") or []):
            if "read_result" in case:
                yield path, document, index, case


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Rebuild the recorded P6 read results from their input files."
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="rewrite nothing; exit 1 if any recorded result differs",
    )
    args = parser.parse_args(argv)
    differing: list[str] = []
    rewritten: dict[Path, tuple[dict, str]] = {}
    total = 0
    for path, document, index, case in _cases(P6_DIR):
        total += 1
        rebuilt = rebuild(P6_DIR, case)
        if rebuilt != case["read_result"]:
            differing.append(f"{path.name}[{index}] {case.get('input_file')}")
            case["read_result"] = rebuilt
            rewritten[path] = (document, path.read_text(encoding="utf-8"))
    if args.check or not differing:
        for line in differing:
            print("differs:", line)
        print(f"{total} read results rebuilt, {len(differing)} differ")
        return 1 if differing else 0
    for path, (document, original) in rewritten.items():
        newline = "\n" if original.endswith("\n") else ""
        path.write_text(
            json.dumps(document, indent=1, ensure_ascii=False) + newline,
            encoding="utf-8",
            newline="\n",
        )
    for line in differing:
        print("rewrote:", line)
    print(f"{total} read results rebuilt, {len(differing)} rewritten")
    return 0


if __name__ == "__main__":
    sys.exit(main())

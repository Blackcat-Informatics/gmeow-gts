#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Guard the Rust all-features wasm dependency tree."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

TREE_COMMAND = [
    "cargo",
    "tree",
    "--locked",
    "--manifest-path",
    "rust/Cargo.toml",
    "--target",
    "wasm32-unknown-unknown",
    "--all-features",
    "--edges",
    "normal,build",
    "--prefix",
    "none",
    "--format",
    "{p}",
]

FORBIDDEN_CRATES = {
    "oxrdf",
    "oxttl",
    "oxrdfxml",
    "oxigraph",
    "uuid",
}

PACKAGE_RE = re.compile(r"^(?P<name>[A-Za-z0-9_.-]+) v(?P<version>[0-9][^\s]*)")


def fail(message: str) -> None:
    print(f"check_rust_wasm_dependency_audit: {message}", file=sys.stderr)
    raise SystemExit(1)


def is_forbidden(name: str, version: str) -> bool:
    if name in FORBIDDEN_CRATES:
        return True
    if name.startswith(("sophia_", "sophia-")):
        return True
    return name == "getrandom" and version.startswith("0.3")


def main() -> int:
    proc = subprocess.run(
        TREE_COMMAND,
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        if proc.stdout:
            print(proc.stdout, end="")
        if proc.stderr:
            print(proc.stderr, end="", file=sys.stderr)
        fail(f"cargo tree failed with exit code {proc.returncode}")

    offenders: list[str] = []
    for raw_line in proc.stdout.splitlines():
        line = raw_line.strip()
        match = PACKAGE_RE.match(line)
        if not match:
            continue
        name = match.group("name")
        version = match.group("version")
        if is_forbidden(name, version):
            offenders.append(f"{name} v{version}")

    if offenders:
        formatted = "\n  - ".join(sorted(set(offenders)))
        fail(
            "forbidden packages in the wasm32 all-features dependency tree:\n"
            f"  - {formatted}\n"
            "inspect with: "
            + " ".join(TREE_COMMAND[:-4])
            + " -i <crate>"
        )

    print("check_rust_wasm_dependency_audit: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

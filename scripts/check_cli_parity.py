#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check the public CLI parity contract against implemented dispatch tables."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "docs" / "GTS-API-CLI-PARITY.md"
README = ROOT / "README.md"
ENGINES = ("Python", "Rust", "Go", "TypeScript", "Smalltalk")


def between(text: str, start: str, end: str) -> str:
    start_parts = text.split(start, 1)
    if len(start_parts) != 2:
        raise ValueError(f"missing marker pair: {start} ... {end}") from None
    end_parts = start_parts[1].split(end, 1)
    if len(end_parts) != 2:
        raise ValueError(f"missing marker pair: {start} ... {end}") from None
    return end_parts[0]


def parse_contract_matrix() -> dict[str, dict[str, bool]]:
    text = CONTRACT.read_text(encoding="utf-8")
    block = between(
        text,
        "<!-- cli-parity-matrix:start -->",
        "<!-- cli-parity-matrix:end -->",
    )
    matrix: dict[str, dict[str, bool]] = {}
    for line in block.splitlines():
        line = line.strip()
        if not line.startswith("| `"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        expected_cells = len(ENGINES) + 2
        if len(cells) != expected_cells:
            raise ValueError(f"bad CLI parity row: {line}")
        verb = cells[0].strip("`")
        row = {}
        for engine, value in zip(ENGINES, cells[1 : 1 + len(ENGINES)], strict=True):
            if value not in {"yes", "no"}:
                raise ValueError(f"{verb}: {engine} must be yes/no, got {value!r}")
            row[engine] = value == "yes"
        matrix[verb] = row
    if not matrix:
        raise ValueError("CLI parity matrix is empty")
    return matrix


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def python_verbs() -> set[str]:
    return set(re.findall(r'add_parser\(\s*"([^"]+)"', read("python/src/gts/cli.py")))


def rust_verbs() -> set[str]:
    text = read("rust/src/bin/gts.rs")
    block = between(text, "match cmd.as_str() {", "_ =>")
    return set(re.findall(r'"([^"]+)"\s*=>\s*cmd_', block))


def go_verbs() -> set[str]:
    text = read("go/cmd/gts/main.go")
    block = between(text, "switch cmd {", 'case "-h", "--help", "help":')
    return set(re.findall(r'case "([^"]+)":', block))


def typescript_verbs() -> set[str]:
    text = read("ts/src/bin/gts.ts")
    block = between(text, "switch (cmd) {", 'case "-h":')
    return set(re.findall(r'case "([^"]+)":', block))


def smalltalk_verbs() -> set[str]:
    text = read("smalltalk/src/Gts-Core/GtsCLI.class.st")
    return set(re.findall(r"command = '([^']+)' ifTrue:", text))


def readme_verbs(start: str, end: str) -> set[str]:
    block = between(README.read_text(encoding="utf-8"), start, end)
    return set(re.findall(r"^gts\s+([a-z0-9-]+)\b", block, flags=re.MULTILINE))


def compare_engine(
    errors: list[str],
    engine: str,
    implemented: set[str],
    matrix: dict[str, dict[str, bool]],
) -> None:
    expected = {verb for verb, row in matrix.items() if row[engine]}
    missing = sorted(expected - implemented)
    extra = sorted(implemented - expected)
    if missing:
        errors.append(f"{engine}: matrix says yes but dispatch lacks {missing}")
    if extra:
        errors.append(f"{engine}: dispatch implements verbs absent/marked no: {extra}")


def main() -> int:
    matrix = parse_contract_matrix()
    errors: list[str] = []

    implemented_by_engine = {
        "Python": python_verbs(),
        "Rust": rust_verbs(),
        "Go": go_verbs(),
        "TypeScript": typescript_verbs(),
        "Smalltalk": smalltalk_verbs(),
    }
    for engine, implemented in implemented_by_engine.items():
        compare_engine(errors, engine, implemented, matrix)

    common = {verb for verb, row in matrix.items() if all(row.values())}
    python_extensions = {
        verb
        for verb, row in matrix.items()
        if row["Python"]
        and not any(row[engine] for engine in ENGINES if engine != "Python")
    }
    readme_common = readme_verbs(
        "<!-- cli-common:start -->",
        "<!-- cli-common:end -->",
    )
    readme_python_extensions = readme_verbs(
        "<!-- cli-python-extensions:start -->",
        "<!-- cli-python-extensions:end -->",
    )
    if readme_common != common:
        errors.append(
            "README common CLI block drifted: "
            f"missing={sorted(common - readme_common)} extra={sorted(readme_common - common)}"
        )
    if readme_python_extensions != python_extensions:
        errors.append(
            "README Python extension CLI block drifted: "
            "missing="
            f"{sorted(python_extensions - readme_python_extensions)} "
            f"extra={sorted(readme_python_extensions - python_extensions)}"
        )

    if errors:
        for error in errors:
            print(f"check_cli_parity: {error}", file=sys.stderr)
        return 1
    print("check_cli_parity: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

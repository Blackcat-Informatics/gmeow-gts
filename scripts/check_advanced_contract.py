#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Guard the advanced-primitives deferral contract."""

from __future__ import annotations

import sys
from pathlib import Path

from check_cli_parity import (
    ENGINES,
    ROOT,
    between,
    go_verbs,
    parse_contract_matrix,
    python_verbs,
    rust_verbs,
    typescript_verbs,
)

CONTRACT = ROOT / "docs" / "GTS-ADVANCED-PRIMITIVES.md"


def deferred_cli_verbs() -> set[str]:
    block = between(
        CONTRACT.read_text(encoding="utf-8"),
        "<!-- advanced-cli-deferred:start -->",
        "<!-- advanced-cli-deferred:end -->",
    )
    verbs: set[str] = set()
    for line in block.splitlines():
        line = line.strip()
        if not line.startswith("| `"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 3:
            raise ValueError(f"bad advanced CLI deferral row: {line}")
        verb = cells[0].strip("`")
        status = cells[1].lower()
        if status != "deferred":
            raise ValueError(f"{verb}: advanced CLI status must be deferred")
        verbs.add(verb)
    return verbs


def main() -> int:
    deferred = deferred_cli_verbs()
    matrix_verbs = set(parse_contract_matrix())
    implemented_by_engine = {
        "Python": python_verbs(),
        "Rust": rust_verbs(),
        "Go": go_verbs(),
        "TypeScript": typescript_verbs(),
    }
    errors: list[str] = []

    matrix_overlap = sorted(deferred & matrix_verbs)
    if matrix_overlap:
        errors.append(
            "deferred advanced verbs appear in the public CLI parity matrix: "
            f"{matrix_overlap}"
        )

    for engine in ENGINES:
        implemented_overlap = sorted(deferred & implemented_by_engine[engine])
        if implemented_overlap:
            errors.append(
                f"{engine}: deferred advanced verbs are implemented: "
                f"{implemented_overlap}"
            )

    if errors:
        for error in errors:
            print(f"check_advanced_contract: {error}", file=sys.stderr)
        return 1
    print("check_advanced_contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

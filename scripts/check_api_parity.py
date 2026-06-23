#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check the public API parity declaration against docs and source evidence."""

from __future__ import annotations

import ast
import json
import re
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DECLARATION = ROOT / "docs" / "api-parity.json"
CONTRACT = ROOT / "docs" / "GTS-API-CLI-PARITY.md"
README = ROOT / "README.md"
ENGINES = ("Python", "Rust", "Go", "TypeScript", "Smalltalk", "Kotlin")
README_ENGINE_HEADERS = {
    "Python": "Python",
    "Rust": "Rust",
    "Go": "Go",
    "TypeScript": "TypeScript",
    "Smalltalk/Pharo": "Smalltalk",
    "Kotlin/JVM": "Kotlin",
}


class ApiParityError(ValueError):
    """Raised when a source document cannot be parsed as expected."""


def fail(message: str) -> None:
    print(f"check_api_parity: {message}", file=sys.stderr)
    raise SystemExit(1)


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT))


def repo_path(value: str) -> Path:
    path = ROOT / value
    if ROOT not in path.resolve().parents and path.resolve() != ROOT:
        raise ApiParityError(f"path escapes repository root: {value}")
    return path


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def between(text: str, start: str, end: str) -> str:
    start_parts = text.split(start, 1)
    if len(start_parts) != 2:
        raise ApiParityError(f"missing marker pair: {start} ... {end}") from None
    end_parts = start_parts[1].split(end, 1)
    if len(end_parts) != 2:
        raise ApiParityError(f"missing marker pair: {start} ... {end}") from None
    return end_parts[0]


def split_table_line(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def is_separator(cells: list[str]) -> bool:
    return all(cell and set(cell) <= {"-", ":"} for cell in cells)


def parse_markdown_table(block: str) -> list[dict[str, str]]:
    header: list[str] | None = None
    rows: list[dict[str, str]] = []
    for line in block.splitlines():
        stripped = line.strip()
        if not stripped.startswith("|"):
            continue
        cells = split_table_line(stripped)
        if is_separator(cells):
            continue
        if header is None:
            header = cells
            continue
        if len(cells) != len(header):
            raise ApiParityError(f"bad table row: {line}")
        rows.append(dict(zip(header, cells, strict=True)))
    if header is None or not rows:
        raise ApiParityError("expected a non-empty Markdown table")
    return rows


def load_declaration() -> Mapping[str, Any]:
    try:
        data = json.loads(read_text(DECLARATION))
    except json.JSONDecodeError as exc:
        fail(f"{rel(DECLARATION)} is not valid JSON: {exc}")
    if not isinstance(data, Mapping):
        fail(f"{rel(DECLARATION)} must be a JSON object")
    return data


def parse_api_shape_rows() -> dict[str, dict[str, str]]:
    block = between(
        read_text(CONTRACT),
        "<!-- api-parity-shape:start -->",
        "<!-- api-parity-shape:end -->",
    )
    rows = parse_markdown_table(block)
    required = {"operation", "contract", "current native surface"}
    if set(rows[0]) != required:
        raise ApiParityError(
            f"{rel(CONTRACT)} API shape table headers must be {sorted(required)}"
        )
    return {row["operation"]: row for row in rows}


def parse_readme_feature_rows() -> dict[str, dict[str, str]]:
    block = between(
        read_text(README),
        "<!-- api-feature-matrix:start -->",
        "<!-- api-feature-matrix:end -->",
    )
    rows = parse_markdown_table(block)
    required_headers = {"Capability", *README_ENGINE_HEADERS}
    if set(rows[0]) != required_headers:
        raise ApiParityError(
            f"{rel(README)} feature matrix headers must be {sorted(required_headers)}"
        )
    parsed: dict[str, dict[str, str]] = {}
    for row in rows:
        parsed[row["Capability"]] = {
            engine: row[header] for header, engine in README_ENGINE_HEADERS.items()
        }
    return parsed


def expect_mapping(value: Any, label: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise ApiParityError(f"{label} must be an object")
    return value


def expect_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ApiParityError(f"{label} must be a non-empty string")
    return value


def expect_list(value: Any, label: str) -> list[Any]:
    if not isinstance(value, list):
        raise ApiParityError(f"{label} must be a list")
    return value


def python_all_symbols(path: Path) -> set[str]:
    module = ast.parse(read_text(path), filename=rel(path))
    for node in module.body:
        if not isinstance(node, ast.Assign):
            continue
        if not any(isinstance(target, ast.Name) and target.id == "__all__" for target in node.targets):
            continue
        if not isinstance(node.value, ast.List):
            raise ApiParityError(f"{rel(path)} __all__ must be a list literal")
        symbols = set()
        for elt in node.value.elts:
            if not isinstance(elt, ast.Constant) or not isinstance(elt.value, str):
                raise ApiParityError(f"{rel(path)} __all__ must contain only strings")
            symbols.add(elt.value)
        return symbols
    raise ApiParityError(f"{rel(path)} missing __all__")


def check_contains(item: Mapping[str, Any], label: str) -> list[str]:
    path_text = expect_string(item.get("path"), f"{label}.path")
    needle = expect_string(item.get("text"), f"{label}.text")
    path = repo_path(path_text)
    if not path.is_file():
        return [f"{label}: missing file {path_text}"]
    if needle not in read_text(path):
        return [f"{label}: {path_text} missing {needle!r}"]
    return []


def check_path_exists(item: Mapping[str, Any], label: str) -> list[str]:
    path_text = expect_string(item.get("path"), f"{label}.path")
    if not repo_path(path_text).exists():
        return [f"{label}: missing path {path_text}"]
    return []


def check_python_all(item: Mapping[str, Any], label: str) -> list[str]:
    symbol = expect_string(item.get("symbol"), f"{label}.symbol")
    path_text = str(item.get("path") or "python/src/gts/__init__.py")
    path = repo_path(path_text)
    if not path.is_file():
        return [f"{label}: missing file {path_text}"]
    if symbol not in python_all_symbols(path):
        return [f"{label}: {path_text} __all__ missing {symbol!r}"]
    return []


def check_rust_pub_mod(item: Mapping[str, Any], label: str) -> list[str]:
    module = expect_string(item.get("module"), f"{label}.module")
    text = read_text(ROOT / "rust" / "src" / "lib.rs")
    pattern = rf"(?m)^\s*(?:#\[cfg\([^\n]+\)\]\s*)?pub mod {re.escape(module)};"
    if not re.search(pattern, text):
        return [f"{label}: rust/src/lib.rs missing public module {module!r}"]
    return []


def check_go_package(item: Mapping[str, Any], label: str) -> list[str]:
    package = expect_string(item.get("package"), f"{label}.package")
    path = ROOT / "go" / package / f"{package}.go"
    if not path.is_file():
        return [f"{label}: missing Go package file {rel(path)}"]
    if f"package {package}" not in read_text(path):
        return [f"{label}: {rel(path)} does not declare package {package!r}"]
    return []


def check_smalltalk_class(item: Mapping[str, Any], label: str) -> list[str]:
    class_name = expect_string(item.get("class"), f"{label}.class")
    path = ROOT / "smalltalk" / "src" / "Gts-Core" / f"{class_name}.class.st"
    if not path.is_file():
        return [f"{label}: missing Smalltalk class {rel(path)}"]
    if f"#name : #{class_name}" not in read_text(path):
        return [f"{label}: {rel(path)} does not declare {class_name}"]
    return []


def check_kotlin_file(item: Mapping[str, Any], label: str) -> list[str]:
    filename = expect_string(item.get("file"), f"{label}.file")
    path = ROOT / "kotlin" / "src" / "main" / "kotlin" / "ca" / "blackcatinformatics" / "gts" / filename
    if not path.is_file():
        return [f"{label}: missing Kotlin file {rel(path)}"]
    return []


EVIDENCE_CHECKS = {
    "contains": check_contains,
    "path_exists": check_path_exists,
    "python_all": check_python_all,
    "rust_pub_mod": check_rust_pub_mod,
    "go_package": check_go_package,
    "smalltalk_class": check_smalltalk_class,
    "kotlin_file": check_kotlin_file,
}


def check_evidence(item: Any, label: str) -> list[str]:
    evidence = expect_mapping(item, label)
    kind = expect_string(evidence.get("kind"), f"{label}.kind")
    checker = EVIDENCE_CHECKS.get(kind)
    if checker is None:
        return [f"{label}: unknown evidence kind {kind!r}"]
    return checker(evidence, label)


def validate_api_shape(declaration: Mapping[str, Any], errors: list[str]) -> None:
    docs = parse_api_shape_rows()
    declared_rows = expect_list(declaration.get("api_shape"), "api_shape")
    declared: dict[str, Mapping[str, Any]] = {}
    for index, raw_row in enumerate(declared_rows):
        row = expect_mapping(raw_row, f"api_shape[{index}]")
        operation = expect_string(row.get("operation"), f"api_shape[{index}].operation")
        if operation in declared:
            errors.append(f"api_shape declares {operation!r} more than once")
        declared[operation] = row

    doc_operations = set(docs)
    declared_operations = set(declared)
    if doc_operations != declared_operations:
        errors.append(
            "API shape table drifted: "
            f"missing={sorted(doc_operations - declared_operations)} "
            f"extra={sorted(declared_operations - doc_operations)}"
        )
        return

    for operation, row in declared.items():
        doc_row = docs[operation]
        expected_contract = expect_string(row.get("contract"), f"{operation}.contract")
        expected_surface = expect_string(
            row.get("current_native_surface"),
            f"{operation}.current_native_surface",
        )
        if doc_row["contract"] != expected_contract:
            errors.append(f"{operation}: API contract text drifted from declaration")
        if doc_row["current native surface"] != expected_surface:
            errors.append(f"{operation}: API native surface text drifted from declaration")


def validate_feature_matrix(declaration: Mapping[str, Any], errors: list[str]) -> None:
    readme_rows = parse_readme_feature_rows()
    declared_rows = expect_list(
        declaration.get("readme_feature_matrix"),
        "readme_feature_matrix",
    )
    declared: dict[str, Mapping[str, Any]] = {}
    for index, raw_row in enumerate(declared_rows):
        row = expect_mapping(raw_row, f"readme_feature_matrix[{index}]")
        capability = expect_string(row.get("capability"), f"feature[{index}].capability")
        if capability in declared:
            errors.append(f"feature matrix declares {capability!r} more than once")
        declared[capability] = row

    readme_capabilities = set(readme_rows)
    declared_capabilities = set(declared)
    if readme_capabilities != declared_capabilities:
        errors.append(
            "README engine feature matrix drifted: "
            f"missing={sorted(readme_capabilities - declared_capabilities)} "
            f"extra={sorted(declared_capabilities - readme_capabilities)}"
        )
        return

    for capability, row in declared.items():
        cells = expect_mapping(row.get("cells"), f"{capability}.cells")
        claims = expect_mapping(row.get("claims"), f"{capability}.claims")
        if set(cells) != set(ENGINES):
            errors.append(f"{capability}: cells must declare exactly {list(ENGINES)}")
            continue
        if set(claims) != set(ENGINES):
            errors.append(f"{capability}: claims must declare exactly {list(ENGINES)}")
            continue
        for engine in ENGINES:
            expected_cell = expect_string(cells.get(engine), f"{capability}.{engine}.cell")
            actual_cell = readme_rows[capability][engine]
            if actual_cell != expected_cell:
                errors.append(
                    f"{capability}: README {engine} cell is {actual_cell!r}, "
                    f"expected {expected_cell!r}"
                )
            claim = expect_mapping(claims[engine], f"{capability}.{engine}")
            status = expect_string(claim.get("status"), f"{capability}.{engine}.status")
            if status == "supported":
                if expected_cell == "no":
                    errors.append(f"{capability}: {engine} is supported but README says no")
                evidence_items = expect_list(
                    claim.get("evidence"),
                    f"{capability}.{engine}.evidence",
                )
                if not evidence_items:
                    errors.append(f"{capability}: {engine} supported claim lacks evidence")
                for evidence_index, evidence in enumerate(evidence_items):
                    errors.extend(
                        check_evidence(
                            evidence,
                            f"{capability}.{engine}.evidence[{evidence_index}]",
                        )
                    )
            elif status == "deferred":
                if expected_cell != "no":
                    errors.append(
                        f"{capability}: {engine} is deferred but README cell is "
                        f"{expected_cell!r}"
                    )
                reason = claim.get("deferred_reason")
                if not isinstance(reason, str) or not reason.strip():
                    errors.append(f"{capability}: {engine} deferral lacks a reason")
            else:
                errors.append(f"{capability}: {engine} has unknown status {status!r}")


def validate_schema(declaration: Mapping[str, Any], errors: list[str]) -> None:
    if declaration.get("schema") != "gts-api-parity-v1":
        errors.append("schema must be 'gts-api-parity-v1'")
    full_engines = declaration.get("full_engines")
    if not isinstance(full_engines, list) or tuple(full_engines) != ENGINES:
        errors.append(f"full_engines must be {list(ENGINES)}")
    wrappers = declaration.get("wrapper_surfaces")
    if not isinstance(wrappers, list) or not all(isinstance(item, str) for item in wrappers):
        errors.append("wrapper_surfaces must be a list of strings")
        return
    overlap = sorted(set(wrappers) & set(ENGINES))
    if overlap:
        errors.append(f"wrapper surfaces overlap full engines: {overlap}")
    contract_text = read_text(CONTRACT)
    for wrapper in wrappers:
        if wrapper not in contract_text:
            errors.append(f"wrapper surface {wrapper!r} is absent from {rel(CONTRACT)}")


def main() -> int:
    try:
        declaration = load_declaration()
        errors: list[str] = []
        validate_schema(declaration, errors)
        validate_api_shape(declaration, errors)
        validate_feature_matrix(declaration, errors)
    except ApiParityError as exc:
        fail(str(exc))

    if errors:
        for error in errors:
            print(f"check_api_parity: {error}", file=sys.stderr)
        return 1
    print("check_api_parity: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

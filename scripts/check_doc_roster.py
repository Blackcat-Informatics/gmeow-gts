#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check current documentation roster and spec-version claims against the registry."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "docs" / "documentation-roster.json"
SPEC = ROOT / "docs" / "GTS-SPEC.md"
CHANGELOG = ROOT / "CHANGELOG.md"

ENGINE_LABELS = {
    "rust": "Rust",
    "python": "Python",
    "go": "Go",
    "typescript": "TypeScript",
    "smalltalk": "Smalltalk/Pharo",
    "kotlin": "Kotlin/JVM",
}
API_ENGINE_LABELS = {
    "rust": "Rust",
    "python": "Python",
    "go": "Go",
    "typescript": "TypeScript",
    "smalltalk": "Smalltalk",
    "kotlin": "Kotlin",
}
ENGINE_PATHS = {
    "rust": "rust",
    "python": "python",
    "go": "go",
    "typescript": "ts",
    "smalltalk": "smalltalk",
    "kotlin": "kotlin",
}
WRAPPER_PATHS = {
    "cpp": "cpp",
    "dotnet": "dotnet",
    "php": "php",
    "lua": "lua",
    "swift": "swift",
    "ruby": "ruby",
    "r": "r",
    "julia": "julia",
}
COUNT_WORDS = {
    0: "zero",
    1: "one",
    2: "two",
    3: "three",
    4: "four",
    5: "five",
    6: "six",
    7: "seven",
    8: "eight",
    9: "nine",
    10: "ten",
    11: "eleven",
    12: "twelve",
}

STALE_ROSTER_PATTERNS = (
    re.compile(r"\b(?:four|five) engines\b", re.IGNORECASE),
    re.compile(r"\b(?:four|five) interoperable(?: full)? engines\b", re.IGNORECASE),
    re.compile(r"\b(?:four|five) reference engines\b", re.IGNORECASE),
    re.compile(r"\bcontains (?:four|five) engines\b", re.IGNORECASE),
    re.compile(r"\ball (?:four|five) engines\b", re.IGNORECASE),
    re.compile(r"\ball (?:four|five) `gts` CLIs\b", re.IGNORECASE),
    re.compile(r"\b(?:four|five)-engine\b", re.IGNORECASE),
)
SPEC_VERSION_CLAIM_RE = re.compile(
    r"GTS-SPEC\.md[^\n]*(?:draft|version)\s+`([^`]+)`",
    re.IGNORECASE,
)


class DocRosterError(ValueError):
    """Raised when registry or documentation content cannot be checked."""


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT))


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise DocRosterError(f"cannot read {rel(path)}: {exc.strerror}") from exc
    except UnicodeDecodeError as exc:
        raise DocRosterError(f"{rel(path)} is not valid UTF-8: {exc}") from exc


def load_json(path: Path) -> Mapping[str, Any]:
    try:
        data = json.loads(read_text(path))
    except json.JSONDecodeError as exc:
        raise DocRosterError(f"{rel(path)} is not valid JSON: {exc}") from exc
    if not isinstance(data, Mapping):
        raise DocRosterError(f"{rel(path)} must be a JSON object")
    return data


def expect_string_list(value: Any, label: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        raise DocRosterError(f"{label} must be a list of strings")
    if len(value) != len(set(value)):
        raise DocRosterError(f"{label} must not contain duplicates")
    return value


def load_registry() -> Mapping[str, Any]:
    registry = load_json(REGISTRY)
    if registry.get("schema") != "gts-documentation-roster-v1":
        raise DocRosterError(
            "docs/documentation-roster.json schema must be gts-documentation-roster-v1"
        )
    expect_string_list(registry.get("independent_engines"), "independent_engines")
    expect_string_list(registry.get("derived_wrappers"), "derived_wrappers")
    version = registry.get("spec_document_version")
    if not isinstance(version, str) or not version:
        raise DocRosterError("spec_document_version must be a non-empty string")
    return registry


def count_word(count: int) -> str:
    try:
        return COUNT_WORDS[count]
    except KeyError as exc:
        raise DocRosterError(f"no count word configured for {count}") from exc


def labels_for(
    slugs: Sequence[str], labels: Mapping[str, str], label: str
) -> list[str]:
    unknown = sorted(set(slugs) - set(labels))
    if unknown:
        raise DocRosterError(f"unknown {label} slug(s): {', '.join(unknown)}")
    return [labels[slug] for slug in slugs]


def plain_list(labels: Sequence[str]) -> str:
    return ", ".join(labels)


def oxford_list(labels: Sequence[str]) -> str:
    if len(labels) < 2:
        return "".join(labels)
    return f"{', '.join(labels[:-1])}, and {labels[-1]}"


def normalized(text: str) -> str:
    return " ".join(text.split())


def require_phrase(
    errors: list[str], path: str, text: str, phrase: str, label: str
) -> None:
    if normalized(phrase) not in normalized(text):
        errors.append(f"{path}: missing {label}: {phrase!r}")


def current_changelog_intro(text: str) -> str:
    marker = "\n## ["
    if marker not in text:
        raise DocRosterError("CHANGELOG.md missing release heading marker: '## ['")
    return text.split(marker, 1)[0]


def check_no_stale_roster(errors: list[str], path: str, text: str) -> None:
    for pattern in STALE_ROSTER_PATTERNS:
        for match in pattern.finditer(text):
            errors.append(f"{path}: stale current roster phrase {match.group(0)!r}")


def normalize_spec_version(value: str) -> str:
    if value.startswith("v") and len(value) > 1 and value[1].isdigit():
        return value[1:]
    return value


def check_spec_version_claims(
    errors: list[str],
    path: str,
    text: str,
    expected_version: str,
) -> None:
    for match in SPEC_VERSION_CLAIM_RE.finditer(text):
        claimed = match.group(1)
        if normalize_spec_version(claimed) != expected_version:
            errors.append(
                f"{path}: stale current spec document version {claimed!r}; "
                f"expected {expected_version!r}"
            )


def check_paths(
    errors: list[str], slugs: Sequence[str], paths: Mapping[str, str], label: str
) -> None:
    unknown = sorted(set(slugs) - set(paths))
    if unknown:
        errors.append(
            f"docs/documentation-roster.json: unknown {label} slug(s): {', '.join(unknown)}"
        )
    for slug in slugs:
        path = paths.get(slug)
        if path is not None and not (ROOT / path).exists():
            errors.append(
                f"docs/documentation-roster.json: {label} {slug!r} path is missing: {path}"
            )


def check_spec(
    errors: list[str], version: str, engine_phrase: str, engine_count_word: str
) -> None:
    text = read_text(SPEC)
    require_phrase(
        errors,
        rel(SPEC),
        text,
        f"**Document version:** {version}",
        "document version header",
    )
    require_phrase(
        errors,
        rel(SPEC),
        text,
        f"| Document version | {version} |",
        "document version table",
    )
    require_phrase(
        errors,
        rel(SPEC),
        text,
        f"alongside {engine_count_word} interoperable reference engines ({engine_phrase})",
        "current independent-engine roster",
    )


def check_api_parity_json(errors: list[str], engines: Sequence[str]) -> None:
    path = ROOT / "docs" / "api-parity.json"
    data = load_json(path)
    expected = labels_for(engines, API_ENGINE_LABELS, "API engine")
    actual = data.get("full_engines")
    if not isinstance(actual, list) or not all(
        isinstance(engine, str) for engine in actual
    ):
        errors.append(f"{rel(path)}: full_engines must be a list of strings")
        return
    if set(actual) != set(expected):
        errors.append(
            f"{rel(path)}: full_engines {actual!r} does not match registry roster {expected!r}"
        )


def check_current_docs(registry: Mapping[str, Any]) -> list[str]:
    engines = expect_string_list(
        registry.get("independent_engines"), "independent_engines"
    )
    wrappers = expect_string_list(registry.get("derived_wrappers"), "derived_wrappers")
    version = str(registry["spec_document_version"])
    engine_count_word = count_word(len(engines))
    engine_labels = labels_for(engines, ENGINE_LABELS, "engine")
    engine_phrase = plain_list(engine_labels)
    engine_phrase_with_and = oxford_list(engine_labels)

    errors: list[str] = []
    check_paths(errors, engines, ENGINE_PATHS, "engine")
    check_paths(errors, wrappers, WRAPPER_PATHS, "wrapper")
    check_api_parity_json(errors, engines)
    check_spec(errors, version, engine_phrase, engine_count_word)

    docs_to_scan = [
        "CITATION.cff",
        "README.md",
        "go/README.md",
        "docs/GTS-PAPER-DRAFT.md",
    ]
    for doc in docs_to_scan:
        text = read_text(ROOT / doc)
        check_no_stale_roster(errors, doc, text)
        check_spec_version_claims(errors, doc, text, version)

    changelog_intro = current_changelog_intro(read_text(CHANGELOG))
    check_no_stale_roster(errors, rel(CHANGELOG), changelog_intro)
    check_spec_version_claims(errors, rel(CHANGELOG), changelog_intro, version)

    current_requirements = {
        "CITATION.cff": [
            f"provides {engine_count_word} interoperable engines ({engine_phrase_with_and})",
        ],
        "README.md": [
            f"**{engine_count_word} interoperable full engines** ({engine_phrase})",
        ],
        "go/README.md": [
            f"one of **{engine_count_word} interoperable full engines** ({engine_phrase})",
            f"all {engine_count_word} `gts` binaries",
        ],
        "docs/GTS-PAPER-DRAFT.md": [
            f"hosts {engine_count_word} reference engines in {engine_phrase_with_and}",
            f"The repository contains {engine_count_word} engines:",
            f"all {engine_count_word} engines are described as gating against the shared corpus",
        ],
        "CHANGELOG.md": [
            f"`GTS-SPEC.md` document version `{version}`",
        ],
    }
    for doc, phrases in current_requirements.items():
        text = changelog_intro if doc == "CHANGELOG.md" else read_text(ROOT / doc)
        for phrase in phrases:
            require_phrase(errors, doc, text, phrase, "registry-backed current claim")

    return errors


def run_self_test() -> int:
    errors: list[str] = []
    check_no_stale_roster(
        errors, "fixture.md", "This repository provides four interoperable engines."
    )
    if not errors:
        print(
            "check_doc_roster self-test: stale four-engine claim was not rejected",
            file=sys.stderr,
        )
        return 1

    errors = []
    check_spec_version_claims(
        errors,
        "CHANGELOG.md",
        "The wire format is a working draft (`GTS-SPEC.md` is at draft `v0.3`).",
        "0.9-draft",
    )
    if not errors:
        print(
            "check_doc_roster self-test: stale spec draft claim was not rejected",
            file=sys.stderr,
        )
        return 1

    historical = (
        "The wire format is a working draft "
        "(`GTS-SPEC.md` document version `0.9-draft`).\n\n"
        "## [0.9.1] - 2026-06-19\n\n"
        "- Historical release note about all four engines.\n"
    )
    errors = []
    intro = current_changelog_intro(historical)
    check_no_stale_roster(errors, "CHANGELOG.md", intro)
    check_spec_version_claims(errors, "CHANGELOG.md", intro, "0.9-draft")
    if errors:
        print(
            "check_doc_roster self-test: historical changelog text was not exempt",
            file=sys.stderr,
        )
        for error in errors:
            print(f"check_doc_roster self-test: {error}", file=sys.stderr)
        return 1

    try:
        current_changelog_intro(
            "# Changelog\n\nCurrent intro without release entries.\n"
        )
    except DocRosterError:
        pass
    else:
        print(
            "check_doc_roster self-test: missing changelog heading was not rejected",
            file=sys.stderr,
        )
        return 1

    print("check_doc_roster: self-test OK")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--self-test", action="store_true", help="run checker regression tests"
    )
    args = parser.parse_args()
    if args.self_test:
        return run_self_test()

    try:
        errors = check_current_docs(load_registry())
    except DocRosterError as exc:
        print(f"check_doc_roster: {exc}", file=sys.stderr)
        return 1

    if errors:
        for error in errors:
            print(f"check_doc_roster: {error}", file=sys.stderr)
        return 1
    print("check_doc_roster: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

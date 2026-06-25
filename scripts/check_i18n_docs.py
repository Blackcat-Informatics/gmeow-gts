#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Validate localized documentation structure and drift-sensitive literals."""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[1]
I18N_ROOT = ROOT / "docs" / "i18n"
LOCALES = ("fr-CA", "zh-Hans")
STATUSES = {"placeholder", "draft", "translated", "reviewed"}
ENFORCED_STATUSES = {"translated", "reviewed"}
METADATA_RE = re.compile(r"<!--\s*(i18n-[a-z-]+):\s*(.*?)\s*-->", re.IGNORECASE)
FENCE_RE = re.compile(r"^(```|~~~)")
INLINE_CODE_RE = re.compile(r"(?<!`)`([^`\n]+)`(?!`)")
URL_RE = re.compile(r"https?://[^\s)>\"']+")
MARKDOWN_TARGET_RE = re.compile(r"!?\[[^\]]*]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
HTML_ATTR_RE = re.compile(r"\b(?:href|src)=\"([^\"]+)\"")


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def fail(errors: list[str], message: str) -> None:
    errors.append(message)


def metadata(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for key, value in METADATA_RE.findall(text):
        values[key.lower()] = value.strip()
    return values


def fenced_blocks(text: str) -> list[str]:
    blocks: list[str] = []
    current: list[str] = []
    marker: str | None = None
    for line in text.splitlines(keepends=True):
        fence = FENCE_RE.match(line)
        if marker is None:
            if fence:
                marker = fence.group(1)
                current = [line]
            continue
        current.append(line)
        if fence and fence.group(1) == marker:
            blocks.append("".join(current))
            current = []
            marker = None
    if marker is not None:
        blocks.append("".join(current))
    return blocks


def has_unclosed_fence(text: str) -> bool:
    marker: str | None = None
    for line in text.splitlines():
        fence = FENCE_RE.match(line)
        if not fence:
            continue
        if marker is None:
            marker = fence.group(1)
        elif marker == fence.group(1):
            marker = None
    return marker is not None


def unique_in_order(values: Iterable[str]) -> list[str]:
    return list(dict.fromkeys(values))


def protected_literals(text: str) -> list[str]:
    tokens: list[str] = []
    tokens.extend(f"`{match}`" for match in INLINE_CODE_RE.findall(text))
    tokens.extend(URL_RE.findall(text))
    tokens.extend(MARKDOWN_TARGET_RE.findall(text))
    tokens.extend(HTML_ATTR_RE.findall(text))
    return unique_in_order(tokens)


def validate_source_path(source: str, errors: list[str], doc_path: Path) -> Path | None:
    if not source:
        fail(errors, f"{relative(doc_path)}: missing i18n-source metadata")
        return None
    source_path = Path(source)
    if source_path.is_absolute() or ".." in source_path.parts:
        fail(errors, f"{relative(doc_path)}: i18n-source must be repo-relative")
        return None
    resolved = ROOT / source_path
    if not resolved.is_file():
        fail(errors, f"{relative(doc_path)}: i18n-source does not exist: {source}")
        return None
    if resolved.is_relative_to(I18N_ROOT):
        fail(errors, f"{relative(doc_path)}: i18n-source cannot point inside docs/i18n")
        return None
    return resolved


def validate_enforced_file(
    source_path: Path,
    source_text: str,
    localized_path: Path,
    localized_text: str,
    errors: list[str],
) -> None:
    source_blocks = fenced_blocks(source_text)
    localized_blocks = fenced_blocks(localized_text)
    if len(source_blocks) != len(localized_blocks):
        fail(
            errors,
            f"{relative(localized_path)}: code fence count differs from "
            f"{relative(source_path)} ({len(localized_blocks)} != {len(source_blocks)})",
        )
    else:
        for index, (source_block, localized_block) in enumerate(
            zip(source_blocks, localized_blocks, strict=True),
            start=1,
        ):
            if source_block != localized_block:
                fail(
                    errors,
                    f"{relative(localized_path)}: code block {index} does not match "
                    f"{relative(source_path)} exactly",
                )

    missing_literals = [
        literal for literal in protected_literals(source_text) if literal not in localized_text
    ]
    if missing_literals:
        preview = ", ".join(missing_literals[:8])
        if len(missing_literals) > 8:
            preview += ", ..."
        fail(errors, f"{relative(localized_path)}: missing protected literals: {preview}")


def main() -> int:
    errors: list[str] = []

    for required in (I18N_ROOT / "README.md", I18N_ROOT / "GLOSSARY.md"):
        if not required.is_file():
            fail(errors, f"missing required localization file: {relative(required)}")

    by_source: dict[str, dict[str, Path]] = defaultdict(dict)

    for locale in LOCALES:
        locale_dir = I18N_ROOT / locale
        if not locale_dir.is_dir():
            fail(errors, f"missing locale directory: {relative(locale_dir)}")
            continue

        for doc_path in sorted(locale_dir.rglob("*.md")):
            text = doc_path.read_text(encoding="utf-8")
            meta = metadata(text)
            source = meta.get("i18n-source", "")
            declared_locale = meta.get("i18n-locale", "")
            status = meta.get("i18n-status", "")

            if declared_locale != locale:
                fail(errors, f"{relative(doc_path)}: i18n-locale must be {locale}")
            if status not in STATUSES:
                fail(
                    errors,
                    f"{relative(doc_path)}: i18n-status must be one of "
                    f"{', '.join(sorted(STATUSES))}",
                )
            if has_unclosed_fence(text):
                fail(errors, f"{relative(doc_path)}: unclosed Markdown code fence")

            source_path = validate_source_path(source, errors, doc_path)
            if source_path is None:
                continue

            normalized_source = relative(source_path)
            if locale in by_source[normalized_source]:
                fail(
                    errors,
                    f"{relative(doc_path)}: duplicate source for locale: "
                    f"{normalized_source}",
                )
            by_source[normalized_source][locale] = doc_path

            if status in ENFORCED_STATUSES:
                source_text = source_path.read_text(encoding="utf-8")
                validate_enforced_file(source_path, source_text, doc_path, text, errors)

    for source, locale_paths in sorted(by_source.items()):
        missing = sorted(set(LOCALES) - set(locale_paths))
        if missing:
            fail(errors, f"{source}: missing localized coverage for {', '.join(missing)}")

    if errors:
        for error in errors:
            print(f"check_i18n_docs: {error}", file=sys.stderr)
        return 1

    print("check_i18n_docs: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

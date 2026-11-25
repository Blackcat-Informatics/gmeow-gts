#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Fail when a tracked Markdown file links to a repository path that does not exist.

Checks inline links and images (`[text](target)`, `![alt](target)`) in every
tracked Markdown file outside `archive/`, the frozen v1 snapshot, whose links
name the layout of its time. External URLs, `mailto:` and same-document
fragments (`#...`) are not checked here; markdownlint checks fragments. Code
blocks and code spans are ignored. Exits 1 and lists every broken link.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]

# Inline link or image: the target is the first token inside the parentheses,
# optionally in <angle brackets>, followed by an optional "title".
LINK = re.compile(r"!?\[[^\]]*\]\(\s*(<[^>]*>|[^\s)]+)(?:\s+\"[^\"]*\")?\s*\)")
CODE_SPAN = re.compile(r"`+[^`]*`+")
FENCE = re.compile(r"^\s*(```|~~~)")
SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")


def tracked_markdown() -> list[Path]:
    """Return every Git-tracked Markdown file outside the frozen archive."""
    listing = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files", "-z", "--", "*.md", ":!archive/"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [ROOT / name for name in listing.split("\0") if name]


def link_targets(text: str) -> list[tuple[int, str]]:
    """Return (line number, target) for each inline link outside code."""
    found: list[tuple[int, str]] = []
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        stripped = CODE_SPAN.sub("", line)
        for match in LINK.finditer(stripped):
            found.append((number, match.group(1).strip("<>")))
    return found


def broken_links(document: Path) -> list[str]:
    """Return a message for each relative link in `document` that does not resolve."""
    problems: list[str] = []
    relative = document.relative_to(ROOT)
    for number, target in link_targets(document.read_text(encoding="utf-8")):
        if not target or target.startswith("#") or SCHEME.match(target):
            continue
        path_part = unquote(target.split("#", 1)[0].split("?", 1)[0])
        if not path_part:
            continue
        base = ROOT if path_part.startswith("/") else document.parent
        resolved = base / path_part.lstrip("/")
        if not resolved.exists():
            problems.append(f"{relative}:{number}: broken link to `{target}`")
    return problems


def main() -> int:
    """Check every tracked Markdown file and report broken relative links."""
    problems: list[str] = []
    for document in tracked_markdown():
        problems.extend(broken_links(document))
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        print(f"check_doc_links: {len(problems)} broken relative link(s)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

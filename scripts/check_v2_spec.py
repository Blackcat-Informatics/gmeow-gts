#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Mechanical checks of the generation-2 draft specification under docs/v2/.

Each check reads the seventeen documents as text and reports places where the text disagrees with itself or with the
bar of the draft: a requirement number defined twice, a citation that does not resolve, a finding and reason pair that
the registry does not allow, a key word of BCP 14 used outside a requirement, and wording that a specification does
not hold. The script reads no implementation and no corpus.

Usage: check_v2_spec.py [--docs DIR] [--only NAME ...] [--summary]
Exit status 0 when no check reports anything, 1 otherwise.
"""

from __future__ import annotations

import argparse
import bisect
import re
import sys
from collections import Counter, defaultdict
from collections.abc import Callable, Iterator
from dataclasses import dataclass, field
from pathlib import Path

PARTS = ("P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9")
PROFILES = ("PF-bundle", "PF-container", "PF-dist", "PF-evidence", "PF-files", "PF-stream")
DOCUMENTS = (*PARTS, *PROFILES, "A1", "L1")
MAX_LINE = 130

REQ_ID = r"(?:P[0-9]|PF-[a-z]+|L1)-[0-9]+[a-z]?"
REQ_DEF = re.compile(r"\*\*(" + REQ_ID + r")\.\*\*")
REQ_DEF_ITEM = re.compile(r"^\s*[0-9]+\.\s+`(" + REQ_ID + r")`\s")
REQ_CITE = re.compile(r"(?<![A-Za-z0-9-])(" + REQ_ID + r")(?![0-9A-Za-z])")
HEADING = re.compile(r"^(#+)\s+(.*)$")
SECTION_NUMBER = re.compile(r"^([0-9]+[a-z]?(?:\.[0-9]+[a-z]?)*)\.?\s+(.*)$")
NUMBERED_PARAGRAPH = re.compile(r"^([0-9]{1,2}\.[0-9]{1,2}[a-z]?)\.? [A-Z`]")
EXTERNAL = re.compile(r"RFC|FIPS|NIST|draft-|Internet-Draft|first generation|first-generation|of \[|\bet al\b")
DOC_NAME = r"P[0-9]|L1|A1|PF-[a-z]+"
TITLE_CITE = re.compile(r"`(" + DOC_NAME + r")`,?\s+(?:\([^)]{1,60}\),?\s+)?\"([^\"]{3,140})\"")
TITLE_CHAIN = re.compile(r"((?:(?:,| and| or|, and|, or)\s+\"[^\"]{3,140}\")+)")
SECTION_CITE = re.compile(r"(?<![`\w.])(?:sections?|subsections?) ([0-9]{1,2}[a-z]?(?:\.[0-9]{1,2}[a-z]?)*)(?![\w])")
PAREN_SECTION_CITE = re.compile(r"\(([0-9]{1,2}[a-z]?\.[0-9]{1,2}[a-z]?(?:\.[0-9]{1,2})?)(?=[,;)])")
PAIR = re.compile(r"`([a-z][a-z0-9_]+)`\s+with\s+(?:the\s+)?reasons?\s+`([a-z][a-z0-9_]+)`")
KEY_WORDS_UPPER = re.compile(
    r"\b(MUST NOT|MUST|SHALL NOT|SHALL|SHOULD NOT|SHOULD|REQUIRED|NOT RECOMMENDED|RECOMMENDED|MAY|OPTIONAL)\b"
)
KEY_WORDS_LOWER = re.compile(r"\b(must|shall|should|required|recommended|may|optional)\b")
LEVELS_IN_USE = {"MUST", "MUST NOT", "MAY"}
BOILERPLATE = (
    'The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", '
    '"NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14 '
    "[RFC2119] [RFC8174] when, and only when, they appear in all capitals, as shown here."
)
EXCLUDED_WORDING = re.compile(
    r"\bOPEN\b|\bopen question\b|(?i:questions?) for the reviewer|\bthe reviewer\b|\bcryptographer\b|"
    r"\bprototype\b|\breference engine\b|\bengines?\b|\bthis repository\b|\bthe repository\b|\bgovernance document\b|"
    r"\b(?:earlier|previous|this|no) revision\b|\bearlier draft\b|\bwas read\b|\bnot verified\b|\bNot verified\b|"
    r"\bNOT MEASURED\b|\bpreviously\b|\bformerly\b|\bno longer\b|\bTODO\b|\bTBD\b|\bFIXME\b|@@|"
    r"\bthe vectors give\b|\bgiven by the vectors?\b|\btestable by a vector\b|\bexperiment [0-9]+\b"
)


@dataclass(frozen=True)
class Finding:
    """One place the text fails a check."""

    document: str
    line: int
    message: str


@dataclass
class Document:
    """One document: its lines, and for each line whether it is prose, table, fence or heading."""

    name: str
    lines: list[str]
    kind: list[str] = field(default_factory=list)
    requirement: list[str | None] = field(default_factory=list)
    section: list[str] = field(default_factory=list)

    def prose(self) -> Iterator[tuple[int, str]]:
        """Yield the number and text of every line that is not inside a code fence."""
        for index, line in enumerate(self.lines):
            if self.kind[index] != "fence":
                yield index + 1, line


def load(docs: Path) -> dict[str, Document]:
    """Read the seventeen documents and classify their lines."""
    out: dict[str, Document] = {}
    for name in DOCUMENTS:
        path = docs / f"{name}.md"
        if not path.is_file():
            raise SystemExit(f"missing document: {path}")
        doc = Document(name, path.read_text(encoding="utf-8").split("\n"))
        in_fence = False
        in_language = False
        current: str | None = None
        heading = ""
        for line in doc.lines:
            stripped = line.strip()
            if stripped.startswith("```"):
                in_fence = not in_fence
                kind = "fence"
            elif in_fence:
                kind = "fence"
            elif HEADING.match(line):
                kind = "heading"
                heading = HEADING.match(line).group(2).strip()  # type: ignore[union-attr]
            elif stripped.startswith("|"):
                kind = "table"
            elif stripped.startswith("<!--"):
                kind = "comment"
            elif stripped.startswith("REQUIREMENT LANGUAGE.") or (in_language and stripped):
                kind = "language"
                in_language = True
            else:
                kind = "prose"
            if not stripped:
                in_language = False
            if not stripped or kind == "heading":
                current = None
            else:
                match = REQ_DEF.search(line) or REQ_DEF_ITEM.match(line)
                if match and kind == "prose":
                    current = match.group(1)
            doc.kind.append(kind)
            doc.requirement.append(current if kind == "prose" else None)
            doc.section.append(heading)
        out[name] = doc
    return out


def normal(title: str) -> str:
    """Reduce a heading or a cited title to the form in which the two are compared."""
    return re.sub(r"\s+", " ", title.replace("`", "")).strip().lower()


def headings(doc: Document) -> tuple[set[str], set[str]]:
    """Return the titles and the section numbers that a document defines."""
    titles: set[str] = set()
    numbers: set[str] = set()
    for index, line in enumerate(doc.lines):
        paragraph = NUMBERED_PARAGRAPH.match(line) if doc.kind[index] == "prose" else None
        if paragraph:
            numbers.add(paragraph.group(1))
        if doc.kind[index] != "heading":
            continue
        text = HEADING.match(line).group(2).strip()  # type: ignore[union-attr]
        titles.add(normal(text))
        numbered = SECTION_NUMBER.match(text)
        if numbered:
            numbers.add(numbered.group(1))
            titles.add(normal(numbered.group(2)))
        named = re.match(r"^(?:Part |Profile )?[A-Za-z0-9-]+:\s+(.*)$", text)
        if named:
            titles.add(normal(named.group(1)))
    return titles, numbers


def check_requirement_numbers(docs: dict[str, Document]) -> Iterator[Finding]:
    """Every numbered requirement is defined once, and every citation of a number resolves."""
    defined: dict[str, list[tuple[str, int]]] = defaultdict(list)
    for doc in docs.values():
        for number, line in doc.prose():
            for match in REQ_DEF.finditer(line):
                defined[match.group(1)].append((doc.name, number))
            item = REQ_DEF_ITEM.match(line)
            if item:
                defined[item.group(1)].append((doc.name, number))
    for req, places in sorted(defined.items()):
        for name, number in places[1:]:
            yield Finding(name, number, f"{req} is defined more than once (first at {places[0][0]}:{places[0][1]})")
    for doc in docs.values():
        for number, line in doc.prose():
            for match in REQ_CITE.finditer(line):
                if match.group(1) not in defined:
                    yield Finding(doc.name, number, f"citation of {match.group(1)}, which no document defines")


def check_cited_titles(docs: dict[str, Document]) -> Iterator[Finding]:
    """Every citation of a document by a quoted section title names a heading of that document."""
    titles = {name: headings(doc)[0] for name, doc in docs.items()}
    for doc in docs.values():
        text = "\n".join(doc.lines)
        starts = [0]
        for line in doc.lines:
            starts.append(starts[-1] + len(line) + 1)
        for match in TITLE_CITE.finditer(text):
            target = match.group(1)
            cited = [match.group(2)]
            chain = TITLE_CHAIN.match(text, match.end())
            if chain:
                cited.extend(re.findall(r"\"([^\"]{3,140})\"", chain.group(1)))
            for title in cited:
                if target in titles and normal(title) not in titles[target]:
                    number = bisect.bisect_right(starts, match.start())
                    yield Finding(doc.name, number, f'`{target}` has no heading "{normal(title)}"')


def check_section_numbers(docs: dict[str, Document]) -> Iterator[Finding]:
    """Every section number cited inside a document is a section of that document."""
    numbers = {name: headings(doc)[1] for name, doc in docs.items()}
    for doc in docs.values():
        for number, line in doc.prose():
            if doc.kind[number - 1] == "heading" or EXTERNAL.search(line):
                continue
            cited = [m.group(1) for m in SECTION_CITE.finditer(line)]
            if doc.name != "A1":
                cited += [m.group(1) for m in PAREN_SECTION_CITE.finditer(line)]
            valid = set(numbers[doc.name])
            for other in re.findall(r"`(" + DOC_NAME + r")`", line):
                valid |= numbers.get(other, set())
            for section in cited:
                if section not in valid:
                    yield Finding(doc.name, number, f"section {section} is cited and is not a section of this document")


def registry(docs: dict[str, Document]) -> tuple[set[str], set[str], set[tuple[str, str]]]:
    """Read the finding codes, the reasons and the allowed pairs from the tables of `P2` and, for version 2, `P8`."""
    findings: set[str] = set()
    reasons: set[str] = set()
    pairs: set[tuple[str, str]] = set()
    for name in ("P2", "P8"):
        read_registry_tables(docs[name], findings, reasons, pairs)
    return findings, reasons, pairs


def read_registry_tables(doc: Document, findings: set[str], reasons: set[str], pairs: set[tuple[str, str]]) -> None:
    """Add the rows of the finding tables and reason tables of one document."""
    header: list[str] = []
    for index, line in enumerate(doc.lines):
        if doc.kind[index] != "table":
            header = []
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if not header:
            header = cells
            continue
        if set(cells[0]) <= set("-: "):
            continue
        label = re.fullmatch(r"`([a-z][a-z0-9_]+)`", cells[1]) if len(cells) > 1 else None
        if not label:
            continue
        if header[:3] == ["code", "label", "severity"]:
            findings.add(label.group(1))
        elif header[:3] == ["code", "label", "qualifies the finding codes"]:
            reasons.add(label.group(1))
            for code in re.findall(r"`([a-z][a-z0-9_]+)`", cells[2]):
                pairs.add((code, label.group(1)))


def check_registry_pairs(docs: dict[str, Document]) -> Iterator[Finding]:
    """Every finding named with a reason is a pair that the tables of the registry allow."""
    findings, reasons, pairs = registry(docs)
    if not findings or not reasons:
        yield Finding("P2", 1, "the finding table or the reason table was not found")
        return
    for doc in docs.values():
        text = "\n".join(doc.lines)
        starts = [0]
        for line in doc.lines:
            starts.append(starts[-1] + len(line) + 1)
        for match in PAIR.finditer(text):
            code, reason = match.group(1), match.group(2)
            number = bisect.bisect_right(starts, match.start())
            if code not in findings:
                continue
            if reason not in reasons:
                yield Finding(doc.name, number, f"`{reason}` is used as a reason and is not in the reason table")
            elif (code, reason) not in pairs:
                yield Finding(doc.name, number, f"the registry does not allow `{code}` with the reason `{reason}`")


def check_key_words(docs: dict[str, Document]) -> Iterator[Finding]:
    """The key words of BCP 14 stand in capitals, inside numbered requirements, at the levels the draft uses."""
    for doc in docs.values():
        text = " ".join(line.strip() for line in doc.lines)
        states_requirements = any(doc.requirement)
        if states_requirements and BOILERPLATE not in re.sub(r"\s+", " ", text):
            yield Finding(doc.name, 1, "the RFC 8174 paragraph is absent or not verbatim")
        for number, line in doc.prose():
            kind = doc.kind[number - 1]
            if kind in ("heading", "comment", "language"):
                continue
            for match in KEY_WORDS_UPPER.finditer(line):
                word = match.group(1)
                if word not in LEVELS_IN_USE:
                    yield Finding(doc.name, number, f"key word {word} is not one of the levels in use")
                elif doc.requirement[number - 1] is None and kind == "prose":
                    yield Finding(doc.name, number, f"key word {word} outside a numbered requirement")
            for match in KEY_WORDS_LOWER.finditer(re.sub(r"`[^`]*`", "", line)):
                yield Finding(doc.name, number, f"lower-case key word '{match.group(1)}'")


def check_excluded_wording(docs: dict[str, Document]) -> Iterator[Finding]:
    """The text holds no question, status, history or reference to things its reader does not hold."""
    for doc in docs.values():
        for number, line in doc.prose():
            for match in EXCLUDED_WORDING.finditer(line):
                yield Finding(doc.name, number, f"excluded wording: {match.group(0)!r}")
            if doc.kind[number - 1] == "prose" and re.search(r"\?(?:\s|$)", re.sub(r"`[^`]*`", "", line)):
                yield Finding(doc.name, number, "a question")


def check_line_length(docs: dict[str, Document]) -> Iterator[Finding]:
    """A line of prose is at most 130 characters."""
    for doc in docs.values():
        for index, line in enumerate(doc.lines):
            if doc.kind[index] == "prose" and len(line) > MAX_LINE:
                yield Finding(doc.name, index + 1, f"line of {len(line)} characters")


CHECKS: dict[str, Callable[[dict[str, Document]], Iterator[Finding]]] = {
    "requirement-numbers": check_requirement_numbers,
    "cited-titles": check_cited_titles,
    "section-numbers": check_section_numbers,
    "registry-pairs": check_registry_pairs,
    "key-words": check_key_words,
    "excluded-wording": check_excluded_wording,
    "line-length": check_line_length,
}


def main() -> int:
    """Run the selected checks and print what they report."""
    parser = argparse.ArgumentParser(description="Mechanical checks of the generation-2 draft specification.")
    parser.add_argument("--docs", type=Path, default=Path(__file__).resolve().parent.parent / "docs" / "v2")
    parser.add_argument("--only", nargs="+", choices=sorted(CHECKS), help="run these checks only")
    parser.add_argument("--summary", action="store_true", help="print counts and no places")
    args = parser.parse_args()
    docs = load(args.docs)
    total = 0
    for name in args.only or CHECKS:
        found = sorted(set(CHECKS[name](docs)), key=lambda f: (DOCUMENTS.index(f.document), f.line, f.message))
        total += len(found)
        per_document = Counter(f.document for f in found)
        counts = ", ".join(f"{doc} {per_document[doc]}" for doc in DOCUMENTS if per_document[doc])
        print(f"== {name}: {len(found)}" + (f" ({counts})" if counts else ""))
        if not args.summary:
            for finding in found:
                print(f"  {finding.document}:{finding.line}: {finding.message}")
    print(f"total: {total}")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Check production-code quality budgets against a checked-in baseline."""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = ROOT / "quality" / "quality-budget-baseline.json"
REVIEW_BASE_ENV = "GTS_QUALITY_BUDGET_REVIEW_BASE"
BASELINE_INCREASE_APPROVED_ENV = "GTS_QUALITY_BUDGET_BASELINE_INCREASE_APPROVED"
BASELINE_INCREASE_REVIEW_LABEL = "quality-budget-baseline-increase"
BASELINE_REVIEW_NOTE_KEY = "baseline_increase_review"

INCLUDE_ROOTS = (
    "cpp/include",
    "dotnet/Gmeow.Gts",
    "go",
    "julia/src",
    "kotlin/src/main",
    "lua/gmeow",
    "php/src",
    "python/src",
    "r/R",
    "r/src",
    "ruby/lib",
    "rust/capi/include",
    "rust/capi/src",
    "rust/src",
    "smalltalk/src",
    "swift/Sources",
    "ts/src",
)

CODE_EXTENSIONS = {
    ".c",
    ".cc",
    ".cpp",
    ".cs",
    ".go",
    ".h",
    ".hpp",
    ".jl",
    ".kt",
    ".lua",
    ".php",
    ".py",
    ".r",
    ".rb",
    ".rs",
    ".st",
    ".swift",
    ".ts",
}

EXCLUDED_PARTS = {
    ".git",
    ".gradle",
    ".mypy_cache",
    ".pytest_cache",
    ".tox",
    ".worktrees",
    "__pycache__",
    "build",
    "dist",
    "examples",
    "fixtures",
    "fuzz",
    "node_modules",
    "target",
    "test",
    "tests",
    "third_party",
    "vendor",
    "vendors",
}

EXCLUDED_SUFFIXES = (
    "_test.go",
    ".test.ts",
    ".spec.ts",
    "Test.kt",
    "_test.py",
    "_tests.py",
)

HOTSPOT_FILES = (
    "rust/src/reader.rs",
    "rust/src/bin/gts.rs",
    "go/reader/reader.go",
    "go/cmd/gts/main.go",
    "python/src/gts/cli.py",
    "ts/src/browser.ts",
    "ts/src/reader.ts",
)


@dataclass(frozen=True)
class PatternSpec:
    extensions: frozenset[str] | None
    regex: re.Pattern[str]
    label: str


@dataclass(frozen=True)
class Occurrence:
    metric: str
    path: str
    line_number: int
    label: str
    text: str


@dataclass(frozen=True)
class BaselineIncrease:
    scope: str
    key: str
    old: int
    new: int


METRICS: dict[str, tuple[PatternSpec, ...]] = {
    "unchecked_panic_calls": (
        PatternSpec(
            frozenset({".rs"}),
            re.compile(r"\.(?:unwrap|expect)\s*\("),
            "rust unwrap/expect",
        ),
        PatternSpec(
            frozenset({".rs"}),
            re.compile(r"\b(?:panic|todo|unimplemented)!\s*\("),
            "rust panic/todo macro",
        ),
        PatternSpec(frozenset({".go"}), re.compile(r"\bpanic\s*\("), "go panic"),
        PatternSpec(
            frozenset({".swift"}),
            re.compile(r"\b(?:fatalError|preconditionFailure)\s*\("),
            "swift fatal/precondition failure",
        ),
        PatternSpec(
            frozenset({".c", ".cc", ".cpp", ".h", ".hpp"}),
            re.compile(r"\b(?:abort|assert)\s*\("),
            "c-family abort/assert",
        ),
        PatternSpec(
            frozenset({".kt"}),
            re.compile(r"\b(?:TODO|error)\s*\("),
            "kotlin unchecked failure",
        ),
        PatternSpec(
            frozenset({".kt"}),
            re.compile(
                r"\bthrow\s+(?:RuntimeException|IllegalStateException|NotImplementedError)\s*\("
            ),
            "kotlin unchecked exception",
        ),
        PatternSpec(
            frozenset({".py"}),
            re.compile(r"\braise\s+NotImplementedError\s*\("),
            "python not implemented",
        ),
        PatternSpec(
            frozenset({".cs"}),
            re.compile(
                r"\bthrow\s+new\s+(?:NotImplementedException|InvalidOperationException)\s*\("
            ),
            "dotnet unchecked exception",
        ),
        PatternSpec(
            frozenset({".php"}),
            re.compile(r"\bthrow\s+new\s+\\?RuntimeException\s*\("),
            "php runtime exception",
        ),
        PatternSpec(
            frozenset({".rb"}),
            re.compile(r"\braise\s+NotImplementedError\b"),
            "ruby not implemented",
        ),
        PatternSpec(
            frozenset({".jl", ".lua"}),
            re.compile(r"\berror\s*\("),
            "dynamic-language error call",
        ),
        PatternSpec(
            frozenset({".st"}),
            re.compile(r"\bself\s+error:"),
            "smalltalk error send",
        ),
    ),
    "generic_parser_throws": (
        PatternSpec(
            frozenset({".ts"}),
            re.compile(r"\bthrow\s+new\s+Error\s*\("),
            "typescript generic Error",
        ),
        PatternSpec(
            frozenset({".py"}),
            re.compile(r"\braise\s+(?:Exception|RuntimeError)\s*\("),
            "python generic exception",
        ),
        PatternSpec(
            frozenset({".php"}),
            re.compile(r"\bthrow\s+new\s+\\?Exception\s*\("),
            "php generic exception",
        ),
        PatternSpec(
            frozenset({".cs"}),
            re.compile(r"\bthrow\s+new\s+Exception\s*\("),
            "dotnet generic exception",
        ),
        PatternSpec(
            frozenset({".rb"}),
            re.compile(r"\braise\s+(?:(?:RuntimeError|StandardError)\b|[\"'])"),
            "ruby generic exception",
        ),
    ),
    "maintenance_markers": (
        PatternSpec(
            None,
            re.compile(r"\b(?:TODO|FIXME|HACK)\b", flags=re.IGNORECASE),
            "TODO/FIXME/HACK marker",
        ),
    ),
}


def posix(path: Path) -> str:
    return path.as_posix()


def is_truthy(value: str | None) -> bool:
    return value is not None and value.strip().lower() in {"1", "true", "yes", "on"}


def is_excluded(path: Path) -> bool:
    parts = set(path.parts)
    if parts & EXCLUDED_PARTS:
        return True
    return path.name.endswith(EXCLUDED_SUFFIXES)


def is_code_file(path: Path) -> bool:
    return path.suffix.lower() in CODE_EXTENSIONS


def production_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for include_root in INCLUDE_ROOTS:
        include_path = root / include_root
        if not include_path.is_dir():
            continue

        for dirpath, dirnames, filenames in os.walk(include_path):
            dir_rel = Path(dirpath).relative_to(root)
            dirnames[:] = [
                dirname
                for dirname in dirnames
                if not is_excluded(dir_rel / dirname)
            ]
            for filename in filenames:
                path = Path(dirpath) / filename
                rel = path.relative_to(root)
                if is_excluded(rel):
                    continue
                if not is_code_file(path):
                    continue
                files.append(path)
    return sorted(files, key=lambda candidate: posix(candidate.relative_to(root)))


def read_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8", errors="replace").splitlines()


def matches_for_line(
    metric: str, suffix: str, line: str
) -> list[tuple[str, re.Match[str]]]:
    matches: list[tuple[str, re.Match[str]]] = []
    for spec in METRICS[metric]:
        if spec.extensions is not None and suffix not in spec.extensions:
            continue
        for match in spec.regex.finditer(line):
            matches.append((spec.label, match))
    return matches


def scan(root: Path) -> dict[str, Any]:
    files = production_files(root)
    line_counts: dict[str, int] = {}
    occurrences: dict[str, list[Occurrence]] = {metric: [] for metric in METRICS}

    for path in files:
        rel = posix(path.relative_to(root))
        lines = read_lines(path)
        line_counts[rel] = len(lines)
        suffix = path.suffix.lower()
        for line_number, line in enumerate(lines, start=1):
            stripped = line.strip()
            for metric in METRICS:
                for label, _match in matches_for_line(metric, suffix, line):
                    occurrences[metric].append(
                        Occurrence(
                            metric=metric,
                            path=rel,
                            line_number=line_number,
                            label=label,
                            text=stripped,
                        )
                    )

    return {"line_counts": line_counts, "occurrences": occurrences}


def count_by_file(occurrences: list[Occurrence]) -> dict[str, int]:
    counts: dict[str, int] = {}
    for occurrence in occurrences:
        counts[occurrence.path] = counts.get(occurrence.path, 0) + 1
    return dict(sorted(counts.items()))


def ratchet_target(lines: int) -> int:
    if lines <= 80:
        return lines
    return max(80, math.floor(lines * 0.9))


def build_baseline(root: Path) -> dict[str, Any]:
    snapshot = scan(root)
    line_counts: dict[str, int] = snapshot["line_counts"]
    occurrences: dict[str, list[Occurrence]] = snapshot["occurrences"]
    line_budgets = {
        path: {
            "max_lines": line_counts[path],
            "target_lines": ratchet_target(line_counts[path]),
        }
        for path in HOTSPOT_FILES
        if path in line_counts
    }
    metrics: dict[str, dict[str, Any]] = {}
    for metric, metric_occurrences in occurrences.items():
        metrics[metric] = {
            "max_total": len(metric_occurrences),
            "by_file": count_by_file(metric_occurrences),
        }
    return {
        "version": 1,
        "production_file_count": len(line_counts),
        "line_budgets": dict(sorted(line_budgets.items())),
        "metrics": metrics,
    }


def grouped_occurrences(
    occurrences: list[Occurrence],
) -> dict[str, list[Occurrence]]:
    grouped: dict[str, list[Occurrence]] = {}
    for occurrence in occurrences:
        grouped.setdefault(occurrence.path, []).append(occurrence)
    return dict(sorted(grouped.items()))


def load_baseline(path: Path) -> dict[str, Any]:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise SystemExit(f"check_quality_budget: missing baseline {path}") from None
    except json.JSONDecodeError as error:
        raise SystemExit(
            f"check_quality_budget: malformed baseline {path}: {error}"
        ) from None


def empty_baseline() -> dict[str, Any]:
    return {"line_budgets": {}, "metrics": {}}


def load_baseline_from_git(root: Path, ref: str, baseline_path: Path) -> dict[str, Any]:
    try:
        rel = posix(baseline_path.resolve().relative_to(root.resolve()))
    except ValueError:
        raise SystemExit(
            "check_quality_budget: --review-base requires the baseline path "
            "to be inside --root"
        ) from None

    result = subprocess.run(
        ["git", "-C", str(root), "show", f"{ref}:{rel}"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        missing_path_markers = (
            "does not exist in",
            "exists on disk, but not in",
            "exists on disk but not in",
        )
        if any(marker in detail for marker in missing_path_markers):
            return empty_baseline()
        raise SystemExit(
            f"check_quality_budget: unable to read {rel} from {ref}: {detail}"
        ) from None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise SystemExit(
            f"check_quality_budget: malformed baseline {rel} at {ref}: {error}"
        ) from None


def append_increase(
    increases: list[BaselineIncrease],
    scope: str,
    key: str,
    old_value: Any,
    new_value: Any,
) -> None:
    old = int(old_value or 0)
    new = int(new_value or 0)
    if new > old:
        increases.append(BaselineIncrease(scope=scope, key=key, old=old, new=new))


def find_baseline_increases(
    current: dict[str, Any], previous: dict[str, Any]
) -> list[BaselineIncrease]:
    increases: list[BaselineIncrease] = []

    previous_line_budgets = previous.get("line_budgets", {})
    for path, current_budget in sorted(current.get("line_budgets", {}).items()):
        previous_budget = previous_line_budgets.get(path, {})
        append_increase(
            increases,
            "line_budgets",
            f"{path}.max_lines",
            previous_budget.get("max_lines"),
            current_budget.get("max_lines"),
        )
        append_increase(
            increases,
            "line_budgets",
            f"{path}.target_lines",
            previous_budget.get("target_lines"),
            current_budget.get("target_lines"),
        )

    previous_metrics = previous.get("metrics", {})
    for metric, current_metric in sorted(current.get("metrics", {}).items()):
        previous_metric = previous_metrics.get(metric, {})
        append_increase(
            increases,
            "metrics",
            f"{metric}.max_total",
            previous_metric.get("max_total"),
            current_metric.get("max_total"),
        )

        previous_by_file = previous_metric.get("by_file", {})
        for path, current_count in sorted(current_metric.get("by_file", {}).items()):
            append_increase(
                increases,
                "metrics",
                f"{metric}.by_file.{path}",
                previous_by_file.get(path),
                current_count,
            )

    return increases


def architecture_review_note(baseline: dict[str, Any]) -> dict[str, str] | None:
    note = baseline.get(BASELINE_REVIEW_NOTE_KEY)
    if not isinstance(note, dict):
        return None
    reviewed_by = note.get("reviewed_by")
    reason = note.get("reason")
    if not isinstance(reviewed_by, str) or not reviewed_by.strip():
        return None
    if not isinstance(reason, str) or not reason.strip():
        return None
    return {"reviewed_by": reviewed_by.strip(), "reason": reason.strip()}


def has_updated_architecture_review_note(
    current: dict[str, Any], previous: dict[str, Any]
) -> bool:
    current_note = architecture_review_note(current)
    previous_note = architecture_review_note(previous)
    return current_note is not None and current_note != previous_note


def baseline_increase_errors(
    current: dict[str, Any],
    previous: dict[str, Any],
    *,
    allow_baseline_increase: bool,
) -> list[str]:
    increases = find_baseline_increases(current, previous)
    if not increases:
        return []
    if allow_baseline_increase or has_updated_architecture_review_note(
        current, previous
    ):
        return []

    errors = [
        "check_quality_budget: baseline increase requires explicit review: "
        f"add the `{BASELINE_INCREASE_REVIEW_LABEL}` PR label, set "
        f"`{BASELINE_INCREASE_APPROVED_ENV}=1`, or update "
        f"`{BASELINE_REVIEW_NOTE_KEY}` with `reviewed_by` and `reason`."
    ]
    for increase in increases[:20]:
        errors.append(
            f"  {increase.scope}: {increase.key} increased "
            f"from {increase.old} to {increase.new}"
        )
    if len(increases) > 20:
        errors.append(f"  ... {len(increases) - 20} more baseline increase(s)")
    return errors


def ratchet_opportunities(
    snapshot: dict[str, Any], baseline: dict[str, Any]
) -> list[tuple[int, str, int, int]]:
    line_counts: dict[str, int] = snapshot["line_counts"]
    opportunities: list[tuple[int, str, int, int]] = []
    for path, budget in sorted(baseline.get("line_budgets", {}).items()):
        current_lines = line_counts.get(path)
        if current_lines is None:
            continue
        target_lines = int(budget.get("target_lines", current_lines))
        if current_lines > target_lines:
            opportunities.append(
                (current_lines - target_lines, path, current_lines, target_lines)
            )
    return sorted(opportunities)


def ratchet_summary(snapshot: dict[str, Any], baseline: dict[str, Any]) -> str:
    opportunities = ratchet_opportunities(snapshot, baseline)
    if not opportunities:
        return "check_quality_budget: all tracked hotspots are at or below target_lines"

    delta, path, current_lines, target_lines = opportunities[0]
    return (
        f"check_quality_budget: {len(opportunities)} over-target hotspot(s); "
        f"closest ratchet is {path} ({current_lines} lines, "
        f"target {target_lines}, reduce by {delta})"
    )


def compare(snapshot: dict[str, Any], baseline: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    line_counts: dict[str, int] = snapshot["line_counts"]
    occurrences: dict[str, list[Occurrence]] = snapshot["occurrences"]

    for path, budget in sorted(baseline.get("line_budgets", {}).items()):
        current_lines = line_counts.get(path)
        if current_lines is None:
            continue
        max_lines = int(budget["max_lines"])
        if current_lines > max_lines:
            target = int(budget.get("target_lines", max_lines))
            errors.append(
                "check_quality_budget: "
                f"{path}: file-size budget regressed to {current_lines} lines "
                f"(baseline max {max_lines}, ratchet target {target})"
            )

    for metric, metric_baseline in sorted(baseline.get("metrics", {}).items()):
        current_occurrences = occurrences.get(metric, [])
        current_total = len(current_occurrences)
        allowed_total = int(metric_baseline.get("max_total", 0))
        if current_total > allowed_total:
            errors.append(
                "check_quality_budget: "
                f"{metric}: total count regressed to {current_total} "
                f"(baseline max {allowed_total})"
            )

        allowed_by_file = metric_baseline.get("by_file", {})
        grouped = grouped_occurrences(current_occurrences)
        for path, path_occurrences in grouped.items():
            current_count = len(path_occurrences)
            allowed_count = int(allowed_by_file.get(path, 0))
            if current_count <= allowed_count:
                continue
            errors.append(
                "check_quality_budget: "
                f"{metric}: {path}: {current_count} occurrence(s) "
                f"(baseline max {allowed_count})"
            )
            for occurrence in path_occurrences[:20]:
                errors.append(
                    f"  {occurrence.path}:{occurrence.line_number}: "
                    f"{occurrence.label}: {occurrence.text}"
                )
            if len(path_occurrences) > 20:
                errors.append(
                    f"  {path}: ... {len(path_occurrences) - 20} more occurrence(s)"
                )

    return errors


def write_baseline(root: Path, path: Path) -> None:
    baseline = build_baseline(root)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(baseline, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def run_check(
    root: Path,
    baseline_path: Path,
    *,
    review_base: str | None,
    allow_baseline_increase: bool,
) -> int:
    baseline = load_baseline(baseline_path)
    snapshot = scan(root)
    errors = compare(snapshot, baseline)
    if review_base:
        previous_baseline = load_baseline_from_git(root, review_base, baseline_path)
        errors.extend(
            baseline_increase_errors(
                baseline,
                previous_baseline,
                allow_baseline_increase=allow_baseline_increase,
            )
        )
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print("check_quality_budget: OK")
    print(ratchet_summary(snapshot, baseline))
    return 0


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def self_test() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        write_text(
            root / "rust/src/lib.rs",
            "# SPDX-License-Identifier: MIT OR Apache-2.0\npub fn ok() -> u8 { 1 }\n",
        )
        write_text(
            root / "rust/tests/lib_test.rs",
            "#[test]\nfn ignored_test_panic() { panic!(\"test-only\"); }\n",
        )
        write_text(
            root / "ts/src/reader.ts",
            "export function ok(): number { return 1; }\n",
        )
        write_text(
            root / "dist/generated.ts",
            "throw new Error(\"generated output is ignored\");\n",
        )

        baseline = build_baseline(root)
        initial_errors = compare(scan(root), baseline)
        if initial_errors:
            print("check_quality_budget: self-test initial scan failed", file=sys.stderr)
            for error in initial_errors:
                print(error, file=sys.stderr)
            return 1

        write_text(
            root / "rust/src/lib.rs",
            "# SPDX-License-Identifier: MIT OR Apache-2.0\n"
            "pub fn ok() -> u8 { 1 }\n"
            "pub fn new_panic() { panic!(\"new unchecked panic\"); }\n",
        )
        panic_errors = compare(scan(root), baseline)
        if not any("rust/src/lib.rs:3" in error for error in panic_errors):
            print(
                "check_quality_budget: self-test did not catch production panic",
                file=sys.stderr,
            )
            return 1

        write_text(
            root / "rust/src/lib.rs",
            "# SPDX-License-Identifier: MIT OR Apache-2.0\npub fn ok() -> u8 { 1 }\n",
        )
        write_text(
            root / "ts/src/reader.ts",
            "export function parse(): never {\n"
            "  throw new Error(\"new generic parser throw\");\n"
            "}\n",
        )
        throw_errors = compare(scan(root), baseline)
        if not any("ts/src/reader.ts:2" in error for error in throw_errors):
            print(
                "check_quality_budget: self-test did not catch generic parser throw",
                file=sys.stderr,
            )
            return 1

        write_text(
            root / "rust/tests/lib_test.rs",
            "#[test]\nfn ignored_test_panic() { panic!(\"still ignored\"); }\n",
        )
        write_text(
            root / "ts/src/reader.ts",
            "export function ok(): number { return 1; }\n",
        )
        test_only_errors = compare(scan(root), baseline)
        if test_only_errors:
            print(
                "check_quality_budget: self-test counted excluded test/generated files",
                file=sys.stderr,
            )
            for error in test_only_errors:
                print(error, file=sys.stderr)
            return 1

        increased_baseline = json.loads(json.dumps(baseline))
        increased_baseline["metrics"]["unchecked_panic_calls"]["max_total"] = 1
        increase_errors = baseline_increase_errors(
            increased_baseline,
            baseline,
            allow_baseline_increase=False,
        )
        if not any(
            "baseline increase requires explicit review" in error
            for error in increase_errors
        ):
            print(
                "check_quality_budget: self-test did not catch silent baseline increase",
                file=sys.stderr,
            )
            return 1

        missing_previous_errors = baseline_increase_errors(
            increased_baseline,
            empty_baseline(),
            allow_baseline_increase=False,
        )
        if not any(
            "baseline increase requires explicit review" in error
            for error in missing_previous_errors
        ):
            print(
                "check_quality_budget: self-test did not handle missing "
                "previous baseline",
                file=sys.stderr,
            )
            return 1

        approved_increase_errors = baseline_increase_errors(
            increased_baseline,
            baseline,
            allow_baseline_increase=True,
        )
        if approved_increase_errors:
            print(
                "check_quality_budget: self-test rejected approved baseline increase",
                file=sys.stderr,
            )
            return 1

        reviewed_baseline = json.loads(json.dumps(increased_baseline))
        reviewed_baseline[BASELINE_REVIEW_NOTE_KEY] = {
            "reviewed_by": "architecture review",
            "reason": "temporary release exception under quality-budget policy",
        }
        reviewed_errors = baseline_increase_errors(
            reviewed_baseline,
            baseline,
            allow_baseline_increase=False,
        )
        if reviewed_errors:
            print(
                "check_quality_budget: self-test rejected reviewed baseline increase",
                file=sys.stderr,
            )
            return 1

        previous_reviewed_baseline = json.loads(json.dumps(baseline))
        previous_reviewed_baseline[BASELINE_REVIEW_NOTE_KEY] = reviewed_baseline[
            BASELINE_REVIEW_NOTE_KEY
        ]
        stale_review_errors = baseline_increase_errors(
            reviewed_baseline,
            previous_reviewed_baseline,
            allow_baseline_increase=False,
        )
        if not stale_review_errors:
            print(
                "check_quality_budget: self-test accepted stale review note",
                file=sys.stderr,
            )
            return 1

        malformed = root / "quality/quality-budget-baseline.json"
        write_text(malformed, "{")
        try:
            load_baseline(malformed)
        except SystemExit as error:
            if "malformed baseline" not in str(error):
                print(
                    "check_quality_budget: self-test reported the wrong malformed "
                    "baseline error",
                    file=sys.stderr,
                )
                return 1
        else:
            print(
                "check_quality_budget: self-test did not catch malformed baseline",
                file=sys.stderr,
            )
            return 1

    print("check_quality_budget: self-test OK")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help="repository root to scan",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=DEFAULT_BASELINE,
        help="quality-budget baseline JSON path",
    )
    parser.add_argument(
        "--write-baseline",
        action="store_true",
        help="replace the baseline with the current scan",
    )
    parser.add_argument(
        "--review-base",
        default=os.environ.get(REVIEW_BASE_ENV),
        help=(
            "git ref whose baseline is used to reject unreviewed baseline "
            f"increases; defaults to ${REVIEW_BASE_ENV}"
        ),
    )
    parser.add_argument(
        "--allow-baseline-increase",
        action="store_true",
        default=is_truthy(os.environ.get(BASELINE_INCREASE_APPROVED_ENV)),
        help=(
            "allow baseline increases, normally set by an approved PR label "
            f"via ${BASELINE_INCREASE_APPROVED_ENV}"
        ),
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run checker self-tests for covered regressions and exclusions",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    baseline = args.baseline.resolve()
    if args.self_test:
        return self_test()
    if args.write_baseline:
        write_baseline(root, baseline)
        print(f"check_quality_budget: wrote {baseline}")
        return 0
    return run_check(
        root,
        baseline,
        review_base=args.review_base,
        allow_baseline_increase=args.allow_baseline_increase,
    )


if __name__ == "__main__":
    raise SystemExit(main())

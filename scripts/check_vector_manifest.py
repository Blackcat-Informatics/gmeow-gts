#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Validate, and optionally regenerate, the committed vector manifest."""

from __future__ import annotations

import argparse
from copy import deepcopy
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
ROOT_RESOLVED = ROOT.resolve()
VECTORS = ROOT / "vectors"
MANIFEST = VECTORS / "manifest.json"
SCHEMA = "https://blackcatinformatics.ca/gts/vector-manifest/v1"
DEFAULT_CORPUS_REVISION = "git:repository-commit-containing-manifest"
FULL_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")

VALID_MODES = {
    "permissive-read",
    "strict-verify",
    "publish-verify",
    "profile-verify",
    "pre-segment",
}
VALID_SUBSETS = {
    "wire-core",
    "total-reader",
    "graph-fold",
    "profile-layout",
    "streaming-property",
    "writer-determinism",
    "crypto-cose",
    "crypto-encrypt",
    "crypto-deferred",
    "openpgp-transport-key",
    "human-hash",
    "security-policy",
    "advanced-index-proof",
}
VALID_TIERS = {
    "baseline-reader",
    "streaming-reader",
    "full-reader",
    "writer",
    "validating-tool",
    "profile-aware-tool",
}

TOP_LEVEL_SUBSETS = {
    "01-minimal": ("wire-core",),
    "02-zstd-frame": ("wire-core",),
    "03-unknown-codec": ("total-reader",),
    "04-damaged-frame": ("total-reader",),
    "05-torn-append": ("total-reader",),
    "06-header-tampered": ("wire-core",),
    "09-suppression": ("graph-fold",),
    "11-datatype-defaulting": ("graph-fold",),
    "12-conflicting-reifier": ("graph-fold",),
    "13-position-constraint": ("graph-fold",),
    "14-bnode-label": ("graph-fold",),
    "15-two-segment-union": ("graph-fold",),
    "15b-anon-bnode-union": ("graph-fold",),
    "16-composed-round-trip": ("graph-fold",),
    "17-pre-segment-hard-fail": ("total-reader",),
    "18-cross-segment-suppression": ("graph-fold",),
    "19-profile-union-opacity": ("total-reader",),
    "20-language-tag-discipline": ("profile-layout",),
    "21-degenerate-composition": ("profile-layout",),
    "22-inline-blob": ("graph-fold",),
    "23-files-profile-tree": ("profile-layout",),
    "24-files-profile-dedup": ("profile-layout",),
    "25-streamable-source": ("profile-layout",),
    "25b-streamable-compacted": ("profile-layout",),
    "26-streamable-lie": ("profile-layout",),
    "27-streamable-tail": ("profile-layout",),
    "28-empty-file": ("total-reader",),
    "28b-non-header-item": ("total-reader",),
    "28c-unsupported-version": ("total-reader",),
    "28d-unknown-frame-type": ("total-reader",),
    "28e-forward-term-reference": ("total-reader",),
    "28f-malformed-transform-shape": ("total-reader",),
    "29-deterministic-writer": ("writer-determinism",),
}

TOP_LEVEL_CAPABILITIES = {
    "02-zstd-frame": ("cbor", "blake3", "identity", "zstd"),
    "22-inline-blob": ("cbor", "blake3", "identity", "inline-blob"),
    "23-files-profile-tree": ("cbor", "blake3", "identity", "files-profile"),
    "24-files-profile-dedup": ("cbor", "blake3", "identity", "files-profile"),
    "25-streamable-source": ("cbor", "blake3", "identity", "cose-sign1"),
    "25b-streamable-compacted": (
        "cbor",
        "blake3",
        "identity",
        "cose-sign1",
        "streamable-index",
    ),
    "26-streamable-lie": ("cbor", "blake3", "identity", "streamable-index"),
    "27-streamable-tail": ("cbor", "blake3", "identity", "streamable-index"),
    "29-deterministic-writer": ("cbor", "blake3", "identity", "inline-blob"),
}

JSON_SUBCORPUS = {
    "cose": {
        "subset": "crypto-cose",
        "tiers": ("full-reader",),
        "mode": "strict-verify",
        "capabilities": ("cose-sign1", "ed25519"),
        "title": "COSE Sign1 fixture",
        "notes": "Pins COSE Sign1 byte serialization and verification behavior.",
    },
    "encrypt0": {
        "subset": "crypto-encrypt",
        "tiers": ("full-reader",),
        "mode": "strict-verify",
        "capabilities": ("encrypt0", "aes-256-gcm"),
        "title": "COSE Encrypt0 fixture",
        "notes": "Pins fixed-IV Encrypt0 sealing and opening behavior.",
    },
    "crypto-deferred": {
        "subset": "crypto-deferred",
        "tiers": ("full-reader",),
        "mode": "strict-verify",
        "capabilities": ("cose-encrypt", "ecdh-es+a256kw", "aes-256-gcm"),
        "title": "deferred COSE Encrypt fixture",
        "notes": (
            "Pins deferred multi-recipient COSE_Encrypt and ECDH key-wrap "
            "contract shape without making an implementation claim."
        ),
    },
    "openpgp": {
        "subset": "openpgp-transport-key",
        "tiers": ("full-reader",),
        "mode": "strict-verify",
        "capabilities": ("openpgp", "ed25519"),
        "title": "OpenPGP transport-key fixture",
        "notes": "Pins OpenPGP transport-key extraction and fingerprint rendering.",
    },
    "emojihash": {
        "subset": "human-hash",
        "tiers": ("validating-tool",),
        "mode": "profile-verify",
        "capabilities": ("human-hash",),
        "title": "emojihash fixture",
        "notes": "Pins human-facing emoji digest rendering.",
    },
    "randomart": {
        "subset": "human-hash",
        "tiers": ("validating-tool",),
        "mode": "profile-verify",
        "capabilities": ("human-hash",),
        "title": "randomart fixture",
        "notes": "Pins human-facing randomart digest rendering.",
    },
    "security": {
        "subset": "security-policy",
        "tiers": ("full-reader", "profile-aware-tool"),
        "mode": "profile-verify",
        "capabilities": ("trust-policy", "nested-gts"),
        "title": "security policy fixture",
        "notes": "Pins security-policy diagnostics and profile findings.",
    },
    "proofs": {
        "subset": "advanced-index-proof",
        "tiers": ("full-reader",),
        "mode": "strict-verify",
        "capabilities": ("mmr-proof", "blake3"),
        "title": "MMR proof fixture",
        "notes": "Pins MMR detached inclusion-proof JSON verification.",
    },
    "signed": {
        "subset": "crypto-cose",
        "tiers": ("full-reader",),
        "mode": "strict-verify",
        "capabilities": ("cose-sign1", "ed25519"),
        "title": "signed GTS fixture",
        "notes": "Pins signed GTS writer output and signature verification.",
    },
}


class ManifestError(Exception):
    """Raised when the manifest fails validation."""


def rel(path: Path) -> str:
    """Return a repository-relative POSIX path."""
    return path.relative_to(ROOT).as_posix()


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def git_stdout(*args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *args],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise ManifestError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout.strip()


def git_ok(*args: str) -> bool:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *args],
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    return result.returncode == 0


def current_head_revision() -> str:
    return f"git:{git_stdout('rev-parse', '--verify', 'HEAD^{commit}')}"


def normalize_corpus_revision(value: str) -> str:
    value = value.strip()
    if value.startswith("git:"):
        return value
    return f"git:{value}"


def require_corpus_revision(value: Any, *, require_release_revision: bool) -> str:
    require(
        isinstance(value, str) and value.startswith("git:"),
        "corpus_revision must be a git: string",
    )
    name = value.removeprefix("git:")
    require(name, "corpus_revision must name a Git revision")
    require(name.strip() == name, "corpus_revision must not contain surrounding whitespace")
    require(
        "\n" not in name and "\r" not in name and "\t" not in name,
        "corpus_revision must not contain control whitespace",
    )
    if not require_release_revision:
        return value

    require(
        value != DEFAULT_CORPUS_REVISION,
        "release corpus_revision must not use the checked-in placeholder",
    )
    if FULL_COMMIT_RE.fullmatch(name):
        require(
            git_ok("cat-file", "-e", f"{name}^{{commit}}"),
            "release corpus_revision commit must resolve in this repository",
        )
        return value
    require(
        git_ok("show-ref", "--verify", "--quiet", f"refs/tags/{name}"),
        "release corpus_revision must name a full 40-character commit or local Git tag",
    )
    return value


def title_for(vector_id: str) -> str:
    words = vector_id.split("-", 1)[1] if "-" in vector_id else vector_id
    return words.replace("-", " ")


def top_level_entry(vector_id: str) -> dict[str, Any]:
    expected_path = VECTORS / f"{vector_id}.expected.json"
    expected = load_json(expected_path)
    mode = "pre-segment" if expected["mode"] == "pre-segment" else "permissive-read"
    diagnostics = expected["diagnostics"]
    segment_heads = expected["segment_heads"]
    expected_fields: dict[str, Any] = {
        "graph": f"vectors/{vector_id}.expected.json",
        "diagnostics": diagnostics,
        "expected_head": segment_heads[-1] if segment_heads else None,
        "segment_heads": segment_heads,
        "opaque_reasons": expected.get("opaque_reasons", []),
    }
    if vector_id == "21-degenerate-composition":
        expected_fields["exit_code"] = 1
        expected_fields["stderr_contains"] = ["hide every quad"]

    primary_subsets = TOP_LEVEL_SUBSETS[vector_id]
    subsets = sorted({*primary_subsets, "streaming-property", "writer-determinism"})
    tiers = {"baseline-reader", "streaming-reader", "writer"}
    if "profile-layout" in primary_subsets:
        tiers.add("validating-tool")
    negative = bool(diagnostics) or vector_id == "21-degenerate-composition"

    return {
        "id": vector_id,
        "title": title_for(vector_id),
        "input": {
            "path": f"vectors/{vector_id}.gts",
            "media_type": "application/vnd.blackcat.gts+cbor-seq",
        },
        "mode": mode,
        "negative": negative,
        "required_capabilities": list(
            TOP_LEVEL_CAPABILITIES.get(vector_id, ("cbor", "blake3", "identity"))
        ),
        "subsets": subsets,
        "tiers": sorted(tiers),
        "expected": expected_fields,
        "notes": f"Top-level GTS conformance vector for {', '.join(primary_subsets)}.",
    }


def json_fixture_entry(path: Path) -> dict[str, Any]:
    subdir = path.parent.name
    meta = JSON_SUBCORPUS[subdir]
    data = load_json(path)
    negative = bool(data.get("negative", False))
    if subdir == "proofs" and "bad" in path.stem:
        negative = True
    expected: dict[str, Any] = {
        "graph": None,
        "diagnostics": data.get("expected_diagnostics", []),
        "expected_head": None,
        "fields": sorted(data),
    }
    if "expected_findings" in data:
        expected["profile_findings"] = data["expected_findings"]
    if "cose" in data:
        expected["encoded"] = "cose"
    if "gts" in data:
        expected["encoded"] = "gts"
    if "emoji" in data:
        expected["rendered"] = "emoji"
    if "art" in data:
        expected["rendered"] = "randomart"

    return {
        "id": f"{subdir}-{path.stem}",
        "title": f"{meta['title']}: {path.stem}",
        "input": {
            "path": rel(path),
            "media_type": "application/json",
        },
        "mode": meta["mode"],
        "negative": negative,
        "required_capabilities": list(meta["capabilities"]),
        "subsets": [meta["subset"]],
        "tiers": list(meta["tiers"]),
        "expected": expected,
        "notes": meta["notes"],
    }


def build_manifest() -> dict[str, Any]:
    top_level_ids = sorted(path.stem for path in VECTORS.glob("*.gts"))
    unknown = sorted(set(top_level_ids) - set(TOP_LEVEL_SUBSETS))
    missing = sorted(set(TOP_LEVEL_SUBSETS) - set(top_level_ids))
    if unknown or missing:
        raise ManifestError(
            f"top-level vector metadata drift: unknown={unknown} missing={missing}"
        )

    json_paths = sorted(path for path in VECTORS.glob("*/*.json"))
    unknown_dirs = sorted({path.parent.name for path in json_paths} - set(JSON_SUBCORPUS))
    if unknown_dirs:
        raise ManifestError(f"JSON subcorpus metadata missing for: {unknown_dirs}")

    entries = [top_level_entry(vector_id) for vector_id in top_level_ids]
    entries.extend(json_fixture_entry(path) for path in json_paths)
    entries.sort(key=lambda item: item["input"]["path"])
    return {
        "schema": SCHEMA,
        "manifest_version": 1,
        "corpus_revision": DEFAULT_CORPUS_REVISION,
        "generated_by": "scripts/check_vector_manifest.py --write",
        "vectors": entries,
    }


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ManifestError(message)


def require_string_list(value: Any, field: str) -> None:
    require(isinstance(value, list), f"{field} must be a list")
    require(all(isinstance(item, str) for item in value), f"{field} must be strings")


def require_safe_id(vector_id: str) -> None:
    require(
        all((char.isascii() and char.isalnum()) or char in "-_" for char in vector_id),
        f"{vector_id}: id must use ASCII alphanumerics, hyphens, or underscores",
    )


def repo_path(path_text: str, field: str, vector_id: str) -> Path:
    path = Path(path_text)
    require(not path.is_absolute(), f"{vector_id}: {field} must be relative")
    require(".." not in path.parts, f"{vector_id}: {field} escapes root")
    resolved = (ROOT / path).resolve()
    try:
        resolved.relative_to(ROOT_RESOLVED)
    except ValueError as exc:
        raise ManifestError(f"{vector_id}: {field} escapes root") from exc
    return resolved


def validate_entry(entry: Any, ids: set[str]) -> None:
    require(isinstance(entry, dict), "vector entry must be an object")
    vector_id = entry.get("id")
    require(isinstance(vector_id, str) and vector_id, "id must be a non-empty string")
    require_safe_id(vector_id)
    require(vector_id not in ids, f"duplicate vector id: {vector_id}")
    ids.add(vector_id)

    require(isinstance(entry.get("title"), str), f"{vector_id}: title must be a string")
    require(entry.get("mode") in VALID_MODES, f"{vector_id}: invalid mode")
    require(isinstance(entry.get("negative"), bool), f"{vector_id}: negative must be bool")
    require_string_list(entry.get("required_capabilities"), f"{vector_id}.required_capabilities")
    require_string_list(entry.get("subsets"), f"{vector_id}.subsets")
    require_string_list(entry.get("tiers"), f"{vector_id}.tiers")
    require(set(entry["subsets"]) <= VALID_SUBSETS, f"{vector_id}: unknown subset")
    require(set(entry["tiers"]) <= VALID_TIERS, f"{vector_id}: unknown tier")
    require(entry["subsets"], f"{vector_id}: subsets must not be empty")
    require(entry["tiers"], f"{vector_id}: tiers must not be empty")
    require(isinstance(entry.get("notes"), str), f"{vector_id}: notes must be a string")

    input_info = entry.get("input")
    require(isinstance(input_info, dict), f"{vector_id}: input must be an object")
    input_path = input_info.get("path")
    require(isinstance(input_path, str), f"{vector_id}: input.path must be a string")
    require(
        input_info.get("media_type")
        in {"application/vnd.blackcat.gts+cbor-seq", "application/json"},
        f"{vector_id}: unsupported input media_type",
    )
    resolved_input = repo_path(input_path, "input.path", vector_id)
    require(
        resolved_input.is_file(),
        f"{vector_id}: missing input path {input_path}",
    )

    expected = entry.get("expected")
    require(isinstance(expected, dict), f"{vector_id}: expected must be an object")
    require("graph" in expected, f"{vector_id}: expected.graph missing")
    graph = expected["graph"]
    require(graph is None or isinstance(graph, str), f"{vector_id}: graph must be string/null")
    if graph is not None:
        resolved_graph = repo_path(graph, "expected.graph", vector_id)
        require(resolved_graph.is_file(), f"{vector_id}: missing expected graph path")
    require_string_list(expected.get("diagnostics"), f"{vector_id}.expected.diagnostics")
    require(
        "expected_head" in expected
        and (expected["expected_head"] is None or isinstance(expected["expected_head"], str)),
        f"{vector_id}: expected_head must be string/null",
    )


PINNED_ENTRY_FIELDS = (
    "id",
    "title",
    "input",
    "mode",
    "negative",
    "required_capabilities",
    "subsets",
    "tiers",
    "expected",
    "notes",
)


def require_generated_metadata(entry: dict[str, Any], expected: dict[str, Any]) -> None:
    vector_id = entry["id"]
    for field in PINNED_ENTRY_FIELDS:
        require(
            entry.get(field) == expected[field],
            f"{vector_id}: {field} drift from generated manifest metadata",
        )


def validate_manifest(manifest: Any, *, require_release_revision: bool = False) -> None:
    require(isinstance(manifest, dict), "manifest must be a JSON object")
    require(manifest.get("schema") == SCHEMA, "manifest schema mismatch")
    require(manifest.get("manifest_version") == 1, "manifest_version must be 1")
    require_corpus_revision(
        manifest.get("corpus_revision"),
        require_release_revision=require_release_revision,
    )
    require(isinstance(manifest.get("generated_by"), str), "generated_by must be a string")
    vectors = manifest.get("vectors")
    require(isinstance(vectors, list) and vectors, "vectors must be a non-empty list")

    ids: set[str] = set()
    for entry in vectors:
        validate_entry(entry, ids)

    top_level_entries = [
        entry for entry in vectors if entry["input"]["path"].startswith("vectors/")
        and "/" not in entry["input"]["path"][len("vectors/") :]
        and entry["input"]["path"].endswith(".gts")
    ]
    manifest_top_level = sorted(entry["id"] for entry in top_level_entries)
    filesystem_top_level = sorted(path.stem for path in VECTORS.glob("*.gts"))
    require(
        manifest_top_level == filesystem_top_level,
        "manifest top-level .gts coverage drift: "
        f"manifest={manifest_top_level} filesystem={filesystem_top_level}",
    )
    expected_filesystem = sorted(
        path.name.removesuffix(".expected.json")
        for path in VECTORS.glob("*.expected.json")
    )
    require(
        manifest_top_level == expected_filesystem,
        "manifest top-level expected-json coverage drift: "
        f"manifest={manifest_top_level} filesystem={expected_filesystem}",
    )

    for entry in top_level_entries:
        vector_id = entry["id"]
        require(
            entry["input"]["path"] == f"vectors/{vector_id}.gts",
            f"{vector_id}: top-level input path must match id",
        )
        require(
            entry["expected"]["graph"] == f"vectors/{vector_id}.expected.json",
            f"{vector_id}: top-level expected graph path must match id",
        )
        expected = load_json(VECTORS / f"{vector_id}.expected.json")
        expected_mode = "pre-segment" if expected["mode"] == "pre-segment" else "permissive-read"
        require(entry["mode"] == expected_mode, f"{vector_id}: manifest/read mode mismatch")
        require(
            entry["expected"]["diagnostics"] == expected["diagnostics"],
            f"{vector_id}: diagnostics drift from expected JSON",
        )
        segment_heads = expected["segment_heads"]
        require(
            entry["expected"].get("segment_heads") == segment_heads,
            f"{vector_id}: segment_heads drift from expected JSON",
        )
        require(
            entry["expected"]["expected_head"] == (segment_heads[-1] if segment_heads else None),
            f"{vector_id}: expected_head drift from expected JSON",
        )
        require(
            "streaming-property" in entry["subsets"],
            f"{vector_id}: top-level vectors must declare streaming-property",
        )
        require_generated_metadata(entry, top_level_entry(vector_id))

    manifest_json_paths = sorted(
        entry["input"]["path"]
        for entry in vectors
        if entry["input"]["path"].startswith("vectors/")
        and "/" in entry["input"]["path"][len("vectors/") :]
        and entry["input"]["path"].endswith(".json")
    )
    filesystem_json_paths = sorted(rel(path) for path in VECTORS.glob("*/*.json"))
    require(
        manifest_json_paths == filesystem_json_paths,
        "manifest JSON subcorpus coverage drift: "
        f"manifest={manifest_json_paths} filesystem={filesystem_json_paths}",
    )
    for entry in vectors:
        input_path = entry["input"]["path"]
        if not input_path.endswith(".json") or "/" not in input_path[len("vectors/") :]:
            continue
        fixture_path = repo_path(input_path, "input.path", entry["id"])
        subdir = fixture_path.parent.name
        require(
            subdir in JSON_SUBCORPUS,
            f"{entry['id']}: JSON subcorpus metadata missing for {subdir}",
        )
        require_generated_metadata(entry, json_fixture_entry(fixture_path))


def expect_invalid(
    manifest: Any,
    text: str,
    *,
    require_release_revision: bool = False,
) -> None:
    try:
        validate_manifest(manifest, require_release_revision=require_release_revision)
    except ManifestError as exc:
        require(text in str(exc), f"expected {text!r} in self-test error {exc!r}")
        return
    raise ManifestError(f"self-test expected invalid manifest containing {text!r}")


def mutated_manifest() -> dict[str, Any]:
    return deepcopy(build_manifest())


def run_self_tests() -> None:
    expect_invalid([], "JSON object")

    manifest = mutated_manifest()
    manifest["vectors"][0]["id"] = "../bad"
    expect_invalid(manifest, "id must use ASCII")

    manifest = mutated_manifest()
    manifest["vectors"][0]["input"]["path"] = "/tmp/outside.gts"
    expect_invalid(manifest, "input.path must be relative")

    manifest = mutated_manifest()
    manifest["vectors"][0]["expected"]["graph"] = "../outside.expected.json"
    expect_invalid(manifest, "expected.graph escapes root")

    manifest = mutated_manifest()
    top_level = next(entry for entry in manifest["vectors"] if entry["input"]["path"].endswith(".gts"))
    top_level["required_capabilities"] = ["cbor"]
    expect_invalid(manifest, "required_capabilities drift")

    manifest = mutated_manifest()
    fixture = next(
        entry
        for entry in manifest["vectors"]
        if entry["input"]["path"].endswith(".json")
        and "/" in entry["input"]["path"][len("vectors/") :]
    )
    fixture["tiers"] = ["baseline-reader"]
    expect_invalid(manifest, "tiers drift")

    manifest = mutated_manifest()
    fixture = next(
        entry
        for entry in manifest["vectors"]
        if entry["input"]["path"].endswith(".json")
        and "/" in entry["input"]["path"][len("vectors/") :]
    )
    fixture["expected"]["fields"] = []
    expect_invalid(manifest, "expected drift")

    manifest = mutated_manifest()
    expect_invalid(
        manifest,
        "checked-in placeholder",
        require_release_revision=True,
    )

    manifest = mutated_manifest()
    manifest["corpus_revision"] = "git:deadbeef"
    expect_invalid(
        manifest,
        "full 40-character commit or local Git tag",
        require_release_revision=True,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write",
        action="store_true",
        help="rewrite vectors/manifest.json from the current corpus files",
    )
    parser.add_argument(
        "--corpus-revision",
        help=(
            "validate with this exact release corpus revision; accepts git:<full-commit>, "
            "<full-commit>, or a local Git tag"
        ),
    )
    parser.add_argument(
        "--release-manifest",
        type=Path,
        help=(
            "write a release manifest artifact with an exact corpus_revision; defaults "
            "to the current HEAD commit when --corpus-revision is omitted"
        ),
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run validator rejection self-tests",
    )
    args = parser.parse_args()

    try:
        if args.self_test:
            run_self_tests()
        if args.write:
            manifest = build_manifest()
            MANIFEST.write_text(
                json.dumps(manifest, indent=2, sort_keys=False) + "\n",
                encoding="utf-8",
            )
        else:
            if not MANIFEST.is_file():
                raise ManifestError(f"missing {rel(MANIFEST)}")
            manifest = load_json(MANIFEST)
        require_release_revision = (
            args.corpus_revision is not None or args.release_manifest is not None
        )
        if args.corpus_revision is not None:
            manifest["corpus_revision"] = normalize_corpus_revision(args.corpus_revision)
        elif args.release_manifest is not None:
            manifest["corpus_revision"] = current_head_revision()

        validate_manifest(manifest, require_release_revision=require_release_revision)

        if args.release_manifest is not None:
            args.release_manifest.parent.mkdir(parents=True, exist_ok=True)
            args.release_manifest.write_text(
                json.dumps(manifest, indent=2, sort_keys=False) + "\n",
                encoding="utf-8",
            )
    except (json.JSONDecodeError, ManifestError) as exc:
        print(f"check_vector_manifest: {exc}", file=sys.stderr)
        return 1

    if args.release_manifest is not None:
        print(f"check_vector_manifest: wrote {args.release_manifest}")
    print("check_vector_manifest: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

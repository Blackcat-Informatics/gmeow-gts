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
MANIFESTS = {
    "aggregate": MANIFEST,
    "core": VECTORS / "manifest.core.json",
    "profiles": VECTORS / "manifest.profiles.json",
    "transforms": VECTORS / "manifest.transforms.json",
}
SCHEMA = "https://blackcatinformatics.ca/gts/vector-manifest/v1"
GTS_MEDIA_TYPE = "application/vnd.blackcat.gts+cbor-seq"
DEFAULT_CORPUS_REVISION = "git:repository-commit-containing-manifest"
FULL_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
CORE_MANIFEST_SUBSETS = {
    "wire-core",
    "total-reader",
    "graph-fold",
    "writer-determinism",
}
PROFILE_MANIFEST_SUBSETS = {
    "profile-layout",
    "okf-bundle",
    "security-policy",
}
TRANSFORM_MANIFEST_SUBSETS = {
    "tar-archive",
}

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
    "resilience-negative",
    "streaming-property",
    "corpus-generator-determinism",
    "writer-determinism",
    "crypto-cose",
    "crypto-encrypt",
    "crypto-deferred",
    "openpgp-transport-key",
    "human-hash",
    "security-policy",
    "advanced-index-proof",
    "okf-bundle",
    "tar-archive",
}
VALID_TIERS = {
    "baseline-reader",
    "streaming-reader",
    "full-reader",
    "writer",
    "validating-tool",
    "profile-aware-tool",
    "transform-tool",
}
RESILIENCE_NEGATIVE_MAX_BYTES = 64 * 1024
RESILIENCE_NEGATIVE_TOP_LEVEL = frozenset(
    {
        "03-unknown-codec",
        "04-damaged-frame",
        "05-torn-append",
        "06-header-tampered",
        "17-pre-segment-hard-fail",
        "19-profile-union-opacity",
        "21-degenerate-composition",
        "26-streamable-lie",
        "28-empty-file",
        "28b-non-header-item",
        "28c-unsupported-version",
        "28d-unknown-frame-type",
        "28e-forward-term-reference",
        "28f-malformed-transform-shape",
        "28g-damaged-compressed-payload",
        "28h-malformed-security-metadata",
    }
)

TOP_LEVEL_SUBSETS = {
    "01-minimal": ("wire-core",),
    "02-zstd-frame": ("wire-core",),
    "03-unknown-codec": ("total-reader", "resilience-negative"),
    "04-damaged-frame": ("total-reader", "resilience-negative"),
    "05-torn-append": ("total-reader", "resilience-negative"),
    "06-header-tampered": ("wire-core", "resilience-negative"),
    "09-suppression": ("graph-fold",),
    "11-datatype-defaulting": ("graph-fold",),
    "12-conflicting-reifier": ("graph-fold",),
    "13-position-constraint": ("graph-fold",),
    "14-bnode-label": ("graph-fold",),
    "15-two-segment-union": ("graph-fold",),
    "15b-anon-bnode-union": ("graph-fold",),
    "16-composed-round-trip": ("graph-fold",),
    "17-pre-segment-hard-fail": ("total-reader", "resilience-negative"),
    "18-cross-segment-suppression": ("graph-fold",),
    "19-profile-union-opacity": ("total-reader", "resilience-negative"),
    "20-language-tag-discipline": ("profile-layout",),
    "21-degenerate-composition": ("profile-layout", "resilience-negative"),
    "22-inline-blob": ("graph-fold",),
    "23-files-profile-tree": ("profile-layout",),
    "24-files-profile-dedup": ("profile-layout",),
    "25-streamable-source": ("profile-layout",),
    "25b-streamable-compacted": ("profile-layout",),
    "26-streamable-lie": ("profile-layout", "resilience-negative"),
    "27-streamable-tail": ("profile-layout",),
    "28-empty-file": ("total-reader", "resilience-negative"),
    "28b-non-header-item": ("total-reader", "resilience-negative"),
    "28c-unsupported-version": ("total-reader", "resilience-negative"),
    "28d-unknown-frame-type": ("total-reader", "resilience-negative"),
    "28e-forward-term-reference": ("total-reader", "resilience-negative"),
    "28f-malformed-transform-shape": ("total-reader", "resilience-negative"),
    "28g-damaged-compressed-payload": ("total-reader", "resilience-negative"),
    "28h-malformed-security-metadata": ("total-reader", "resilience-negative"),
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
    "28g-damaged-compressed-payload": ("cbor", "blake3", "identity", "zstd"),
    "28h-malformed-security-metadata": (
        "cbor",
        "blake3",
        "identity",
        "cose-encrypt0",
    ),
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

OKF_MEDIA_TYPE = "application/vnd.blackcat.gts.okf-bundle"
TAR_MEDIA_TYPES = {
    ".tar": "application/x-tar",
    ".tar.gz": "application/gzip",
    ".tar.zst": "application/zstd",
}

OKF_BUNDLES = {
    "bigquery-join": {
        "title": "OKF bundle fixture: BigQuery join model",
        "notes": (
            "Pins a synthetic BigQuery-style OKF bundle with schema-table prose, "
            "resource IRIs, typed extension fields, and cross-document joins."
        ),
    },
    "extensions": {
        "title": "OKF bundle fixture: producer extensions",
        "notes": (
            "Pins producer extension key projection for integer, decimal, boolean, "
            "null, sequence, and nested-object frontmatter values."
        ),
    },
    "full-frontmatter": {
        "title": "OKF bundle fixture: full frontmatter",
        "notes": (
            "Pins the standard OKF frontmatter fields: title, description, "
            "resource, tags, and timestamp."
        ),
    },
    "minimal": {
        "title": "OKF bundle fixture: minimal document",
        "notes": "Pins the smallest accepted OKF document with only required type metadata.",
    },
    "nested-links": {
        "title": "OKF bundle fixture: nested links",
        "notes": (
            "Pins nested directories, index.md, relative Markdown links, repeated "
            "links, and link annotation ordering."
        ),
    },
    "unmapped-sidecar": {
        "title": "OKF bundle fixture: unmapped sidecar",
        "notes": (
            "Pins the clean OKF import projection and the companion Rust sidecar "
            "test for out-of-profile RDF export."
        ),
    },
}

TAR_FIXTURES = {
    "ustar-basic": {
        "input": "ustar-basic.tar",
        "title": "tar fixture: USTAR basic tree",
        "notes": "Pins USTAR regular-file, directory, owner, and mode import.",
    },
    "gnu-links": {
        "input": "gnu-links.tar",
        "title": "tar fixture: GNU links and long path",
        "notes": "Pins GNU long path, symlink, hardlink, owner, and deduplicated payload import.",
    },
    "pax-metadata": {
        "input": "pax-metadata.tar",
        "title": "tar fixture: PAX metadata",
        "notes": "Pins PAX subsecond mtime, comment, and SCHILY xattr-style records.",
    },
    "special-nodes": {
        "input": "special-nodes.tar",
        "title": "tar fixture: special node metadata",
        "notes": "Pins fifo, character-device, and block-device metadata import.",
    },
    "gzip-basic": {
        "input": "gzip-basic.tar.gz",
        "title": "tar fixture: gzip-compressed tar",
        "notes": "Pins transparent gzip-compressed tar import.",
    },
    "zstd-basic": {
        "input": "zstd-basic.tar.zst",
        "title": "tar fixture: zstd-compressed tar",
        "notes": "Pins transparent zstd-compressed tar import.",
    },
    "danger-absolute": {
        "input": "danger-absolute.tar",
        "title": "tar refusal fixture: absolute path",
        "negative": True,
        "notes": "Pins refusal of tar entries with absolute paths.",
    },
    "danger-traversal": {
        "input": "danger-traversal.tar",
        "title": "tar refusal fixture: parent traversal",
        "negative": True,
        "notes": "Pins refusal of tar entries containing parent-directory traversal.",
    },
    "danger-symlink-escape": {
        "input": "danger-symlink-escape.tar",
        "title": "tar refusal fixture: symlink escape",
        "negative": True,
        "notes": "Pins extraction refusal for symlink targets that escape the destination.",
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

    negative = bool(diagnostics) or vector_id == "21-degenerate-composition"
    primary_subsets = TOP_LEVEL_SUBSETS[vector_id]
    subsets = {
        *primary_subsets,
        "streaming-property",
        "corpus-generator-determinism",
    }
    tiers: set[str]
    if "profile-layout" in primary_subsets:
        tiers = {"validating-tool"}
    else:
        tiers = {"baseline-reader", "streaming-reader"}
    if not negative:
        subsets.add("writer-determinism")
        tiers.add("writer")

    return {
        "id": vector_id,
        "title": title_for(vector_id),
        "input": {
            "path": f"vectors/{vector_id}.gts",
            "media_type": GTS_MEDIA_TYPE,
        },
        "mode": mode,
        "negative": negative,
        "required_capabilities": list(
            TOP_LEVEL_CAPABILITIES.get(vector_id, ("cbor", "blake3", "identity"))
        ),
        "subsets": sorted(subsets),
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


def path_contains_markdown(path: Path) -> bool:
    return any(not child.is_symlink() and child.is_file() for child in path.rglob("*.md"))


def okf_bundle_dirs() -> list[Path]:
    root = VECTORS / "okf"
    if not root.exists():
        return []
    return sorted(
        path
        for path in root.iterdir()
        if path.is_dir() and path_contains_markdown(path)
    )


def okf_fixture_entry(path: Path) -> dict[str, Any]:
    fixture_id = path.name
    meta = OKF_BUNDLES[fixture_id]
    expected: dict[str, Any] = {
        "graph": rel(VECTORS / "okf" / f"{fixture_id}.folded.nq"),
        "diagnostics": [],
        "expected_head": None,
        "documents": okf_concept_document_count(path),
    }
    sidecar = VECTORS / "okf" / f"{fixture_id}.expected-unmapped.nq"
    if sidecar.exists():
        expected["unmapped_sidecar"] = rel(sidecar)

    return {
        "id": f"okf-{fixture_id}",
        "title": meta["title"],
        "input": {
            "path": rel(path),
            "media_type": OKF_MEDIA_TYPE,
        },
        "mode": "profile-verify",
        "negative": False,
        "required_capabilities": ["cbor", "blake3", "identity", "okf"],
        "subsets": ["okf-bundle"],
        "tiers": ["profile-aware-tool"],
        "expected": expected,
        "notes": meta["notes"],
    }


def tar_fixture_paths() -> list[Path]:
    root = VECTORS / "tar"
    if not root.exists():
        return []
    return sorted(
        path
        for path in root.iterdir()
        if path.is_file() and any(path.name.endswith(suffix) for suffix in TAR_MEDIA_TYPES)
    )


def tar_input_media_type(path: Path) -> str:
    for suffix, media_type in TAR_MEDIA_TYPES.items():
        if path.name.endswith(suffix):
            return media_type
    raise ManifestError(f"{rel(path)}: unsupported tar fixture extension")


def tar_fixture_id(path: Path) -> str:
    matches = [
        fixture_id
        for fixture_id, meta in TAR_FIXTURES.items()
        if meta["input"] == path.name
    ]
    if len(matches) != 1:
        raise ManifestError(f"{rel(path)}: tar fixture metadata missing")
    return matches[0]


def tar_fixture_entry(path: Path) -> dict[str, Any]:
    fixture_id = tar_fixture_id(path)
    meta = TAR_FIXTURES[fixture_id]
    expected_path = VECTORS / "tar" / f"{fixture_id}.expected.json"
    expected_data = load_json(expected_path)
    if expected_data.get("fixture") != fixture_id:
        raise ManifestError(f"tar-{fixture_id}: tar expected fixture id drift")
    if expected_data.get("input") != path.name:
        raise ManifestError(f"tar-{fixture_id}: tar expected input drift")
    negative = bool(meta.get("negative", False))
    graph_path = VECTORS / "tar" / f"{fixture_id}.folded.nq"
    expected: dict[str, Any] = {
        "graph": rel(graph_path) if graph_path.exists() else None,
        "diagnostics": expected_data.get("diagnostics", []),
        "expected_head": None,
        "fixture_kind": expected_data.get("kind"),
        "entries": len(expected_data.get("expected_entries", [])),
    }
    capabilities = ["cbor", "blake3", "identity", "files-profile", "tar"]
    if path.name.endswith(".tar.gz"):
        capabilities.append("gzip")
    elif path.name.endswith(".tar.zst"):
        capabilities.append("zstd")

    subsets = ["tar-archive"]
    tiers = ["transform-tool"]

    return {
        "id": f"tar-{fixture_id}",
        "title": meta["title"],
        "input": {
            "path": rel(path),
            "media_type": tar_input_media_type(path),
        },
        "mode": "profile-verify",
        "negative": negative,
        "required_capabilities": capabilities,
        "subsets": subsets,
        "tiers": tiers,
        "expected": expected,
        "notes": meta["notes"],
    }


def okf_concept_document_count(path: Path) -> int:
    count = 0
    for child in path.rglob("*.md"):
        if not child.is_file():
            continue
        try:
            text = child.read_text(encoding="utf-8")
        except UnicodeDecodeError as exc:
            raise ManifestError(f"{rel(child)}: markdown is not UTF-8: {exc}") from exc
        if text.startswith("---\n") or text.startswith("---\r\n"):
            count += 1
    return count


def build_entries() -> list[dict[str, Any]]:
    top_level_ids = sorted(path.stem for path in VECTORS.glob("*.gts"))
    unknown = sorted(set(top_level_ids) - set(TOP_LEVEL_SUBSETS))
    missing = sorted(set(TOP_LEVEL_SUBSETS) - set(top_level_ids))
    if unknown or missing:
        raise ManifestError(
            f"top-level vector metadata drift: unknown={unknown} missing={missing}"
        )
    expected_filesystem = sorted(
        path.name.removesuffix(".expected.json")
        for path in VECTORS.glob("*.expected.json")
    )
    if top_level_ids != expected_filesystem:
        raise ManifestError(
            "top-level expected-json coverage drift: "
            f"vectors={top_level_ids} expected={expected_filesystem}"
        )
    resilience_ids = {
        vector_id
        for vector_id, subsets in TOP_LEVEL_SUBSETS.items()
        if "resilience-negative" in subsets
    }
    if resilience_ids != RESILIENCE_NEGATIVE_TOP_LEVEL:
        raise ManifestError(
            "resilience-negative metadata drift: "
            f"declared={sorted(resilience_ids)} "
            f"expected={sorted(RESILIENCE_NEGATIVE_TOP_LEVEL)}"
        )

    json_paths = sorted(
        path for path in VECTORS.glob("*/*.json") if path.parent.name != "tar"
    )
    unknown_dirs = sorted({path.parent.name for path in json_paths} - set(JSON_SUBCORPUS))
    if unknown_dirs:
        raise ManifestError(f"JSON subcorpus metadata missing for: {unknown_dirs}")

    okf_paths = okf_bundle_dirs()
    okf_names = {path.name for path in okf_paths}
    unknown_okf = sorted(okf_names - set(OKF_BUNDLES))
    missing_okf = sorted(set(OKF_BUNDLES) - okf_names)
    if unknown_okf or missing_okf:
        raise ManifestError(
            f"OKF bundle metadata drift: unknown={unknown_okf} missing={missing_okf}"
        )
    filesystem_okf_expected = sorted(
        path.name.removesuffix(".folded.nq")
        for path in (VECTORS / "okf").glob("*.folded.nq")
    )
    if sorted(okf_names) != filesystem_okf_expected:
        raise ManifestError(
            "OKF folded expectation coverage drift: "
            f"bundles={sorted(okf_names)} expected={filesystem_okf_expected}"
        )

    tar_paths = tar_fixture_paths()
    tar_inputs = {path.name for path in tar_paths}
    expected_tar_inputs = {meta["input"] for meta in TAR_FIXTURES.values()}
    unknown_tar = sorted(tar_inputs - expected_tar_inputs)
    missing_tar = sorted(expected_tar_inputs - tar_inputs)
    if unknown_tar or missing_tar:
        raise ManifestError(
            f"tar fixture metadata drift: unknown={unknown_tar} missing={missing_tar}"
        )
    filesystem_tar_expected = sorted(
        path.name.removesuffix(".expected.json")
        for path in (VECTORS / "tar").glob("*.expected.json")
    )
    if sorted(TAR_FIXTURES) != filesystem_tar_expected:
        raise ManifestError(
            "tar expected-json coverage drift: "
            f"metadata={sorted(TAR_FIXTURES)} expected={filesystem_tar_expected}"
        )

    entries = [top_level_entry(vector_id) for vector_id in top_level_ids]
    entries.extend(json_fixture_entry(path) for path in json_paths)
    entries.extend(okf_fixture_entry(path) for path in okf_paths)
    entries.extend(tar_fixture_entry(path) for path in tar_paths)
    entries.sort(key=lambda item: item["input"]["path"])
    return entries


def entry_in_manifest_scope(entry: dict[str, Any], scope: str) -> bool:
    if scope == "aggregate":
        return True

    subsets = set(entry["subsets"])
    if scope == "core":
        return (
            entry["input"]["media_type"] == GTS_MEDIA_TYPE
            and "profile-layout" not in subsets
            and bool(subsets & CORE_MANIFEST_SUBSETS)
        )
    if scope == "profiles":
        return bool(subsets & PROFILE_MANIFEST_SUBSETS)
    if scope == "transforms":
        return bool(subsets & TRANSFORM_MANIFEST_SUBSETS)
    raise ManifestError(f"unknown manifest scope: {scope}")


def build_manifest(scope: str = "aggregate") -> dict[str, Any]:
    if scope not in MANIFESTS:
        raise ManifestError(f"unknown manifest scope: {scope}")
    entries = [
        entry for entry in build_entries() if entry_in_manifest_scope(entry, scope)
    ]
    if not entries:
        raise ManifestError(f"{scope} manifest would be empty")
    return {
        "schema": SCHEMA,
        "manifest_version": 1,
        "manifest_scope": scope,
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
    subsets = set(entry["subsets"])
    tiers = set(entry["tiers"])
    negative = entry["negative"]
    require(
        not negative or "writer" not in tiers,
        f"{vector_id}: negative vectors must not claim writer tier",
    )
    require(
        not negative or "writer-determinism" not in subsets,
        f"{vector_id}: negative vectors must not claim writer-determinism",
    )
    require(
        "writer" not in tiers or "writer-determinism" in subsets,
        f"{vector_id}: writer tier requires writer-determinism subset",
    )

    input_info = entry.get("input")
    require(isinstance(input_info, dict), f"{vector_id}: input must be an object")
    input_path = input_info.get("path")
    require(isinstance(input_path, str), f"{vector_id}: input.path must be a string")
    require(
        input_info.get("media_type")
        in {
            GTS_MEDIA_TYPE,
            "application/json",
            OKF_MEDIA_TYPE,
            *TAR_MEDIA_TYPES.values(),
        },
        f"{vector_id}: unsupported input media_type",
    )
    media_type = input_info["media_type"]
    resolved_input = repo_path(input_path, "input.path", vector_id)
    if media_type == OKF_MEDIA_TYPE:
        require(
            resolved_input.is_dir(),
            f"{vector_id}: missing input directory {input_path}",
        )
    else:
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
    if "resilience-negative" in subsets:
        rest = input_path.removeprefix("vectors/")
        is_top_level_gts = (
            media_type == GTS_MEDIA_TYPE
            and input_path.startswith("vectors/")
            and "/" not in rest
            and input_path.endswith(".gts")
        )
        require(
            is_top_level_gts,
            f"{vector_id}: resilience-negative entries must be top-level GTS vectors",
        )
        require(
            negative,
            f"{vector_id}: resilience-negative vectors must be negative",
        )
        require(
            vector_id in RESILIENCE_NEGATIVE_TOP_LEVEL,
            f"{vector_id}: unexpected resilience-negative vector",
        )
        if "profile-layout" in subsets:
            require(
                "validating-tool" in tiers,
                f"{vector_id}: profile resilience-negative vectors must exercise validating tools",
            )
        else:
            require(
                {"baseline-reader", "streaming-reader"} <= tiers,
                f"{vector_id}: resilience-negative vectors must exercise reader tiers",
            )
        require(
            {"streaming-property", "corpus-generator-determinism"} <= subsets,
            f"{vector_id}: resilience-negative vectors must use shared top-level gates",
        )
        require(
            graph == f"vectors/{vector_id}.expected.json",
            f"{vector_id}: resilience-negative expected graph must match id",
        )
        require(
            resolved_input.stat().st_size <= RESILIENCE_NEGATIVE_MAX_BYTES,
            f"{vector_id}: resilience-negative vector exceeds bounded size",
        )
        require(
            bool(expected.get("diagnostics"))
            or "exit_code" in expected
            or bool(expected.get("profile_findings")),
            f"{vector_id}: resilience-negative vectors must document diagnostics or refusal",
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


def validate_manifest(
    manifest: Any,
    *,
    require_release_revision: bool = False,
    expected_scope: str | None = None,
) -> None:
    require(isinstance(manifest, dict), "manifest must be a JSON object")
    require(manifest.get("schema") == SCHEMA, "manifest schema mismatch")
    require(manifest.get("manifest_version") == 1, "manifest_version must be 1")
    scope = manifest.get("manifest_scope", "aggregate")
    require(scope in MANIFESTS, "manifest_scope must name a committed manifest scope")
    if expected_scope is not None:
        require(scope == expected_scope, f"manifest_scope must be {expected_scope}")
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

    expected_manifest = build_manifest(scope)
    require(
        manifest.get("generated_by") == expected_manifest["generated_by"],
        f"{scope} manifest generated_by drift",
    )
    manifest_ids = [entry["id"] for entry in vectors]
    expected_ids = [entry["id"] for entry in expected_manifest["vectors"]]
    require(
        manifest_ids == expected_ids,
        f"{scope} manifest vector coverage drift: "
        f"manifest={manifest_ids} generated={expected_ids}",
    )
    expected_entries = {entry["id"]: entry for entry in expected_manifest["vectors"]}
    for entry in vectors:
        require_generated_metadata(entry, expected_entries[entry["id"]])


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


def load_committed_manifest(scope: str) -> dict[str, Any]:
    path = MANIFESTS[scope]
    if not path.is_file():
        raise ManifestError(f"missing {rel(path)}")
    manifest = load_json(path)
    validate_manifest(manifest, expected_scope=scope)
    return manifest


def write_manifest(scope: str) -> None:
    path = MANIFESTS[scope]
    path.write_text(
        json.dumps(build_manifest(scope), indent=2, sort_keys=False) + "\n",
        encoding="utf-8",
    )


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
    negative_top_level = next(
        entry for entry in manifest["vectors"] if entry["id"] == "04-damaged-frame"
    )
    negative_top_level["tiers"].append("writer")
    expect_invalid(manifest, "negative vectors must not claim writer tier")

    manifest = mutated_manifest()
    negative_top_level = next(
        entry for entry in manifest["vectors"] if entry["id"] == "04-damaged-frame"
    )
    negative_top_level["subsets"].append("writer-determinism")
    expect_invalid(manifest, "negative vectors must not claim writer-determinism")

    manifest = mutated_manifest()
    writer_top_level = next(
        entry for entry in manifest["vectors"] if entry["id"] == "29-deterministic-writer"
    )
    writer_top_level["subsets"].remove("writer-determinism")
    expect_invalid(manifest, "writer tier requires writer-determinism subset")

    manifest = mutated_manifest()
    positive_top_level = next(
        entry for entry in manifest["vectors"] if entry["id"] == "01-minimal"
    )
    positive_top_level["subsets"].append("resilience-negative")
    expect_invalid(manifest, "resilience-negative vectors must be negative")

    manifest = mutated_manifest()
    json_negative = next(
        entry for entry in manifest["vectors"] if entry["id"] == "security-profile-policy"
    )
    json_negative["subsets"].append("resilience-negative")
    expect_invalid(
        manifest,
        "resilience-negative entries must be top-level GTS vectors",
    )

    manifest = mutated_manifest()
    resilience_top_level = next(
        entry for entry in manifest["vectors"] if entry["id"] == "04-damaged-frame"
    )
    resilience_top_level["expected"]["diagnostics"] = []
    expect_invalid(
        manifest,
        "resilience-negative vectors must document diagnostics or refusal",
    )

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
    tar_fixture = next(
        entry
        for entry in manifest["vectors"]
        if entry["id"] == "tar-ustar-basic"
    )
    tar_fixture["expected"]["entries"] = 0
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
        help="rewrite committed vector manifests from the current corpus files",
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
            for scope in MANIFESTS:
                write_manifest(scope)

        manifests = {scope: load_committed_manifest(scope) for scope in MANIFESTS}
        manifest = deepcopy(manifests["aggregate"])
        require_release_revision = (
            args.corpus_revision is not None or args.release_manifest is not None
        )
        if args.corpus_revision is not None:
            manifest["corpus_revision"] = normalize_corpus_revision(args.corpus_revision)
        elif args.release_manifest is not None:
            manifest["corpus_revision"] = current_head_revision()

        validate_manifest(
            manifest,
            require_release_revision=require_release_revision,
            expected_scope="aggregate",
        )

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

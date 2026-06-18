#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Guard the deferred multi-recipient crypto contract."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
VECTOR_DIR = ROOT / "vectors" / "crypto-deferred"
DOCS = [
    ROOT / "docs" / "GTS-SPEC.md",
    ROOT / "docs" / "GTS-CONFORMANCE.md",
    ROOT / "docs" / "GTS-SECURITY-POLICY.md",
]

REQUIRED_DOC_MARKERS = [
    "COSE_Encrypt",
    "ECDH-ES+A256KW",
    "A256KW",
    "KeyWrapFailed",
    "Deferred outside v1 conformance",
    "vectors/crypto-deferred/*.json",
]

REQUIRED_VECTORS = {
    "multi-recipient-cose-encrypt.json",
    "wrong-key-opacity.json",
    "missing-key-opacity.json",
    "key-wrap-failure-diagnostics.json",
}


def fail(message: str) -> None:
    print(f"check_crypto_deferrals: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_json(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"{path.relative_to(ROOT)} is not valid JSON: {exc}")
    if not isinstance(data, dict):
        fail(f"{path.relative_to(ROOT)} must be a JSON object")
    return data


def require(condition: bool, message: str) -> None:
    if not condition:
        fail(message)


def require_deferred(path: Path, data: dict[str, Any]) -> None:
    require(data.get("deferred") is True, f"{path.name}: deferred must be true")
    require(
        data.get("status") == "deferred-outside-v1-conformance",
        f"{path.name}: wrong deferred status",
    )
    require(data.get("codec") == "cose-encrypt", f"{path.name}: wrong codec")
    require(data.get("cose_type") == "COSE_Encrypt", f"{path.name}: wrong COSE type")
    diagnostics = data.get("expected_diagnostics", [])
    require(isinstance(diagnostics, list), f"{path.name}: expected_diagnostics list required")


def main() -> int:
    combined_docs = "\n".join(path.read_text(encoding="utf-8") for path in DOCS)
    for marker in REQUIRED_DOC_MARKERS:
        if marker not in combined_docs:
            fail(f"docs missing marker: {marker}")

    if not VECTOR_DIR.is_dir():
        fail(f"missing {VECTOR_DIR.relative_to(ROOT)}")
    names = {path.name for path in VECTOR_DIR.glob("*.json")}
    missing = sorted(REQUIRED_VECTORS - names)
    extra = sorted(names - REQUIRED_VECTORS)
    require(not missing, f"missing deferred vectors: {missing}")
    require(not extra, f"unexpected deferred vectors: {extra}")

    vectors = {path.name: load_json(path) for path in VECTOR_DIR.glob("*.json")}
    for name, data in vectors.items():
        require_deferred(VECTOR_DIR / name, data)

    positive = vectors["multi-recipient-cose-encrypt.json"]
    recipients = positive.get("recipients")
    require(isinstance(recipients, list), "positive vector recipients must be a list")
    require(len(recipients) >= 2, "positive vector must cover at least two recipients")
    require(
        positive.get("key_management", {}).get("alg") == "ECDH-ES+A256KW",
        "positive vector must pin ECDH-ES+A256KW",
    )
    expected = positive.get("expected", {})
    require(isinstance(expected, dict), "positive vector expected must be an object")
    successes = expected.get("unwrap_success_kids")
    require(isinstance(successes, list), "positive vector unwrap_success_kids required")
    require(len(successes) >= 2, "positive vector must expect two successful unwrap kids")

    missing_key = vectors["missing-key-opacity.json"]
    missing_expected = missing_key.get("expected", {})
    require(
        isinstance(missing_expected, dict),
        "missing-key vector expected must be an object",
    )
    require(
        "MissingKey" in missing_key.get("expected_diagnostics", []),
        "missing-key vector must expect MissingKey",
    )
    require(
        missing_expected.get("opaque_reason") == "missing-key",
        "missing-key vector must preserve missing-key opacity",
    )
    require(
        missing_expected.get("plaintext_available") is False,
        "missing-key vector plaintext_available must be false",
    )

    for name in ["wrong-key-opacity.json", "key-wrap-failure-diagnostics.json"]:
        data = vectors[name]
        require(
            "KeyWrapFailed" in data.get("expected_diagnostics", []),
            f"{name}: must expect KeyWrapFailed",
        )
        require(
            data.get("expected", {}).get("opaque_reason") == "missing-key",
            f"{name}: must preserve missing-key opacity",
        )
        require(
            data.get("expected", {}).get("plaintext_available") is False,
            f"{name}: plaintext_available must be false",
        )

    print("check_crypto_deferrals: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

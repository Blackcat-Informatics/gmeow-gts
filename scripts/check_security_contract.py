#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Guard the security/trust policy contract against documentation drift."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs" / "GTS-SECURITY-POLICY.md"
VECTOR_DIR = ROOT / "vectors" / "security"

REQUIRED_MARKERS = [
    "<!-- gts-security-policy:v1 -->",
    "## Trust Separation",
    "Signature.status == \"valid\"",
    "gts.policy.TrustPolicy",
    "## Profile Enforcement",
    "OpaqueRecipientKidPublic",
    "EvidenceHeadCommitmentRequired",
    "## Nested GTS Budgets",
    "gts.read_nested",
    "RecursionLimit",
    "## Crypto Deferrals",
    "COSE_Encrypt multi-recipient envelopes",
    "Deferred outside v1 conformance",
]

REQUIRED_VECTORS = {
    "nested-recursion-limit.json": {
        "id": "nested-recursion-limit",
        "expected": "RecursionLimit",
    },
    "profile-policy.json": {
        "id": "profile-policy",
        "expected": "OpaqueRecipientKidPublic",
    },
}


def fail(message: str) -> None:
    print(f"check_security_contract: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> int:
    if not DOC.is_file():
        fail(f"missing security policy document: {DOC.relative_to(ROOT)}")
    text = DOC.read_text(encoding="utf-8")
    for marker in REQUIRED_MARKERS:
        if marker not in text:
            fail(f"{DOC.relative_to(ROOT)} missing marker: {marker}")
    for filename, required in REQUIRED_VECTORS.items():
        path = VECTOR_DIR / filename
        if not path.is_file():
            fail(f"missing security vector: {path.relative_to(ROOT)}")
        data = json.loads(path.read_text(encoding="utf-8"))
        if data.get("id") != required["id"]:
            fail(f"{path.relative_to(ROOT)} has wrong id")
        serialized = json.dumps(data, sort_keys=True)
        if required["expected"] not in serialized:
            fail(f"{path.relative_to(ROOT)} missing {required['expected']}")
    print("check_security_contract: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

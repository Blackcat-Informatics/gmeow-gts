# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Cross-engine COSE_Sign1 vectors (§9.2): the Python oracle gates against the
same frozen ``vectors/cose/*.json`` that the Rust/Go/TS engines do."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from gts.crypto import InMemoryKeys, Signer, sign_id, verify_sig

COSE_DIR = Path(__file__).resolve().parents[2] / "vectors" / "cose"


@pytest.mark.parametrize("vector", sorted(p.name for p in COSE_DIR.glob("*.json")))
def test_cose_vector_signs_and_verifies(vector: str) -> None:
    case = json.loads((COSE_DIR / vector).read_text(encoding="utf-8"))
    sk = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(case["seed"]))
    kid = case["kid"]
    frame_id = bytes.fromhex(case["frame_id"])

    # Deterministic Ed25519: signing reproduces the frozen bytes.
    assert sign_id(frame_id, Signer(kid, sk)).hex() == case["cose"]

    # The frozen COSE verifies against the (resolved) public key.
    keys = InMemoryKeys(verifiers={kid: sk.public_key()})
    assert verify_sig(bytes.fromhex(case["cose"]), frame_id, keys) == ("valid", kid)

    # No key resolved -> present but unverified.
    assert verify_sig(bytes.fromhex(case["cose"]), frame_id, InMemoryKeys()) == (
        "unverified",
        kid,
    )


def test_cose_vectors_reproducible() -> None:
    """The committed vectors are exactly what the generator emits."""
    import subprocess
    import sys

    gen = Path(__file__).parents[1] / "scripts" / "gen_cose_vectors.py"
    before = {p.name: p.read_text() for p in COSE_DIR.glob("*.json")}
    subprocess.run([sys.executable, str(gen)], check=True, capture_output=True)
    after = {p.name: p.read_text() for p in COSE_DIR.glob("*.json")}
    assert before == after


def test_cli_verify_key(tmp_path, capsys) -> None:
    """`gts verify --key kid:hexpub` verifies a signed file's signatures."""
    import json

    from gts.cli import main

    c = json.loads((COSE_DIR.parent / "signed" / "basic.json").read_text())
    f = tmp_path / "s.gts"
    f.write_bytes(bytes.fromhex(c["gts"]))

    assert main(["verify", "--key", f"{c['kid']}:{c['pub']}", str(f)]) == 0
    out = capsys.readouterr().out
    assert out.count("signature test-kid: valid") == 2

    # A wrong key -> invalid -> exit 1.
    wrong = "0" * 64
    assert main(["verify", "--key", f"{c['kid']}:{wrong}", str(f)]) == 1

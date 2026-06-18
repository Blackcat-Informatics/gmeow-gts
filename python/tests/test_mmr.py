# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

from __future__ import annotations

from pathlib import Path

import pytest

from gts.cli import main
from gts.mmr import proof_from_json, verify_proof, verify_proof_json

VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "proofs"


def _proof(name: str) -> str:
    return (VECTORS_DIR / name).read_text(encoding="utf-8")


def test_positive_proof_fixture_verifies() -> None:
    proof = proof_from_json(_proof("mmr-basic-proof.json"))
    verify_proof(proof)
    assert proof.count == 4
    assert proof.leaf_index == 2


def test_negative_proof_fixture_fails() -> None:
    proof = proof_from_json(_proof("mmr-basic-proof-bad-root.json"))
    with pytest.raises(ValueError, match="root"):
        verify_proof(proof)


def test_verify_proof_json_returns_verified_proof() -> None:
    proof = verify_proof_json(_proof("mmr-basic-proof.json"))
    assert (
        proof.root.hex()
        == "c24901580895f06da6f51ee3d1ec215890c2ecb611a9e5729579bf967717a738"
    )


def test_cli_verify_proof_fixture(capsys: pytest.CaptureFixture[str]) -> None:
    path = VECTORS_DIR / "mmr-basic-proof.json"
    assert main(["verify-proof", str(path)]) == 0
    out = capsys.readouterr().out
    assert "proof ok" in out
    assert "c24901580895f06da6f51ee3d1ec215890c2ecb611a9e5729579bf967717a738" in out


def test_cli_verify_proof_rejects_bad_root(
    capsys: pytest.CaptureFixture[str],
) -> None:
    path = VECTORS_DIR / "mmr-basic-proof-bad-root.json"
    assert main(["verify-proof", str(path)]) == 1
    err = capsys.readouterr().err
    assert "invalid proof" in err


def test_cli_verify_proof_rejects_non_utf8_json(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    path = tmp_path / "proof.json"
    path.write_bytes(b"\xff")

    assert main(["verify-proof", str(path)]) == 1
    err = capsys.readouterr().err
    assert "invalid proof JSON" in err

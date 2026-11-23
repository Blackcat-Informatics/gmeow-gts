# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check P5 range roots and detached proofs against published outcomes."""

import json
from pathlib import Path

import pytest

from gts.v2.range import (
    ProofFinding,
    RangeAccumulator,
    RangeRoot,
    verify_consistency,
    verify_inclusion,
    verify_range,
)

P5 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P5"


def test_range_digest_inputs_must_be_immutable_bytes() -> None:
    with pytest.raises(ValueError):
        RangeAccumulator("a" * 32)
    with pytest.raises(ValueError):
        RangeRoot(1, bytearray(32))


def test_prefix_roots_match_corpus() -> None:
    case = json.loads((P5 / "range-roots-1-17.json").read_text())["cases"][0]
    opener, *ordinary = map(bytes.fromhex, case["identifiers"])
    accumulator = RangeAccumulator(opener)
    assert accumulator.root().digest.hex() == case["result"]["roots"]["1"]

    for count, identifier in enumerate(ordinary, start=2):
        accumulator.append(identifier)
        root = accumulator.root()
        assert root.count == count
        assert root.digest.hex() == case["result"]["roots"][str(count)]


def test_inclusion_proofs_match_corpus() -> None:
    positive = json.loads((P5 / "proof-inclusion-all.json").read_text())
    for case in positive["cases"]:
        root = RangeRoot(case["count"], bytes.fromhex(case["root"]))
        for item in case["proofs"]:
            identifier = bytes.fromhex(case["identifiers"][item["index"]])
            result = verify_inclusion(bytes.fromhex(item["proof"]), root, identifier)
            assert _finding(result) == case["result"]["each"]

    negative = json.loads((P5 / "proof-inclusion-negative.json").read_text())
    for case in negative["cases"]:
        root = RangeRoot(case["count"], bytes.fromhex(case["root"]))
        proof = (P5 / case["proof_file"]).read_bytes()
        result = verify_inclusion(proof, root, bytes.fromhex(case["index_identifier"]))
        assert _finding(result) == case["result"][case["label"]]


def test_consistency_proofs_match_corpus() -> None:
    positive = json.loads((P5 / "proof-consistency-all.json").read_text())
    for case in positive["cases"]:
        new_root = RangeRoot(case["new_count"], bytes.fromhex(case["new_root"]))
        for item in case["proofs"]:
            old_root = RangeRoot(item["old_count"], bytes.fromhex(item["old_root"]))
            result = verify_consistency(
                bytes.fromhex(item["proof"]), old_root, new_root
            )
            assert _finding(result) == case["result"]["each"]

    negative = json.loads((P5 / "proof-consistency-negative.json").read_text())
    for case in negative["cases"]:
        old_root = RangeRoot(case["old_count"], bytes.fromhex(case["old_root"]))
        new_root = RangeRoot(case["new_count"], bytes.fromhex(case["new_root"]))
        proof = (P5 / case["proof_file"]).read_bytes()
        result = verify_consistency(proof, old_root, new_root)
        assert _finding(result) == case["result"][case["label"]]


def test_range_proofs_match_corpus() -> None:
    entry = json.loads((P5 / "proof-range.json").read_text())
    for case in entry["cases"]:
        root = RangeRoot(case["count"], bytes.fromhex(case["root"]))
        proof = (P5 / case["proof_file"]).read_bytes()
        result = verify_range(
            proof, root, list(map(bytes.fromhex, case["identifiers"]))
        )
        assert _finding(result) == case["result"][case["label"]]


def _finding(result: ProofFinding) -> str:
    return f"{result.code}/{result.reason}"

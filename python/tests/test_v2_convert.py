# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise public v2 conversion and its independent result read."""

import json
import re
from dataclasses import replace
from pathlib import Path

import cbor2
import pytest

from gts.v2 import ConversionCapacity, ConversionStop, Converted, convert, read_content
from gts.v2.__main__ import main
from gts.v2._conversion_output import ConversionCandidate, prepare_conversion
from gts.v2.conversion import _ERROR_FINDINGS, _SEPARATE_ERROR_CHECKS

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"


def _conversion_vector() -> tuple[bytes, bytes, bytes]:
    vector = json.loads((_P9 / "convert__signatures.json").read_bytes())
    example = vector["cases"][0]["result"]["conversions"][0]
    return (
        bytes.fromhex(example["source"]["hex"]),
        bytes.fromhex(example["result"]["hex"]),
        bytes.fromhex(example["proof"]),
    )


def test_public_converter_matches_published_result_and_proof() -> None:
    source, expected_result, expected_proof = _conversion_vector()
    inputs = iter((source, source))
    result = convert(lambda: next(inputs))
    assert isinstance(result, Converted)
    assert result.file == expected_result
    assert result.proof.encode() == expected_proof
    assert list(inputs) == []


@pytest.mark.parametrize(
    ("name", "capacity"),
    (
        ("capacity-rebuilt", ConversionCapacity(rebuilt_bytes_per_read=0)),
        (
            "result-frame-above-capacity",
            ConversionCapacity(decoded_bytes_per_frame=3000),
        ),
    ),
)
def test_public_converter_applies_limits_on_independent_result_read(
    name: str, capacity: ConversionCapacity
) -> None:
    source = (_P9 / f"convert__{name}.bin").read_bytes()
    assert convert(lambda: source, capacity) == ConversionStop(
        "source_exceeds_capacity", 0
    )


def test_parity_shard_damage_prevents_conversion_proof(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    source, _, _ = _conversion_vector()
    candidate = prepare_conversion(lambda: source)
    assert isinstance(candidate, ConversionCandidate)
    (segment,) = read_content(candidate.file).wire.segments
    parity = next(frame for frame in segment.frames if frame.frame_type == 16)
    item = segment.items[parity.position]
    envelope = cbor2.loads(candidate.file[item.offset : item.end])
    payload = cbor2.loads(envelope[8])
    shard_bytes = payload[-1]
    payload[-1] = shard_bytes[:-1] + bytes((shard_bytes[-1] ^ 1,))
    envelope[8] = cbor2.dumps(payload)
    encoded = cbor2.dumps(envelope)
    damaged = candidate.file[: item.offset] + encoded + candidate.file[item.end :]
    monkeypatch.setattr(
        "gts.v2._conversion_result.prepare_conversion",
        lambda *_: replace(candidate, file=damaged),
    )
    assert convert(lambda: source) == ConversionStop("content_mismatch", 0)


def test_undelimitable_result_tail_prevents_conversion_proof(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    source, _, _ = _conversion_vector()
    candidate = prepare_conversion(lambda: source)
    assert isinstance(candidate, ConversionCandidate)
    monkeypatch.setattr(
        "gts.v2._conversion_result.prepare_conversion",
        lambda *_: replace(candidate, file=candidate.file + b"\x1c"),
    )
    assert convert(lambda: source) == ConversionStop("content_mismatch", 0)


def test_convert_command_writes_both_outputs_and_preserves_existing_paths(
    tmp_path: Path,
) -> None:
    source, expected_result, expected_proof = _conversion_vector()
    original = tmp_path / "source.gts"
    output = tmp_path / "result.gts"
    proof = tmp_path / "proof.cbor"
    original.write_bytes(source)
    assert (
        main(["convert", str(original), "-o", str(output), "--proof", str(proof)]) == 0
    )
    assert output.read_bytes() == expected_result
    assert proof.read_bytes() == expected_proof
    assert original.read_bytes() == source
    extra_proof = tmp_path / "extra.cbor"
    assert (
        main(["convert", str(original), "-o", str(output), "--proof", str(extra_proof)])
        == 2
    )
    assert output.read_bytes() == expected_result
    assert not extra_proof.exists()


def test_error_severity_codes_are_admitted_or_checked_separately() -> None:
    """Every error-severity code of the P2 table is in the admission set or checked."""
    table = (_P9.parents[2] / "docs" / "v2" / "P2.md").read_text(encoding="utf-8")
    section = table.split("### 5.1 Codes, severities")[1].split("\n\n")[1]
    rows = re.findall(r"^\| \d+ \| `(\w+)` \| (\w+) \|", section, re.MULTILINE)
    errors = {code for code, severity in rows if severity == "error"}
    assert len(rows) == 48
    covered = _ERROR_FINDINGS | _SEPARATE_ERROR_CHECKS.keys()
    assert errors <= covered, sorted(errors - covered)
    assert covered <= errors, sorted(covered - errors)
    assert not _ERROR_FINDINGS & _SEPARATE_ERROR_CHECKS.keys()

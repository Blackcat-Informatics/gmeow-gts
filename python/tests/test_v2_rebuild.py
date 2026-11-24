# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check recipe decoding, verified units and ordered rebuild charges against P4."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2.rebuild import (
    Entry,
    EntryListError,
    IncompleteRebuild,
    RebuildBudget,
    RefusedRebuild,
    UncheckedRebuild,
    UnitIndex,
    VerifiedRebuild,
    decode_entries,
    unit_digest,
)

_P4 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P4"
_MALFORMED = {
    3: "not_deterministic",
    4: "wrong_type",
    5: "wrong_length",
    6: "out_of_range",
    9: "inconsistent_counts",
}
_INCOMPLETE = {14: "blob_length", 15: "blob_digest", 24: "chunk"}


@pytest.mark.parametrize(
    "name",
    [
        "recipe-one-entry",
        "recipe-digest-length",
        "recipe-list-length",
        "recipe-order",
        "recipe-zero-entry-length",
        "recipe-sum",
        "recipe-sum-overflow",
        "rebuild-blob-digest",
        "rebuild-blob-length",
        "rebuild-capacity",
        "rebuild-charge-missing-unit",
        "rebuild-charge-repeated",
        "rebuild-charge-segments",
        "rebuild-charge-then-small",
        "rebuild-first-verified",
        "rebuild-missing-damaged",
        "rebuild-other-segment",
        "rebuild-unit-after-value",
        "rebuild-unaffected",
    ],
)
def test_recipe_and_value_cases(name: str) -> None:
    for case in json.loads((_P4 / f"{name}.json").read_text())["cases"]:
        data = (_P4 / case["input_file"]).read_bytes()
        expected_read = cbor2.loads(bytes.fromhex(case["read_result"]))
        segments = expected_read[-1][1]
        stored, decoded, _, _, read_decoded, _, rebuilt, _, _, access = expected_read[3]
        budget = RebuildBudget(rebuilt, enabled=access == 1)
        payloads = PayloadBudget(
            stored_bytes_per_frame=stored,
            decoded_bytes_per_frame=decoded,
            decoded_bytes_per_read=read_decoded,
        )
        for segment in segments:
            units = UnitIndex()
            for record in segment[-2]:
                position, offset, _, frame_kind, link, payload_state, *_ = record
                if frame_kind[0] != 2 or link == 3 or payload_state[0] != 1:
                    continue
                kind = frame_kind[1]
                assert kind != 13
                if kind not in (8, 9, 10, 11):
                    continue
                frame = decode_frame(data, offset)
                payload = payloads.check(frame).decoded
                assert payload is not None
                fields = cbor2.loads(frame.content)
                digest = fields[0]
                findings = record[-1]
                if kind in (8, 9):
                    verified = units.add(position, digest, payload)
                    assert verified == (
                        not any(f[:2] == [13, [2, 12]] for f in findings)
                    )
                    continue
                declaration_kind = "recipe" if kind == 10 else "value"
                invalid = next((f[1][1] for f in findings if f[0] == 7), None)
                if invalid is not None:
                    with pytest.raises(EntryListError) as error:
                        decode_entries(
                            bytes(payload),
                            kind=declaration_kind,
                            entry_count=fields[-1],
                            total_length=fields[1],
                        )
                    assert error.value.reason == _MALFORMED[invalid]
                    continue
                entries = decode_entries(
                    bytes(payload),
                    kind=declaration_kind,
                    entry_count=fields[-1],
                    total_length=fields[1],
                )
                result = budget.rebuild(
                    units,
                    position=position,
                    digest=digest,
                    total_length=fields[1],
                    entries=entries,
                )
                _, expected = record[-2][0]
                match expected:
                    case [1]:
                        assert isinstance(result, VerifiedRebuild)
                        assert len(result.to_bytes()) == fields[1]
                        assert unit_digest(result.to_bytes()) == digest
                    case [2, reason]:
                        assert isinstance(result, IncompleteRebuild)
                        assert result.reason == _INCOMPLETE[reason]
                    case [3]:
                        count = next(
                            f[-1][1] for f in findings if f[:2] == [20, [2, 89]]
                        )
                        assert result == RefusedRebuild(count)
                    case [4]:
                        assert result == UncheckedRebuild()
                    case _:
                        pytest.fail(f"unrecognized rebuild outcome {expected}")


@pytest.mark.parametrize("raw", [b"unit", bytearray(b"unit")])
def test_shared_pack_position_and_owned_octets(raw: bytes | bytearray) -> None:
    digest = unit_digest(bytes(raw))
    view = memoryview(raw).toreadonly()
    units = UnitIndex()
    assert units.add(3, digest, view)
    if isinstance(raw, bytearray):
        raw[0] = ord("x")
    view.release()
    entries = (Entry(digest, 4),)
    budget = RebuildBudget(8)
    assert budget.rebuild(
        units,
        position=2,
        digest=digest,
        total_length=4,
        entries=entries,
    ) == IncompleteRebuild("chunk")
    same_pack = budget.rebuild(
        units,
        position=3,
        digest=digest,
        total_length=4,
        entries=entries,
    )
    assert isinstance(same_pack, VerifiedRebuild)
    assert same_pack.to_bytes() == b"unit"


def test_invalid_charge_cannot_credit_the_budget() -> None:
    units = UnitIndex()
    budget = RebuildBudget(0)
    with pytest.raises(ValueError, match="charge is outside uint64"):
        budget.rebuild(units, position=1, digest=b"", total_length=-1, entries=())
    result = budget.rebuild(
        units, position=1, digest=b"", total_length=1, entries=(Entry(b"", 1),)
    )
    assert result == RefusedRebuild(1)

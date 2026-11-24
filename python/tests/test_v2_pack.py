# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check directory and member decoding against the P4 corpus."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2._content import UnknownContentCode
from gts.v2.pack import MemberKind, PackError, decode_pack
from gts.v2.rebuild import (
    RebuildBudget,
    RefusedRebuild,
    UnitIndex,
    VerifiedRebuild,
    unit_digest,
)

_P4 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P4"
_MALFORMED = {
    5: "wrong_length",
    6: "out_of_range",
    7: "unregistered_value",
    9: "inconsistent_counts",
}


@pytest.mark.parametrize(
    "manifest",
    [*sorted(_P4.glob("pack-*.json")), _P4 / "rebuild-charge-order.json"],
    ids=lambda p: p.stem,
)
def test_pack_corpus(manifest: Path) -> None:
    for case in json.loads(manifest.read_text())["cases"]:
        data = (_P4 / case["input_file"]).read_bytes()
        expected = cbor2.loads(bytes.fromhex(case["read_result"]))
        stored, decoded, _, _, read_decoded, _, rebuilt, _, _, access = expected[3]
        payloads = PayloadBudget(
            stored_bytes_per_frame=stored,
            decoded_bytes_per_frame=decoded,
            decoded_bytes_per_read=read_decoded,
        )
        rebuilds = RebuildBudget(rebuilt, enabled=access == 1)
        for segment in expected[-1][1]:
            units = UnitIndex()
            for record in segment[-2]:
                position, offset, _, frame_type, _, payload_state, *_ = record
                if frame_type not in ([2, 8], [2, 9], [2, 13]) or payload_state[0] != 1:
                    continue
                frame = decode_frame(data, offset)
                payload = payloads.check(frame).decoded
                assert payload is not None
                fields = cbor2.loads(frame.content)
                if frame.frame_type in (8, 9):
                    assert units.add(position, fields[0], payload)
                    continue
                invalid = next((f[1][1] for f in record[-1] if f[0] == 7), None)
                count, unit_length = fields
                if invalid is not None:
                    with pytest.raises(PackError) as error:
                        decode_pack(
                            payload, member_count=count, unit_length=unit_length
                        )
                    assert error.value.reason == _MALFORMED[invalid]
                    continue
                members = decode_pack(
                    payload, member_count=count, unit_length=unit_length
                )
                assert len(members) == count
                failures = sum(
                    not units.add(position, member.digest, member.body)
                    for member in members
                    if member.kind <= MemberKind.BLOB
                )
                assert failures == sum(f[:2] == [13, [2, 12]] for f in record[-1])
                for member_index, state in record[-2]:
                    member = members[member_index[1]]
                    result = rebuilds.rebuild(
                        units,
                        position=position,
                        digest=member.digest,
                        total_length=member.total_length,
                        entries=member.entries,
                    )
                    if state == [3]:
                        count = next(
                            f[-1][1] for f in record[-1] if f[:2] == [20, [2, 89]]
                        )
                        assert result == RefusedRebuild(count)
                    else:
                        assert state == [1]
                        assert isinstance(result, VerifiedRebuild)
                        assert len(result.to_bytes()) == member.total_length


def test_new_registry_distinguishes_unknown_member_tag() -> None:
    payload = cbor2.dumps([[[5, [], 0]], b""])
    with pytest.raises(PackError) as known:
        decode_pack(payload, member_count=1, unit_length=0)
    assert known.value.reason == "unregistered_value"
    with pytest.raises(UnknownContentCode) as newer:
        decode_pack(payload, member_count=1, unit_length=0, registry_version=2)
    assert newer.value.code == 5


def test_media_option_tag_is_fixed_across_registries() -> None:
    payload = cbor2.dumps([[[3, [bytes(32), 2, [4], 2], 0]], b""])
    for registry in (1, 2):
        with pytest.raises(PackError) as error:
            decode_pack(
                payload, member_count=1, unit_length=0, registry_version=registry
            )
        assert error.value.reason == "unregistered_value"


def test_pack_accepts_signed_view_and_owns_mutable_octets() -> None:
    body = b"unit"
    payload = cbor2.dumps([[[1, [unit_digest(body)], len(body)]], body])
    mutable = bytearray(payload)
    for view in (memoryview(payload).cast("b"), memoryview(mutable)):
        (member,) = decode_pack(view, member_count=1, unit_length=len(body))
        view.release()
        assert member.body == body
    mutable[-1] = 0
    assert member.body == body

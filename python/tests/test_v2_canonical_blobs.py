# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the canonical P4 cut rule and value/blob frames."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2.blobs import BlobEntry, BlobSet
from gts.v2.canonical_blobs import canonical_binary_frames
from gts.v2.chunker import chunks
from gts.v2.pack import MemberKind, decode_pack
from gts.v2.rebuild import unit_digest

_VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P4"


@pytest.mark.parametrize(
    "test", ["cut-min", "cut-strict", "cut-loose", "cut-max", "restart"]
)
def test_chunk_boundaries_match_the_corpus(test: str) -> None:
    cases = json.loads((_VECTORS / f"chunker-{test}.json").read_text())["cases"]
    for case in cases:
        data = (_VECTORS / case["input_file"]).read_bytes()
        assert [len(chunk) for chunk in chunks(data)] == case["result"]["lengths"]


def test_blob_units_can_declare_a_later_one_unit_blob() -> None:
    prefix = bytes(1 << 18)
    data = prefix + b"x"
    whole = (1, unit_digest(data))
    unit = (1, unit_digest(prefix))
    empty = (1, unit_digest(b""))
    external = (1, unit_digest(b"elsewhere"))
    assert whole[1] < unit[1]
    blobs = BlobSet(
        (
            BlobEntry(unit, len(prefix), ("aa", "z"), "complete"),
            BlobEntry(whole, len(data), (), "complete"),
            BlobEntry(empty, 0, (), "complete"),
            BlobEntry(external, 9, ("type/else",), "external"),
        ),
        (),
    )
    values, bodies, *extras = canonical_binary_frames(
        (data, data), blobs, {whole: data, unit: prefix, empty: b""}
    )
    kind, [count, length], payload = values
    assert kind == 13
    value_members = decode_pack(payload, member_count=count, unit_length=length)
    assert [member.kind for member in value_members] == [
        MemberKind.CHUNK,
        MemberKind.CHUNK,
        MemberKind.VALUE,
    ]
    assert value_members[0].digest == unit[1]
    assert value_members[-1].digest == whole[1]
    kind, [count, length], payload = bodies
    assert kind == 13
    body_members = decode_pack(payload, member_count=count, unit_length=length)
    (shared,) = [member for member in body_members if member.digest == unit[1]]
    assert shared.kind == MemberKind.BLOB
    assert bytes(shared.body) == prefix
    assert shared.media_type == "z"
    (recipe,) = [member for member in body_members if member.kind == MemberKind.RECIPE]
    assert recipe.digest == whole[1]
    assert body_members.index(shared) < body_members.index(recipe)
    (empty_member,) = [member for member in body_members if member.digest == empty[1]]
    assert empty_member.kind == MemberKind.BLOB and not empty_member.body
    expected = [(unit, len(prefix), "aa"), (external, 9, "type/else")]
    expected.sort(key=lambda item: cbor2.dumps(item[0]))
    assert extras == [
        (12, [name, size, (2, media)], b"") for name, size, media in expected
    ]


def test_present_blob_identity_is_checked_before_emitting_frames() -> None:
    name = (1, unit_digest(b"original bytes"))
    blobs = BlobSet((BlobEntry(name, 14, (), "complete"),), ())
    with pytest.raises(ValueError, match="length and digest"):
        next(
            canonical_binary_frames((b"first value",), blobs, {name: b"distinct data!"})
        )

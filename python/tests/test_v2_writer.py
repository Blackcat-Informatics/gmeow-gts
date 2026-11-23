# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Reproduce committed and indexed v2 segments from the published corpus."""

from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame, decode_header
from gts.v2.index_claims import IndexStatistics
from gts.v2.items import classify_item
from gts.v2.rebuild import unit_digest
from gts.v2.rows import AnnotationRow, EmptyGraphRow, QuadRow, encode_rows
from gts.v2.writer import SegmentWriter

P5 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P5"
P7 = P5.parent / "P7"


@pytest.mark.parametrize(
    ("name", "closing"), [("commit-valid", False), ("index-intact", True)]
)
def test_segment_writer_reproduces_golden_bytes(name: str, closing: bool) -> None:
    data = (P5 / f"{name}.bin").read_bytes()
    opener = decode_header(data)
    writer = SegmentWriter(opener.header)
    offset = opener.next_offset
    while offset < len(data) and classify_item(data, offset).kind == "frame":
        frame = decode_frame(data, offset)
        assert frame.storage is None and not frame.signatures
        if frame.frame_type == 14:
            identifier = writer.commit()
        else:
            identifier = writer.append(
                frame.frame_type,
                cbor2.loads(frame.content),
                bytes(frame.stored_payload),
            )
        assert identifier == frame.identifier
        if not frame.transparent:
            assert writer.head == identifier
        offset = frame.next_offset
    assert writer.to_bytes(offsets=closing, locator=closing) == data


def test_locator_without_index_cannot_be_written() -> None:
    with pytest.raises(ValueError, match="locator requires an index"):
        SegmentWriter().to_bytes(locator=True)


def test_writer_reproduces_canonical_construction() -> None:
    data = (P7 / "canonical__construction.bin").read_bytes()
    opener = decode_header(data)
    writer = SegmentWriter(opener.header, collect_index=True, compress=True)
    budget = PayloadBudget(
        stored_bytes_per_frame=len(data),
        decoded_bytes_per_frame=len(data),
        decoded_bytes_per_read=len(data),
    )
    offset = opener.next_offset
    while True:
        frame = decode_frame(data, offset)
        decoded = budget.check(frame).decoded
        assert decoded is not None
        if frame.frame_type == 15:
            break
        if frame.frame_type == 14:
            identifier = writer.commit()
        else:
            identifier = writer.append(
                frame.frame_type, cbor2.loads(frame.content), bytes(decoded)
            )
        assert identifier == frame.identifier
        offset = frame.next_offset

    digest = bytes.fromhex(
        "0f3a76a6062fb6d4dfd92ac4b3d432fe51f8cd98e3216815e706cfdc3fb2ebb8"
    )
    assert (
        writer.append_index(IndexStatistics(3, 2, 1), content_digest=digest)
        == frame.identifier
    )
    assert writer.to_bytes(parity=True, offsets=True, locator=True) == data


@pytest.mark.parametrize(
    ("label", "compressed"),
    [
        ("zeros-63", False),
        ("zeros-64", True),
        ("incompressible-64", False),
        ("saves-16", False),
        ("saves-17", True),
    ],
)
def test_storage_choice_keeps_identity(label: str, compressed: bool) -> None:
    stem = f"canonical__constants__storage-choice.{label}"
    payload = (P7 / f"{stem}.decoded.bin").read_bytes()
    expected = (P7 / f"{stem}.stored.bin").read_bytes()
    writer = SegmentWriter(compress=True)
    offset = len(writer.to_bytes())
    content = [unit_digest(payload)]
    identifier = writer.append(8, content, payload)
    assert SegmentWriter().append(8, content, payload) == identifier

    frame = decode_frame(writer.to_bytes(), offset)
    assert (frame.storage is not None) == compressed
    assert frame.stored_payload == expected
    budget = PayloadBudget(
        stored_bytes_per_frame=128,
        decoded_bytes_per_frame=128,
        decoded_bytes_per_read=128,
    )
    checked = budget.check(frame)
    assert checked.stored_matches is (True if compressed else None)
    assert checked.decoded == payload


def test_index_extends_graph_ranges_across_frames_of_the_same_type() -> None:
    writer = SegmentWriter(collect_index=True)
    writer.append(*encode_rows([QuadRow(1, 4, 2), QuadRow(1, 5, 2, 7)]))
    writer.append(*encode_rows([AnnotationRow(8, 2, 4, 1, 7)]))
    writer.append(*encode_rows([QuadRow(2, 4, 1, 7), QuadRow(2, 5, 1)]))
    writer.append(*encode_rows([EmptyGraphRow(3)]))
    writer.commit()
    prefix = writer.to_bytes()
    writer.append_index(IndexStatistics(6, 0, 0), content_digest=None)

    index = decode_frame(writer.to_bytes(), len(prefix))
    assert cbor2.loads(index.content)[0] == 5
    runs, graphs, counts, _, digest, _ = cbor2.loads(index.stored_payload)
    assert runs == [[2, 1], [4, 1], [2, 1], [5, 1], [14, 1]]
    assert graphs == [
        [[1], [[2, 0, 0, 1], [2, 2, 1, 1]]],
        [[2, 3], [[5, 3, 0, 1]]],
        [[2, 7], [[2, 0, 1, 2], [4, 1, 0, 1]]],
    ]
    assert counts == [6, 0, 0, [[4, 2], [5, 2]]]
    assert digest == [1]

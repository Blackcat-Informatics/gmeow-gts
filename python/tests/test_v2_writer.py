# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Reproduce committed and indexed v2 segments from the published corpus."""

from pathlib import Path

import cbor2
import pytest

from gts.v2 import decode_frame, decode_header
from gts.v2.items import classify_item
from gts.v2.writer import SegmentWriter

P5 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P5"


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

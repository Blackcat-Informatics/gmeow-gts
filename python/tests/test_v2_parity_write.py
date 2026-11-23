# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Reproduce P5 parity bytes and their placement over stored frames."""

from pathlib import Path

import cbor2
import pytest
import zstandard

from gts.v2 import decode_frame, decode_header
from gts.v2.index_claims import IndexStatistics
from gts.v2.items import classify_item
from gts.v2.rebuild import unit_digest
from gts.v2.writer import SegmentWriter

P5 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P5"


@pytest.mark.parametrize(
    ("name", "accelerators"),
    [
        ("parity-placement.crossing.bin.zst", False),
        ("parity-span-4096.bin", True),
        ("tail-none.canonical-with-parity.bin", False),
    ],
)
def test_writer_places_exact_parity_frames(name: str, accelerators: bool) -> None:
    path = P5 / name
    if path.suffix == ".zst":
        with (
            path.open("rb") as archive,
            zstandard.ZstdDecompressor().stream_reader(archive) as decoded,
        ):
            data = decoded.read()
    else:
        data = path.read_bytes()

    opener = decode_header(data)
    writer = SegmentWriter(opener.header)
    offset = opener.next_offset
    while offset < len(data) and classify_item(data, offset).kind == "frame":
        frame = decode_frame(data, offset)
        assert frame.storage is None and not frame.signatures
        if not frame.transparent:
            identifier = (
                writer.commit()
                if frame.frame_type == 14
                else writer.append(
                    frame.frame_type,
                    cbor2.loads(frame.content),
                    bytes(frame.stored_payload),
                )
            )
            assert identifier == frame.identifier
        offset = frame.next_offset

    assert (
        writer.to_bytes(parity=True, offsets=accelerators, locator=accelerators) == data
    )


def test_short_span_keeps_later_tail_unprotected() -> None:
    writer = SegmentWriter(collect_index=True)
    writer.commit()
    writer.append_index(IndexStatistics(0, 0, 0), content_digest=None)
    protected = writer.to_bytes(parity=True)
    payload = b"x" * (1 << 22)
    identifier = writer.append(8, [unit_digest(payload)], payload)
    extended = writer.to_bytes(parity=True)

    assert extended.startswith(protected)
    tail = decode_frame(extended, len(protected))
    assert tail.identifier == identifier
    assert tail.next_offset == len(extended)

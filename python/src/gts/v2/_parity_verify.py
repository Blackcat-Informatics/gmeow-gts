# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check P5 shard digests in a structurally verified canonical segment."""

from bisect import bisect_right

import cbor2

from ._decode import WireError, _Decoder
from .frame import _PARITY, Frame, _frame_field_offsets
from .parity import SPAN_LENGTH, _layout, _read_content, _valid_table
from .parity_write import _shard_digest
from .reader import WireSegment


def parity_mismatch(data: bytes, segment: WireSegment) -> bool:
    """Find a shard mismatch using the stored bytes and the checked table.

    The caller supplies a segment with intact ordinary frames and verified
    canonical structure. Treat damaged parity metadata and stale placement
    as unusable, as P5's warning cases require.
    """
    opener = segment.opener.value
    assert not isinstance(opener, WireError) and opener.identifier is not None
    root = opener.identifier
    head = root
    view = memoryview(data)
    header = view[segment.opener.item.offset : segment.opener.item.end]
    parts = [header]
    starts = [0]
    end_of_stream = len(header)
    seen = set()
    short_seen = False
    for read in segment.frames:
        if read.frame_type != _PARITY:
            item = segment.items[read.position]
            starts.append(end_of_stream)
            part = view[item.offset : item.end]
            parts.append(part)
            end_of_stream += len(part)
            identifier = read.link.effective_identifier
            assert identifier is not None
            head = identifier
            continue
        frame = read.envelope
        if read.link.state != "linked":
            continue
        if isinstance(frame, Frame):
            content = frame.content
        else:
            fields = _frame_field_offsets(data, segment.items[read.position].offset)
            content = view[fields[3] : fields[4]]
        try:
            geometry, first, end, anchor, table = _read_content(
                _Decoder(content, 0, WireError)
            )
        except WireError:
            continue
        span = end - first
        repeated = short_seen or first in seen
        seen.add(first)
        short = 1 <= span < SPAN_LENGTH
        short_seen |= short
        if (
            repeated
            or not isinstance(frame, Frame)
            or geometry != 1
            or first % SPAN_LENGTH
            or not 1 <= span <= SPAN_LENGTH
            or anchor != head
            or read.payload.state == "unchecked"
            or not starts[-1] < end <= end_of_stream
            or short
            and end != end_of_stream
        ):
            continue
        layout = _layout(span)
        payload = frame.stored_payload
        if not _valid_table(payload, 0, len(payload), layout, root, table):
            continue
        data_digests, parity_digests, parity = cbor2.loads(payload)
        first_part = bisect_right(starts, first) - 1
        last_part = bisect_right(starts, end - 1)
        covered = bytearray()
        for position in range(first_part, last_part):
            start = starts[position]
            covered.extend(parts[position][max(first - start, 0) : end - start])
        size = layout.shard_length
        if any(
            _shard_digest(covered[offset : offset + size].ljust(size, b"\x00"))
            != digest
            for offset, digest in zip(range(0, span, size), data_digests, strict=True)
        ) or any(
            _shard_digest(parity[offset : offset + size]) != digest
            for offset, digest in zip(
                range(0, len(parity), size), parity_digests, strict=True
            )
        ):
            return True
    return False

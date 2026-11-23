# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Write a header-led GTS v2 segment with its links and closing items."""

from .frame import _COMMIT, _PARITY, _encode_plain_frame
from .header import Header, decode_header, encode_header
from .index_claims import IndexStatistics, _IndexClaims
from .items import encode_locator, encode_offsets
from .range import RangeAccumulator

_INDEX = 15


class SegmentWriter:
    """Assemble a new segment of unsigned, uncompressed v2 frames.

    The writer derives wire links, commit counts and roots, and byte offsets.
    The caller supplies the records and payloads defined by each frame type.
    """

    def __init__(
        self, header: Header | None = None, *, collect_index: bool = False
    ) -> None:
        encoded = encode_header(Header() if header is None else header)
        identifier = decode_header(encoded).identifier
        if identifier is None:
            raise ValueError("segment writer requires a supported hash algorithm")
        self._items = [encoded]
        self._length = len(encoded)
        self._head = identifier
        self._range = RangeAccumulator(identifier)
        self._last_index: int | None = None
        self._index_claims = _IndexClaims() if collect_index else None

    @property
    def head(self) -> bytes:
        """The segment head after the frames written so far."""
        return self._head

    def _append(self, frame_type: int, content: list[object], payload: bytes) -> bytes:
        encoded, identifier = _encode_plain_frame(
            self._head, frame_type, content, payload
        )
        if frame_type != _PARITY:
            if self._index_claims is not None:
                self._index_claims.add(frame_type, content, payload)
            self._range.append(identifier)
            self._head = identifier
        if frame_type == _INDEX:
            self._last_index = self._length
        self._items.append(encoded)
        self._length += len(encoded)
        return identifier

    def append(self, frame_type: int, content: list[object], payload: bytes) -> bytes:
        """Append an envelope using this segment's predecessor and return its ID."""
        if frame_type == _COMMIT:
            raise ValueError("use commit to append a commit frame")
        return self._append(frame_type, content, payload)

    def commit(self) -> bytes:
        """Append a commit over the opener and all preceding ordinary frames."""
        root = self._range.root()
        return self._append(_COMMIT, [root.count - 1, root.digest], b"")

    def append_index(
        self, statistics: IndexStatistics, *, content_digest: bytes | None
    ) -> bytes:
        """Append a P5 index for this prefix with statistics from its folded content."""
        if self._index_claims is None:
            raise ValueError("set collect_index when constructing the segment writer")
        root = self._range.root()
        payload = self._index_claims.payload(statistics, root.digest, content_digest)
        return self._append(_INDEX, [root.count - 1, self._head], payload)

    def to_bytes(self, *, offsets: bool = False, locator: bool = False) -> bytes:
        """Materialize the segment and any requested closing records."""
        if locator and self._last_index is None:
            raise ValueError("a locator requires an index frame")
        closing = []
        if offsets:
            closing.append(encode_offsets([len(item) for item in self._items]))
        if locator:
            assert self._last_index is not None
            position = self._length + sum(map(len, closing))
            closing.append(encode_locator(position - self._last_index))
        return b"".join([*self._items, *closing])

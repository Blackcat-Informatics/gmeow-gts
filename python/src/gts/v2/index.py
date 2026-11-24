# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Validate the grammar of a decoded GTS v2 index (P5 §§4.1, 4.3)."""

from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Literal

from ._content import ContentError, UnknownContentCode, content_code
from ._decode import _Decoder
from .frame import _LAST_FRAME_TYPE, Frame
from .range import RangeAccumulator
from .reader import WireFrame
from .rows import _graph


@dataclass(frozen=True, slots=True)
class IndexPayload:
    type_runs: tuple[tuple[int, int], ...]
    range_root: bytes
    content_digest: bytes | None


def check_index_payload(
    payload: bytes | memoryview, *, registry_version: int = 1, after_stop: bool = False
) -> IndexPayload:
    """Check the index payload grammar and retain its prefix and digest claims."""
    reader = _Decoder(payload, 0, ContentError)

    def frame_type() -> int:
        if after_stop and registry_version > 1:
            return reader.uint(minimum=1, reason="unregistered_value")
        return content_code(reader, _LAST_FRAME_TYPE, registry_version)

    reader.record(6)
    runs = []
    for _ in range(reader.array()):
        reader.record(2)
        runs.append((frame_type(), reader.uint()))

    for _ in range(reader.array()):
        reader.record(2)
        graph_at = reader.offset
        try:
            _graph(reader, registry_version)
        except UnknownContentCode:
            if not after_stop:
                raise
            reader.offset = graph_at
            fields = reader.array()
            reader.uint()
            for _ in range(fields - 1):
                reader.closed_item()
        for _ in range(reader.array()):
            reader.record(4)
            frame_type()
            reader.uint()
            reader.uint()
            reader.uint()

    reader.record(4)
    reader.uint()
    reader.uint()
    reader.uint()
    for _ in range(reader.array()):
        reader.record(2)
        reader.uint()
        reader.uint()

    root = bytes(reader.bytes(32))
    digest = reader.option(lambda: bytes(reader.bytes(32)))
    for _ in range(reader.array()):
        reader.record(3)
        reader.bytes(32)
        reader.uint()
        reader.uint()
    if reader.offset != len(payload):
        raise ContentError("wrong_length", reader.offset)
    return IndexPayload(tuple(runs), root, digest)


type PrefixReason = Literal["index_head", "index_root", "index_directory"]


def check_prefix_claims(
    opener: bytes, frames: Iterable[WireFrame], indexes: Mapping[int, IndexPayload]
) -> Iterable[tuple[int, PrefixReason]]:
    """Compare the count, head, root and type runs over the observed prefix."""
    if not indexes:
        return
    history = RangeAccumulator(opener)
    head = opener
    types: list[tuple[int | None, int]] = []
    for frame in frames:
        position = frame.position
        if position in indexes:
            payload = indexes[position]
            envelope = frame.envelope
            assert isinstance(envelope, Frame)
            content = _Decoder(envelope.content, 0, ContentError)
            content.record(2)
            count, claimed_head = content.uint(), content.bytes(32)
            root = history.root()
            if count != root.count - 1 or claimed_head != head:
                yield position, "index_head"
            if payload.range_root != root.digest:
                yield position, "index_root"
            if types != list(payload.type_runs):
                yield position, "index_directory"
        if (identifier := frame.link.effective_identifier) is not None:
            head = identifier
            history.append(identifier)
            kind = frame.frame_type
            if types and types[-1][0] == kind:
                types[-1] = kind, types[-1][1] + 1
            else:
                types.append((kind, 1))

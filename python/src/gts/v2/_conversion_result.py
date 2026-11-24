# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Admit the independent P9 result read and produce the conversion proof."""

from collections.abc import Callable
from dataclasses import dataclass
from typing import Self

import cbor2

from ._content import UINT_MAX
from ._conversion_output import (
    _DEFAULT_CAPACITY,
    ConversionCapacity,
    prepare_conversion,
)
from ._decode import WireError, _Decoder
from .conversion_fold import ConversionStop
from .frame import decode_frame
from .items import classify_item


@dataclass(frozen=True, slots=True, kw_only=True)
class ResultSegment:
    """Segment observations needed by P9 step 4."""

    canonical_claimed: bool
    structure_verified: bool
    labelling_verified: bool
    intact: bool
    content_digest: bytes | None


@dataclass(frozen=True, slots=True, kw_only=True)
class ResultRead:
    """P9's projection of the whole read of the candidate bytes.

    Segments is absent on first-generation dispatch. Capacity refusal covers
    every frame payload, formation and rebuild refusal, the item ceiling and
    labelling refusals named by P9 step 4(a). Error or fatal findings include
    the file, its segments and their frames.
    """

    segments: tuple[ResultSegment, ...] | None
    content_digest: bytes | None
    visible_complete: bool
    capacity_refused: bool
    error_or_fatal: bool

    def admitted_digest(self) -> bytes | None:
        """Return the digest only when P9 step 4(b) holds."""
        if (
            self.segments is None
            or len(self.segments) != 1
            or self.error_or_fatal
            or not self.visible_complete
            or self.content_digest is None
        ):
            return None
        (segment,) = self.segments
        if (
            segment.canonical_claimed
            and segment.structure_verified
            and segment.labelling_verified
            and segment.intact
            and segment.content_digest == self.content_digest
        ):
            return self.content_digest
        return None


@dataclass(frozen=True, slots=True)
class ConversionProof:
    """The four deterministic fields of the sidecar proof of P9 §7."""

    source_bytes: bytes
    content: bytes
    source_segments: int
    source_signatures: int

    def __post_init__(self) -> None:
        if any(
            not isinstance(digest, bytes) or len(digest) != 32
            for digest in (self.source_bytes, self.content)
        ):
            raise ValueError("conversion proof digests must be 32-byte strings")
        if any(
            type(count) is not int or not 0 <= count <= UINT_MAX
            for count in (self.source_segments, self.source_signatures)
        ):
            raise ValueError("conversion proof counts must be uint64")

    def encode(self) -> bytes:
        """Write the deterministic conversion_proof record."""
        return cbor2.dumps(
            [
                self.source_bytes,
                self.content,
                self.source_segments,
                self.source_signatures,
            ]
        )

    @classmethod
    def decode(cls, data: bytes) -> Self:
        """Read exactly one deterministic conversion_proof record."""
        decoder = _Decoder(data, 0, WireError)
        decoder.record(4)
        source_bytes = bytes(decoder.bytes(32))
        content = bytes(decoder.bytes(32))
        segments = decoder.uint()
        signatures = decoder.uint()
        if decoder.offset != len(data):
            raise WireError("wrong_length", decoder.offset)
        return cls(source_bytes, content, segments, signatures)


@dataclass(frozen=True, slots=True)
class Converted:
    file: bytes
    proof: ConversionProof


def _frame_limit_exceeded(data: bytes, capacity: ConversionCapacity) -> bool:
    """Check the stored content, payload, and decoded length of written frames."""
    offset = 0
    while offset < len(data):
        item = classify_item(data, offset)
        if item.kind == "frame":
            frame = decode_frame(data, offset)
            if (
                len(frame.content) > capacity.stored_bytes_per_frame
                or len(frame.stored_payload) > capacity.stored_bytes_per_frame
                or frame.decoded_length > capacity.decoded_bytes_per_frame
            ):
                return True
        offset = item.end
    return False


def _convert(
    read: Callable[[], bytes],
    read_result: Callable[[bytes, ConversionCapacity], ResultRead],
    capacity: ConversionCapacity = _DEFAULT_CAPACITY,
) -> Converted | ConversionStop:
    """Run P9 conversion with a whole result reader at the same capacity."""
    candidate = prepare_conversion(read, capacity)
    if isinstance(candidate, ConversionStop):
        return candidate
    observed = read_result(candidate.file, capacity)
    if observed.capacity_refused or _frame_limit_exceeded(candidate.file, capacity):
        return ConversionStop("source_exceeds_capacity", 0)
    if observed.admitted_digest() != candidate.expected_digest:
        return ConversionStop("content_mismatch", 0)
    source = candidate.source
    if (stop := source.visibility_stop()) is not None:
        return stop
    return Converted(
        candidate.file,
        ConversionProof(
            source.digest, candidate.expected_digest, source.segments, source.signatures
        ),
    )

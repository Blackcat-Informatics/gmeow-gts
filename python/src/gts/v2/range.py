# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Merkle range roots and detached proof checks for GTS v2 (P5 §§1, 3.1–3.4)."""

from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal

import cbor2
from blake3 import blake3

from ._decode import WireError, _Decoder

_UINT_MAX = (1 << 64) - 1
# Each digest is delimited and checked before accessing these fixed-size records.
_ENCODED_DIGEST_SIZE = 34

type _Peak = tuple[int, bytes]
type _ProofKind = Literal["inclusion", "range"]
type _MalformedReason = Literal["length", "count", "order"]


@dataclass(frozen=True, slots=True)
class RangeRoot:
    count: int
    digest: bytes

    def __post_init__(self) -> None:
        if type(self.count) is not int or not 0 <= self.count <= _UINT_MAX:
            raise ValueError("range leaf count must be a uint")
        if not isinstance(self.digest, bytes) or len(self.digest) != 32:
            raise ValueError("range root must be a 32-byte digest")


@dataclass(frozen=True, slots=True)
class ProofFinding:
    code: Literal["proof_holds", "proof_failed", "proof_malformed"]
    reason: Literal[
        "length",
        "count",
        "order",
        "inclusion",
        "consistency",
        "range",
        "consistency_old_root",
        "consistency_new_root",
        "consistency_diverged",
    ]


def _leaf(identifier: bytes) -> bytes:
    if not isinstance(identifier, bytes) or len(identifier) != 32:
        raise ValueError("range identifier must be a 32-byte digest")
    return blake3(
        cbor2.dumps([identifier]), derive_key_context="gts2 range leaf"
    ).digest()


def _node(left: bytes, right: bytes) -> bytes:
    return blake3(
        cbor2.dumps([left, right]), derive_key_context="gts2 range node"
    ).digest()


def _root(count: int, peaks: list[bytes]) -> bytes:
    return blake3(
        cbor2.dumps([count, peaks]), derive_key_context="gts2 range root"
    ).digest()


def _push(stack: list[_Peak], height: int, digest: bytes) -> None:
    while stack and stack[-1][0] == height:
        _, left = stack.pop()
        digest = _node(left, digest)
        height += 1
    stack.append((height, digest))


def _decompose(first: int, end: int) -> list[tuple[int, int]]:
    subtrees = []
    while first < end:
        height = (end - first).bit_length() - 1
        if first:
            height = min(height, (first & -first).bit_length() - 1)
        subtrees.append((first, height))
        first += 1 << height
    return subtrees


class RangeAccumulator:
    """Keep the peaks of a P5 segment range in logarithmic space."""

    def __init__(self, opener_identifier: bytes) -> None:
        self._peaks = [(0, _leaf(opener_identifier))]
        self._count = 1

    def append(self, effective_identifier: bytes) -> None:
        """Add one ordinary frame with an effective identifier."""
        if self._count == _UINT_MAX:
            raise ValueError("range leaf count would exceed uint")
        _push(self._peaks, 0, _leaf(effective_identifier))
        self._count += 1

    def root(self) -> RangeRoot:
        return RangeRoot(
            self._count, _root(self._count, [digest for _, digest in self._peaks])
        )


def _decode_proof(
    data: bytes, n_uints: int
) -> tuple[tuple[int, ...], memoryview] | None:
    decoder = _Decoder(data, 0, WireError)
    try:
        decoder.record(n_uints + 1)
        fields = tuple(decoder.uint() for _ in range(n_uints))
        length = decoder.array()
        start = decoder.offset
        for _ in range(length):
            decoder.bytes(32)
    except WireError:
        return None
    if decoder.offset != len(data):
        return None
    return fields, memoryview(data)[start:]


def _path(encoded: memoryview) -> list[bytes]:
    # Only a path whose count matches the proof's derived geometry arrives here.
    return [
        bytes(encoded[offset + 2 : offset + _ENCODED_DIGEST_SIZE])
        for offset in range(0, len(encoded), _ENCODED_DIGEST_SIZE)
    ]


def _malformed(reason: _MalformedReason) -> ProofFinding:
    return ProofFinding("proof_malformed", reason)


def _matches(holds: bool, kind: _ProofKind) -> ProofFinding:
    return ProofFinding("proof_holds" if holds else "proof_failed", kind)


def verify_inclusion(proof: bytes, root: RangeRoot, identifier: bytes) -> ProofFinding:
    """Verify an inclusion proof against an independently supplied root."""
    record = _decode_proof(proof, 2)
    if record is None:
        return _malformed("length")
    ((count, index), encoded) = record
    if count == 0 or index >= count:
        return _malformed("order")
    if count != root.count:
        return _malformed("count")

    peaks = _decompose(0, count)
    peak_index, (start, height) = next(
        (i, (start, height))
        for i, (start, height) in enumerate(peaks)
        if start <= index < start + (1 << height)
    )
    if len(encoded) != (height + len(peaks) - 1) * _ENCODED_DIGEST_SIZE:
        return _malformed("length")

    path = _path(encoded)
    current = _leaf(identifier)
    relative = index - start
    for level, sibling in enumerate(path[:height]):
        if relative & (1 << level):
            current = _node(sibling, current)
        else:
            current = _node(current, sibling)
    hashes = path[height:]
    hashes.insert(peak_index, current)
    return _matches(_root(count, hashes) == root.digest, "inclusion")


def verify_consistency(
    proof: bytes, old_root: RangeRoot, new_root: RangeRoot
) -> ProofFinding:
    """Verify that the new range extends the one committed by the old root."""
    record = _decode_proof(proof, 2)
    if record is None:
        return _malformed("length")
    ((old_count, new_count), encoded) = record
    if not 1 <= old_count <= new_count:
        return _malformed("order")
    if old_count != old_root.count or new_count != new_root.count:
        return _malformed("count")

    peaks = _decompose(0, old_count)
    suffix = _decompose(old_count, new_count)
    if len(encoded) != (len(peaks) + len(suffix)) * _ENCODED_DIGEST_SIZE:
        return _malformed("length")
    path = _path(encoded)
    old_peaks = path[: len(peaks)]
    if _root(old_count, old_peaks) != old_root.digest:
        return ProofFinding("proof_failed", "consistency_old_root")
    if old_count == new_count:
        if old_root.digest == new_root.digest:
            return ProofFinding("proof_holds", "consistency")
        return ProofFinding("proof_failed", "consistency_diverged")

    stack = [
        (height, digest) for (_, height), digest in zip(peaks, old_peaks, strict=True)
    ]
    for (_, height), digest in zip(suffix, path[len(peaks) :], strict=True):
        _push(stack, height, digest)
    if _root(new_count, [digest for _, digest in stack]) != new_root.digest:
        return ProofFinding("proof_failed", "consistency_new_root")
    return ProofFinding("proof_holds", "consistency")


def verify_range(
    proof: bytes, root: RangeRoot, identifiers: Sequence[bytes]
) -> ProofFinding:
    """Verify the given ordered identifiers as consecutive leaves of a range."""
    record = _decode_proof(proof, 3)
    if record is None:
        return _malformed("length")
    ((first, end, count), encoded) = record
    if count == 0 or not first < end <= count:
        return _malformed("order")
    if count != root.count:
        return _malformed("count")

    prefix = _decompose(0, first)
    suffix = _decompose(end, count)
    if (
        len(identifiers) != end - first
        or len(encoded) != (len(prefix) + len(suffix)) * _ENCODED_DIGEST_SIZE
    ):
        return _malformed("length")

    path = _path(encoded)
    stack: list[_Peak] = []
    for (_, height), digest in zip(prefix, path[: len(prefix)], strict=True):
        _push(stack, height, digest)
    for identifier in identifiers:
        _push(stack, 0, _leaf(identifier))
    for (_, height), digest in zip(suffix, path[len(prefix) :], strict=True):
        _push(stack, height, digest)
    return _matches(
        _root(count, [digest for _, digest in stack]) == root.digest, "range"
    )

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Detached Merkle-Mountain-Range proof verification for ``index.mmr`` roots.

The proof format is intentionally standalone: a verifier receives JSON, parses
it into a :class:`Proof`, and checks that the declared frame id reconstructs the
selected peak and aggregate root without reading the original GTS file.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from gts.wire import blake3_256, canonical

PROOF_SCHEMA = "gts-mmr-proof-v1"
HASH_ALGORITHM = "blake3-256"
PREIMAGE_VERSION = "gts-mmr-v1"
LEAF_DOMAIN = "gts-mmr-leaf-v1"
PARENT_DOMAIN = "gts-mmr-parent-v1"
ROOT_DOMAIN = "gts-mmr-root-v1"


@dataclass(frozen=True)
class MmrPeak:
    """One Merkle-Mountain-Range peak in left-to-right order."""

    height: int
    hash: bytes


@dataclass(frozen=True)
class ProofStep:
    """One sibling step from a leaf toward its containing peak."""

    parent_height: int
    side: str
    hash: bytes


@dataclass(frozen=True)
class Proof:
    """Detached proof that one frame id is included in an MMR root."""

    count: int
    leaf_index: int
    frame_id: bytes
    root: bytes
    peak_index: int
    peaks: tuple[MmrPeak, ...]
    path: tuple[ProofStep, ...]


def _leaf_hash(index: int, frame_id: bytes) -> bytes:
    """Hash the leaf preimage for a frame at ``index``."""
    return blake3_256(canonical([LEAF_DOMAIN, index, frame_id]))


def _parent_hash(parent_height: int, left: bytes, right: bytes) -> bytes:
    """Hash a parent preimage with explicit height and child ordering."""
    return blake3_256(canonical([PARENT_DOMAIN, parent_height, left, right]))


def _root_hash(count: int, peaks: tuple[MmrPeak, ...]) -> bytes:
    """Hash the aggregate root over count and ordered peak hashes."""
    peak_values = [[peak.height, peak.hash] for peak in peaks]
    return blake3_256(canonical([ROOT_DOMAIN, count, peak_values]))


def _expected_peak_heights(count: int) -> list[int]:
    """Return the canonical left-to-right peak heights for ``count`` leaves."""
    remaining = count
    heights: list[int] = []
    while remaining > 0:
        height = remaining.bit_length() - 1
        heights.append(height)
        remaining -= 1 << height
    return heights


def _peak_index_for_leaf(count: int, heights: list[int], leaf_index: int) -> int:
    """Return the peak that covers ``leaf_index`` under canonical peaks."""
    if leaf_index >= count:
        msg = f"leaf_index {leaf_index} is outside covered count {count}"
        raise ValueError(msg)
    start = 0
    for index, height in enumerate(heights):
        end = start + (1 << height)
        if start <= leaf_index < end:
            return index
        start = end
    msg = f"peak ranges do not cover leaf_index {leaf_index} for count {count}"
    raise ValueError(msg)


def parse_hex_32(value: str) -> bytes:
    """Parse a 32-byte lowercase or uppercase hex id, with optional ``blake3:``."""

    raw = value.strip()
    if raw.startswith("blake3:"):
        raw = raw.removeprefix("blake3:")
    if len(raw) != 64:
        msg = "expected a 32-byte hex value"
        raise ValueError(msg)
    try:
        out = bytes.fromhex(raw)
    except ValueError as exc:
        msg = "hex value contains a non-hex character"
        raise ValueError(msg) from exc
    if len(out) != 32:
        msg = "expected a 32-byte hex value"
        raise ValueError(msg)
    return out


def _object(value: Any, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        msg = f"{context} must be a JSON object"
        raise ValueError(msg)
    return value


def _array(value: Any, context: str) -> list[Any]:
    if not isinstance(value, list):
        msg = f"{context} must be a JSON array"
        raise ValueError(msg)
    return value


def _string_field(obj: dict[str, Any], key: str) -> str:
    value = obj.get(key)
    if not isinstance(value, str):
        msg = f"{key!r} must be a string"
        raise ValueError(msg)
    return value


def _int_field(obj: dict[str, Any], key: str) -> int:
    value = obj.get(key)
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        msg = f"{key!r} must be an unsigned integer"
        raise ValueError(msg)
    return value


def proof_from_json(text: str) -> Proof:
    """Parse the stable detached proof JSON form.

    Raises:
        ValueError: if the schema marker, hash algorithm, field types, or hex
            values do not match the supported proof version.
    """

    try:
        root = json.loads(text)
    except json.JSONDecodeError as exc:
        msg = f"invalid proof JSON: {exc}"
        raise ValueError(msg) from exc
    obj = _object(root, "proof")
    schema = _string_field(obj, "schema")
    if schema != PROOF_SCHEMA:
        msg = f"unsupported proof schema {schema!r}"
        raise ValueError(msg)
    hash_alg = _string_field(obj, "hash")
    if hash_alg != HASH_ALGORITHM:
        msg = f"unsupported hash algorithm {hash_alg!r}"
        raise ValueError(msg)
    preimage = _string_field(obj, "preimage")
    if preimage != PREIMAGE_VERSION:
        msg = f"unsupported preimage version {preimage!r}"
        raise ValueError(msg)

    peaks = tuple(
        MmrPeak(
            height=_int_field(_object(item, "peak"), "height"),
            hash=parse_hex_32(_string_field(_object(item, "peak"), "hash")),
        )
        for item in _array(obj.get("peaks"), "peaks")
    )
    path: list[ProofStep] = []
    for item in _array(obj.get("path"), "path"):
        step = _object(item, "path step")
        side = _string_field(step, "side")
        if side not in {"left", "right"}:
            msg = f"unsupported proof side {side!r}"
            raise ValueError(msg)
        path.append(
            ProofStep(
                parent_height=_int_field(step, "parent_height"),
                side=side,
                hash=parse_hex_32(_string_field(step, "hash")),
            )
        )

    return Proof(
        count=_int_field(obj, "count"),
        leaf_index=_int_field(obj, "leaf_index"),
        frame_id=parse_hex_32(_string_field(obj, "frame_id")),
        root=parse_hex_32(_string_field(obj, "root")),
        peak_index=_int_field(obj, "peak_index"),
        peaks=peaks,
        path=tuple(path),
    )


def verify_proof(proof: Proof) -> None:
    """Verify a detached proof without access to the original GTS file.

    Raises:
        ValueError: if the path, peaks, selected leaf, or aggregate root do not
            describe a valid inclusion proof.
    """

    if len(proof.frame_id) != 32:
        msg = "frame_id must be 32 bytes"
        raise ValueError(msg)
    if len(proof.root) != 32:
        msg = "root must be 32 bytes"
        raise ValueError(msg)
    if proof.leaf_index >= proof.count:
        msg = f"leaf_index {proof.leaf_index} is outside covered count {proof.count}"
        raise ValueError(msg)
    if proof.peak_index >= len(proof.peaks):
        msg = f"peak_index {proof.peak_index} is out of range"
        raise ValueError(msg)
    expected_heights = _expected_peak_heights(proof.count)
    actual_heights = [peak.height for peak in proof.peaks]
    if actual_heights != expected_heights:
        msg = f"peak heights {actual_heights} do not match count {proof.count}"
        raise ValueError(msg)
    computed_peak_index = _peak_index_for_leaf(
        proof.count, actual_heights, proof.leaf_index
    )
    if computed_peak_index != proof.peak_index:
        msg = (
            f"leaf_index {proof.leaf_index} belongs to peak "
            f"{computed_peak_index}, not {proof.peak_index}"
        )
        raise ValueError(msg)
    for peak in proof.peaks:
        if len(peak.hash) != 32:
            msg = "peak hash must be 32 bytes"
            raise ValueError(msg)

    carried = _leaf_hash(proof.leaf_index, proof.frame_id)
    height = 0
    for step in proof.path:
        if len(step.hash) != 32:
            msg = "path hash must be 32 bytes"
            raise ValueError(msg)
        if step.parent_height != height + 1:
            msg = (
                f"path parent height {step.parent_height} does not follow "
                f"height {height}"
            )
            raise ValueError(msg)
        if step.side == "left":
            carried = _parent_hash(step.parent_height, step.hash, carried)
        elif step.side == "right":
            carried = _parent_hash(step.parent_height, carried, step.hash)
        else:
            msg = f"unsupported proof side {step.side!r}"
            raise ValueError(msg)
        height = step.parent_height

    peak = proof.peaks[proof.peak_index]
    if height != peak.height:
        msg = f"path height {height} does not reach peak height {peak.height}"
        raise ValueError(msg)
    if carried != peak.hash:
        msg = "proof path does not reconstruct the selected peak"
        raise ValueError(msg)
    if _root_hash(proof.count, proof.peaks) != proof.root:
        msg = "proof peaks do not reconstruct the declared root"
        raise ValueError(msg)


def verify_proof_json(text: str) -> Proof:
    """Parse and verify a detached proof JSON document."""

    proof = proof_from_json(text)
    verify_proof(proof)
    return proof

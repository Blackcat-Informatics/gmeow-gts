# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Canonical GTS v2 binding and suppression frames (P3 sections 11.3 and 11.4)."""

from collections.abc import Iterator, Mapping
from itertools import batched

import cbor2

from ._fold_record_lines import (
    RecordBody,
    _array_head,
    _binding_body,
    _encode_part,
    _parts,
    _Slot,
    _suppression_body,
)
from .canonical_graph import _ROWS_PER_FRAME, EncodedFrame
from .record_fold import GroundTerm, RecordFold


def _encode_record(
    body: RecordBody, ground: tuple[GroundTerm, ...], term_ids: Mapping[int, int]
) -> bytes:
    payload = bytearray()
    for part in _parts(body, ground, wire=True):
        match part:
            case _Slot(int(ref)):
                payload.extend(cbor2.dumps((1, term_ids[ref])))
            case _Slot():
                raise AssertionError("unresolved record slot")
            case _:
                payload.extend(_encode_part(part))
    return bytes(payload)


def canonical_record_frames(
    records: RecordFold, term_ids: Mapping[int, int]
) -> Iterator[EncodedFrame]:
    """Yield sorted canonical bindings followed by the suppression frames."""
    bindings = (
        _binding_body(binding.term, binding.referent) for binding in records.bindings
    )
    suppression = (_suppression_body(entry) for entry in records.suppression)
    for kind, bodies in ((6, bindings), (7, suppression)):
        entries = {
            _encode_record(body, records.ground_terms, term_ids) for body in bodies
        }
        for batch in batched(sorted(entries), _ROWS_PER_FRAME, strict=False):
            count = len(batch)
            yield kind, [count], _array_head(count) + b"".join(batch)

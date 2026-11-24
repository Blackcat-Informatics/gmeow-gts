# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check wire values and byte ordering of canonical P3 record frames."""

import cbor2

from gts.v2.canonical_records import canonical_record_frames
from gts.v2.record_fold import (
    GroundRef,
    GroundTriple,
    RecordFold,
    ResolvedBinding,
    ResolvedSuppression,
)
from gts.v2.records import (
    BlankPair,
    BlobReference,
    FrameReference,
    StateReference,
    SuppressionKind,
    decode_bindings,
    decode_suppression,
)


def test_canonical_records_write_named_ids_and_unnamed_nested_slots() -> None:
    frame = b"f" * 32
    digest = b"b" * 32
    state = b"s" * 32
    pair = BlankPair(frame, b"\x00\xff")
    blob = BlobReference(1, digest)
    frame_ref = FrameReference(frame)
    state_ref = StateReference(state)
    bindings = frozenset(
        (
            ResolvedBinding(0, blob),
            ResolvedBinding(2, frame_ref),
            ResolvedBinding(GroundRef(1), state_ref),
        )
    )
    terms = (pair, GroundTriple(2, 1, GroundRef(0)))
    suppression = frozenset(
        (
            ResolvedSuppression(SuppressionKind.TERM, (2,)),
            ResolvedSuppression(SuppressionKind.QUAD, (0, 1, 2), graph=3),
            ResolvedSuppression(SuppressionKind.REIFIER, (0,)),
            ResolvedSuppression(SuppressionKind.GRAPH, (3,)),
            ResolvedSuppression(SuppressionKind.BLOB, referent=blob),
            ResolvedSuppression(SuppressionKind.REIFIER_BINDING, (0, 4)),
            ResolvedSuppression(SuppressionKind.ANNOTATION, (0, 1, 2)),
            ResolvedSuppression(
                SuppressionKind.BINDING, (GroundRef(1),), referent=state_ref
            ),
        )
    )
    records = RecordFold(
        bindings, suppression, terms, (), frozenset(), frozenset(), frozenset()
    )
    ids = {0: 300, 1: 1, 2: 24, 3: 2, 4: 301}
    binding_frame, suppression_frame = canonical_record_frames(records, ids)
    subject, predicate, object_id, graph, triple = ([1, ids[i]] for i in range(5))
    unresolved_triple = [3, [object_id, predicate, [2, [frame, pair.label]]]]
    bindings_as_written = [
        [subject, [1, [1, digest]]],
        [object_id, [2, frame]],
        [unresolved_triple, [3, state]],
    ]
    suppression_as_written = [
        [1, object_id],
        [2, [subject, predicate, object_id, [2, graph]]],
        [3, subject],
        [4, graph],
        [5, [1, digest]],
        [6, [subject, triple, [1]]],
        [7, [subject, predicate, object_id, [1]]],
        [8, [unresolved_triple, [3, state]]],
    ]
    for (kind, content, payload), expected_kind, raw, decode in (
        (binding_frame, 6, bindings_as_written, decode_bindings),
        (suppression_frame, 7, suppression_as_written, decode_suppression),
    ):
        expected = sorted(raw, key=cbor2.dumps)
        assert kind == expected_kind
        assert content == [len(expected)]
        assert payload == cbor2.dumps(expected)
        assert len(decode(cbor2.dumps(content), payload)) == len(expected)


def test_canonical_record_encoding_accepts_deep_unnamed_triples() -> None:
    depth = 1100
    pair = BlankPair(bytes(32), b"missing")
    ground = (pair, *(GroundTriple(1, 1, GroundRef(i)) for i in range(depth)))
    record = ResolvedBinding(GroundRef(depth), FrameReference(bytes(32)))
    records = RecordFold(
        frozenset((record,)),
        frozenset(),
        ground,
        (),
        frozenset(),
        frozenset(),
        frozenset(),
    )
    ((kind, content, payload),) = canonical_record_frames(records, {1: 0})
    assert kind == 6
    (binding,) = decode_bindings(cbor2.dumps(content), payload)
    assert len(binding.term) == 3 * depth + 1
    assert binding.term[-1] == pair

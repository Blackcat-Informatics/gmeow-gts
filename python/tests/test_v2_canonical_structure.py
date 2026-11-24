# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check how the structure level judges a segment's own frames."""

from dataclasses import replace

import cbor2
import pytest

from gts.v2.blobs import BlobSet
from gts.v2.canonical_blobs import canonical_value_frames
from gts.v2.canonical_structure import (
    StructureCheck,
    StructureDifference,
    StructureFold,
    StructureItem,
    check_structure,
)
from gts.v2.dictionary import Blank, Iri, TermPool
from gts.v2.frame import decode_frame
from gts.v2.header import Header
from gts.v2.index_claims import IndexStatistics
from gts.v2.items import classify_item
from gts.v2.pack import decode_pack
from gts.v2.parity_write import _SPAN_LENGTH
from gts.v2.payload import PayloadBudget
from gts.v2.record_fold import RecordFold
from gts.v2.rows import QuadRow, encode_rows
from gts.v2.terms import BlankTerm, IriTerm, _encode_terms
from gts.v2.writer import SegmentWriter

_HEADER = Header(canonical=1)


def _empty_fold() -> StructureFold:
    return StructureFold(
        terms=TermPool(),
        rows=(),
        records=RecordFold(
            frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
        ),
        values=frozenset(),
        blobs=BlobSet((), ()),
        verified_blobs={},
        labelling_succeeded=True,
    )


def _observed(data: bytes) -> list[StructureItem]:
    budget = PayloadBudget(
        stored_bytes_per_frame=1 << 26,
        decoded_bytes_per_frame=1 << 26,
        decoded_bytes_per_read=1 << 36,
    )
    items: list[StructureItem] = []
    offset = 0
    while offset < len(data):
        item = classify_item(data, offset)
        frame = decode_frame(data, offset) if item.kind == "frame" else None
        decoded = budget.check(frame).decoded if frame is not None else None
        members = None
        if frame is not None and frame.frame_type == 13:
            assert decoded is not None
            count, length = cbor2.loads(frame.content)
            members = decode_pack(decoded, member_count=count, unit_length=length)
        items.append(
            StructureItem(
                kind=item.kind,
                size=item.end - offset,
                frame_type=frame.frame_type if frame is not None else None,
                envelope=frame,
                decoded=decoded,
                pack_members=members,
            )
        )
        offset = item.end
    return items


def _finish(
    writer: SegmentWriter, stats: IndexStatistics, digest: bytes | None
) -> list[StructureItem]:
    writer.commit()
    writer.append_index(stats, content_digest=digest)
    return _observed(writer.to_bytes(parity=True, offsets=True, locator=True))


def test_structure_uses_stored_labels_and_compares_dictionary_coding() -> None:
    fold = _empty_fold()
    predicate = fold.terms._intern(Iri("http://e/p"))
    blank = fold.terms._intern(Blank(0, b"stored"))
    fold = replace(fold, rows=(QuadRow(blank, predicate, blank),))
    for group, expected in ((64, None), (8, StructureDifference(1, "terms"))):
        writer = SegmentWriter(_HEADER, collect_index=True)
        content, payload = _encode_terms(
            (IriTerm(b"http://e/p"), BlankTerm(b"stored")), 0, group, canonical=True
        )
        writer.append(1, content, payload)
        writer.append(*encode_rows((QuadRow(1, 0, 1),), canonical=True))
        items = _finish(writer, IndexStatistics(1, 0, 1), b"x" * 32)
        assert check_structure(_HEADER, items, fold) == expected


@pytest.mark.parametrize(
    ("stats", "digest", "with_labelling"),
    (
        (IndexStatistics(0, 0, 0), b"d" * 32, None),
        (IndexStatistics(0, 0, 0), None, StructureDifference(2, "index")),
        (IndexStatistics(1, 0, 0), b"d" * 32, StructureDifference(2, "index")),
    ),
)
def test_structure_checks_index_statistics_and_presence_after_labelling(
    stats: IndexStatistics,
    digest: bytes | None,
    with_labelling: StructureDifference | None,
) -> None:
    items = _finish(SegmentWriter(_HEADER, collect_index=True), stats, digest)
    fold = _empty_fold()
    assert check_structure(_HEADER, items, fold) == with_labelling
    assert (
        check_structure(_HEADER, items, replace(fold, labelling_succeeded=False))
        is None
    )


def test_structure_checks_parity_using_observed_item_sizes() -> None:
    items = _finish(
        SegmentWriter(_HEADER, collect_index=True), IndexStatistics(0, 0, 0), bytes(32)
    )
    header, commit, index, parity, offsets, locator = items
    assert check_structure(_HEADER, items, _empty_fold()) is None
    full_span = replace(commit, size=_SPAN_LENGTH - header.size)
    opaque_parity = replace(parity, envelope=None, decoded=None, counted_failure=True)
    with_early_span = [
        header,
        full_span,
        opaque_parity,
        index,
        parity,
        offsets,
        locator,
    ]
    assert check_structure(_HEADER, with_early_span, _empty_fold()) is None
    assert check_structure(_HEADER, [header, full_span, *items[2:]], _empty_fold()) == (
        StructureDifference(2, "frame_sequence")
    )
    full_at_index = replace(index, size=_SPAN_LENGTH - header.size - commit.size)
    assert (
        check_structure(
            _HEADER,
            [header, commit, full_at_index, parity, offsets, locator],
            _empty_fold(),
        )
        is None
    )


def test_structure_names_an_unfolded_pack_by_the_next_decisive_pack() -> None:
    writer = SegmentWriter(_HEADER, collect_index=True)
    writer.append(*next(canonical_value_frames((b"value",))))
    header, value, *_ = _finish(writer, IndexStatistics(0, 0, 0), bytes(32))
    failed = replace(value, pack_members=None, counted_failure=True)
    assert check_structure(_HEADER, [header, failed, value], None) == (
        StructureDifference(1, "value_region")
    )
    assert check_structure(_HEADER, [header, failed], None) == (
        StructureDifference(1, "blob_region")
    )


@pytest.mark.parametrize(
    ("frame_type", "check"),
    ((1, "frame_sequence"), (11, "value_region"), (8, "blob_region")),
)
def test_structure_finds_illegal_items_before_a_counted_failure(
    frame_type: int, check: StructureCheck
) -> None:
    header = StructureItem(kind="header", size=12)
    rows = StructureItem(kind="frame", size=128, frame_type=2)
    extra = StructureItem(kind="frame", size=128, frame_type=frame_type)
    failed = StructureItem(kind="frame", size=64, frame_type=15, counted_failure=True)
    assert check_structure(_HEADER, [header, rows, extra, failed], None) == (
        StructureDifference(2, check)
    )
    assert check_structure(_HEADER, [header, rows, failed], None) == (
        StructureDifference(2, "index")
    )


def test_structure_keeps_existing_short_parity_before_an_opaque_append() -> None:
    header, commit, index, parity, *_ = _finish(
        SegmentWriter(_HEADER, collect_index=True), IndexStatistics(0, 0, 0), bytes(32)
    )
    failed = replace(index, envelope=None, decoded=None, counted_failure=True)
    assert check_structure(_HEADER, [header, commit, index, parity, failed], None) == (
        StructureDifference(4, "index")
    )

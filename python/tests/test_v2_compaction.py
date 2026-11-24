# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check canonical compaction output and the ordered P7 stop cases."""

from dataclasses import replace
from pathlib import Path

import pytest

from gts.v2.blobs import BlobDeclaration, BlobLocation, BlobSet, blob_set
from gts.v2.compaction import (
    CompactionSource,
    FileEnd,
    ReadCoverage,
    Stopped,
    Written,
    compact,
)
from gts.v2.dictionary import Blank, Iri, TermPool, TypedLiteral
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.lines import LineResult
from gts.v2.rebuild import (
    Entry,
    IncompleteRebuild,
    RebuildBudget,
    UnitIndex,
    unit_digest,
)
from gts.v2.record_fold import RecordFold, ResolvedBinding
from gts.v2.records import BlobReference
from gts.v2.rows import QuadRow

P7 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P7"
_EMPTY = CompactionSource(
    terms=TermPool(),
    lines=LineResult(frozenset(), False, False),
    records=RecordFold(
        frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
    ),
    values=frozenset(),
    blobs=BlobSet((), ()),
    declarations=(),
    rebuilds={},
    coverage=ReadCoverage(FileEnd.NO_COMMIT, 0),
    content_digest=None,
    blank_positions=None,
)


def test_compact_writes_the_canonical_fold_and_retains_source_coverage() -> None:
    pool = TermPool()
    a, b, g, p, q = (pool._intern(Iri(f"http://e/{name}")) for name in "abgpq")
    x = pool._intern(Blank(3, b"noncanonical"))
    v = pool._intern(TypedLiteral("v", "http://www.w3.org/2001/XMLSchema#string"))
    lines = LineResult(
        frozenset((QuadRow(a, p, x), QuadRow(x, p, v), QuadRow(a, q, b, g))),
        False,
        False,
    )
    name = (1, unit_digest(b"abc"))
    declarations = (
        BlobDeclaration(BlobLocation(0, 1), name, 3, "text/plain", "complete", b"abc"),
    )
    blobs = blob_set(declarations)
    records = replace(
        _EMPTY.records, bindings=frozenset((ResolvedBinding(a, BlobReference(*name)),))
    )
    labelled = FoldLabellingBudget(
        labelled_lines=16, labelled_bytes_per_read=4096, labelling_units=0
    ).label_fold(pool, lines, records, blobs, (), folded_size=5)
    coverage = ReadCoverage(FileEnd.TORN, 1)
    source = replace(
        _EMPTY,
        terms=pool,
        lines=lines,
        records=records,
        blobs=blobs,
        declarations=declarations,
        coverage=coverage,
        content_digest=labelled.content_digest,
        blank_positions=labelled.blank_positions,
    )
    result = compact(source)
    assert result.outcome == Written((P7 / "canonical__construction.bin").read_bytes())
    assert result.coverage == coverage
    assert result.coverage.covers_content is False


def test_compaction_stop_order_prefers_formed_then_held_then_labelling() -> None:
    unreadable = replace(
        _EMPTY,
        formed_refused=True,
        lines=replace(_EMPTY.lines, held_rows=True),
        labelling_refusal="refusal_width",
    )
    assert compact(unreadable).encode().hex() == "8302148202185a"
    held = replace(unreadable, formed_refused=False)
    assert compact(held).encode().hex() == "830218198101"
    refused = replace(held, lines=_EMPTY.lines)
    assert compact(refused).encode().hex() == "8302181a82021820"


@pytest.mark.parametrize(
    ("last_present", "expected"),
    [
        (False, Stopped("missing_dependency", "chunk")),
        (True, Stopped("damaged_frame", "blob_length")),
    ],
)
def test_compaction_checks_all_recipe_units_after_a_length_mismatch(
    last_present: bool, expected: Stopped
) -> None:
    units = UnitIndex()
    first, last = map(unit_digest, (b"a", b"b"))
    assert units.add(1, first, b"a")
    if last_present:
        assert units.add(1, last, b"b")
    name = (1, unit_digest(b"abb"))
    location = BlobLocation(0, 2)
    rebuild = RebuildBudget(3).rebuild(
        units,
        position=2,
        digest=name[1],
        total_length=3,
        entries=(Entry(first, 2), Entry(last, 1)),
        kind="recipe",
    )
    assert isinstance(rebuild, IncompleteRebuild)
    assert rebuild.reason == "blob_length"
    declarations = (BlobDeclaration(location, name, 3, None, "incomplete"),)
    source = replace(
        _EMPTY,
        blobs=blob_set(declarations),
        declarations=declarations,
        rebuilds={location: rebuild},
    )
    assert compact(source).outcome == expected

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise canonical claim checks on resolved own folds."""

import cbor2
import pytest

from gts.v2.blobs import BlobSet
from gts.v2.canonical_labelling import (
    BlankOrigin,
    LabellingDifference,
    check_canonical_labelling,
)
from gts.v2.dictionary import Blank, Iri, TermPool, Triple
from gts.v2.fold_labelling import FoldLabellingBudget
from gts.v2.lines import LineResult
from gts.v2.record_fold import (
    GroundRef,
    GroundTriple,
    RecordFold,
    ResolvedBinding,
    ResolvedSuppression,
)
from gts.v2.records import BlankPair, BlobReference, SuppressionKind
from gts.v2.rows import AnnotationRow, QuadRow

_NO_RECORDS = RecordFold(
    frozenset(), frozenset(), (), (), frozenset(), frozenset(), frozenset()
)


def _label(pool, rows, records):
    return FoldLabellingBudget(
        labelled_lines=1 << 26,
        labelled_bytes_per_read=1 << 37,
        labelling_units=(1 << 64) - 1,
    ).label_fold(
        pool,
        LineResult(rows, False, False),
        records,
        BlobSet((), ()),
        (),
        folded_size=len(rows) + len(records.bindings) + len(records.suppression) + 2,
    )


def _fold(labels, difference):
    pool = TermPool()
    p = pool._intern(Iri("urn:predicate"))
    item = pool._intern(Iri("urn:item"))
    one, two = (pool._intern(Blank(0, label)) for label in labels)
    inner = [pool._intern(Triple(node, p, item)) for node in (one, two)]
    outer = [pool._intern(Triple(item, p, child)) for child in inner]
    rows = frozenset(
        (
            QuadRow(item, p, outer[0]),
            QuadRow(item, p, outer[1]),
            AnnotationRow(1, one, p, item),
            AnnotationRow(2 if difference == "count" else 1, two, p, item),
        )
    )
    referent = BlobReference(1, b"a" * 32)
    bindings = frozenset(
        (
            ResolvedBinding(GroundRef(1), referent),
            ResolvedBinding(
                GroundRef(2),
                BlobReference(1, b"b" * 32) if difference == "binding" else referent,
            ),
        )
    )
    targets = [ResolvedSuppression(SuppressionKind.TERM, (one,))]
    if difference != "suppression":
        targets.append(ResolvedSuppression(SuppressionKind.TERM, (two,)))
    records = RecordFold(
        bindings,
        frozenset(targets),
        (
            BlankPair(b"c" * 32, b"unresolved"),
            GroundTriple(one, p, GroundRef(0)),
            GroundTriple(two, p, GroundRef(0)),
        ),
        (),
        frozenset(),
        frozenset(),
        frozenset(),
    )
    return pool, rows, records, (one, two)


@pytest.mark.parametrize("difference", [None, "count", "binding", "suppression"])
def test_stored_permutation_preserves_rows_and_records(difference) -> None:
    pool, rows, records, (one, two) = _fold((b"a", b"b"), difference)
    labelled = _label(pool, rows, records)
    stored = tuple(cbor2.dumps(labelled.blank_positions[ref]) for ref in (two, one))
    pool, rows, records, (one, two) = _fold(stored, difference)
    # Keep the original semantic L1 positions and test the induced swap.
    result = check_canonical_labelling(
        pool,
        rows,
        records,
        frozenset(),
        labelled,
        (BlankOrigin(one, 3), BlankOrigin(two, 5)),
        None,
    )
    assert result == (
        None if difference is None else LabellingDifference(3, "labelling")
    )


@pytest.mark.parametrize(
    ("index_position", "expected"),
    [
        (None, LabellingDifference(3, "labelling")),
        (2, LabellingDifference(2, "content_digest")),
        (3, LabellingDifference(3, "labelling")),
        (9, LabellingDifference(3, "labelling")),
    ],
)
def test_first_difference_considers_label_origins_and_index_position(
    index_position, expected
) -> None:
    pool = TermPool()
    b = pool._intern(Blank(0, b"\x18\x00"))
    p = pool._intern(Iri("urn:predicate"))
    rows = frozenset((QuadRow(b, p, p),))
    labelled = _label(pool, rows, _NO_RECORDS)
    index = None if index_position is None else (index_position, bytes(32))
    assert (
        check_canonical_labelling(
            pool,
            rows,
            _NO_RECORDS,
            frozenset(),
            labelled,
            (BlankOrigin(b, 3),),
            index,
        )
        == expected
    )


def test_digest_is_checked_for_a_fold_without_blank_nodes() -> None:
    pool = TermPool()
    ref = pool._intern(Iri("urn:item"))
    rows = frozenset((QuadRow(ref, ref, ref),))
    labelled = _label(pool, rows, _NO_RECORDS)
    assert (
        check_canonical_labelling(
            pool,
            rows,
            _NO_RECORDS,
            frozenset(),
            labelled,
            (),
            (5, labelled.labelling.sequence.digest),
        )
        is None
    )
    assert check_canonical_labelling(
        pool, rows, _NO_RECORDS, frozenset(), labelled, (), (5, bytes(32))
    ) == LabellingDifference(5, "content_digest")

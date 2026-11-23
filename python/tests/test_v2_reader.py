# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise recovery look-ahead and its share of the item count."""

from pathlib import Path

import cbor2
import pytest

from gts.v2 import Header, encode_header
from gts.v2.payload import PayloadBudget
from gts.v2.reader import _Boundary, _ItemWalk, _open_input, _Opener
from gts.v2.resync import RESYNC_MAX, DecodedResync, ResyncScanner, encode_resync

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"


def test_incomplete_first_item_does_not_consume_a_slot() -> None:
    data = (
        P1 / "resync-incomplete-item-not-counted.first-item-incomplete.bin"
    ).read_bytes()
    budget = PayloadBudget(
        stored_bytes_per_frame=0,
        decoded_bytes_per_frame=0,
        decoded_bytes_per_read=RESYNC_MAX,
    )
    walk = _ItemWalk(data, items_per_read=3)
    opened = _open_input(data, walk, ResyncScanner(data, budget), first_and_second=True)
    assert opened.damaged_prefix == _Boundary(0, 199, "torn_tail")
    assert isinstance(opened.decision, _Opener)
    assert opened.decision.scanned
    assert isinstance(opened.decision.value, DecodedResync)
    assert walk.take() == _Boundary(opened.decision.value.next_offset, 298)
    assert walk.take() == _Boundary(298, 299)
    assert walk.take() is None
    assert walk.stopped_at == 299


def test_region_at_decision_point_exhausts_count_before_candidate() -> None:
    data = (P1 / "item-limit-region-candidate-untested.bin").read_bytes()
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    walk = _ItemWalk(data, items_per_read=5)
    failed = walk.take()
    assert failed is not None
    assert failed == _Boundary(0, 234)
    assert walk.recover(ResyncScanner(data, budget), 0, failed) is None
    # The candidate would refuse the decoded-byte charge if it was tested.
    assert not budget.total_refused
    assert [walk.take() for _ in range(3)] == [
        _Boundary(234, 236),
        _Boundary(236, 237),
        _Boundary(237, 271),
    ]
    assert walk.take() == _Boundary(271, len(data), "undelimitable_input")
    assert walk.take() is None
    assert walk.stopped_at is None


@pytest.mark.parametrize(
    ("record", "first_and_second", "expected"),
    [
        ({"v": 1}, True, "first_generation"),
        ({"v": 1}, False, "unsupported_generation"),
        ({"v": 2}, True, "unsupported_generation"),
        ([1, 3], True, "unsupported_generation"),
        ([5, 1], True, "not_this_format"),
    ],
)
def test_generation_dispatch_does_not_select_later_opener(
    record: object, first_and_second: bool, expected: str
) -> None:
    first = cbor2.dumps(record)
    data = first + encode_resync(Header(), offset=len(first))
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    opened = _open_input(
        data,
        _ItemWalk(data, items_per_read=8),
        ResyncScanner(data, budget),
        first_and_second=first_and_second,
    )
    assert opened.decision == expected
    assert opened.refusal is None


def test_walked_header_has_absolute_item_boundary() -> None:
    header = encode_header(Header())
    data = header + b"\xff"
    budget = PayloadBudget(
        stored_bytes_per_frame=0, decoded_bytes_per_frame=0, decoded_bytes_per_read=0
    )
    walk = _ItemWalk(data, items_per_read=1)
    opening = _open_input(
        data, walk, ResyncScanner(data, budget), first_and_second=False
    )
    assert isinstance(opening.decision, _Opener)
    assert opening.decision.item == _Boundary(0, len(header))
    assert opening.decision.kind == "header"
    assert walk.stopped_at == len(header)

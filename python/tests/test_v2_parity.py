# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check P5 candidate eligibility, ordering and capacity against real frames."""

from pathlib import Path

import pytest

from gts.v2 import PayloadBudget, decode_frame, decode_header
from gts.v2.parity import (
    PARITY_PATTERN,
    ParityCandidate,
    ParityRefusal,
    check_parity_candidate,
)

VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2"


@pytest.mark.parametrize(
    ("part", "name"),
    [("P5", "parity-base"), ("P1", "append-check-recovery-resumed")],
)
def test_accepts_parity_candidate_from_corpus(part: str, name: str) -> None:
    data = (VECTORS / part / f"{name}.bin").read_bytes()
    offset = data.index(PARITY_PATTERN)
    frame = decode_frame(data, offset)
    opener = decode_header(data).identifier
    assert opener is not None
    budget = PayloadBudget(
        stored_bytes_per_frame=frame.decoded_length,
        decoded_bytes_per_frame=frame.decoded_length,
        decoded_bytes_per_read=frame.decoded_length,
    )
    result = check_parity_candidate(
        data,
        offset,
        opener=opener,
        budget=budget,
        prior_starts=set(),
        prior_short_span=False,
    )
    assert isinstance(result, ParityCandidate)
    assert result.frame == frame
    assert result.offset == offset
    assert budget.charge_decoded(1) == frame.decoded_length + 1


@pytest.mark.parametrize("damage", ["table", "link", "local_capacity", "duplicate"])
def test_candidate_charges_only_after_envelope_link_and_local_limits(
    damage: str,
) -> None:
    data = (VECTORS / "P5" / "parity-base.bin").read_bytes()
    offset = data.index(PARITY_PATTERN)
    frame = decode_frame(data, offset)
    opener = decode_header(data).identifier
    assert opener is not None
    if damage == "table":
        opener = bytes(32)
    if damage == "link":
        last_digest_byte = frame.next_offset - 2
        data = (
            data[:last_digest_byte]
            + bytes([data[last_digest_byte] ^ 1])
            + data[last_digest_byte + 1 :]
        )
    length = frame.decoded_length
    budget = PayloadBudget(
        stored_bytes_per_frame=length - 1 if damage == "local_capacity" else length,
        decoded_bytes_per_frame=length,
        decoded_bytes_per_read=length,
    )
    assert (
        check_parity_candidate(
            data,
            offset,
            opener=opener,
            budget=budget,
            prior_starts={0} if damage == "duplicate" else set(),
            prior_short_span=False,
        )
        is None
    )
    assert budget.charge_decoded(length) == (2 * length if damage == "table" else None)


def test_parity_candidate_refusal_records_span_of_candidate() -> None:
    data = (VECTORS / "P5" / "parity-base.bin").read_bytes()
    offset = data.index(PARITY_PATTERN)
    frame = decode_frame(data, offset)
    opener = decode_header(data).identifier
    assert opener is not None
    budget = PayloadBudget(
        stored_bytes_per_frame=frame.decoded_length,
        decoded_bytes_per_frame=frame.decoded_length,
        decoded_bytes_per_read=0,
    )
    assert check_parity_candidate(
        data,
        offset,
        opener=opener,
        budget=budget,
        prior_starts=set(),
        prior_short_span=False,
    ) == ParityRefusal(offset, frame.next_offset - offset, frame.decoded_length)
    assert (
        check_parity_candidate(
            data,
            offset,
            opener=opener,
            budget=budget,
            prior_starts=set(),
            prior_short_span=False,
        )
        is None
    )

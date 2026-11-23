# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare P1 payload decisions with the frame records in the published corpus."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame, encode_plain_frame

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"
# P2 reason codes for findings produced by the payload check.
_REASONS = {
    1: "unknown_codec",
    11: "codec_constraint",
    12: "payload_digest",
    13: "decoded_length",
    16: "decode_failed",
    30: "decoded_size",
    31: "stored_size",
    86: "total_decoded",
}


@pytest.mark.parametrize(
    "name",
    [
        "zstd-ok",
        "zstd-header-rows",
        "zstd-header-differs",
        "zstd-header-short",
        "zstd-two-frames",
        "zstd-trailing-byte",
        "zstd-output-short",
        "stored-mismatch",
        "payload-output-longer",
        "payload-length",
        "payload-digest",
        "payload-unknown-codec",
        "payload-exceeds-stored",
        "payload-exceeds-decoded",
        "payload-exceeds-total",
        "payload-total-at-capacity",
        "payload-total-after-refusal",
        "payload-total-order",
    ],
)
def test_payload_states_and_capacity_match_corpus(name: str) -> None:
    entry = json.loads((P1 / f"{name}.json").read_text())
    for case in entry["cases"]:
        data = (P1 / case["input_file"]).read_bytes()
        capacity = case["capacity"]
        budget = PayloadBudget(
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
        )
        result = cbor2.loads(bytes.fromhex(case["read_result"]))
        for segment in result[-1][1]:
            registry_version = segment[5][1]
            for record in segment[-2]:
                _, offset, _, _, _, state, stored, *_, findings = record
                frame = decode_frame(data, offset, registry_version=registry_version)
                checked = budget.check(frame)

                assert (
                    checked.state == ("verified", "damaged", "unchecked")[state[0] - 1]
                )
                assert checked.stored_matches is (
                    None if stored == [1] else stored == [2, 1]
                )
                reasons = [
                    (_REASONS[reason[1]], count[1] if count[0] == 2 else None)
                    for _, reason, _, _, _, _, count in findings
                    if reason[0] == 2 and reason[1] in _REASONS
                ]
                assert reasons == (
                    [] if checked.reason is None else [(checked.reason, checked.count)]
                )
                assert (checked.decoded is not None) == (state[0] == 1)


def test_refused_total_also_refuses_a_later_frame_above_its_local_limit() -> None:
    budget = PayloadBudget(
        stored_bytes_per_frame=64, decoded_bytes_per_frame=1, decoded_bytes_per_read=0
    )
    chunk = [bytes(32)]
    first = decode_frame(encode_plain_frame(bytes(32), 8, chunk, b"x"))
    later = decode_frame(encode_plain_frame(bytes(32), 8, chunk, b"xy"))
    assert budget.check(first).reason == "total_decoded"
    result = budget.check(later)
    assert (result.reason, result.count) == ("total_decoded", 2)


def test_stored_limit_counts_complete_signature_item_after_content() -> None:
    commit = encode_plain_frame(bytes(32), 14, [0, bytes(32)], b"")
    envelope = cbor2.loads(commit)
    envelope[-1] = [[1, bytes(32), bytes(64)]]
    frame = decode_frame(cbor2.dumps(envelope))
    content_size = len(frame.content)
    signature_size = len(cbor2.dumps(envelope[-1]))

    for limit, refused_size in (
        (content_size - 1, content_size),
        (content_size, signature_size),
    ):
        budget = PayloadBudget(
            stored_bytes_per_frame=limit,
            decoded_bytes_per_frame=0,
            decoded_bytes_per_read=0,
        )
        result = budget.check(frame)
        assert (result.reason, result.count) == ("stored_size", refused_size)

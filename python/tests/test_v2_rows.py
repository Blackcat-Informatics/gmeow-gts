# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare the Python row payload codec with the published P3 corpus."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2._content import ContentError, UnknownContentCode
from gts.v2.rows import (
    AnnotationRow,
    DecodedRows,
    EmptyGraphRow,
    QuadRow,
    decode_rows,
    encode_rows,
)

P3 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P3"
_MALFORMED = {
    3: "not_deterministic",
    4: "wrong_type",
    5: "wrong_length",
    6: "out_of_range",
    7: "unregistered_value",
    8: "unordered_or_repeated",
    9: "inconsistent_counts",
}
_GOLDEN = {
    "quads-four-rows",
    "annotations-counts-add",
    "empty-graphs-column",
    "reifier-bindings-triple-not-triple",
}


@pytest.mark.parametrize(
    "name",
    [
        *sorted(_GOLDEN),
        "rows-value-negative",
        "rows-value-above-range",
        "rows-run-count-zero",
        "rows-run-column-sum",
        "rows-run-column-adjacent-equal",
        "rows-row-count-zero",
        "rows-record-field-order",
        "rows-payload-wrong-record",
        "rows-order-unregistered",
        "rows-order-type-before-count",
        "rows-order-trailing-before-count",
        "rows-order-record-before-payload",
        "rows-order-canonical-violated",
        "rows-order-canonical-two-runs",
        "rows-order-canonical-ok",
        "rows-order-canonical-across-frames",
        "rows-canonical-follows-opaque",
        "rows-graph-runs-sum",
        "rows-graph-runs-adjacent-equal",
        "rows-graph-run-zero",
        "rows-column-length",
        "annotations-repeat",
        "annotations-count-zero",
    ],
)
def test_row_payload_and_declared_order_match_corpus(name: str) -> None:
    cases = json.loads((P3 / f"{name}.json").read_text())["cases"]
    for case in cases:
        data = (P3 / case["input_file"]).read_bytes()
        segments = cbor2.loads(bytes.fromhex(case["read_result"]))[-1][1]
        capacity = case["capacity"]
        budget = PayloadBudget(
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
        )
        for segment in segments:
            previous: dict[int, DecodedRows | None] = {}
            for record in segment[-2]:
                _, offset, _, kind, link, payload_state, *_ = record
                if link == 3 or payload_state[0] != 1 or kind == [1]:
                    if kind[0] == 2:
                        previous[kind[1]] = None
                    continue
                frame = decode_frame(data, offset)
                if frame.frame_type not in range(2, 6):
                    continue
                payload = budget.check(frame).decoded
                assert payload is not None
                frame_type = frame.frame_type
                earlier = previous.get(frame_type)
                malformed = [finding[1][1] for finding in record[-1] if finding[0] == 7]
                if malformed:
                    with pytest.raises(ContentError) as error:
                        decode_rows(
                            frame_type, frame.content, payload, previous=earlier
                        )
                    assert error.value.reason == _MALFORMED[malformed[0]]
                    previous[frame_type] = None
                else:
                    decoded = decode_rows(
                        frame_type, frame.content, payload, previous=earlier
                    )
                    assert (decoded.order_holds is False) == any(
                        finding[0] == 21 for finding in record[-1]
                    )
                    previous[frame_type] = decoded
                    if name in _GOLDEN:
                        kind, content, encoded = encode_rows(
                            decoded.rows,
                            canonical=decoded.order == "canonical",
                            previous=earlier,
                        )
                        assert (kind, cbor2.dumps(content), encoded) == (
                            frame_type,
                            frame.content,
                            payload,
                        )


def test_delta_round_trip_across_the_uint_range() -> None:
    largest = (1 << 64) - 1
    rows = (
        QuadRow(0, largest, largest),
        QuadRow(largest, 0, 0),
        QuadRow(0, largest, largest, graph=0),
    )
    frame_type, content, payload = encode_rows(rows)
    assert decode_rows(frame_type, cbor2.dumps(content), payload).rows == rows


def test_written_alike_annotations_are_rejected_before_folding() -> None:
    rows = (AnnotationRow(1, 0, 1, 2), AnnotationRow(2, 0, 1, 3))
    frame_type, content, payload = encode_rows(rows)
    aliases = {2: 2, 3: 2}
    with pytest.raises(ContentError) as error:
        decode_rows(
            frame_type,
            cbor2.dumps(content),
            payload,
            written_alike=lambda tid: aliases.get(tid, tid),
        )
    assert error.value.reason == "unordered_or_repeated"


def test_later_registry_row_order_is_an_unknown_content_code() -> None:
    with pytest.raises(UnknownContentCode) as error:
        decode_rows(2, b"\x82\x03\x00", b"", registry_version=2)
    assert (error.value.code, error.value.offset) == (3, 1)


def test_canonical_empty_graph_writer_checks_previous_frame() -> None:
    kind, content, payload = encode_rows([EmptyGraphRow(5)], canonical=True)
    earlier = decode_rows(kind, cbor2.dumps(content), payload)
    assert earlier.order is None
    with pytest.raises(ValueError, match="strictly ascending"):
        encode_rows([EmptyGraphRow(2)], canonical=True, previous=earlier)

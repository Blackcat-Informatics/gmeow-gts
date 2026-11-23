# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise the graph content and resolution phases together."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2._content import ContentError
from gts.v2.dictionary import RowResolution, TermPool, TermResolution
from gts.v2.graph import GraphContent, GraphFold
from gts.v2.lines import LineFold
from gts.v2.rows import DecodedRows, QuadRow, encode_rows
from gts.v2.terms import (
    FormedBudget,
    IriTerm,
    LiteralTerm,
    ValueReference,
    encode_terms,
)

_P3 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P3"


@pytest.mark.parametrize(
    "name",
    [
        "terms-id-base-after-opaque",
        "terms-canonical-follows-opaque",
        "rows-order-canonical-across-frames",
        "annotations-triple-components-written-alike",
        "annotations-repeat-after-resolution",
        "rows-opaque-term-held",
    ],
)
def test_graph_content_and_fold_match_the_published_cases(name: str) -> None:
    for case in json.loads((_P3 / f"{name}.json").read_text())["cases"]:
        data = (_P3 / case["input_file"]).read_bytes()
        expected = cbor2.loads(bytes.fromhex(case["read_result"]))
        stored, decoded, _, _, read_decoded, _, _, formed, _, _ = expected[3]
        payloads = PayloadBudget(
            stored_bytes_per_frame=stored,
            decoded_bytes_per_frame=decoded,
            decoded_bytes_per_read=read_decoded,
        )
        formed_budget = FormedBudget(formed)
        pool = TermPool()
        for segment in expected[-1][1]:
            content = GraphContent(formed_budget)
            fold = GraphFold(pool, segment[0], LineFold())
            for record in segment[-2]:
                _, offset, _, kind, link, *_ = record
                if kind[0] != 2 or kind[1] not in range(1, 6):
                    continue
                frame = decode_frame(data, offset)
                payload = payloads.check(frame).decoded
                checked = content.read(
                    kind[1], frame.content, payload if link != 3 else None
                )
                expected_codes = {finding[0] for finding in record[-1]}
                if 7 in expected_codes:
                    assert isinstance(checked.error, ContentError)
                    assert checked.error.reason == "unordered_or_repeated"
                else:
                    assert checked.error is None
                resolved = fold.add(checked)
                order_fails = (
                    isinstance(checked.contribution, DecodedRows)
                    and checked.contribution.order_holds is False
                ) or (
                    isinstance(resolved, TermResolution)
                    and resolved.order_holds is False
                )
                assert order_fails == (21 in expected_codes)
                held = (
                    isinstance(resolved, RowResolution)
                    and "opaque_term" in resolved.findings
                )
                assert held == (25 in expected_codes)


def test_rebuilt_literal_supplies_a_row_only_in_the_fold_phase() -> None:
    reference = ValueReference(b"v" * 32, 3)
    record, payload = encode_terms(
        [
            IriTerm(b"https://example.test/subject"),
            IriTerm(b"https://example.test/predicate"),
            IriTerm(b"http://www.w3.org/2001/XMLSchema#string"),
            LiteralTerm(reference, datatype=2),
        ]
    )
    row_type, row_record, row_payload = encode_rows([QuadRow(0, 1, 3)])
    grammar = GraphContent(FormedBudget(1024))
    terms = grammar.read(1, cbor2.dumps(record), payload)
    rows = grammar.read(row_type, cbor2.dumps(row_record), row_payload)
    assert terms.error is None and rows.error is None

    for values in ({reference: b"abc"}, {reference: None}, {}):
        fold = GraphFold(TermPool(), 1, LineFold())
        terms_result = fold.add(terms, values=values)
        assert isinstance(terms_result, TermResolution)
        assert terms_result.missing_values == (
            () if reference in values else (reference,)
        )
        rows_result = fold.add(rows)
        assert isinstance(rows_result, RowResolution)
        assert bool(fold.result().lines) == (values.get(reference) == b"abc")
        assert fold.result().held_rows == (values.get(reference) is None)

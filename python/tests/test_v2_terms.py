# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare P3 dictionary storage and capacity decisions with the corpus."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2._content import ContentError, UnknownContentCode
from gts.v2.terms import (
    BlankTerm,
    FormedBudget,
    FormedCapacityError,
    IriTerm,
    LiteralTerm,
    TimeValue,
    TripleTerm,
    TypedValue,
    TypedValueTerm,
    ValueReference,
    decode_terms,
    encode_terms,
    reserved_term_count,
)

P3 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P3"
_MALFORMED = {
    3: "not_deterministic",
    4: "wrong_type",
    5: "wrong_length",
    6: "out_of_range",
    7: "unregistered_value",
    9: "inconsistent_counts",
    10: "forbidden_combination",
}


@pytest.mark.parametrize(
    "name",
    [
        "terms-six-terms",
        "terms-value-each-kind",
        "terms-value-kind-unregistered",
        "terms-value-time-range",
        "terms-runs-empty",
        "terms-runs-adjacent-same-kind",
        "terms-run-kind-unregistered",
        "terms-run-count-zero",
        "terms-id-above-uint",
        "terms-restart-group-zero",
        "terms-prefix-splits-character",
        "terms-prefix-too-long",
        "terms-prefix-at-restart",
        "terms-prefix-not-maximal",
        "terms-suffix-total",
        "terms-form-unregistered",
        "terms-form-list-counts",
        "terms-language-index-range",
        "terms-language-table-both-blocks",
        "terms-language-table-unused",
        "terms-language-table-repeat",
        "terms-language-table-order",
        "terms-id-base-mismatch",
        "terms-order-count-sequence",
        "terms-order-type-before-count",
        "terms-record-field-order-newer-version",
        "terms-formed-then-small",
        "terms-formed-malformed-first",
        "terms-formed-order",
        "terms-text-not-utf8",
    ],
)
def test_term_storage_and_formation_match_corpus(name: str) -> None:
    cases = json.loads((P3 / f"{name}.json").read_text())["cases"]
    for case in cases:
        if case["capacity"]["access"] != "whole":
            continue
        data = (P3 / case["input_file"]).read_bytes()
        segments = cbor2.loads(bytes.fromhex(case["read_result"]))[-1][1]
        capacity = case["capacity"]
        formed = FormedBudget(capacity["formed_bytes_per_read"])
        budget = PayloadBudget(
            stored_bytes_per_frame=capacity["stored_bytes_per_frame"],
            decoded_bytes_per_frame=capacity["decoded_bytes_per_frame"],
            decoded_bytes_per_read=capacity["decoded_bytes_per_read"],
        )
        for segment in segments:
            base = 0
            registry = segment[5][1]
            for record in segment[-2]:
                _, offset, _, kind, link, payload_state, *_ = record
                if kind != [2, 1]:
                    continue
                frame = decode_frame(data, offset, registry_version=registry)
                expected_base = base
                base += reserved_term_count(
                    frame.content,
                    expected_base=base,
                    registry_version=registry,
                )
                if link == 3 or payload_state[0] != 1:
                    continue
                payload = budget.check(frame).decoded
                assert payload is not None
                findings = record[-1]
                malformed = [reason[1] for code, reason, *_ in findings if code == 7]
                refused = [f[6][1] for f in findings if f[0] == 20 and f[1] == [2, 90]]
                if malformed:
                    with pytest.raises(ContentError) as error:
                        decode_terms(
                            frame.content,
                            payload,
                            expected_base=expected_base,
                            budget=formed,
                            registry_version=registry,
                        )
                    assert error.value.reason == _MALFORMED[malformed[0]]
                elif refused:
                    with pytest.raises(FormedCapacityError) as error:
                        decode_terms(
                            frame.content,
                            payload,
                            expected_base=expected_base,
                            budget=formed,
                            registry_version=registry,
                        )
                    assert error.value.count == refused[0]
                elif any(f[0] == 6 for f in findings):
                    with pytest.raises(UnknownContentCode):
                        decode_terms(
                            frame.content,
                            payload,
                            expected_base=expected_base,
                            budget=formed,
                            registry_version=registry,
                        )
                else:
                    result = decode_terms(
                        frame.content,
                        payload,
                        expected_base=expected_base,
                        budget=formed,
                        registry_version=registry,
                    )
                    assert len(result.terms) == base - expected_base
                    if name in {
                        "terms-six-terms",
                        "terms-value-each-kind",
                        "terms-prefix-splits-character",
                    }:
                        _, encoded = encode_terms(
                            result.terms,
                            id_base=result.id_base,
                            restart_group=result.restart_group,
                        )
                        assert encoded == payload


def test_writes_all_term_storage_forms_in_arrival_order() -> None:
    value = ValueReference(bytes(range(32)), 123)
    terms = (
        IriTerm(b"term"),
        LiteralTerm(b"lexical", datatype=0),
        BlankTerm(b"\xff\0"),
        TripleTerm(2, 0, 1),
        LiteralTerm(value, datatype=0),
        TypedValueTerm(TypedValue(1, True)),
        TypedValueTerm(value),
        LiteralTerm(b"language", language="EN", direction="rtl"),
        LiteralTerm(value, language="en", direction="ltr"),
    )
    content, payload = encode_terms(terms, restart_group=3)
    result = decode_terms(
        cbor2.dumps(content), payload, expected_base=0, budget=FormedBudget(0)
    )
    assert result.terms == terms


def test_id_reservation_uses_runs_despite_a_malformed_base() -> None:
    content = cbor2.dumps(["invalid", [[1, 2]], 2, 4])
    assert reserved_term_count(content, expected_base=0) == 2
    assert reserved_term_count(content, expected_base=(1 << 64) - 1) == 0


@pytest.mark.parametrize(
    "content",
    [bytes.fromhex("980400818201020204"), bytes.fromhex("9f00818201020204ff")],
)
def test_id_reservation_does_not_require_deterministic_outer_content(
    content: bytes,
) -> None:
    assert reserved_term_count(content, expected_base=0) == 2


def test_later_restart_checks_prefix_range_before_restart_rule() -> None:
    content = cbor2.dumps([0, [[1, 2]], 2, 1])
    payload = cbor2.dumps([[[1, [0, 2], [1, 1], b"ab"]], []])
    with pytest.raises(ContentError) as error:
        decode_terms(content, payload, expected_base=0, budget=FormedBudget(2))
    assert error.value.reason == "out_of_range"


@pytest.mark.parametrize(
    ("value", "lexical", "datatype"),
    [
        (TypedValue(1, True), "true", "boolean"),
        (TypedValue(2, (1 << 64) - 1), "18446744073709551615", "nonNegativeInteger"),
        (TypedValue(3, -(1 << 64)), "-18446744073709551616", "integer"),
        (TypedValue(4, TimeValue(-62135596800, 0)), "0001-01-01T00:00:00Z", "dateTime"),
        (
            TypedValue(4, TimeValue(253402300799, 900030000)),
            "9999-12-31T23:59:59.90003Z",
            "dateTime",
        ),
        (TypedValue(5, b"\xffA"), "FF41", "hexBinary"),
    ],
)
def test_typed_values_generate_the_specified_literal(
    value: TypedValue, lexical: str, datatype: str
) -> None:
    assert value.literal() == (lexical, "http://www.w3.org/2001/XMLSchema#" + datatype)

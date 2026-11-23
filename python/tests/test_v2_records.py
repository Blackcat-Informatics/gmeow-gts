# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check binding and suppression grammar against the published P3 vectors."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2 import PayloadBudget, decode_frame
from gts.v2._content import ContentError, UnknownContentCode
from gts.v2.records import BlankPair, TripleLeaf, decode_bindings, decode_suppression

_VECTORS = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P3"
_REASONS = {
    3: "not_deterministic",
    4: "wrong_type",
    5: "wrong_length",
    6: "out_of_range",
    7: "unregistered_value",
    8: "unordered_or_repeated",
    9: "inconsistent_counts",
    10: "forbidden_combination",
}


@pytest.mark.parametrize(
    "name",
    [
        "bindings-basic",
        "bindings-count-mismatch",
        "bindings-digest-length",
        "bindings-leaf-deep",
        "bindings-leaf-tag-unregistered",
        "bindings-order-type-before-count",
        "bindings-triple-leaf-no-pair",
        "suppress-12-4",
        "suppress-annotation-any-count",
        "suppress-digest-length",
        "suppress-entry-count-mismatch",
        "suppress-tag-unregistered",
        "suppress-triple-leaf-no-blank",
    ],
)
def test_record_payload_grammar(name: str) -> None:
    for case in json.loads((_VECTORS / f"{name}.json").read_text())["cases"]:
        raw = (_VECTORS / case["input_file"]).read_bytes()
        report = cbor2.loads(bytes.fromhex(case["read_result"]))
        stored, decoded, _, _, read_decoded, *_ = report[3]
        budget = PayloadBudget(
            stored_bytes_per_frame=stored,
            decoded_bytes_per_frame=decoded,
            decoded_bytes_per_read=read_decoded,
        )
        found = False
        for segment in report[-1][1]:
            for record in segment[-2]:
                _, offset, _, kind, _, payload_state, *_ = record
                if kind not in ([2, 6], [2, 7]) or payload_state != [1]:
                    continue
                found = True
                frame = decode_frame(raw, offset)
                payload = budget.check(frame).decoded
                assert payload is not None
                expected = next((f[1][1] for f in record[-1] if f[0] == 7), None)
                decode = (
                    decode_bindings if frame.frame_type == 6 else decode_suppression
                )
                if expected is not None:
                    with pytest.raises(ContentError) as error:
                        decode(frame.content, payload)
                    assert error.value.reason == _REASONS[expected]
                    continue
                if frame.frame_type == 6:
                    bindings = decode_bindings(frame.content, payload)
                    if name == "bindings-leaf-deep":
                        (binding,) = bindings
                        assert (
                            sum(isinstance(part, TripleLeaf) for part in binding.term)
                            == 100_000
                        )
                        assert any(isinstance(part, BlankPair) for part in binding.term)
                else:
                    decode_suppression(frame.content, payload)
        assert found


def test_unknown_binding_referent_keeps_future_registry_distinction() -> None:
    payload = cbor2.dumps([[[1, 0], [4, bytes(32)]]])
    with pytest.raises(ContentError) as malformed:
        decode_bindings(b"\x81\x01", payload)
    assert malformed.value.reason == "unregistered_value"
    with pytest.raises(UnknownContentCode) as unknown:
        decode_bindings(b"\x81\x01", payload, registry_version=2)
    assert unknown.value.code == 4


def test_suppression_graph_option_tag_is_fixed_across_registries() -> None:
    leaf = [1, 0]
    payload = cbor2.dumps([[2, [leaf, leaf, leaf, [3]]]])
    for registry in (1, 2):
        with pytest.raises(ContentError) as error:
            decode_suppression(b"\x81\x01", payload, registry_version=registry)
        assert error.value.reason == "unregistered_value"

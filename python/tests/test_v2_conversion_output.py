# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exercise the canonical candidate and capacity stops of P9 conversion."""

import json
from pathlib import Path

import cbor2
import pytest

from gts.v2._conversion_output import (
    ConversionCandidate,
    ConversionCapacity,
    prepare_conversion,
)
from gts.v2.conversion_fold import ConversionStop
from gts.wire import content_id, header_id

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"


@pytest.mark.parametrize(
    "name", ["signatures", "suppress-reason-by", "zstd-dictionary"]
)
def test_candidate_bytes_and_source_claims_match_published_conversion(
    name: str,
) -> None:
    fixture = _P9 / f"convert__{name}.json"
    expected = json.loads(fixture.read_bytes())["cases"][0]["result"]["conversions"][0]
    data = bytes.fromhex(expected["source"]["hex"])
    result = prepare_conversion(lambda: data)
    assert isinstance(result, ConversionCandidate)
    assert result.file == bytes.fromhex(expected["result"]["hex"])
    assert [
        result.source.digest,
        result.expected_digest,
        result.source.segments,
        result.source.signatures,
    ] == cbor2.loads(bytes.fromhex(expected["proof"]))


def test_nested_dictionary_candidate_counts_at_the_labelled_size_limit() -> None:
    header = {"gts": "GTS1", "v": 1, "cat": {}}
    header["id"] = header_id(header)
    terms = [
        {"k": 0, "v": "urn:subject"},
        {"k": 0, "v": "urn:predicate"},
        {"k": 0, "v": "urn:object"},
        {"k": 3, "tt": [0, 1, 2]},
        {"k": 3, "tt": [0, 1, 3]},
    ]
    dictionary = {"t": "terms", "prev": header["id"], "d": terms}
    dictionary["id"] = content_id(dictionary)
    quads = {"t": "quads", "prev": dictionary["id"], "d": [[0, 1, 4]]}
    quads["id"] = content_id(quads)
    data = b"".join(cbor2.dumps(item) for item in (header, dictionary, quads))
    # Six quads, one typed ordinal, and one nested dictionary candidate.
    assert prepare_conversion(
        lambda: data, ConversionCapacity(labelled_lines=7)
    ) == ConversionStop("source_exceeds_capacity", 0)
    assert isinstance(
        prepare_conversion(lambda: data, ConversionCapacity(labelled_lines=8)),
        ConversionCandidate,
    )


def test_canonical_frame_can_exceed_a_capacity_that_admits_the_source() -> None:
    data = (_P9 / "convert__result-frame-above-capacity.bin").read_bytes()
    assert isinstance(prepare_conversion(lambda: data), ConversionCandidate)
    # The index decodes to 3773 bytes. Each source payload and the ordinary
    # result frames fit within a 3000-byte decoded limit.
    assert prepare_conversion(
        lambda: data, ConversionCapacity(decoded_bytes_per_frame=3000)
    ) == ConversionStop("source_exceeds_capacity", 0)

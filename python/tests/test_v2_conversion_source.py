# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check first-generation item observations and limits against P9."""

import gzip
import json
from pathlib import Path

import cbor2
import pytest
import zstandard

from gts.codec import Codec, decode_chain
from gts.reader import read
from gts.v2.conversion_source import SourceCapacity, read_source_items
from gts.vectors import zstd_window_frame
from gts.wire import content_id, header_id

_P9 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P9"


def _input(name: str) -> bytes:
    return (_P9 / f"convert__{name}.bin").read_bytes()


@pytest.mark.parametrize(
    ("name", "rule"),
    [
        ("not-first-generation", 1),
        ("later-header-v2", 1),
        ("index-mmr-wrong", 3),
        ("torn-tail", 4),
        ("encrypted-frame", 7),
        ("unknown-codec", 7),
        ("opaque-frame", 12),
        ("unknown-frame-type", 12),
    ],
)
def test_source_item_stops_match_published_offsets(name: str, rule: int) -> None:
    manifest = json.loads((_P9 / f"convert__{name}.json").read_bytes())
    for case in manifest["cases"][0]["result"]["conversions"]:
        data = bytes.fromhex(case["source"]["hex"])
        stop = next(item for item in read_source_items(data) if item.rule is not None)
        assert stop.rule == rule
        assert stop.boundary.offset == case["stopped"]["offset"]


@pytest.mark.parametrize(
    "name",
    ["float-hash-special", "float-hash-shortest", "index-frame", "key-structure"],
)
def test_source_ids_accept_both_float_forms_and_valid_index_claims(name: str) -> None:
    data = _input(name)
    observations = list(read_source_items(data))
    assert observations[0].header
    assert observations[-1].boundary.end == len(data)
    assert all(item.rule is None and not item.exceeds_capacity for item in observations)


def test_in_band_dictionary_decodes_the_same_rows_in_both_read_paths() -> None:
    data = _input("zstd-dictionary")
    _, terms, quads = read_source_items(data)
    assert terms.rule is None and quads.rule is None
    assert (terms.kind, quads.kind, quads.payload) == ("terms", "quads", ((0, 1, 2),))
    legacy = read(data)
    assert len(legacy.terms) == 3
    assert len(legacy.quads) == 1
    assert not legacy.diagnostics


@pytest.mark.parametrize(
    ("cap", "fixture", "offset"),
    [
        (SourceCapacity(items_per_read=4), "capacity-items", 494),
        (SourceCapacity(items_per_read=3), "capacity-items", 404),
        (SourceCapacity(decoded_bytes_per_read=300), "capacity-total", 746),
        (SourceCapacity(decoded_bytes_per_read=200), "capacity-total", 558),
        (SourceCapacity(stored_bytes_per_frame=1), "not-first-generation", 0),
    ],
)
def test_capacity_reports_first_crossing(
    cap: SourceCapacity, fixture: str, offset: int
) -> None:
    observations = list(read_source_items(_input(fixture), cap))
    stop = observations[-1]
    assert stop.exceeds_capacity and stop.rule is None
    assert stop.boundary.offset == offset


def _file_with_payload(data, transforms=()):
    header = {
        "gts": "GTS1",
        "v": 1,
        "cat": {
            1: {"name": "gzip", "cls": "compress"},
            2: {"name": "zstd", "cls": "compress"},
        },
    }
    header["id"] = header_id(header)
    frame = {"t": "blob", "d": data, "prev": header["id"]}
    if transforms:
        frame["x"] = transforms
    frame["id"] = content_id(frame)
    return cbor2.dumps(header) + cbor2.dumps(frame)


@pytest.mark.parametrize(
    ("encode", "transform"), [(gzip.compress, 1), (zstandard.compress, 2)]
)
def test_reverse_transform_checks_exact_output_limit(encode, transform: int) -> None:
    content = b"x" * 8192
    data = _file_with_payload(encode(content), (transform,))
    _, fits = read_source_items(
        data, SourceCapacity(decoded_bytes_per_frame=len(content))
    )
    _, overflows = read_source_items(
        data, SourceCapacity(decoded_bytes_per_frame=len(content) - 1)
    )
    assert fits.payload == content and fits.decoded_size == len(content)
    assert overflows.exceeds_capacity and overflows.rule is None


def test_intermediate_transform_limit_precedes_later_missing_capability() -> None:
    data = _file_with_payload(gzip.compress(b"x" * 8192), (999, 1))
    _, unavailable = read_source_items(data)
    _, exceeds = read_source_items(data, SourceCapacity(decoded_bytes_per_frame=4096))
    assert unavailable.rule == 7
    assert exceeds.exceeds_capacity and exceeds.rule is None


@pytest.mark.parametrize("payload", [(), b"invalid gzip data"])
def test_undefined_transform_precedes_payload_failure(payload) -> None:
    data = _file_with_payload(payload, (999, 1))
    _, failure = read_source_items(data)
    assert failure.rule == 7


def test_negative_source_capacity_is_rejected() -> None:
    with pytest.raises(ValueError, match="uint64"):
        SourceCapacity(decoded_bytes_per_frame=-2)


def test_bounded_gzip_rejects_negative_output_limit() -> None:
    with pytest.raises(ValueError, match="nonnegative"):
        decode_chain([Codec("gzip", "compress")], gzip.compress(b"body"), max_output=-2)


def test_conversion_budget_counts_decoded_zstd_content_instead_of_declared_window() -> (
    None
):
    content = b"fits"
    data = _file_with_payload(zstd_window_frame(28, content), (2,))
    _, frame = read_source_items(data, SourceCapacity(decoded_bytes_per_frame=4))
    assert frame.rule is None and frame.payload == content


def test_correctly_hashed_header_without_catalog_is_not_a_source_header() -> None:
    header = {"gts": "GTS1", "v": 1}
    header["id"] = header_id(header)
    (item,) = read_source_items(cbor2.dumps(header))
    assert item.boundary.offset == 0 and item.rule == 1

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""P1 frame shape, link preimage and payload checks against published bytes."""

import json
from pathlib import Path

import cbor2
import pytest
import zstandard

from gts.v2 import FrameError, decode_frame, decode_header, encode_plain_frame, item_end

P1 = Path(__file__).resolve().parents[2] / "vectors" / "v2" / "P1"


def test_plain_frame_writer_matches_preimage_vector() -> None:
    case = json.loads((P1 / "preimage-identity.json").read_text())["cases"][0]["result"]
    predecessor, frame_type, _, _, content, _, _ = cbor2.loads(
        bytes.fromhex(case["preimage"])
    )
    encoded = encode_plain_frame(predecessor, frame_type, content, b"")
    frame = decode_frame(encoded)
    assert frame.payload_digest.hex() == case["payload_digest"]
    assert frame.identifier.hex() == case["identifier"]
    assert frame.computed_identifier(predecessor) == frame.identifier
    assert b"\x87\x58\x20" + predecessor + frame.identity_fields == bytes.fromhex(
        case["preimage"]
    )


def test_commit_frames_link_and_reproduce_original_bytes() -> None:
    data = (P1 / "link-ok.bin").read_bytes()
    header = decode_header(data)
    predecessor = header.identifier
    assert predecessor is not None
    offset = header.next_offset
    while offset < len(data):
        frame = decode_frame(data, offset)
        assert frame.computed_identifier(predecessor) == frame.identifier
        assert frame.check_decoded_payload(frame.stored_payload)
        assert (
            encode_plain_frame(
                predecessor,
                frame.frame_type,
                cbor2.loads(frame.content),
                bytes(frame.stored_payload),
            )
            == data[offset : frame.next_offset]
        )
        predecessor = frame.identifier
        offset = frame.next_offset


def test_compressed_frame_has_separate_stored_and_decoded_checks() -> None:
    data = (P1 / "frame-overhead-compressed.bin").read_bytes()
    offset = item_end(data)
    frame = decode_frame(data, offset)
    assert frame.check_storage() is True
    decoded = zstandard.ZstdDecompressor().decompress(
        frame.stored_payload, max_output_size=frame.decoded_length
    )
    assert frame.check_decoded_payload(decoded)
    assert not frame.check_decoded_payload(bytes(len(decoded)))
    # Replace one stored payload byte without changing the identity fields.
    raw = cbor2.loads(data[offset : frame.next_offset])
    raw[8] = bytes([raw[8][0] ^ 1]) + raw[8][1:]
    damaged = decode_frame(cbor2.dumps(raw))
    assert damaged.check_storage() is False
    assert damaged.identity_fields == frame.identity_fields


def test_link_uses_content_bytes_even_with_unassigned_option_tag() -> None:
    data = (P1 / "framing-test-content-value.bin").read_bytes()
    terms = decode_frame(data, item_end(data))
    blob = decode_frame(data, terms.next_offset)
    assert blob.computed_identifier(terms.identifier) == blob.identifier


def test_empty_chunk_is_an_envelope_field_error() -> None:
    data = (P1.parent / "P4" / "chunk-empty.bin").read_bytes()
    with pytest.raises(FrameError) as error:
        decode_frame(data, item_end(data))
    assert error.value.reason == "out_of_range"


@pytest.mark.parametrize(
    "fixture, reason",
    [
        ("frame-flags-order", "forbidden_combination"),
        ("frame-signatures-unordered", "unordered_or_repeated"),
        ("frame-unknown-content-bad-utf8", "wrong_type"),
        ("stored-check-short", "wrong_length"),
    ],
)
def test_envelope_reports_first_field_violation(fixture: str, reason: str) -> None:
    data = (P1 / f"{fixture}.bin").read_bytes()
    header = decode_header(data)
    with pytest.raises(FrameError) as error:
        decode_frame(
            data, header.next_offset, registry_version=header.header.registry_version
        )
    assert error.value.reason == reason

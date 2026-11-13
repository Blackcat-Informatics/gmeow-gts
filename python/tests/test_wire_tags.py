# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""The reference reader never lets its CBOR library interpret a tag.

GTS gives no CBOR tag a meaning except the self-describe tag around a Header (§3).
If the library built a set, a decimal or a date from a tagged item, what the reader
accepted would depend on the library release: tag 258 over a text string was a set
in one release and a decode error in the next, and a decode error at an item is a
torn append (§3), which ends the fold. These tests pin the outcome that does not
depend on the library: a tagged item is an uninterpreted :class:`cbor2.CBORTag`,
a well-formed item that is not a frame is a damaged frame, and the fold goes on.
"""

from __future__ import annotations

import io
import struct
from collections.abc import Mapping
from pathlib import Path

import cbor2
import pytest

from gts import read
from gts.codec import CodecUnavailableError
from gts.crypto import (
    InMemoryKeys,
    Signer,
    decrypt0,
    encrypt0,
    recipient_kid,
    sign_id,
    verify_sig,
)
from gts.wire import canonical, decoder, iter_items, loads

VECTORS = Path(__file__).resolve().parents[2] / "vectors"


def _tag_head(tag: int) -> bytes:
    if tag < 24:
        return bytes([0xC0 | tag])
    if tag < 256:
        return bytes([0xD8, tag])
    if tag < 65536:
        return b"\xd9" + struct.pack(">H", tag)
    if tag < 2**32:
        return b"\xda" + struct.pack(">I", tag)
    return b"\xdb" + struct.pack(">Q", tag)


# Payload shapes that make a library's tag decoders succeed or fail: integers
# (dates), byte and text strings (big numbers, regular expressions, UUIDs, network
# addresses), arrays (sets, rationals, decimal fractions, big floats, references)
# and a float part (the shape a newer release rejects and an older one accepted).
_PAYLOADS = [
    (b"\x00", 0),
    (b"\x41\x01", b"\x01"),
    (b"\x62ab", "ab"),
    (b"\x82\x01\x02", [1, 2]),
    (b"\x82\x21\xf9\x3e\x00", [-2, 1.5]),
    (b"\xa0", {}),
]
_TAGS = [*range(65536), 2**32 - 1, 2**32, 2**63, 2**64 - 1]


def _plain(value: object) -> object:
    """Tuples and frozen maps, as the library returns tag content, as plain ones."""
    if isinstance(value, list | tuple):
        return [_plain(v) for v in value]
    if isinstance(value, Mapping):
        return {k: _plain(v) for k, v in value.items()}
    return value


def test_loads_leaves_every_tag_uninterpreted() -> None:
    """Tag space scan: a newer ``cbor2`` that interprets one more tag fails here."""
    for tag in _TAGS:
        head = _tag_head(tag)
        for payload, value in _PAYLOADS:
            got = loads(head + payload)
            assert isinstance(got, cbor2.CBORTag), (tag, payload.hex(), got)
            assert got.tag == tag, (tag, payload.hex(), got)
            assert _plain(got.value) == value, (tag, payload.hex(), got)


def test_decoder_leaves_tags_uninterpreted_in_a_sequence() -> None:
    """The streaming decoder the reader uses behaves as :func:`loads` does."""
    data = b"".join(_tag_head(t) + p for t in (4, 258, 55799) for p, _ in _PAYLOADS)
    buf = io.BytesIO(data)
    dec = decoder(buf)
    count = 0
    while buf.tell() < len(data):
        assert isinstance(dec.decode(), cbor2.CBORTag)
        count += 1
    assert count == 3 * len(_PAYLOADS)


def test_self_referential_tag_is_data_not_a_crash() -> None:
    """Tag 28 over tag 29 reference 0, as a map key or set element, crashed 6.1.5."""
    as_key = loads(bytes.fromhex("a1d81cd99999d81d0000"))
    assert isinstance(as_key, dict)
    (key,) = as_key
    assert key == cbor2.CBORTag(28, cbor2.CBORTag(39321, cbor2.CBORTag(29, 0)))
    in_set = loads(bytes.fromhex("d9010281d81cd99999d81d00"))
    assert in_set == cbor2.CBORTag(258, [key])


def test_map_inside_tag_content_is_still_key_sorted_for_hashing() -> None:
    """Tag content may arrive as a tuple; its maps are ordered like any other (§4)."""
    # tag 6000 over [{"b": 1, "a": 2}]
    unsorted = bytes.fromhex("d9177081a2616201616102")
    ordered = bytes.fromhex("d9177081a2616102616201")
    assert canonical(loads(unsorted)) == ordered


_STRAY_VECTORS = [
    ("28i-stray-set-between-frames", "frame is not a map"),
    ("28j-stray-set-last-item", "frame is not a map"),
    ("28k-stray-decimal-between-frames", "frame is not a map"),
    ("28l-stray-decimal-last-item", "frame is not a map"),
    ("28m-stray-self-reference-in-set", "frame is not a map"),
    ("28n-stray-self-reference-as-key", "frame self-hash mismatch"),
]


@pytest.mark.parametrize(("name", "detail"), _STRAY_VECTORS)
def test_stray_tagged_item_is_a_damaged_frame_and_the_fold_goes_on(
    name: str, detail: str
) -> None:
    """A well-formed tagged item that is not a frame never ends the fold (§3, §3.1).

    §3 allows only an incomplete item at end of input to be a torn append. The
    stray item here is complete, so it is reported as a damaged frame; the frames
    after it, and the quad they carry, still fold.
    """
    g = read((VECTORS / f"{name}.gts").read_bytes())
    assert [(d.code, d.detail) for d in g.diagnostics] == [("DamagedFrame", detail)]
    assert len(g.terms) == 3
    assert len(g.quads) == 1
    items, torn = iter_items((VECTORS / f"{name}.gts").read_bytes())
    assert torn is None
    assert len(items) == 4


def test_stray_tagged_item_does_not_open_a_segment() -> None:
    """Only tag 55799 may wrap a Header (§3); under another tag it is a stray item."""
    data = (VECTORS / "01-minimal.gts").read_bytes()
    header = data[3 : iter_items(data)[0][1][0]]  # the Header without tag 55799
    other_tag = _tag_head(6000) + header
    g = read(data + other_tag)
    assert [(d.code, d.detail) for d in g.diagnostics] == [
        ("DamagedFrame", "frame is not a map")
    ]
    assert len(g.segment_heads) == 1


_SIGNER = Signer.generate("k")
_SEALED_KEY = bytes(32)


def _retag(cose: bytes, tag: int) -> bytes:
    """Replace the outer COSE tag (one byte, 16 or 18) with ``tag``."""
    assert cose[0] in {0xD0, 0xD2}
    return _tag_head(tag) + cose[1:]


def test_cose_sign1_under_another_tag_has_one_deterministic_outcome() -> None:
    """The tag number is not signed content; every call, in any process, agrees.

    With a library that builds a Python set for tag 258, unpacking the message
    depended on set order and a call could return a different status from the next.
    """
    keys = InMemoryKeys(verifiers={"k": _SIGNER.key.public_key()})
    frame_id = bytes(32)
    cose = sign_id(frame_id, _SIGNER)
    results = {verify_sig(_retag(cose, 258), frame_id, keys) for _ in range(20)}
    assert len(results) == 1


def test_cose_encrypt0_under_another_tag_has_one_deterministic_outcome() -> None:
    keys = InMemoryKeys(content={"k": _SEALED_KEY})
    blob = encrypt0(b"payload", "k", _SEALED_KEY)
    outcomes = {decrypt0(_retag(blob, 258), keys) for _ in range(20)}
    assert len(outcomes) == 1
    assert recipient_kid(_retag(blob, 258)) == "k"


@pytest.mark.parametrize(
    "blob",
    [
        b"",
        b"\xff",
        b"\x1f",
        b"\x80",
        bytes.fromhex("83408040"),  # unprotected header is an empty array
        bytes.fromhex("8340a040"),
        bytes.fromhex("d9010283408040"),  # tag 258 over that array
        bytes.fromhex("d90102420102"),  # tag 258 over a byte string
        bytes.fromhex("d90102626162"),  # tag 258 over a text string
        bytes.fromhex("c48221f93e00"),  # decimal fraction, float part
        bytes.fromhex("d81cd99999d81d00"),  # self-referential tag
    ],
)
def test_malformed_cose_is_reported_never_raised(blob: bytes) -> None:
    """A malformed COSE structure is reported as such, whatever CBOR it holds."""
    keys = InMemoryKeys(
        verifiers={"k": _SIGNER.key.public_key()}, content={"k": _SEALED_KEY}
    )
    assert verify_sig(blob, bytes(32), keys) == ("invalid", None)
    assert recipient_kid(blob) is None
    with pytest.raises(CodecUnavailableError):
        decrypt0(blob, keys)

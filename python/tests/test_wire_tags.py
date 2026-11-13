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
from gts.wire import blake3_256, canonical, decoder, iter_items, loads

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


# Tags 2 and 3 are integers beyond 64 bits, not meaning GTS invents: a frame is hashed
# over the integer, as every other engine does, so they are decoded to it (R3 below).
_BIGNUM_TAGS = (2, 3)


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
        if tag in _BIGNUM_TAGS:
            continue
        head = _tag_head(tag)
        for payload, value in _PAYLOADS:
            got = loads(head + payload)
            assert isinstance(got, cbor2.CBORTag), (tag, payload.hex(), got)
            assert got.tag == tag, (tag, payload.hex(), got)
            assert _plain(got.value) == value, (tag, payload.hex(), got)


def test_bignum_tags_decode_to_the_integer_and_other_content_stays_tagged() -> None:
    assert loads(bytes.fromhex("c24105")) == 5
    assert loads(bytes.fromhex("c349010000000000000000")) == -(2**64) - 1
    assert loads(bytes.fromhex("c2626162")) == cbor2.CBORTag(2, "ab")
    assert loads(bytes.fromhex("c38101")) == cbor2.CBORTag(3, [1])


def test_frame_with_a_non_preferred_bignum_is_a_self_hash_mismatch() -> None:
    """The id covers bytes as received; a bignum that is not preferred form is damage.

    Rust, Go, TypeScript and Python before this change report the frame damaged,
    because they re-encode the integer 5 as one byte, not as ``c2 41 05``.
    """
    data = (VECTORS / "01-minimal.gts").read_bytes()
    prev = iter_items(data)[0][-1][1]["id"]
    body = bytes.fromhex("a36174627a7a6178c2410564707265765820") + prev
    ident = blake3_256(body)
    frame = bytes.fromhex("a46174627a7a6178c241056269645820")
    frame += ident + bytes.fromhex("64707265765820") + prev
    g = read(data + frame)
    assert [(d.code, d.detail) for d in g.diagnostics] == [
        ("DamagedFrame", "frame self-hash mismatch")
    ]
    assert [o.reason for o in g.opaque] == ["damaged"]


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


def test_header_under_a_foreign_tag_still_opens_a_segment() -> None:
    """A tagged Header starts a new segment and is rejected, never folded into the last.

    The second segment's frames must not fold into the first segment's term table:
    applying file-global term ids across a boundary is the one forbidden outcome (§17).
    Rust, Go and TypeScript read the same file the same way.
    """
    data = (VECTORS / "01-minimal.gts").read_bytes()
    items, _ = iter_items(data)
    header = data[3 : items[1][0]]  # the Header without tag 55799
    frames = data[items[1][0] :]
    g = read(data + _tag_head(6000) + header + frames)
    assert [(d.code, d.detail) for d in g.diagnostics] == [
        ("DamagedFrame", "invalid header: unexpected CBOR tag 6000 on the header item")
    ]
    assert len(g.terms) == 3
    assert len(g.quads) == 1


_SIGNER = Signer.generate("k")
_SEALED_KEY = bytes(32)


def _retag(cose: bytes, tag: int) -> bytes:
    """Replace the outer COSE tag (one byte, 16 or 18) with ``tag``."""
    assert cose[0] in {0xD0, 0xD2}
    return _tag_head(tag) + cose[1:]


def test_cose_sign1_under_another_tag_is_read_as_the_message_inside() -> None:
    """The outer tag number is not signed content and is ignored, as Rust and Go do.

    With a library that builds a Python set for tag 258, unpacking the message
    depended on set order and the outcome could differ between calls and processes.
    """
    keys = InMemoryKeys(verifiers={"k": _SIGNER.key.public_key()})
    frame_id = bytes(32)
    cose = sign_id(frame_id, _SIGNER)
    for tag in (258, 6000, 4):
        assert verify_sig(_retag(cose, tag), frame_id, keys) == ("valid", "k")


def test_cose_encrypt0_under_another_tag_is_read_as_the_message_inside() -> None:
    keys = InMemoryKeys(content={"k": _SEALED_KEY})
    blob = encrypt0(b"payload", "k", _SEALED_KEY)
    for tag in (258, 6000, 4):
        assert decrypt0(_retag(blob, tag), keys) == b"payload"
        assert recipient_kid(_retag(blob, tag)) == "k"


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


def test_suppression_naming_ids_of_a_damaged_terms_frame_does_not_raise() -> None:
    """One flipped bit in a later segment's ``terms`` frame left its ids unresolved.

    ``read`` raised ``IndexError`` while re-interning the suppression's term ids.
    The id is now kept as received, as Rust, Go and TypeScript do, and the damage
    is the one diagnostic.
    """
    data = (VECTORS / "18-cross-segment-suppression.gts").read_bytes()
    items, _ = iter_items(data)
    flipped = bytearray(data)
    flipped[items[4][0] + 30] ^= 0x01
    g = read(bytes(flipped))
    assert [d.code for d in g.diagnostics] == ["DamagedFrame"]
    assert len(g.suppressions) == 1

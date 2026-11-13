# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Wire primitives: deterministic CBOR, BLAKE3 content-ids, and the id/prev rule.

A frame's ``"id"`` is ``BLAKE3-256`` of the deterministic CBOR (RFC 8949 §4.2) of its
content — every key except ``"id"`` and ``"sig"`` (§6, §9.1). The Header is hashed the
same way, excluding only ``"id"`` (§5). ``"prev"`` names the previous item's ``"id"``;
the first frame's ``"prev"`` is the Header's ``"id"``.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import IO, Any

import cbor2
from blake3 import blake3

# CBOR self-describe tag (RFC 8949 §3.4.6); MAY prefix the Header item (§3).
SELF_DESCRIBE_TAG = 55799

MAGIC = "GTS1"
VERSION = 1

# Frame map keys excluded from the self-hash preimage.
_ID_EXCLUDED = ("id", "sig")

# CBOR tags that ``cbor2`` gives a meaning to when it decodes (date and time, big
# numbers, decimal fractions and big floats, shared and string references,
# rationals, regular expressions, MIME messages, UUIDs, IP addresses, sets, complex
# numbers and the self-describe tag). GTS gives none of them a meaning, so every one
# is decoded to an uninterpreted :class:`cbor2.CBORTag`. Without this the set of
# accepted inputs, and what a decoded value looks like, would depend on the library
# and its version: tag 258 over a text string became a set under one release and a
# decode error under the next, which the reader treats as a torn append. A new
# ``cbor2`` release that interprets another tag is caught by
# ``tests/test_wire_tags.py``, which scans the tag space.
_LIBRARY_TAGS = (
    0,
    1,
    2,
    3,
    4,
    5,
    25,
    28,
    29,
    30,
    35,
    36,
    37,
    52,
    54,
    100,
    256,
    258,
    260,
    261,
    1004,
    43000,
    SELF_DESCRIBE_TAG,
)


def _keep_tag(tag: int) -> Callable[[object, bool], object]:
    """Return a ``cbor2`` semantic decoder that keeps ``tag`` and its content as is."""

    def keep(value: object, _immutable: bool) -> object:
        return cbor2.CBORTag(tag, value)

    return keep


def _bignum(tag: int) -> Callable[[object, bool], object]:
    """Return a semantic decoder for tag 2 or 3 (RFC 8949 §3.4.3).

    A byte string becomes the integer it carries, as in Rust, Go and TypeScript, so
    a frame is re-encoded in preferred form and a non-preferred bignum is a self-hash
    mismatch there and here. Any other content stays an uninterpreted tag.
    """

    def decode(value: object, _immutable: bool) -> object:
        if not isinstance(value, bytes):
            return cbor2.CBORTag(tag, value)
        n = int.from_bytes(value, "big")
        return n if tag == 2 else -1 - n

    return decode


# Tags 2 and 3 are how CBOR carries integers beyond 64 bits; GTS hashes frames over the
# integer, so they are decoded to it (see :func:`_bignum`). Every other tag is kept.
_TAG_PRESERVING = {
    **{tag: _keep_tag(tag) for tag in _LIBRARY_TAGS if tag not in (2, 3)},
    2: _bignum(2),
    3: _bignum(3),
}


# Decoder limits, stated rather than inherited from the library. They equal the
# defaults of both ``cbor2`` 6.1.5 and 6.2.1, so no corpus file folds differently:
# nesting beyond 400 levels is refused (a decode failure, reported as a torn
# append), indefinite lengths are accepted (every engine reads them), and a map
# with a repeated key keeps its last value, as the fold has always seen it.
_OPTIONS: dict[str, Any] = {
    "max_depth": 400,
    "allow_indefinite": True,
    "allow_duplicate_keys": True,
}


def decoder(fp: IO[bytes]) -> cbor2.CBORDecoder:
    """Return a CBOR decoder over ``fp`` that leaves every tag uninterpreted.

    A tagged item surfaces as :class:`cbor2.CBORTag` whatever the tag number, so the
    reader treats a tag the same way under every ``cbor2`` release.
    """
    return cbor2.CBORDecoder(fp, semantic_decoders=_TAG_PRESERVING, **_OPTIONS)


def loads(data: bytes) -> Any:
    """Decode one CBOR item from ``data`` with every tag left uninterpreted."""
    return cbor2.loads(data, semantic_decoders=_TAG_PRESERVING, **_OPTIONS)


def _deterministic(obj: object) -> object:
    """Recursively order map keys per RFC 8949 §4.2 (bytewise on encoded keys).

    ``cbor2``'s ``canonical=True`` implements the older RFC 7049 ordering
    (length-first), which DIVERGES from RFC 8949 on keys like ``"x"`` vs
    ``"id"``. The spec mandates 8949, so maps are re-built here in 8949 order
    and encoded with the default (insertion-order, definite-length,
    shortest-int) encoder. Tags recurse into their value, and the library hands some
    tag content back as tuples, which are arrays like lists.
    """
    if isinstance(obj, cbor2.CBORTag):
        return cbor2.CBORTag(obj.tag, _deterministic(obj.value))
    if isinstance(obj, Mapping):
        entries = [(cbor2.dumps(k), k, _deterministic(v)) for k, v in obj.items()]
        entries.sort(key=lambda e: e[0])
        return {k: v for _, k, v in entries}
    if isinstance(obj, list | tuple):
        return [_deterministic(v) for v in obj]
    return obj


def canonical(obj: object) -> bytes:
    """Encode an object as deterministic CBOR (RFC 8949 §4.2).

    Definite lengths and shortest-form integers come from the default
    encoder; key ordering is enforced by :func:`_deterministic`.
    """
    return cbor2.dumps(_deterministic(obj))


def blake3_256(data: bytes) -> bytes:
    """Return the 32-byte BLAKE3-256 digest of ``data``."""
    return blake3(data).digest()


def digest_str(data: bytes) -> str:
    """Return a ``blake3:<hex>`` content digest for inline blob addressing (§12)."""
    return "blake3:" + blake3_256(data).hex()


def content_id(frame: Mapping[str, object]) -> bytes:
    """Compute a frame's ``"id"`` over its content (excluding ``"id"``/``"sig"``)."""
    content = {k: v for k, v in frame.items() if k not in _ID_EXCLUDED}
    return blake3_256(canonical(content))


def header_id(header: Mapping[str, object]) -> bytes:
    """Compute the Header's genesis ``"id"`` (excluding only ``"id"``) — §5."""
    content = {k: v for k, v in header.items() if k != "id"}
    return blake3_256(canonical(content))


def iter_items(data: bytes) -> tuple[list[tuple[int, object]], int | None]:
    """Decode a CBOR Sequence into ``(offset, item)`` pairs plus a torn marker.

    Detects a torn append (a partial trailing item) by position: at an item boundary
    the offset is either end-of-data (clean end) or the start of a complete item; a
    decode failure there is a torn append (§3). Survivors are returned regardless, so
    a reader can fold the intact prefix.

    Returns:
        ``(items, torn_offset)`` — ``torn_offset`` is ``None`` for a clean end, or the
        byte offset of the incomplete trailing item.
    """
    import io

    buf = io.BytesIO(data)
    dec = decoder(buf)
    out: list[tuple[int, object]] = []
    length = len(data)
    torn: int | None = None
    while True:
        start = buf.tell()
        if start == length:
            break
        try:
            item = dec.decode()
        except cbor2.CBORDecodeError:  # partial (EOF) or corrupt trailing item
            torn = start
            break
        out.append((start, item))
    return out, torn


def is_header_item(item: object) -> bool:
    """§3.1 boundary rule: a map carrying ``"gts"`` and lacking ``"t"``.

    A tag around the map does not change the test, so an item under a foreign tag
    still starts a new segment; :func:`unwrap_header` then rejects the tag and
    reports an invalid header, as every other engine does.
    """
    if isinstance(item, cbor2.CBORTag):
        item = item.value
    return isinstance(item, Mapping) and "gts" in item and "t" not in item


def unwrap_header(item: object) -> Mapping[str, object]:
    """Return the Header map, unwrapping the optional self-describe tag (§3)."""
    if isinstance(item, cbor2.CBORTag):
        if item.tag != SELF_DESCRIBE_TAG:
            msg = f"unexpected CBOR tag {item.tag} on the header item"
            raise ValueError(msg)
        item = item.value
    if not isinstance(item, Mapping):
        msg = "header item is not a CBOR map"
        raise ValueError(msg)
    return item

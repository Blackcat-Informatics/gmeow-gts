# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Read and write the GTS v2 header record (P1 §§1, 2.4, 2.7, 4).

These functions operate on one header item at a known offset. File framing,
generation detection and lineage comparison belong to the file reader.
"""

from dataclasses import dataclass

import cbor2
from blake3 import blake3

from ._decode import WireError, _Decoder

_REGISTRY_VERSION = 1
_BLAKE3_256 = 1
_CANONICAL_FORM = 1
_LAST_PROFILE = 6


class HeaderError(WireError):
    """A P1 header rejection reason and absolute byte offset.

    If an argument or string body is truncated, the offset is where it begins.
    """


@dataclass(frozen=True, slots=True)
class Lineage:
    previous_head: bytes
    ordinal: int


@dataclass(frozen=True, slots=True)
class Header:
    registry_version: int = _REGISTRY_VERSION
    hash_algorithm: int = _BLAKE3_256
    profile: int | None = None
    canonical: int | None = None
    lineage: Lineage | None = None


@dataclass(frozen=True, slots=True)
class DecodedHeader:
    header: Header
    next_offset: int
    identifier: bytes | None


def _opener_registry_code(decoder: _Decoder, registry_version: int, last: int) -> int:
    if registry_version <= _REGISTRY_VERSION:
        return decoder.uint(minimum=1, maximum=last, reason="unregistered_value")
    return decoder.uint(minimum=1, reason="unregistered_value")


def _decode_opener_fields(
    decoder: _Decoder, *, require_known_hash: bool = False
) -> tuple[int, int, int | None]:
    registry = decoder.uint(minimum=1)
    hash_registry = _REGISTRY_VERSION if require_known_hash else registry
    algorithm = _opener_registry_code(decoder, hash_registry, _BLAKE3_256)
    profile = decoder.option(
        lambda: decoder.uint(minimum=1, reason="unregistered_value")
    )
    return registry, algorithm, profile


def _decode_lineage(decoder: _Decoder, digest_size: int | None) -> Lineage | None:
    def read_lineage() -> Lineage:
        decoder.record(2)
        head = decoder.bytes(digest_size)
        return Lineage(bytes(head), decoder.uint(minimum=1))

    return decoder.option(read_lineage)


def decode_header(data: bytes, offset: int = 0) -> DecodedHeader:
    """Read one header and return the next item position and P1 header identifier.

    A supported hash is evaluated on the original bytes. A newer registry
    with an unknown hash returns a header whose identifier is absent.
    """
    if not 0 <= offset <= len(data):
        raise ValueError("header offset is outside the input")

    decoder = _Decoder(data, offset, HeaderError)
    decoder.record(7)
    decoder.uint(minimum=1, maximum=1, reason="unregistered_value")
    decoder.uint(minimum=2, maximum=2)
    registry_version, algorithm, profile = _decode_opener_fields(decoder)
    has_hash = algorithm == _BLAKE3_256
    canonical = decoder.option(
        lambda: _opener_registry_code(decoder, registry_version, _CANONICAL_FORM)
    )
    lineage = _decode_lineage(decoder, 32 if has_hash else None)
    header = Header(registry_version, algorithm, profile, canonical, lineage)

    identifier = None
    if has_hash:
        encoded = memoryview(data)[offset : decoder.offset]
        identifier = blake3(encoded, derive_key_context="gts2 header").digest()
    return DecodedHeader(header, decoder.offset, identifier)


def _encode_opener_fields(
    header: Header,
) -> tuple[int, int, list[object], list[object]]:
    if (
        type(header.registry_version) is not int
        or header.registry_version != _REGISTRY_VERSION
    ):
        raise ValueError("only registry version 1 can be written")
    if type(header.hash_algorithm) is not int or header.hash_algorithm != _BLAKE3_256:
        raise ValueError("only BLAKE3-256 can be written")
    if header.profile is not None and (
        type(header.profile) is not int or not 1 <= header.profile <= _LAST_PROFILE
    ):
        raise ValueError("profile must be assigned in the profile table")
    lineage: list[object] = [1]
    if header.lineage is not None:
        previous_head, ordinal = header.lineage.previous_head, header.lineage.ordinal
        if not isinstance(previous_head, bytes) or len(previous_head) != 32:
            raise ValueError("previous_head must be a 32-byte digest")
        if type(ordinal) is not int or not 1 <= ordinal < 1 << 64:
            raise ValueError("lineage ordinal must be a positive uint")
        lineage = [2, [previous_head, ordinal]]
    profile: list[object] = [1] if header.profile is None else [2, header.profile]
    return header.registry_version, header.hash_algorithm, profile, lineage


def encode_header(header: Header) -> bytes:
    """Encode a registry version 1 header for a segment writer.

    A caller sets lineage from the preceding segment and sets the canonical
    claim only when it has produced the corresponding canonical file.
    """
    registry, algorithm, profile, lineage = _encode_opener_fields(header)
    if header.canonical is not None and (
        type(header.canonical) is not int or header.canonical != _CANONICAL_FORM
    ):
        raise ValueError("canonical form must be assigned in registry version 1")
    return cbor2.dumps(
        [
            1,
            2,
            registry,
            algorithm,
            profile,
            [1] if header.canonical is None else [2, header.canonical],
            lineage,
        ]
    )

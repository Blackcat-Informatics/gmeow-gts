# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Frame envelope, identity and stored bytes (P1 §§3.1–3.3, 4.2, 5.3)."""

from dataclasses import dataclass

import cbor2
from blake3 import blake3

from ._decode import WireError, _Decoder
from .framing import item_end

_LAST_FRAME_TYPE = 16
_CHUNK = 8
_EXTERNAL = 12
_COMMIT = 14
_PARITY = 16


class FrameError(WireError):
    """The first envelope field violation in wire order."""


@dataclass(frozen=True, slots=True)
class CompressedStorage:
    codec: int
    check: bytes


@dataclass(frozen=True, slots=True)
class UnknownStorage:
    tag: int
    encoded: memoryview


@dataclass(frozen=True, slots=True)
class Signature:
    algorithm: int
    key_id: bytes
    value: memoryview


@dataclass(frozen=True, slots=True)
class Frame:
    frame_type: int
    critical: bool
    transparent: bool
    content: memoryview
    decoded_length: int
    payload_digest: bytes
    storage: CompressedStorage | UnknownStorage | None
    stored_payload: memoryview
    identifier: bytes
    signatures: tuple[Signature, ...]
    identity_fields: memoryview
    next_offset: int

    def computed_identifier(self, predecessor: bytes) -> bytes:
        """Hash the predecessor and the original bytes of identity elements."""
        return _identifier(predecessor, self.identity_fields)

    def check_storage(self) -> bool | None:
        """Check a compressed frame's stored bytes when its storage tag is known."""
        if isinstance(self.storage, CompressedStorage):
            return _stored_check(self.storage.codec, self.stored_payload) == (
                self.storage.check
            )
        return None

    def check_decoded_payload(self, decoded: bytes | memoryview) -> bool:
        """Check the length and digest of bytes already decoded by the caller."""
        return len(decoded) == self.decoded_length and (
            _payload_digest(decoded) == self.payload_digest
        )


def _identifier(predecessor: bytes, fields: bytes | memoryview) -> bytes:
    if len(predecessor) != 32:
        raise ValueError("frame predecessor must be a 32-byte digest")
    hasher = blake3(derive_key_context="gts2 frame")
    hasher.update(b"\x87\x58\x20")
    hasher.update(predecessor)
    hasher.update(fields)
    return hasher.digest()


def _payload_digest(decoded: bytes | memoryview) -> bytes:
    return blake3(decoded, derive_key_context="gts2 payload").digest()


def _stored_check(codec: int, payload: bytes | memoryview) -> bytes:
    hasher = blake3(cbor2.dumps([codec]), derive_key_context="gts2 stored")
    hasher.update(payload)
    return hasher.digest(length=16)


def _registry_code(decoder: _Decoder, last: int, newer: bool) -> int:
    offset = decoder.offset
    code = decoder.uint(minimum=1, reason="unregistered_value")
    if code > last and not newer:
        raise FrameError("unregistered_value", offset)
    return code


def _content(decoder: _Decoder, frame_type: int) -> None:
    # P1 §3.2 checks the shape here and leaves values to the part that owns
    # the type.
    if frame_type > _LAST_FRAME_TYPE:
        decoder.closed_item()
        return
    if frame_type == 1:
        decoder.record(4)
        decoder.uint()
        for _ in range(decoder.array()):
            decoder.record(2)
            decoder.uint()
            decoder.uint()
        decoder.uint()
        decoder.uint()
        return

    # u = uint, d = digest, m = optional media_type, n = qualified_name
    shapes = (
        "uu",
        "uu",
        "uu",
        "u",
        "u",
        "u",
        "d",
        "dm",
        "dumu",
        "duu",
        "num",
        "uu",
        "ud",
        "ud",
        "uuudd",
    )
    shape = shapes[frame_type - 2]
    decoder.record(len(shape))
    for field in shape:
        match field:
            case "u":
                decoder.uint()
            case "d":
                decoder.bytes(32)
            case "m":
                _media_type_shape(decoder)
            case "n":
                decoder.record(2)
                algorithm = decoder.uint()
                decoder.bytes(32 if algorithm == 1 else None)


def _media_type_shape(decoder: _Decoder) -> None:
    start = decoder.offset
    length = decoder.array()
    if length == 0:
        raise FrameError("wrong_length", start)
    tag = decoder.uint()
    if tag in (1, 2):
        if length != tag:
            raise FrameError("wrong_length", start)
        if tag == 2:
            decoder.text()
    else:
        # The content phase judges the tag. Its raw value participates in
        # the link even when this reader does not recognize it.
        for _ in range(length - 1):
            decoder.closed_item()


def _storage(
    decoder: _Decoder, *, newer: bool, decoded_length: int
) -> CompressedStorage | UnknownStorage | None:
    start = decoder.offset
    size = decoder.array()
    if size == 0:
        raise FrameError("wrong_length", start)
    tag = _registry_code(decoder, 2, newer)
    if tag > 2:
        for _ in range(size - 1):
            decoder.closed_item()
        return UnknownStorage(tag, memoryview(decoder.data)[start : decoder.offset])

    if size != (1 if tag == 1 else 3):
        raise FrameError("wrong_length", start)
    if tag == 1:
        return None
    codec = _registry_code(decoder, 1, newer)
    check = bytes(decoder.bytes(16))
    if decoded_length == 0:
        raise FrameError("forbidden_combination", start)
    return CompressedStorage(codec, check)


def _signatures(decoder: _Decoder, *, newer: bool) -> tuple[Signature, ...]:
    previous = b""
    signatures = []
    for _ in range(decoder.array()):
        start = decoder.offset
        decoder.record(3)
        algorithm = _registry_code(decoder, 2, newer)
        key_id = bytes(decoder.bytes(32))
        value = decoder.bytes()
        encoded = decoder.data[start : decoder.offset]
        if encoded <= previous:
            raise FrameError("unordered_or_repeated", start)
        previous = encoded
        signatures.append(Signature(algorithm, key_id, value))
    return tuple(signatures)


def decode_frame(data: bytes, offset: int = 0, *, registry_version: int = 1) -> Frame:
    """Decode one BLAKE3 frame envelope for a reader of registry version 1.

    Field errors identify where envelope parsing failed. The file reader owns
    recovery, payload decoding and capacity, signature verification, and the
    interpretation of content after the link check.
    """
    if registry_version < 1:
        raise ValueError("registry version must be positive")
    item_end(data, offset)
    decoder = _Decoder(data, offset, FrameError)
    decoder.record(11)
    decoder.uint(minimum=2, maximum=2, reason="unregistered_value")
    identity_start = decoder.offset
    newer = registry_version > 1
    frame_type = _registry_code(decoder, _LAST_FRAME_TYPE, newer)

    offset = decoder.offset
    critical = decoder.boolean()
    if frame_type <= _LAST_FRAME_TYPE and critical != (frame_type < _COMMIT):
        raise FrameError("forbidden_combination", offset)
    offset = decoder.offset
    transparent = decoder.boolean()
    if frame_type <= _LAST_FRAME_TYPE and transparent != (frame_type == _PARITY):
        raise FrameError("forbidden_combination", offset)

    data_view = memoryview(data)
    offset = decoder.offset
    _content(decoder, frame_type)
    content = data_view[offset : decoder.offset]

    decoded_length_offset = decoder.offset
    decoded_length = decoder.uint()
    digest = bytes(decoder.bytes(32))
    identity = data_view[identity_start : decoder.offset]

    offset = decoder.offset
    storage = _storage(decoder, newer=newer, decoded_length=decoded_length)
    if frame_type == _PARITY and storage is not None:
        raise FrameError("forbidden_combination", offset)
    payload = decoder.bytes()
    identifier = bytes(decoder.bytes(32))
    offset = decoder.offset
    signatures = _signatures(decoder, newer=newer)
    if frame_type == _PARITY and signatures:
        raise FrameError("forbidden_combination", offset)
    if (frame_type in (_COMMIT, _EXTERNAL) and decoded_length) or (
        frame_type == _CHUNK and not decoded_length
    ):
        raise FrameError("out_of_range", decoded_length_offset)

    return Frame(
        frame_type,
        critical,
        transparent,
        content,
        decoded_length,
        digest,
        storage,
        payload,
        identifier,
        signatures,
        identity,
        decoder.offset,
    )


def encode_plain_frame(
    predecessor: bytes, frame_type: int, content: list[object], payload: bytes
) -> bytes:
    """Encode an unsigned, uncompressed registry version 1 frame envelope.

    Supply the content record and decoded payload defined by the frame type.
    The caller handles the frame's semantic and segment-level constraints.
    """
    if type(frame_type) is not int or not 1 <= frame_type <= _LAST_FRAME_TYPE:
        raise ValueError("frame type must be assigned in registry version 1")
    if not isinstance(payload, bytes):
        raise TypeError("frame payload must be bytes")

    fields = b"".join(
        cbor2.dumps(field)
        for field in (
            frame_type,
            frame_type < _COMMIT,
            frame_type == _PARITY,
            content,
            len(payload),
            _payload_digest(payload),
        )
    )
    identifier = _identifier(predecessor, fields)
    encoded = (
        b"\x8b\x02"
        + fields
        + b"\x81\x01"
        + cbor2.dumps(payload)
        + cbor2.dumps(identifier)
        + b"\x80"
    )
    decode_frame(encoded)
    return encoded

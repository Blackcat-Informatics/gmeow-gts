# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Read the first-generation source items used by P9 conversion."""

from collections.abc import Iterator
from dataclasses import dataclass
from itertools import islice
from typing import Literal
from zlib import error as ZlibError

from gts.codec import (
    Codec,
    CodecClass,
    CodecUnavailableError,
    PayloadDecodeError,
    PayloadLimitError,
    decode_chain,
)
from gts.mmr import root_for_ids
from gts.wire import MAGIC, SELF_DESCRIBE_TAG

from ._legacy_cbor import (
    MapValue,
    TaggedValue,
    Value,
    _decode_framed_item,
    decode_legacy_item,
    source_id_matches,
)
from .framing import ItemFramingError, _head, item_end

type SourceRule = Literal[1, 2, 3, 4, 7, 8, 12]
_BASELINE = {"identity", "gzip", "zstd"}
_KINDS = {
    "terms",
    "quads",
    "reifies",
    "annot",
    "blob",
    "suppress",
    "snapshot",
    "meta",
    "index",
}
_UNAVAILABLE = Codec("", "encode")
_EMPTY_MAP = MapValue(())


@dataclass(frozen=True, slots=True)
class SourceCapacity:
    stored_bytes_per_frame: int = 1 << 26
    decoded_bytes_per_frame: int = 1 << 26
    decoded_bytes_per_read: int = 1 << 36
    items_per_read: int = 1 << 26

    def __post_init__(self) -> None:
        limits = (
            self.stored_bytes_per_frame,
            self.decoded_bytes_per_frame,
            self.decoded_bytes_per_read,
            self.items_per_read,
        )
        if any(type(limit) is not int or not 0 <= limit < 1 << 64 for limit in limits):
            raise ValueError("source capacity limits must be uint64 values")


_DEFAULT_SOURCE_CAPACITY = SourceCapacity()


@dataclass(frozen=True, slots=True)
class SourceBoundary:
    offset: int
    end: int
    rule: Literal[2, 4] | None = None


@dataclass(frozen=True, slots=True)
class SourceItem:
    boundary: SourceBoundary
    envelope: MapValue
    header: bool = False
    kind: str | None = None
    payload: Value = None
    decoded_size: int = 0
    rule: SourceRule | None = None
    exceeds_capacity: bool = False
    id_holds: bool = False


def source_boundaries(data: bytes) -> Iterator[SourceBoundary]:
    """Report exact item ranges, including a terminal damaged or torn item."""
    offset = 0
    while offset < len(data):
        try:
            end = item_end(data, offset)
        except ItemFramingError as error:
            yield SourceBoundary(
                offset, len(data), 4 if error.reason == "torn_tail" else 2
            )
            return
        yield SourceBoundary(offset, end)
        offset = end


def _fields(item: Value) -> dict[str, Value]:
    if isinstance(item, MapValue):
        return {key: value for key, value in item.entries if isinstance(key, str)}
    return {}


def _valid_catalog(value: Value, dictionaries: Value) -> bool:
    if not isinstance(value, MapValue) or not isinstance(dictionaries, MapValue):
        return False
    names: set[str] = set()
    for name, data in dictionaries.entries:
        if not isinstance(name, str) or not isinstance(data, bytes):
            return False
        names.add(name)
    for key, entry in value.entries:
        fields = _fields(entry)
        if (
            type(key) is not int
            or not 0 <= key < 1 << 64
            or not isinstance(fields.get("name"), str)
            or fields.get("cls") not in ("encode", "compress", "encrypt")
        ):
            return False
        if "dct" in fields:
            reference = fields["dct"]
            if not isinstance(reference, str) or reference not in names:
                return False
    return True


def _catalog(header: dict[str, Value]) -> dict[int, Codec]:
    catalog = header.get("cat")
    dictionaries = _fields(header.get("dct"))
    codecs: dict[int, Codec] = {}
    if isinstance(catalog, MapValue):
        for key, entry in catalog.entries:
            fields = _fields(entry)
            name, cls = fields.get("name"), fields.get("cls")
            if type(key) is not int or key < 0 or not isinstance(name, str):
                continue
            kind: CodecClass
            if cls == "encrypt":
                kind = "encrypt"
            elif cls == "compress":
                kind = "compress"
            elif cls == "encode":
                kind = "encode"
            else:
                continue
            dictionary = None
            if name == "zstd" and "dct" in fields:
                tag = fields["dct"]
                selected = dictionaries.get(tag) if isinstance(tag, str) else None
                if not isinstance(selected, bytes):
                    continue
                dictionary = selected
            if name in _BASELINE or kind == "encrypt":
                codecs[key] = Codec(name, kind, dictionary)
    return codecs


def _index_holds(payload: MapValue, identifiers: list[bytes]) -> bool:
    fields = _fields(payload)
    if "count" not in fields:
        return "head" not in fields and "mmr" not in fields
    count = fields["count"]
    if type(count) is not int or not 0 <= count <= len(identifiers):
        return False
    if count and "head" in fields and fields["head"] != identifiers[count - 1]:
        return False
    return "mmr" not in fields or fields["mmr"] == root_for_ids(
        islice(identifiers, count)
    )


def _wire_payload_size(data: bytes, offset: int, envelope: MapValue) -> int:
    """Measure the original bytes of a directly carried structured payload."""
    _, _, position = _head(data, offset, offset)
    size = 0
    for key, _ in envelope.entries:
        start = item_end(data, position)
        position = item_end(data, start)
        if key == "d":
            size = position - start
    return size


def _plain_payload_shape(kind: str, value: Value, *, present: bool) -> bool:
    if kind in ("terms", "quads", "reifies", "annot"):
        return isinstance(value, tuple)
    if kind == "snapshot" and isinstance(value, MapValue):
        snapshot = _fields(value)
        return all(
            isinstance(snapshot.get(key, ()), tuple)
            for key in ("terms", "quads", "reifies", "annot")
        ) and isinstance(snapshot.get("blobs", _EMPTY_MAP), MapValue)
    if kind in ("snapshot", "index"):
        return isinstance(value, MapValue)
    if kind == "blob":
        return not present or isinstance(value, bytes)
    return True


def _payload(
    data: bytes,
    start: int,
    envelope: MapValue,
    frame: dict[str, Value],
    kind: str,
    codecs: dict[int, Codec],
    limit: int,
) -> tuple[Value, int, Literal[7, 8] | None]:
    value = frame.get("d")
    transforms = frame.get("x", ())
    if not isinstance(transforms, tuple):
        return None, 0, 7
    if not transforms:
        size = (
            len(value)
            if isinstance(value, bytes)
            else _wire_payload_size(data, start, envelope)
        )
        if size > limit:
            raise PayloadLimitError("decoded payload exceeds its byte budget")
        return (
            value,
            size,
            None if _plain_payload_shape(kind, value, present="d" in frame) else 8,
        )
    chain = [
        codecs.get(ident, _UNAVAILABLE) if type(ident) is int else _UNAVAILABLE
        for ident in transforms
    ]
    known = all(codec is not _UNAVAILABLE and codec.cls != "encrypt" for codec in chain)
    if not isinstance(value, bytes):
        return None, 0, 8 if known else 7
    try:
        decoded = decode_chain(chain, value, max_output=limit)
    except CodecUnavailableError:
        return None, 0, 7
    except (PayloadDecodeError, OSError, EOFError, ZlibError):
        return None, 0, 8 if known else 7
    if kind == "blob":
        return decoded, len(decoded), None
    try:
        item = decode_legacy_item(decoded)
    except ItemFramingError:
        return None, len(decoded), 8
    rule: Literal[8] | None = (
        None
        if (
            item.end == len(decoded)
            and _plain_payload_shape(kind, item.value, present=True)
        )
        else 8
    )
    return item.value, len(decoded), rule


def read_source_items(
    data: bytes, capacity: SourceCapacity = _DEFAULT_SOURCE_CAPACITY
) -> Iterator[SourceItem]:
    """Yield bounded P9 item observations for the source fold."""
    codecs: dict[int, Codec] = {}
    previous: bytes | None = None
    identifiers: list[bytes] = []
    decoded_total = 0
    if not data:
        yield SourceItem(SourceBoundary(0, 0), _EMPTY_MAP, rule=1)
        return
    for index, boundary in enumerate(source_boundaries(data)):
        start, end = boundary.offset, boundary.end
        if (
            index >= capacity.items_per_read
            or end - start > capacity.stored_bytes_per_frame
        ):
            yield SourceItem(boundary, _EMPTY_MAP, exceeds_capacity=True)
            return
        if boundary.rule is not None:
            yield SourceItem(boundary, _EMPTY_MAP, rule=boundary.rule)
            return
        raw = _decode_framed_item(data, start, end).value
        item = raw.value if isinstance(raw, TaggedValue) else raw
        fields = _fields(item)
        header = "gts" in fields and "t" not in fields
        if header and isinstance(item, MapValue):
            valid = (
                fields["gts"] == MAGIC
                and type(fields.get("v")) is int
                and fields["v"] == 1
                and _valid_catalog(fields.get("cat"), fields.get("dct", _EMPTY_MAP))
                and (
                    not isinstance(raw, TaggedValue) or raw.number == SELF_DESCRIBE_TAG
                )
            )
            rule: SourceRule | None = (
                1 if not valid else None if source_id_matches(item, header=True) else 3
            )
            codecs = _catalog(fields)
            identifier = fields.get("id")
            previous = identifier if isinstance(identifier, bytes) else None
            identifiers.clear()
            yield SourceItem(boundary, item, header=True, rule=rule)
            continue
        kind = fields.get("t")
        if not isinstance(raw, MapValue) or not isinstance(kind, str):
            yield SourceItem(boundary, _EMPTY_MAP, rule=1 if index == 0 else 2)
            continue
        if index == 0:
            yield SourceItem(boundary, raw, kind=kind, rule=1)
            continue
        rule = None
        try:
            payload, size, rule = _payload(
                data, start, raw, fields, kind, codecs, capacity.decoded_bytes_per_frame
            )
        except PayloadLimitError:
            yield SourceItem(boundary, _EMPTY_MAP, exceeds_capacity=True)
            return
        decoded_total += size
        if decoded_total > capacity.decoded_bytes_per_read:
            yield SourceItem(boundary, _EMPTY_MAP, exceeds_capacity=True)
            return
        id_holds = source_id_matches(raw)
        if (
            not id_holds
            or previous is None
            or fields.get("prev") != previous
            or (
                kind == "index"
                and isinstance(payload, MapValue)
                and not _index_holds(payload, identifiers)
            )
        ):
            rule = 3
        elif rule is None and kind not in _KINDS:
            rule = 12
        identifier = fields.get("id")
        previous = identifier if isinstance(identifier, bytes) else None
        identifiers.append(previous if previous is not None else b"")
        yield SourceItem(
            boundary,
            raw,
            kind=kind,
            payload=payload,
            decoded_size=size,
            rule=rule,
            id_holds=id_holds,
        )

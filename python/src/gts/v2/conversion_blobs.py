# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Collect and project first-generation blob declarations under P9."""

from dataclasses import dataclass
from typing import Literal

from gts.wire import blake3_256

from ._conversion_fields import contains_invalid_text, extensions, frame_values
from ._legacy_cbor import InvalidText, MapValue, Value, encode_legacy_value
from .blobs import BlobDeclaration, BlobLocation
from .conversion_graph import ConversionGraph
from .rebuild import unit_digest
from .record_fold import RecordLocation, ResolvedBinding
from .records import BlobReference

_EMPTY_MAP = MapValue(())
_BLOB_FIELDS = {"digest", "mt", "rep"}
type _Digest = bytes | str
type _Rule = Literal[6, 8, 9, 14]


class BlobSourceError(ValueError):
    """A P9 item rule triggered by a source blob field."""

    def __init__(self, rule: _Rule, location: BlobLocation, detail: str) -> None:
        self.rule = rule
        self.location = location
        super().__init__(detail)

    def _order(self) -> tuple[int, int, int]:
        return self.location.segment, self.location.position, self.rule


@dataclass(frozen=True, slots=True)
class _Declaration:
    digest: _Digest
    body: bytes | None
    pub: MapValue
    location: BlobLocation


@dataclass(frozen=True, slots=True)
class BlobImage:
    node: int | None
    reference: BlobReference | None


@dataclass(frozen=True, slots=True)
class ConvertedBlobs:
    declarations: tuple[BlobDeclaration, ...]
    bindings: frozenset[ResolvedBinding]
    images: dict[_Digest, BlobImage]
    frames: dict[RecordLocation, list[BlobImage]]


def _matches(value: Value, digest: bytes) -> bool:
    return value == digest or value == _digest_text(digest)


def _digest_text(value: bytes) -> str:
    return "blake3:" + value.hex()


def _earlier(stop: BlobSourceError | None, other: BlobSourceError) -> BlobSourceError:
    return other if stop is None or other._order() < stop._order() else stop


def _invalid_pub_text(key: Value, value: Value) -> bool:
    if key == "digest":
        return False
    if key in ("mt", "rep"):
        return isinstance(value, InvalidText)
    return contains_invalid_text(key) or contains_invalid_text(value)


def _merge_pub(
    group: list[_Declaration], stop: BlobSourceError | None
) -> tuple[MapValue, BlobSourceError | None]:
    by_key: dict[bytes, tuple[tuple[Value, Value], BlobLocation]] = {}
    for item in group:
        for key, value in item.pub.entries:
            by_key[encode_legacy_value(key)] = ((key, value), item.location)
    entries = []
    for encoded in sorted(by_key):
        pair, location = by_key[encoded]
        entries.append(pair)
        key, value = pair
        if _invalid_pub_text(key, value):
            stop = _earlier(
                stop,
                BlobSourceError(9, location, "source blob pub contains invalid UTF-8"),
            )
    return MapValue(tuple(entries)), stop


class ConversionBlobs:
    """Resolve the file's digest texts only after all its digests are held."""

    def __init__(self) -> None:
        self._declarations: list[_Declaration] = []
        self._held: dict[str, bytes] = {}
        self._stop: BlobSourceError | None = None

    def _note(self, rule: _Rule, location: BlobLocation, detail: str) -> None:
        self._stop = _earlier(self._stop, BlobSourceError(rule, location, detail))

    def hold_digest(self, value: Value) -> None:
        """Remember a stored frame identifier or a digest in a digest position."""
        if isinstance(value, bytes) and len(value) == 32:
            self._held[_digest_text(value)] = value

    def resolve_digest(self, value: _Digest) -> _Digest:
        """Resolve a digest position against all digests held by the source."""
        return self._held.get(value, value) if isinstance(value, str) else value

    def add_frame(
        self, frame: MapValue, payload: Value, location: BlobLocation
    ) -> None:
        """Judge one blob frame and collect its declaration before merging pubs."""
        fields = {key: value for key, value in frame.entries if isinstance(key, str)}
        pub_field = fields.get("pub", _EMPTY_MAP)
        pub = pub_field if isinstance(pub_field, MapValue) else _EMPTY_MAP
        metadata = {
            key: value
            for key, value in pub.entries
            if isinstance(key, str) and key in _BLOB_FIELDS
        }
        stored = metadata.get("digest")
        self.hold_digest(stored)
        body = payload if isinstance(payload, bytes) else None
        digest: _Digest | None = blake3_256(body) if body is not None else None
        if isinstance(digest, bytes):
            if "digest" in metadata and not _matches(stored, digest):
                self._note(6, location, "source blob digest does not match its bytes")
        else:
            if "d" in fields:
                self._note(8, location, "source blob payload must be bytes")
            if (
                isinstance(stored, str)
                or isinstance(stored, bytes)
                and len(stored) == 32
            ):
                digest = stored
            else:
                if isinstance(stored, InvalidText) or any(
                    _invalid_pub_text(key, value) for key, value in pub.entries
                ):
                    self._note(9, location, "source blob pub contains invalid UTF-8")
                self._note(14, location, "external source blob requires a digest")

        if any(
            contains_invalid_text(value)
            for _, value in frame_values("blob", frame, payload)
        ):
            self._note(9, location, "source blob frame contains invalid UTF-8")
        if not isinstance(pub_field, MapValue):
            self._note(14, location, "source blob pub must be a map")
        for key in ("mt", "rep"):
            if key in metadata and not isinstance(metadata[key], str | InvalidText):
                self._note(14, location, f"source blob {key} must be text")
        if digest is not None:
            self._add(digest, body, pub, location)

    def add_snapshot(self, payload: Value, location: BlobLocation) -> None:
        """Collect snapshot blobs and judge all entries at the same item."""
        if not isinstance(payload, MapValue):
            self._note(8, location, "source snapshot payload must be a map")
            return
        fields = {key: value for key, value in payload.entries if isinstance(key, str)}
        blobs = fields.get("blobs", _EMPTY_MAP)
        if not isinstance(blobs, MapValue):
            self._note(8, location, "source snapshot blobs must be a map")
            return
        for index, (key, body) in enumerate(blobs.entries):
            self.hold_digest(key)
            if isinstance(body, bytes):
                digest = blake3_256(body)
                if not _matches(key, digest):
                    self._note(
                        6, location, "snapshot blob digest does not match its bytes"
                    )
                self._add(
                    digest,
                    body,
                    _EMPTY_MAP,
                    BlobLocation(location.segment, location.position, index),
                )
            else:
                self._note(14, location, "snapshot blob entry must hold bytes")

    def _add(
        self, digest: _Digest, body: bytes | None, pub: MapValue, location: BlobLocation
    ) -> None:
        self.hold_digest(digest)
        self._declarations.append(_Declaration(digest, body, pub, location))

    def project(self, graph: ConversionGraph) -> ConvertedBlobs:
        """Merge declarations and add their RDF and v2 blob bindings."""
        groups: dict[_Digest, list[_Declaration]] = {}
        for declaration in self._declarations:
            digest = self.resolve_digest(declaration.digest)
            groups.setdefault(digest, []).append(declaration)

        declarations = []
        bindings = set()
        images = {}
        frames: dict[RecordLocation, list[BlobImage]] = {}
        stop = self._stop
        merged = []
        for digest, group in groups.items():
            pub, stop = _merge_pub(group, stop)
            merged.append(
                (
                    digest,
                    pub,
                    next((item for item in group if item.body is not None), None),
                )
            )
        if stop is not None:
            raise stop

        for digest, pub, held in merged:
            fields = {key: value for key, value in pub.entries if isinstance(key, str)}
            media_type = _blob_text(fields, "mt")
            representation = _blob_text(fields, "rep")
            extra = extensions(pub, _BLOB_FIELDS)
            node = None
            reference = None
            if held is None:
                node = graph.add_blob(
                    digest, media_type, representation, extra, external=True
                )
            else:
                assert held.body is not None
                name = (1, unit_digest(held.body))
                reference = BlobReference(*name)
                declarations.append(
                    BlobDeclaration(
                        held.location,
                        name,
                        len(held.body),
                        media_type,
                        "complete",
                        held.body,
                    )
                )
                if representation is not None or extra.entries:
                    node = graph.add_blob(
                        digest, None, representation, extra, external=False
                    )
                    bindings.add(ResolvedBinding(node, reference))
            images[digest] = BlobImage(node, reference)
            for item in groups[digest]:
                location = RecordLocation(item.location.segment, item.location.position)
                frames.setdefault(location, []).append(
                    BlobImage(node, reference if item.body is not None else None)
                )
        return ConvertedBlobs(tuple(declarations), frozenset(bindings), images, frames)


def _blob_text(fields: dict[str, Value], key: str) -> str | None:
    if key not in fields:
        return None
    value = fields[key]
    assert isinstance(value, str)
    return value

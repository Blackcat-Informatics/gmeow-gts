# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Build the metadata graph and typed VALUES of P9 conversion."""

from collections.abc import Iterable, Iterator

import cbor2

from ._conversion_fields import extensions, frame_values
from ._legacy_cbor import (
    FloatValue,
    InvalidText,
    MapValue,
    SimpleValue,
    TaggedValue,
    Value,
    merge_legacy_maps,
)
from .conversion_rdf import TermNote
from .dictionary import Blank, Iri, TermPool, TypedLiteral
from .rows import QuadRow
from .terms import TypedValue

_NS = "https://w3id.org/gts/conversion#"
_RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
_STRING = "http://www.w3.org/2001/XMLSchema#string"
_HEADER_FIELDS = {"gts", "v", "cat", "dct", "id", "prof", "layout", "meta"}
_CODEC_FIELDS = {"name", "cls", "dct", "p"}
type _Children = list[tuple[int, int, Value]]


class InvalidCarriedText(ValueError):
    """A source text that the conversion cannot preserve as text."""

    def __init__(self) -> None:
        super().__init__("carried source value contains invalid UTF-8")


def _text(value: Value, field: str) -> str:
    if isinstance(value, InvalidText):
        raise InvalidCarriedText
    if not isinstance(value, str):
        raise ValueError(f"source {field} must be text")
    return value


def _metadata_maps(maps: Iterable[MapValue]) -> Iterator[MapValue]:
    for value in maps:
        for key, _ in value.entries:
            _text(key, "segment metadata key")
        yield value


def _catalog_extensions(catalog: Value) -> MapValue:
    if not isinstance(catalog, MapValue):
        raise ValueError("source codec catalogue must be a map")
    found = []
    for key, codec in catalog.entries:
        if not isinstance(codec, MapValue):
            raise ValueError("source codec entry must be a map")
        extras = extensions(codec, _CODEC_FIELDS)
        if extras.entries:
            found.append((key, extras))
    return MapValue(tuple(found))


class ConversionGraph:
    """Accumulate conversion vocabulary quads in a fresh named graph."""

    def __init__(self, pool: TermPool) -> None:
        self.pool = pool
        self.rows: set[QuadRow] = set()
        self.values: dict[int, TypedValue] = {}
        self._blank_count = 0
        self.dataset = self._blank()
        self._quad(self.dataset, _RDF + "type", self._iri(_NS + "Dataset"))

    def _iri(self, text: str) -> int:
        return self.pool._intern(Iri(text))

    def _blank(self) -> int:
        # Source blank scopes are nonnegative. Conversion vocabulary nodes use
        # a separate scope so no source term can become one of these nodes.
        node = self.pool._intern(Blank(-1, cbor2.dumps(self._blank_count)))
        self._blank_count += 1
        return node

    def _quad(self, subject: int, predicate: str, object_id: int) -> None:
        self.rows.add(QuadRow(subject, self._iri(predicate), object_id, self.dataset))

    def _literal(self, text: str, datatype: str = _STRING) -> int:
        return self.pool._intern(TypedLiteral(text, datatype))

    def _typed(self, value: TypedValue) -> int:
        ref = self._literal(*value.literal())
        self.values[ref] = value
        return ref

    def _list_term(self, items: tuple[Value, ...], pending: _Children) -> int:
        node = self._iri(_RDF + "nil")
        for item in reversed(items):
            cell = self._blank()
            self._quad(cell, _RDF + "rest", node)
            pending.append((cell, self._iri(_RDF + "first"), item))
            node = cell
        return node

    def _map_term(
        self, entries: tuple[tuple[Value, Value], ...], pending: _Children
    ) -> int:
        node = self._blank()
        self._quad(node, _RDF + "type", self._iri(_NS + "Map"))
        key_predicate, value_predicate = (
            self._iri(_NS + "key"),
            self._iri(_NS + "value"),
        )
        for key, item in entries:
            entry = self._blank()
            self._quad(node, _NS + "entry", entry)
            pending.append((entry, key_predicate, key))
            pending.append((entry, value_predicate, item))
        return node

    def _term(self, value: Value, pending: _Children) -> int:
        match value:
            case str():
                return self._literal(value)
            case bool():
                return self._typed(TypedValue(1, value))
            case int():
                return self._typed(TypedValue(3, value))
            case bytes():
                return self._typed(TypedValue(5, value))
            case FloatValue(bits):
                return self._literal(f"{bits:016X}", _NS + "Float64")
            case None:
                return self._iri(_NS + "Null")
            case SimpleValue(23):
                return self._iri(_NS + "Undefined")
            case SimpleValue(number):
                return self._literal(str(number), _NS + "Simple")
            case InvalidText():
                raise InvalidCarriedText
            case tuple():
                return self._list_term(value, pending)
            case MapValue(entries):
                return self._map_term(entries, pending)
            case TaggedValue(number, child):
                node = self._blank()
                self._quad(node, _RDF + "type", self._iri(_NS + "Tagged"))
                self._quad(node, _NS + "tag", self._typed(TypedValue(3, number)))
                pending.append((node, self._iri(_NS + "content"), child))
                return node
        raise TypeError("expected a source CBOR value")

    def map_value(self, value: Value) -> int:
        """Return the term of a carried source value and add its P9 statements."""
        pending: _Children = []
        term = self._term(value, pending)
        while pending:
            subject, predicate, child = pending.pop()
            self.rows.add(
                QuadRow(subject, predicate, self._term(child, pending), self.dataset)
            )
        return term

    def add_segment(
        self, ordinal: int, header: MapValue, metadata: Iterable[MapValue] = ()
    ) -> int:
        """Record one source segment and its own metadata in the conversion graph."""
        fields = {
            key: value
            for key, value in header.entries
            if isinstance(key, str) and key in _HEADER_FIELDS
        }
        profile = _text(fields.get("prof", "generic"), "profile")
        layout = _text(fields["layout"], "layout") if "layout" in fields else None
        merged = merge_legacy_maps(_metadata_maps(metadata))
        catalog = (
            _catalog_extensions(fields["cat"]) if "cat" in fields else MapValue(())
        )
        segment = self._blank()
        self._quad(self.dataset, _NS + "segment", segment)
        self._quad(segment, _RDF + "type", self._iri(_NS + "Segment"))
        self._quad(segment, _NS + "ordinal", self._typed(TypedValue(3, ordinal)))
        self._quad(segment, _NS + "profile", self._literal(profile))
        if layout is not None:
            self._quad(segment, _NS + "layout", self._literal(layout))
        if "meta" in fields:
            self._quad(segment, _NS + "headerMeta", self.map_value(fields["meta"]))
        for predicate, value in (
            ("headerExtension", extensions(header, _HEADER_FIELDS)),
            ("catalogExtension", catalog),
            ("segmentMeta", merged),
        ):
            if value.entries:
                self._quad(segment, _NS + predicate, self.map_value(value))
        return segment

    def add_frame(
        self,
        segment: int,
        ordinal: int,
        frame_type: str,
        envelope: MapValue,
        payload: Value,
        term_notes: tuple[TermNote, ...] = (),
    ) -> None:
        """Carry frame fields that have no image in the source rows or blob set."""
        values = frame_values(frame_type, envelope, payload)
        if not values and not term_notes:
            return
        node = self._blank()
        self._quad(node, _RDF + "type", self._iri(_NS + "Frame"))
        self._quad(node, _NS + "inSegment", segment)
        self._quad(node, _NS + "frameOrdinal", self._typed(TypedValue(3, ordinal)))
        self._quad(node, _NS + "frameType", self._literal(frame_type))
        for predicate, value in values:
            self._quad(node, _NS + predicate, self.map_value(value))
        for note in term_notes:
            term_node = self._blank()
            self._quad(node, _NS + "termNote", term_node)
            self._quad(term_node, _RDF + "type", self._iri(_NS + "TermNote"))
            self._quad(
                term_node, _NS + "termIndex", self._typed(TypedValue(3, note.index))
            )
            self._quad(term_node, _NS + "termValue", note.term)
            self._quad(term_node, _NS + "termKeys", self.map_value(note.keys))

    def add_blob(
        self,
        digest: bytes | str,
        media_type: str | None,
        representation: str | None,
        extras: MapValue,
        *,
        external: bool,
    ) -> int:
        """State a blob declaration or an external blob in the conversion graph."""
        node = self._blank()
        kind = "ExternalBlob" if external else "BlobDeclaration"
        self._quad(node, _RDF + "type", self._iri(_NS + kind))
        if isinstance(digest, bytes):
            self._quad(node, _NS + "digest", self._typed(TypedValue(5, digest)))
        else:
            self._quad(node, _NS + "digestText", self._literal(digest))
        if media_type is not None:
            self._quad(node, _NS + "mediaType", self._literal(media_type))
        if representation is not None:
            self._quad(node, _NS + "representation", self._literal(representation))
        if extras.entries:
            self._quad(node, _NS + "pub", self.map_value(extras))
        return node

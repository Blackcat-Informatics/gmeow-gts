# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Classify and inspect CBOR fields carried in the P9 vocabulary."""

from ._legacy_cbor import InvalidText, MapValue, TaggedValue, Value

_FRAME_FIELDS = {"t", "d", "id", "prev", "x", "sig", "to", "pub"}
_PAYLOAD_FIELDS = {
    "snapshot": {"terms", "quads", "reifies", "annot", "blobs", "meta"},
    "index": {"count", "head", "off", "ti", "dict", "mmr"},
}


def extensions(value: MapValue, fields: set[str]) -> MapValue:
    return MapValue(
        tuple(
            (key, item)
            for key, item in value.entries
            if not isinstance(key, str) or key not in fields
        )
    )


def frame_values(
    frame_type: str, envelope: MapValue, payload: Value
) -> list[tuple[str, Value]]:
    """Return P9 frame fields represented by the generic value mapping."""
    values = [
        (key, value)
        for key, value in envelope.entries
        if isinstance(key, str)
        and (key == "to" or (key == "pub" and frame_type != "blob"))
    ]
    extras = extensions(envelope, _FRAME_FIELDS)
    if extras.entries:
        values.append(("extension", extras))
    fields = _PAYLOAD_FIELDS.get(frame_type)
    if fields is not None and isinstance(payload, MapValue):
        extras = extensions(payload, fields)
        if extras.entries:
            values.append(("payloadExtension", extras))
    return values


def contains_invalid_text(value: Value) -> bool:
    """Find invalid UTF-8 at any depth of a value carried as RDF."""
    open_values: list[Value] = [value]
    while open_values:
        item = open_values.pop()
        match item:
            case InvalidText():
                return True
            case TaggedValue(_, child):
                open_values.append(child)
            case tuple():
                open_values.extend(item)
            case MapValue(entries):
                for key, child in entries:
                    open_values.extend((key, child))
    return False

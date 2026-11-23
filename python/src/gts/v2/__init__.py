# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS v2 wire primitives."""

from .framing import GenerationField, ItemFramingError, generation_field, item_end
from .header import (
    DecodedHeader,
    Header,
    HeaderError,
    Lineage,
    decode_header,
    encode_header,
)

__all__ = [
    "DecodedHeader",
    "GenerationField",
    "Header",
    "HeaderError",
    "ItemFramingError",
    "Lineage",
    "decode_header",
    "encode_header",
    "generation_field",
    "item_end",
]

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS v2 wire primitives."""

from .frame import Frame, FrameError, decode_frame, encode_plain_frame
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
    "Frame",
    "FrameError",
    "GenerationField",
    "Header",
    "HeaderError",
    "ItemFramingError",
    "Lineage",
    "decode_frame",
    "decode_header",
    "encode_header",
    "encode_plain_frame",
    "generation_field",
    "item_end",
]

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS v2 records, wire reading, and content folding."""

from ._conversion_output import ConversionCapacity
from ._conversion_result import ConversionProof, Converted
from .content import ContentCapacity, ContentRead, read_content
from .conversion import convert
from .conversion_fold import ConversionStop
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
from .payload import PayloadBudget, PayloadCheck
from .reader import WireFrame, WireRead, WireSegment, read_wire

__all__ = [
    "ContentCapacity",
    "ContentRead",
    "ConversionCapacity",
    "ConversionProof",
    "ConversionStop",
    "Converted",
    "DecodedHeader",
    "Frame",
    "FrameError",
    "GenerationField",
    "Header",
    "HeaderError",
    "ItemFramingError",
    "Lineage",
    "PayloadBudget",
    "PayloadCheck",
    "WireFrame",
    "WireRead",
    "WireSegment",
    "convert",
    "decode_frame",
    "decode_header",
    "encode_header",
    "encode_plain_frame",
    "generation_field",
    "item_end",
    "read_content",
    "read_wire",
]

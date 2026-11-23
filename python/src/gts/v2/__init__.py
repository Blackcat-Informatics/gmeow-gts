# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""GTS v2 wire primitives."""

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
    "Header",
    "HeaderError",
    "Lineage",
    "decode_header",
    "encode_header",
]

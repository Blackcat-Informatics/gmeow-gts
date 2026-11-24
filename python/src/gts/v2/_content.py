# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Shared decoded-content field checks for registry version 1."""

from ._decode import WireError, _Decoder

UINT_MAX = (1 << 64) - 1


class ContentError(WireError):
    """A malformed value in a content record or decoded payload."""


class UnknownContentCode(ValueError):
    """A content code that a reader of registry version 1 does not hold."""

    def __init__(self, code: int, offset: int) -> None:
        self.code = code
        self.offset = offset
        super().__init__(f"unknown content code {code} at offset {offset}")


def content_code(decoder: _Decoder, last: int, registry_version: int) -> int:
    offset = decoder.offset
    value = decoder.uint(minimum=1, reason="unregistered_value")
    if value <= last:
        return value
    if registry_version > 1:
        raise UnknownContentCode(value, offset)
    raise decoder.error("unregistered_value", offset)

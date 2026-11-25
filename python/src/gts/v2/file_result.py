# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed frame states of the GTS v2 file result (P7 section 1.1, P2 enumerations)."""

from dataclasses import dataclass
from typing import Literal

from .payload import PayloadCheck


@dataclass(frozen=True, slots=True)
class UncheckedPayload:
    """The `payload_state` variant `unchecked`, carrying its P2 reason."""

    reason: Literal["unknown_codec", "exceeds_capacity"]


type PayloadState = Literal["verified", "damaged"] | UncheckedPayload

# The three size reasons of a payload check are the `exceeds_capacity` reason of
# the typed result (P2-28, and the R7 findings of P7).
_CAPACITY_REASONS = frozenset(("stored_size", "decoded_size", "total_decoded"))


def payload_state(check: PayloadCheck) -> PayloadState:
    """Return the P7 `payload_state` of a payload check.

    Raise ValueError when an unchecked payload carries no reason the P2
    enumeration admits, since no state can then be stated.
    """
    match check.state:
        case "verified" | "damaged":
            return check.state
        case "unchecked":
            if check.reason == "unknown_codec":
                return UncheckedPayload("unknown_codec")
            if check.reason in _CAPACITY_REASONS:
                return UncheckedPayload("exceeds_capacity")
            raise ValueError("an unchecked payload carries no P2 reason")

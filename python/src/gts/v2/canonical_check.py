# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Result of judging a segment's P7 canonical claim."""

from dataclasses import dataclass
from typing import Literal

from .canonical_labelling import LabellingDifference
from .canonical_structure import StructureDifference

type UncheckedReason = Literal[
    "not_claimed", "exceeds_capacity", "labelling_refused", "encoder_not_held"
]
type Difference = StructureDifference | LabellingDifference


@dataclass(frozen=True, slots=True)
class CheckState:
    state: Literal["verified", "failed", "unchecked"]
    reason: UncheckedReason | None = None


@dataclass(frozen=True, slots=True)
class CanonicalResult:
    claimed: bool
    structure: CheckState
    labelling: CheckState
    storage: CheckState
    first_difference: Difference | None


_NO_CLAIM = CheckState("unchecked", "not_claimed")
NOT_CLAIMED = CanonicalResult(False, _NO_CLAIM, _NO_CLAIM, _NO_CLAIM, None)
_CAPACITY = CheckState("unchecked", "exceeds_capacity")
ITEM_LIMIT = CanonicalResult(True, _CAPACITY, _CAPACITY, _CAPACITY, None)


def checked(
    difference: Difference | None, reason: UncheckedReason | None = None
) -> CheckState:
    if reason is not None:
        return CheckState("unchecked", reason)
    return CheckState("verified" if difference is None else "failed")

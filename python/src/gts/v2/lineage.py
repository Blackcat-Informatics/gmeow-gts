# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Compare GTS v2 segment lineages and identify graph state runs."""

from dataclasses import dataclass
from typing import Literal

from ._decode import WireError
from .reader import WireSegment

LineageFinding = Literal["lineage_begins", "head", "ordinal"]


@dataclass(frozen=True, slots=True)
class SegmentLineage:
    """P1 comparison and the first file ordinal of the P3 run, if defined."""

    checked: bool
    findings: tuple[LineageFinding, ...]
    run_start: int | None


def _compare(
    current: WireSegment, previous: WireSegment | None
) -> tuple[bool, tuple[LineageFinding, ...]]:
    opener = current.opener.value
    if isinstance(opener, WireError):
        return False, ()
    declared = opener.header.lineage
    if declared is None:
        return False, () if previous is None else ("lineage_begins",)
    if opener.identifier is None:
        return False, ()
    if previous is None:
        return False, ("ordinal",)
    prior = previous.opener.value
    if isinstance(prior, WireError) or previous.head is None:
        return False, ()

    findings: list[LineageFinding] = []
    if declared.previous_head != previous.head:
        findings.append("head")
    prior_lineage = prior.header.lineage
    if prior_lineage is not None and declared.ordinal != prior_lineage.ordinal + 1:
        findings.append("ordinal")
    return not findings, tuple(findings)


def check_lineages(segments: tuple[WireSegment, ...]) -> tuple[SegmentLineage, ...]:
    """Compare each opener with its immediate predecessor, then derive its run."""
    lineages: list[SegmentLineage] = []
    previous: WireSegment | None = None
    for ordinal, segment in enumerate(segments):
        checked, findings = _compare(segment, previous)
        opener = segment.opener.value
        run_start = None
        if not isinstance(opener, WireError) and opener.identifier is not None:
            if segment.opener.kind == "resync" or opener.header.lineage is None:
                run_start = ordinal
            elif checked:
                run_start = lineages[-1].run_start
        lineages.append(SegmentLineage(checked, findings, run_start))
        previous = segment
    return tuple(lineages)

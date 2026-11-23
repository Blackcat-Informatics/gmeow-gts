# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""L1 structural records and the result of an unlimited run."""

from collections import Counter
from collections.abc import Mapping
from dataclasses import dataclass
from enum import IntEnum
from itertools import groupby, pairwise
from typing import NamedTuple

from ._certificates import (
    Call,
    CanonicalNode,
    ComponentCertificate,
    canonical_map,
    compare_certificates,
    solve_components,
)
from ._labelling_structure import (
    ComponentDimensions,
    Diagnostic,
    Failure,
    PreparedInput,
    prepare_input,
)
from ._labelling_walk import walk
from .labelling import _MAX_UINT, LabellingInput, LabellingRefused, PositionedSequence


class FactorKind(IntEnum):
    ATTAIN = 0
    SYM = 1


class Factor(NamedTuple):
    kind: FactorKind
    value: int
    exponent: int


class Work(NamedTuple):
    start: int
    refine: int
    fix: int
    entries: int


@dataclass(frozen=True, slots=True)
class ComponentStatistics:
    dimensions: ComponentDimensions
    calls: tuple[int, int, int]
    chosen_children: tuple[int, int]
    target_sizes: tuple[int, ...]
    attaining: tuple[int, ...]
    nestings: tuple[tuple[int, ...], ...]
    nesting_depth: int
    branching_depth: int
    units: Work
    group_order: tuple[Factor, ...]


@dataclass(frozen=True, slots=True)
class InputStatistics:
    top_components: int
    size_histogram: tuple[tuple[int, int], ...]
    single_node_components: int
    blank_nodes: int
    label_elements: int
    incidences: int
    singled_out: int
    unsettled: int
    blank_bearing_lines: int
    blank_bearing_bytes: int
    calls: tuple[int, int, int]
    chosen_children: tuple[int, int]
    units: Work
    allowance: int
    top_multiplicities: tuple[int, ...]
    group_order: tuple[Factor, ...]
    broken_by: tuple[Failure, ...]

    @property
    def guaranteed(self) -> bool:
        return not self.broken_by


@dataclass(frozen=True, slots=True)
class LabellingResult:
    labels: Mapping[bytes, CanonicalNode]
    sequence: PositionedSequence
    components: tuple[ComponentStatistics, ...]
    statistics: InputStatistics
    diagnostic: tuple[Diagnostic, ...]


def _nestings(nested: tuple[Call, ...]) -> tuple[int, ...]:
    sizes = [1]
    for left, right in pairwise(nested):
        if compare_certificates(left, right) == 0:
            sizes[-1] += 1
        else:
            sizes.append(1)
    return tuple(sizes)


def _factor_records(
    factors: Mapping[tuple[FactorKind, int], int],
) -> tuple[Factor, ...]:
    return tuple(
        Factor(kind, value, factors[kind, value]) for kind, value in sorted(factors)
    )


def _component_statistics(component: ComponentCertificate) -> ComponentStatistics:
    calls = [0, 0, 0]
    chosen = [0, 0]
    target_sizes: list[int] = []
    attaining: list[int] = []
    nestings: list[tuple[int, ...]] = []
    factors: Counter[tuple[FactorKind, int]] = Counter()
    nesting_depth = branching_depth = fix = entries = 0
    refine = component.refinement_units
    stack = [(component.call, 0, 0)]
    while stack:
        call, depth, branches = stack.pop()
        rule = int(bool(call.nested))
        if call.target_size:
            calls[2] += 1
            chosen[rule] += 1
            target_sizes.append(call.target_size)
            attaining.append(call.attaining)
            branches += 1
            if call.attaining > 1:
                factors[FactorKind.ATTAIN, call.attaining] += 1
        else:
            calls[rule] += 1
        nesting_depth = max(nesting_depth, depth)
        branching_depth = max(branching_depth, branches)
        refine += call.refinement_units
        fix += call.fix_units
        entries += sum(1 + (len(encoding) + 7) // 8 for _, encoding in call.entries)
        if call.nested:
            multiplicities = _nestings(call.nested)
            nestings.append(multiplicities)
            for count in multiplicities:
                if count > 1:
                    factors[FactorKind.SYM, count] += 1
            # Each copy contributes its own factors to the sum of exponents.
            stack.extend(
                (child, depth + 1, branches) for child in reversed(call.nested)
            )
    return ComponentStatistics(
        component.dimensions,
        (calls[0], calls[1], calls[2]),
        (chosen[0], chosen[1]),
        tuple(target_sizes),
        tuple(attaining),
        tuple(nestings),
        nesting_depth,
        branching_depth,
        Work(component.start_units, refine, fix, entries),
        _factor_records(factors),
    )


def _input_statistics(
    prepared: PreparedInput,
    certificates: tuple[ComponentCertificate, ...],
    components: tuple[ComponentStatistics, ...],
    diagnostic: tuple[Diagnostic, ...],
) -> InputStatistics:
    dimensions = [component.dimensions for component in components]
    histogram = Counter(shape.blank_nodes.bit_length() - 1 for shape in dimensions)
    multiplicities = []
    factors: Counter[tuple[FactorKind, int]] = Counter()
    copies = groupby(
        zip(certificates, components, strict=True), key=lambda pair: pair[0].identity
    )
    for _, group in copies:
        _, component = next(group)
        count = 1 + sum(1 for _ in group)
        multiplicities.append(count)
        if count > 1:
            factors[FactorKind.SYM, count] += 1
        for kind, value, exponent in component.group_order:
            factors[kind, value] += exponent * count
    work = Work(
        sum(component.units.start for component in components),
        sum(component.units.refine for component in components),
        prepared.split_units + sum(component.units.fix for component in components),
        sum(component.units.entries for component in components),
    )
    if max(work) > _MAX_UINT:
        raise LabellingRefused("width", "units")
    return InputStatistics(
        len(components),
        tuple(sorted(histogram.items())),
        histogram[0],
        sum(shape.blank_nodes for shape in dimensions),
        sum(shape.label_elements for shape in dimensions),
        sum(shape.incidences for shape in dimensions),
        sum(shape.singled_out for shape in dimensions),
        sum(shape.unsettled for shape in dimensions),
        sum(shape.lines for shape in dimensions),
        sum(shape.line_bytes for shape in dimensions),
        (
            sum(component.calls[0] for component in components),
            sum(component.calls[1] for component in components),
            sum(component.calls[2] for component in components),
        ),
        (
            sum(component.chosen_children[0] for component in components),
            sum(component.chosen_children[1] for component in components),
        ),
        work,
        prepared.allowance,
        tuple(multiplicities),
        _factor_records(factors),
        tuple(item.failure for item in diagnostic),
    )


def label(source: LabellingInput, *, work_limit: int = _MAX_UINT) -> LabellingResult:
    """Compute the L1 result or raise its typed refusal at the reader's limit."""
    if type(work_limit) is not int or not 0 <= work_limit <= _MAX_UINT:
        raise ValueError("labelling work limit must be an unsigned 64-bit integer")
    prepared = prepare_input(source)
    if max(work_limit, prepared.allowance) == _MAX_UINT:
        certificates = solve_components(prepared)
    else:
        certificates, _ = walk(prepared, work_limit)
    components = tuple(_component_statistics(item) for item in certificates)
    diagnostic = tuple(
        sorted(item for result in certificates for item in result.diagnostic)
    )
    statistics = _input_statistics(prepared, certificates, components, diagnostic)
    labels = canonical_map(certificates)
    sequence = source.positioned_sequence(
        {node: label.position for node, label in labels.items()}
    )
    return LabellingResult(labels, sequence, components, statistics, diagnostic)

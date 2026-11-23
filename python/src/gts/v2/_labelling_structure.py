# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Input dimensions and search-free diagnostics for L1 labelling."""

from collections.abc import Sequence
from dataclasses import dataclass
from typing import NamedTuple

from .labelling import _MAX_UINT, LabellingInput
from .refinement import RefinedComponent, refine_top_components


class Failure(NamedTuple):
    path_position: int
    tied_cell: int
    component_nodes: int
    component_lines: int


class Diagnostic(NamedTuple):
    failure: Failure
    component_source: bytes
    cell_source: bytes


@dataclass(frozen=True, slots=True)
class ComponentDimensions:
    blank_nodes: int
    lines: int
    label_elements: int
    incidences: int
    line_bytes: int
    singled_out: int
    largest_cell: int | None
    allowance: int

    @property
    def unsettled(self) -> int:
        return self.blank_nodes - self.singled_out

    @property
    def vertices(self) -> int:
        return self.blank_nodes + self.lines

    @property
    def forest(self) -> bool:
        return self.incidences == self.vertices - 1


def measure_component(
    source: LabellingInput,
    partition: RefinedComponent,
    line_nodes: Sequence[frozenset[bytes]],
) -> ComponentDimensions:
    """Measure a top component and compute its allowance before search."""
    nodes = sum(map(len, partition.node_cells))
    lines = sum(map(len, partition.line_cells))
    singled_out = sum(len(cell) == 1 for cell in partition.node_cells)
    largest_cell = max(
        (len(cell) for cell in partition.node_cells if len(cell) > 1), default=None
    )
    length = occurrences = incidences = weighted_charge = 0
    for cell in partition.line_cells:
        for index in cell:
            size, count = source.free_shape(index)
            degree = len(line_nodes[index])
            length += size
            occurrences += count
            incidences += degree
            weighted_charge += degree * (1 + (size + 7) // 8)
    vertices = nodes + lines
    bits = vertices.bit_length()
    allowance = (
        2 * partition.start_units
        + occurrences
        + 2 * occurrences * bits
        + incidences
        + vertices * bits
        + 4 * weighted_charge
    )
    return ComponentDimensions(
        nodes,
        lines,
        occurrences,
        incidences,
        length,
        singled_out,
        largest_cell,
        min(_MAX_UINT, allowance),
    )


@dataclass(frozen=True, slots=True)
class PreparedInput:
    source: LabellingInput
    line_nodes: tuple[frozenset[bytes], ...]
    components: tuple[tuple[RefinedComponent, ComponentDimensions], ...]
    split_units: int
    allowance: int


def prepare_input(source: LabellingInput) -> PreparedInput:
    """Discover top components and compute the input allowance before search."""
    line_nodes = tuple(
        frozenset(source.incidences(index, {})) for index in range(len(source))
    )
    measured = tuple(
        (partition, measure_component(source, partition, line_nodes))
        for partition in refine_top_components(source)
    )
    sizes = [dimensions.vertices for _, dimensions in measured]
    split = sum(sizes) - max(sizes, default=0)
    allowance = min(_MAX_UINT, split + sum(shape.allowance for _, shape in measured))
    return PreparedInput(source, line_nodes, measured, split, allowance)

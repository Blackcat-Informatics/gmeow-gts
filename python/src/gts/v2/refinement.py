# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""L1 incidence components and ordered colour refinement."""

from collections import defaultdict
from collections.abc import Iterable, Iterator, Mapping
from dataclasses import dataclass
from heapq import heapify, heappop, heappush

from .labelling import LabellingInput

type _Label = tuple[int, ...]
type _Key = tuple[_Label, ...]
type _Cell = tuple[int, int]
type _ByNode = dict[bytes, dict[int, _Label]]
type _ByLine = dict[int, dict[bytes, _Label]]


class _Partition[Vertex]:
    def __init__(self, groups: Iterable[set[Vertex]]) -> None:
        self.cells: dict[int, set[Vertex]] = {}
        self.colours: dict[Vertex, int] = {}
        colour = 0
        for group in groups:
            self.cells[colour] = group
            self.colours.update((vertex, colour) for vertex in group)
            colour += len(group)

    def split(self, colour: int, keys: Mapping[Vertex, _Key]) -> list[_Cell]:
        source = self.cells[colour]
        groups: dict[_Key, set[Vertex]] = {}
        for vertex, key in keys.items():
            groups.setdefault(key, set()).add(vertex)
        if len(groups) == 1 and len(keys) == len(source):
            return []
        if len(keys) < len(source):
            source.difference_update(keys)
            groups[()] = source
        fragments = []
        original_colour = colour
        for key in sorted(groups):
            group = groups[key]
            self.cells[colour] = group
            if colour != original_colour:
                self.colours.update((vertex, colour) for vertex in group)
            fragments.append((colour, len(group)))
            colour += len(group)
        return fragments

    def ordered(self) -> tuple[frozenset[Vertex], ...]:
        return tuple(frozenset(self.cells[colour]) for colour in sorted(self.cells))


def _refine_from[Source, Target](
    vertices: set[Source],
    target: _Partition[Target],
    incidences: Mapping[Source, Mapping[Target, _Label]],
    target_side: int,
    queue: list[tuple[int, int]],
    queued: set[tuple[int, int]],
) -> int:
    units = 0
    labels: dict[int, dict[Target, list[_Label]]] = {}
    for vertex in vertices:
        for neighbour, label in incidences[vertex].items():
            units += len(label)
            colour = target.colours[neighbour]
            if len(target.cells[colour]) > 1:
                labels.setdefault(colour, {}).setdefault(neighbour, []).append(label)
    for colour in sorted(labels):
        keys = {vertex: tuple(sorted(key)) for vertex, key in labels[colour].items()}
        fragments = target.split(colour, keys)
        if not fragments:
            continue
        omit = None
        if (target_side, colour) not in queued:
            # A tie leaves out the fragment of smallest colour.
            omit = max(fragments, key=lambda fragment: (fragment[1], -fragment[0]))[0]
        for new_colour, _ in fragments:
            pair = (target_side, new_colour)
            if new_colour != omit and pair not in queued:
                queued.add(pair)
                heappush(queue, pair)
    return units


@dataclass(frozen=True, slots=True)
class RefinedComponent:
    node_cells: tuple[frozenset[bytes], ...]
    line_cells: tuple[frozenset[int], ...]
    start_units: int
    refinement_units: int


def _refine_component(
    source: LabellingInput,
    component_nodes: set[bytes],
    component_lines: set[int],
    by_node: Mapping[bytes, Mapping[int, _Label]],
    by_line: Mapping[int, Mapping[bytes, _Label]],
) -> RefinedComponent:
    initial: dict[bytes, set[int]] = {}
    start_units = 0
    for vertex in component_lines:
        encoded = source.mask(vertex, {})
        initial.setdefault(encoded, set()).add(vertex)
        start_units += 1 + (len(encoded) + 7) // 8
    nodes = _Partition((component_nodes,))
    lines = _Partition(initial[encoded] for encoded in sorted(initial))
    queue = [(0, 0), *((1, colour) for colour in lines.cells)]
    return _refine(nodes, lines, by_node, by_line, queue, start_units)


def _refine(
    nodes: _Partition[bytes],
    lines: _Partition[int],
    by_node: Mapping[bytes, Mapping[int, _Label]],
    by_line: Mapping[int, Mapping[bytes, _Label]],
    queue: list[tuple[int, int]],
    start_units: int,
) -> RefinedComponent:
    heapify(queue)
    queued = set(queue)
    refinement_units = 0
    while queue:
        side, colour = heappop(queue)
        queued.remove((side, colour))
        if side == 0:
            refinement_units += _refine_from(
                nodes.cells[colour], lines, by_node, 1, queue, queued
            )
        else:
            refinement_units += _refine_from(
                lines.cells[colour], nodes, by_line, 0, queue, queued
            )
    return RefinedComponent(
        nodes.ordered(), lines.ordered(), start_units, refinement_units
    )


def _incidences(
    source: LabellingInput, lines: Iterable[int], fixed: Mapping[bytes, int]
) -> tuple[_ByNode, _ByLine]:
    by_node: _ByNode = defaultdict(dict)
    by_line = {index: source.incidences(index, fixed) for index in lines}
    for line, labels in by_line.items():
        for node, label in labels.items():
            by_node[node][line] = label
    return by_node, by_line


def _components(
    by_node: _ByNode, by_line: _ByLine
) -> Iterator[tuple[set[bytes], set[int]]]:
    unseen = set(by_node)
    for first in sorted(by_node):
        if first not in unseen:
            continue
        pending = [first]
        unseen.remove(first)
        nodes = {first}
        lines: set[int] = set()
        while pending:
            for line in by_node[pending.pop()]:
                if line in lines:
                    continue
                lines.add(line)
                for neighbour in by_line[line]:
                    if neighbour in unseen:
                        unseen.remove(neighbour)
                        pending.append(neighbour)
                        nodes.add(neighbour)
        yield nodes, lines


def refine_top_components(source: LabellingInput) -> tuple[RefinedComponent, ...]:
    """Discover incidence components and apply the L1 fresh start to each."""
    by_node, by_line = _incidences(source, range(len(source)), {})
    return tuple(
        _refine_component(source, nodes, lines, by_node, by_line)
        for nodes, lines in _components(by_node, by_line)
    )


def _line_vertices(component: RefinedComponent) -> Iterator[int]:
    for cell in component.line_cells:
        yield from cell


def individualize(
    source: LabellingInput,
    component: RefinedComponent,
    fixed: Mapping[bytes, int],
    vertex: bytes,
) -> RefinedComponent:
    """Split one node from its stable cell and refine the resulting partitions."""
    nodes = _Partition(set(cell) for cell in component.node_cells)
    colour = nodes.colours[vertex]
    cell = nodes.cells[colour]
    if len(cell) < 2:
        raise ValueError("individualization requires a non-singleton node cell")
    queued_colour = colour + 1 if len(cell) == 2 else colour
    cell.remove(vertex)
    nodes.cells[colour] = {vertex}
    nodes.cells[colour + 1] = cell
    nodes.colours.update((other, colour + 1) for other in cell)
    lines = _Partition(set(cell) for cell in component.line_cells)
    by_node, by_line = _incidences(source, _line_vertices(component), fixed)
    return _refine(nodes, lines, by_node, by_line, [(0, queued_colour)], 0)


def inherit_components(
    source: LabellingInput, parent: RefinedComponent, fixed: Mapping[bytes, int]
) -> tuple[RefinedComponent, ...]:
    """Restrict the parent's stable cells to its remaining free components."""
    by_node, by_line = _incidences(source, _line_vertices(parent), fixed)
    node_order = {
        node: index for index, cell in enumerate(parent.node_cells) for node in cell
    }
    line_order = {
        line: index for index, cell in enumerate(parent.line_cells) for line in cell
    }
    return tuple(
        RefinedComponent(
            _restrict(nodes, node_order),
            _restrict(lines, line_order),
            0,
            0,
        )
        for nodes, lines in _components(by_node, by_line)
    )


def _restrict[Vertex](
    vertices: Iterable[Vertex], order: Mapping[Vertex, int]
) -> tuple[frozenset[Vertex], ...]:
    groups: dict[int, set[Vertex]] = defaultdict(set)
    for vertex in vertices:
        groups[order[vertex]].add(vertex)
    return tuple(frozenset(groups[index]) for index in sorted(groups))

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""L1 certificate search and canonical positions."""

from __future__ import annotations

from collections.abc import Generator, Iterator, Mapping
from dataclasses import dataclass, replace
from functools import cmp_to_key
from itertools import groupby, zip_longest

from blake3 import blake3

from .labelling import LabellingInput, _uint
from .refinement import (
    RefinedComponent,
    individualize,
    inherit_components,
    refine_top_components,
)

type _Entries = tuple[tuple[int, bytes], ...]

_COMPONENT_CONTEXT = "nested-refinement-labelling 1 component"


@dataclass(frozen=True, slots=True, eq=False)
class Call:
    """The fixed nodes, certificate and structural data for one result-path call."""

    fixed: tuple[bytes, ...]
    entries: _Entries
    nested: tuple[Call, ...]
    smallest_source: bytes
    refinement_units: int
    fix_units: int
    target_size: int = 0
    attaining: int = 0

    def prefix(self) -> tuple[int, _Entries]:
        return len(self.fixed), self.entries

    def certificate_chunks(self) -> Iterator[bytes]:
        """Encode the certificate in preorder without recursing into nested calls."""
        for call in self.preorder():
            yield _uint(len(call.fixed))
            yield _uint(len(call.entries))
            for maximum, encoded in call.entries:
                yield _uint(maximum)
                yield encoded
            yield _uint(len(call.nested))

    def preorder(self) -> Iterator[Call]:
        stack = [iter((self,))]
        while stack:
            call = next(stack[-1], None)
            if call is None:
                stack.pop()
            else:
                yield call
                if call.nested:
                    stack.append(iter(call.nested))


def compare_certificates(left: Call, right: Call) -> int:
    """Compare typed certificates in the order of L1 section 6.3."""
    stack: list[Iterator[tuple[Call | None, Call | None]]] = [iter(((left, right),))]
    while stack:
        pair = next(stack[-1], None)
        if pair is None:
            stack.pop()
            continue
        lhs, rhs = pair
        if lhs is rhs:
            continue
        if lhs is None:
            return -1
        if rhs is None:
            return 1
        a, b = lhs.prefix(), rhs.prefix()
        if a != b:
            return -1 if a < b else 1
        if lhs.nested or rhs.nested:
            stack.append(zip_longest(lhs.nested, rhs.nested))
    return 0


def _compare_nested(left: Call, right: Call) -> int:
    order = compare_certificates(left, right)
    if order:
        return order
    return (left.smallest_source > right.smallest_source) - (
        left.smallest_source < right.smallest_source
    )


type _SearchStep = Generator[RefinedComponent, Call, Call]


class _Search:
    def __init__(self, source: LabellingInput) -> None:
        self.source = source
        self.positions: dict[bytes, int] = {}
        self.line_nodes = tuple(
            frozenset(source.incidences(index, {})) for index in range(len(source))
        )

    def solve(self, component: RefinedComponent) -> Call:
        # The explicit stack sends each nested result back to its parent.
        stack = [self._call(component)]
        nested = None
        while stack:
            try:
                current = next(stack[-1]) if nested is None else stack[-1].send(nested)
            except StopIteration as completed:
                stack.pop()
                nested = completed.value
            else:
                stack.append(self._call(current))
                nested = None
        assert nested is not None
        return nested

    def _call(self, component: RefinedComponent) -> _SearchStep:
        smallest = min(min(cell) for cell in component.node_cells)
        if any(len(cell) == 1 for cell in component.node_cells):
            result = yield from self._fix(component, smallest, 0, None)
            assert result is not None
            return result
        target = sorted(component.node_cells[0])
        best = None
        attaining = 0
        for vertex in target:
            child = individualize(self.source, component, self.positions, vertex)
            result = yield from self._fix(child, smallest, child.refinement_units, best)
            if result is None:
                continue
            comparison = -1 if best is None else compare_certificates(result, best)
            if comparison < 0:
                best, attaining = result, 1
            elif comparison == 0:
                attaining += 1
        assert best is not None
        return replace(best, target_size=len(target), attaining=attaining)

    def _fix(
        self,
        component: RefinedComponent,
        smallest: bytes,
        refinement_units: int,
        best: Call | None,
    ) -> Generator[RefinedComponent, Call, Call | None]:
        fixed = tuple(
            next(iter(cell)) for cell in component.node_cells if len(cell) == 1
        )
        if best is not None and len(fixed) > len(best.fixed):
            return None
        start = len(self.positions)
        self.positions.update((node, start + index) for index, node in enumerate(fixed))
        try:
            entries, fix_units = self._entries(component, fixed)
            if best is not None and (len(fixed), entries) > best.prefix():
                return None
            remaining = inherit_components(self.source, component, self.positions)
            sizes = [
                sum(map(len, child.node_cells)) + sum(map(len, child.line_cells))
                for child in remaining
            ]
            fix_units += sum(sizes) - max(sizes, default=0)
            nested = []
            for child in remaining:
                result = yield child
                nested.append(result)
            nested.sort(key=cmp_to_key(_compare_nested))
            return Call(
                fixed,
                entries,
                tuple(nested),
                smallest,
                refinement_units,
                fix_units,
            )
        finally:
            for node in fixed:
                del self.positions[node]

    def _entries(
        self, component: RefinedComponent, fixed: tuple[bytes, ...]
    ) -> tuple[_Entries, int]:
        removed = frozenset(fixed)
        known = self.positions.keys()
        entries: list[tuple[int, bytes]] = []
        units = len(fixed)
        for cell in component.line_cells:
            for index in cell:
                nodes = self.line_nodes[index]
                units += len(nodes & removed)
                if nodes <= known:
                    maximum = max(self.positions[node] for node in nodes)
                    entries.append((maximum, self.source.mask(index, self.positions)))
                    units += 1
        return tuple(sorted(entries)), units


@dataclass(frozen=True, slots=True)
class ComponentCertificate:
    identity: bytes
    call: Call
    start_units: int
    refinement_units: int


@dataclass(frozen=True, slots=True)
class CanonicalNode:
    position: int
    component: bytes
    local_position: int
    copy_index: int


def component_certificates(source: LabellingInput) -> tuple[ComponentCertificate, ...]:
    """Find the minimum certificates and order the top components by identity."""
    search = _Search(source)
    components = []
    for partition in refine_top_components(source):
        call = search.solve(partition)
        digest = blake3(derive_key_context=_COMPONENT_CONTEXT)
        for chunk in call.certificate_chunks():
            digest.update(chunk)
        components.append(
            ComponentCertificate(
                digest.digest(), call, partition.start_units, partition.refinement_units
            )
        )
    return tuple(
        sorted(
            components,
            key=lambda result: (result.identity, result.call.smallest_source),
        )
    )


def canonical_map(
    components: tuple[ComponentCertificate, ...],
) -> Mapping[bytes, CanonicalNode]:
    """Assign positions in preorder and rank copies within each identity."""
    nodes: dict[bytes, CanonicalNode] = {}
    for identity, copies in groupby(components, key=lambda result: result.identity):
        for copy_index, component in enumerate(copies):
            ordered = (
                node for call in component.call.preorder() for node in call.fixed
            )
            for local, node in enumerate(ordered):
                nodes[node] = CanonicalNode(len(nodes), identity, local, copy_index)
    return nodes

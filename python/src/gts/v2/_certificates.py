# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""L1 certificate search and canonical positions."""

from __future__ import annotations

from collections.abc import Callable, Generator, Iterator, Mapping
from contextlib import contextmanager
from dataclasses import dataclass, replace
from functools import cmp_to_key
from itertools import groupby, zip_longest

from blake3 import blake3

from ._labelling_structure import (
    ComponentDimensions,
    Diagnostic,
    Failure,
    PreparedInput,
    prepare_input,
)
from ._twins import TwinClasses, TwinIndex
from .labelling import LabellingInput, _uint
from .refinement import (
    RefinedComponent,
    individualize,
    inherit_components,
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

    def ordered_nodes(self) -> Iterator[bytes]:
        """Visit the nodes in final position order within the call."""
        for call in self.preorder():
            yield from call.fixed


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


def drive_calls[Result](
    component: RefinedComponent,
    visit: Callable[[RefinedComponent], Generator[RefinedComponent, Result, Result]],
) -> Result:
    """Evaluate nested call frames without Python recursion."""
    stack = [visit(component)]
    nested = None
    try:
        while stack:
            try:
                current = next(stack[-1]) if nested is None else stack[-1].send(nested)
            except StopIteration as completed:
                stack.pop()
                nested = completed.value
            else:
                stack.append(visit(current))
                nested = None
    finally:
        # A refusal can leave ancestor frames suspended inside a fixing.
        for frame in reversed(stack):
            frame.close()
    assert nested is not None
    return nested


class _Search:
    def __init__(
        self,
        source: LabellingInput,
        line_nodes: tuple[frozenset[bytes], ...],
        top: RefinedComponent,
    ) -> None:
        self.source = source
        self.positions: dict[bytes, int] = {}
        self.line_nodes = line_nodes
        self.top = top
        self.twin_classes = TwinClasses()
        self.twin_index: TwinIndex | None = None

    def first_failures(self, dimensions: ComponentDimensions) -> tuple[Diagnostic, ...]:
        """Visit the search-free run, stopping the descent at each failed twin test."""
        failures: list[Diagnostic] = []
        smallest = _smallest_source(self.top)
        stack = [self._search_free_nested(self.top, dimensions, smallest, failures)]
        while stack:
            child = next(stack[-1], None)
            if child is None:
                stack.pop()
            else:
                stack.append(
                    self._search_free_nested(child, dimensions, smallest, failures)
                )
        return tuple(failures)

    def _search_free_nested(
        self,
        component: RefinedComponent,
        dimensions: ComponentDimensions,
        smallest: bytes,
        failures: list[Diagnostic],
    ) -> Iterator[RefinedComponent]:
        fixed = _singletons(component)
        if not fixed:
            target = sorted(component.node_cells[0])
            if not self._test_twins(target, stop_on_failure=True):
                failure = Failure(
                    len(self.positions),
                    len(target),
                    dimensions.blank_nodes,
                    dimensions.lines,
                )
                failures.append(Diagnostic(failure, smallest, target[0]))
                return
            component = individualize(self.source, component, self.positions, target[0])
            fixed = _singletons(component)
        with self._positioned(fixed):
            yield from inherit_components(self.source, component, self.positions)

    def solve(self) -> Call:
        return drive_calls(self.top, self._call)

    def _call(self, component: RefinedComponent) -> _SearchStep:
        smallest = _smallest_source(component)
        if any(len(cell) == 1 for cell in component.node_cells):
            result = yield from self._fix(component, smallest, 0, None)
            assert result is not None
            return result
        target = sorted(component.node_cells[0])
        best = None
        attaining = 0
        for vertex, count in self._twin_representatives(target):
            child = individualize(self.source, component, self.positions, vertex)
            result = yield from self._fix(child, smallest, child.refinement_units, best)
            if result is None:
                continue
            comparison = -1 if best is None else compare_certificates(result, best)
            if comparison < 0:
                best, attaining = result, count
            elif comparison == 0:
                attaining += count
        assert best is not None
        return replace(best, target_size=len(target), attaining=attaining)

    def _twin_representatives(self, target: list[bytes]) -> Iterator[tuple[bytes, int]]:
        self._test_twins(target, stop_on_failure=False)
        groups: dict[bytes, list[bytes]] = {}
        for vertex in target:
            root = self.twin_classes.representative(vertex)
            groups.setdefault(root, []).append(vertex)
        # Classes are visited by their smallest source identifier.
        return ((group[0], len(group)) for group in groups.values())

    def _test_twins(self, target: list[bytes], *, stop_on_failure: bool) -> bool:
        index = self.permutation_index()
        first = target[0]
        all_twins = True
        for vertex in target[1:]:
            if self.twin_classes.representative(
                first
            ) != self.twin_classes.representative(vertex):
                passes, _ = index.test(first, vertex)
                if passes:
                    self.twin_classes.join(first, vertex)
                elif stop_on_failure:
                    return False
                else:
                    all_twins = False
        return all_twins

    def permutation_index(self) -> TwinIndex:
        # Only the top component's vertices and line set define a twin test.
        if self.twin_index is None:
            lines = (index for cell in self.top.line_cells for index in cell)
            self.twin_index = TwinIndex(self.source, lines, self.line_nodes)
        return self.twin_index

    def _fix(
        self,
        component: RefinedComponent,
        smallest: bytes,
        refinement_units: int,
        best: Call | None,
    ) -> Generator[RefinedComponent, Call, Call | None]:
        fixed = _singletons(component)
        if best is not None and len(fixed) > len(best.fixed):
            return None
        with self._positioned(fixed):
            entries, fix_units = self._entries(component, fixed)
            if best is not None and (len(fixed), entries) > best.prefix():
                return None
            remaining, split_units = self.nest(component)
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
                fix_units + split_units,
            )

    def nest(
        self, component: RefinedComponent
    ) -> tuple[tuple[RefinedComponent, ...], int]:
        remaining = inherit_components(self.source, component, self.positions)
        sizes = [
            sum(map(len, child.node_cells)) + sum(map(len, child.line_cells))
            for child in remaining
        ]
        return remaining, sum(sizes) - max(sizes, default=0)

    @contextmanager
    def _positioned(self, fixed: tuple[bytes, ...]) -> Iterator[None]:
        start = len(self.positions)
        self.positions.update((node, start + index) for index, node in enumerate(fixed))
        try:
            yield
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


def _smallest_source(component: RefinedComponent) -> bytes:
    return min(min(cell) for cell in component.node_cells)


def _singletons(component: RefinedComponent) -> tuple[bytes, ...]:
    return tuple(next(iter(cell)) for cell in component.node_cells if len(cell) == 1)


@dataclass(frozen=True, slots=True)
class ComponentCertificate:
    identity: bytes
    call: Call
    start_units: int
    refinement_units: int
    dimensions: ComponentDimensions
    diagnostic: tuple[Diagnostic, ...]


@dataclass(frozen=True, slots=True)
class CanonicalNode:
    position: int
    component: bytes
    local_position: int
    copy_index: int


def component_certificates(source: LabellingInput) -> tuple[ComponentCertificate, ...]:
    """Find the minimum certificates and order the top components by identity."""
    return solve_components(prepare_input(source))


def solve_components(prepared: PreparedInput) -> tuple[ComponentCertificate, ...]:
    components = []
    for partition, dimensions in prepared.components:
        search = _Search(prepared.source, prepared.line_nodes, partition)
        diagnostic = search.first_failures(dimensions)
        components.append(
            certified_component(search.solve(), partition, dimensions, diagnostic)
        )
    return order_components(components)


def certified_component(
    call: Call,
    partition: RefinedComponent,
    dimensions: ComponentDimensions,
    diagnostic: tuple[Diagnostic, ...],
) -> ComponentCertificate:
    digest = blake3(derive_key_context=_COMPONENT_CONTEXT)
    for chunk in call.certificate_chunks():
        digest.update(chunk)
    return ComponentCertificate(
        digest.digest(),
        call,
        partition.start_units,
        partition.refinement_units,
        dimensions,
        diagnostic,
    )


def order_components(
    components: list[ComponentCertificate],
) -> tuple[ComponentCertificate, ...]:
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
            for local, node in enumerate(component.call.ordered_nodes()):
                nodes[node] = CanonicalNode(len(nodes), identity, local, copy_index)
    return nodes

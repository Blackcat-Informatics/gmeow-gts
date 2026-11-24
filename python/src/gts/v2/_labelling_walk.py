# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""The sequential L1 walk and its exact refusal boundary."""

from collections.abc import Generator, Iterable
from dataclasses import dataclass, replace
from functools import cmp_to_key
from itertools import pairwise

from ._certificates import (
    Call,
    ComponentCertificate,
    _compare_nested,
    _Search,
    _singletons,
    _smallest_source,
    certified_component,
    compare_certificates,
    drive_calls,
    order_components,
)
from ._labelling_structure import Failure, PreparedInput
from ._twins import TwinClasses
from .labelling import _MAX_UINT, LabellingRefused
from .refinement import RefinedComponent, individualize

type _Permutation = dict[bytes, bytes]


class LabellingLimitRefused(LabellingRefused):
    """A limit refusal carrying the first charge that crossed the threshold."""

    def __init__(
        self, broken_by: tuple[Failure, ...], source: bytes, nodes: int, units: int
    ) -> None:
        super().__init__("limit", "units")
        self.broken_by = broken_by
        self.source = source
        self.nodes = nodes
        self.units = units


@dataclass(slots=True)
class _Budget:
    ceiling: int
    broken_by: tuple[Failure, ...]
    units: int = 0
    source: bytes = b""
    nodes: int = 0

    def charge(self, units: int) -> None:
        if units == 0:
            return
        self.units = min(_MAX_UINT, self.units + units)
        if self.units > self.ceiling:
            assert self.nodes > 0 and self.broken_by
            raise LabellingLimitRefused(
                self.broken_by, self.source, self.nodes, self.units
            )


@dataclass(frozen=True, slots=True)
class _Walked:
    call: Call
    maps: tuple[_Permutation, ...]
    exact_attaining: bool


type _WalkStep = Generator[RefinedComponent, _Walked, _Walked]


def _align(first: Call, second: Call) -> _Permutation:
    """Map the nodes of one certificate to the other's nodes at equal positions."""
    return {
        left: right
        for left, right in zip(
            first.ordered_nodes(), second.ordered_nodes(), strict=True
        )
        if left != right
    }


def _carried(nested: list[_Walked]) -> tuple[_Permutation, ...]:
    maps = [permutation for item in nested for permutation in item.maps]
    for left, right in pairwise(nested):
        if compare_certificates(left.call, right.call) == 0:
            exchange = _align(left.call, right.call)
            reverse = {image: node for node, image in exchange.items()}
            maps.append(exchange | reverse)
    return tuple(maps)


class _ComponentWalk:
    def __init__(self, search: _Search, budget: _Budget) -> None:
        self.search = search
        self.budget = budget
        # The search-free run has its own classes.
        self.twins = TwinClasses()

    def run(self) -> _Walked:
        self.budget.charge(self.search.top.start_units)
        self.budget.charge(self.search.top.refinement_units)
        return drive_calls(self.search.top, self._call)

    def _call(self, component: RefinedComponent) -> _WalkStep:
        fixed = _singletons(component)
        smallest = _smallest_source(component)
        if fixed:
            result = yield from self._fix(component, fixed, smallest, 0, None, ())
            assert result is not None
            return result
        target = tuple(sorted(component.node_cells[0]))
        return (yield from self._branch(component, target, smallest))

    def _branch(
        self, component: RefinedComponent, target: tuple[bytes, ...], smallest: bytes
    ) -> _WalkStep:
        free = frozenset(node for cell in component.node_cells for node in cell)
        target_nodes = frozenset(target)
        witness_limit = max((1 << 22) // len(free), 8)
        witnesses: list[Call] = []
        known: list[_Permutation] = []
        orbits = TwinClasses()
        best: _Walked | None = None
        best_vertex = target[0]
        for vertex in target:
            # An orbit representative is always its least source identifier.
            if orbits.representative(vertex) != vertex:
                continue
            child = individualize(
                self.search.source, component, self.search.positions, vertex
            )
            self.budget.charge(child.refinement_units)
            result = yield from self._fix(
                child,
                _singletons(child),
                smallest,
                child.refinement_units,
                None if best is None else best.call,
                target if vertex == target[0] else (),
            )
            if result is None:
                continue
            previous = next(
                (w for w in witnesses if compare_certificates(w, result.call) == 0),
                None,
            )
            if previous is not None:
                self._add_maps(
                    (_align(previous, result.call),), free, target_nodes, orbits, known
                )
            elif len(witnesses) < witness_limit:
                witnesses.append(result.call)
            self._add_maps(result.maps, free, target_nodes, orbits, known)
            self._merge_twins(target, orbits)
            if best is None or compare_certificates(result.call, best.call) < 0:
                best, best_vertex = result, vertex
        assert best is not None
        root = orbits.representative(best_vertex)
        attaining = sum(orbits.representative(vertex) == root for vertex in target)
        return _Walked(
            replace(best.call, target_size=len(target), attaining=attaining),
            tuple(known),
            best.exact_attaining and (len(free) <= 2048 or attaining == len(target)),
        )

    def _fix(
        self,
        component: RefinedComponent,
        fixed: tuple[bytes, ...],
        smallest: bytes,
        refinement_units: int,
        best: Call | None,
        first_target: tuple[bytes, ...],
    ) -> Generator[RefinedComponent, _Walked, _Walked | None]:
        with self.search._positioned(fixed):
            entries, fix_units = self.search._entries(component, fixed)
            entry_units = sum(1 + (len(encoded) + 7) // 8 for _, encoded in entries)
            if best is not None and (len(fixed), entries) > best.prefix():
                self.budget.charge(fix_units + entry_units)
                return None
            remainder, split_units = self.search.nest(component)
            self.budget.charge(fix_units + entry_units + split_units)
            if first_target:
                self._test_twins(first_target)
            nested = []
            for child in remainder:
                result = yield child
                nested.append(result)
            nested.sort(key=cmp_to_key(lambda a, b: _compare_nested(a.call, b.call)))
            call = Call(
                fixed,
                entries,
                tuple(item.call for item in nested),
                smallest,
                refinement_units,
                fix_units + split_units,
            )
            return _Walked(
                call, _carried(nested), all(item.exact_attaining for item in nested)
            )

    def _test_twins(self, target: tuple[bytes, ...]) -> None:
        first = target[0]
        index = self.search.permutation_index()
        units = 0
        for vertex in target[1:]:
            if self.twins.representative(first) == self.twins.representative(vertex):
                continue
            passes, charge = index.test(first, vertex)
            units += charge
            if passes:
                self.twins.join(first, vertex)
        self.budget.charge(units)

    def _merge_twins(self, target: tuple[bytes, ...], orbits: TwinClasses) -> None:
        representatives: dict[bytes, bytes] = {}
        for vertex in target:
            root = self.twins.representative(vertex)
            orbits.join(representatives.setdefault(root, vertex), vertex)

    def _add_maps(
        self,
        maps: Iterable[_Permutation],
        free: frozenset[bytes],
        target: frozenset[bytes],
        orbits: TwinClasses,
        known: list[_Permutation],
    ) -> None:
        index = self.search.permutation_index()
        for moved in maps:
            if not moved.keys() <= free or not index.preserves(moved):
                continue
            # With fixed ancestors and free nodes in this component, the affected
            # full lines are exactly the affected lines of this call.
            for node, image in moved.items():
                if node in target:
                    assert image in target
                    orbits.join(node, image)
            known.append(moved)


def walk(
    prepared: PreparedInput, limit: int
) -> tuple[tuple[ComponentCertificate, ...], int]:
    """Walk every top component in source order and return the charged total."""
    searches = []
    for partition, dimensions in prepared.components:
        search = _Search(prepared.source, prepared.line_nodes, partition)
        searches.append((search, dimensions, search.first_failures(dimensions)))
    broken_by = tuple(
        sorted(item.failure for _, _, faults in searches for item in faults)
    )
    # The input split is the first charge and is at most the already computed allowance.
    budget = _Budget(max(limit, prepared.allowance), broken_by)
    budget.charge(prepared.split_units)
    completed = []
    for search, dimensions, diagnostic in searches:
        budget.source = _smallest_source(search.top)
        budget.nodes = dimensions.blank_nodes
        result = _ComponentWalk(search, budget).run()
        completed.append(
            (search, dimensions, diagnostic, result.call, result.exact_attaining)
        )
    # A witness cap can hide some attaining children in a large call. Finish
    # their classification after every charge has cleared the limit.
    components = [
        certified_component(
            call if exact_attaining else search.solve(),
            search.top,
            dimensions,
            diagnostic,
        )
        for search, dimensions, diagnostic, call, exact_attaining in completed
    ]
    return order_components(components), budget.units

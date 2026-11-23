# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Exact transposition tests for L1 blank nodes."""

from collections import ChainMap, defaultdict
from collections.abc import Iterable, Mapping, Sequence

from .labelling import LabellingInput


class TwinIndex:
    """An exact line-set index scoped to one top component."""

    def __init__(
        self,
        source: LabellingInput,
        lines: Iterable[int],
        line_nodes: Sequence[frozenset[bytes]],
    ) -> None:
        self._source = source
        self._incident: dict[bytes, set[int]] = defaultdict(set)
        self._charges: dict[int, int] = {}
        self._names: dict[bytes, int] = {}
        self._encoded: set[bytes] = set()
        for line in lines:
            for node in line_nodes[line]:
                self._incident[node].add(line)
                if node not in self._names:
                    self._names[node] = len(self._names)
            length, _ = source.free_shape(line)
            self._charges[line] = 1 + (length + 7) // 8
        self._encoded.update(source.mask(line, self._names) for line in self._charges)

    def test(self, left: bytes, right: bytes) -> tuple[bool, int]:
        """Check the transposition and return the charge for all affected lines."""
        affected = self._incident[left] | self._incident[right]
        units = sum(self._charges[line] for line in affected)
        return self._matches(affected, {left: right, right: left}), units

    def preserves(self, moved: Mapping[bytes, bytes]) -> bool:
        """Verify an exact permutation against the affected typed lines."""
        if (
            moved.keys() != set(moved.values())
            or not moved.keys() <= self._names.keys()
        ):
            return False
        affected: set[int] = set()
        for node in moved:
            affected.update(self._incident[node])
        return self._matches(affected, moved)

    def _matches(self, affected: set[int], moved: Mapping[bytes, bytes]) -> bool:
        positions = ChainMap(
            {node: self._names[image] for node, image in moved.items()}, self._names
        )
        return all(
            self._source.mask(line, positions) in self._encoded for line in affected
        )


class TwinClasses:
    """The union of the transpositions that have passed."""

    def __init__(self) -> None:
        self._parent: dict[bytes, bytes] = {}

    def representative(self, node: bytes) -> bytes:
        while (parent := self._parent.get(node)) is not None:
            if (grandparent := self._parent.get(parent)) is not None:
                self._parent[node] = grandparent
            node = parent
        return node

    def join(self, left: bytes, right: bytes) -> None:
        left_root = self.representative(left)
        right_root = self.representative(right)
        if left_root != right_root:
            self._parent[max(left_root, right_root)] = min(left_root, right_root)

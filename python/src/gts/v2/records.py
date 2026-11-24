# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Wire records for GTS v2 bindings and suppression (P3 §§6.3, 7.1, 9.1)."""

from collections.abc import Callable
from dataclasses import dataclass, field
from enum import IntEnum

from ._content import ContentError, content_code
from ._decode import _Decoder


@dataclass(frozen=True, slots=True)
class BlankPair:
    frame: bytes
    label: bytes


@dataclass(frozen=True, slots=True)
class TripleLeaf:
    """A triple followed by the encodings of its three children."""

    offset: int = field(compare=False, repr=False)


type LeafNode = int | BlankPair | TripleLeaf
type Leaf = tuple[LeafNode, ...]


@dataclass(frozen=True, slots=True)
class BlobReference:
    algorithm: int
    digest: bytes


@dataclass(frozen=True, slots=True)
class FrameReference:
    identifier: bytes


@dataclass(frozen=True, slots=True)
class StateReference:
    identifier: bytes


type Referent = BlobReference | FrameReference | StateReference


@dataclass(frozen=True, slots=True)
class Binding:
    term: Leaf
    referent: Referent


class SuppressionKind(IntEnum):
    TERM = 1
    QUAD = 2
    REIFIER = 3
    GRAPH = 4
    BLOB = 5
    REIFIER_BINDING = 6
    ANNOTATION = 7
    BINDING = 8


@dataclass(frozen=True, slots=True)
class LineTarget:
    terms: tuple[Leaf, ...]
    graph: Leaf | None


@dataclass(frozen=True, slots=True)
class SuppressionEntry:
    kind: SuppressionKind
    target: Leaf | LineTarget | BlobReference | Binding


class _RecordsDecoder(_Decoder):
    def __init__(self, data: bytes, registry_version: int) -> None:
        super().__init__(data, 0, ContentError)
        self.registry = registry_version
        self.triples: list[Leaf] = []

    def tag(self, last: int) -> int:
        offset = self.offset
        size = self.array()
        if not size:
            raise ContentError("wrong_length", offset)
        kind = content_code(self, last, self.registry)
        if size != 2:
            raise ContentError("wrong_length", offset)
        return kind

    def leaf(self) -> Leaf:
        """Return the leaf's preorder nodes without using Python recursion."""
        nodes: list[LeafNode] = []
        pending = 1
        has_triple = False
        while pending:
            pending -= 1
            offset = self.offset
            match self.tag(3):
                case 1:
                    nodes.append(self.uint())
                case 2:
                    self.record(2)
                    nodes.append(BlankPair(bytes(self.bytes(32)), bytes(self.bytes())))
                case _:
                    self.record(3)
                    nodes.append(TripleLeaf(offset))
                    pending += 3
                    has_triple = True
        result = tuple(nodes)
        if has_triple:
            self.triples.append(result)
        return result

    def blob(self) -> BlobReference:
        self.record(2)
        algorithm = content_code(self, 1, self.registry)
        return BlobReference(algorithm, bytes(self.bytes(32)))

    def referent(self) -> Referent:
        match self.tag(3):
            case 1:
                referent: Referent = self.blob()
            case 2:
                referent = FrameReference(bytes(self.bytes(32)))
            case _:
                referent = StateReference(bytes(self.bytes(32)))
        return referent

    def binding(self) -> Binding:
        self.record(2)
        return Binding(self.leaf(), self.referent())

    def suppression(self) -> SuppressionEntry:
        kind = SuppressionKind(self.tag(8))
        match kind:
            case SuppressionKind.TERM | SuppressionKind.REIFIER | SuppressionKind.GRAPH:
                target: Leaf | LineTarget | BlobReference | Binding = self.leaf()
            case SuppressionKind.BLOB:
                target = self.blob()
            case SuppressionKind.BINDING:
                target = self.binding()
            case _:
                width = 2 if kind == SuppressionKind.REIFIER_BINDING else 3
                self.record(width + 1)
                terms = tuple(self.leaf() for _ in range(width))
                graph = self.option(self.leaf)
                target = LineTarget(terms, graph)
        return SuppressionEntry(kind, target)

    def records[T](self, expected: int, decode: Callable[[], T]) -> tuple[T, ...]:
        count = self.array()
        records = tuple(decode() for _ in range(count))
        if self.offset != len(self.data):
            raise ContentError("wrong_length", self.offset)
        if count != expected:
            raise ContentError("inconsistent_counts", 0)
        for leaf in self.triples:
            bad = _triple_without_pair(leaf)
            if bad is not None:
                raise ContentError("forbidden_combination", bad)
        return records


def _triple_without_pair(leaf: Leaf) -> int | None:
    flags: list[bool] = []
    invalid = None
    for part in reversed(leaf):
        if isinstance(part, TripleLeaf):
            subject, predicate, object_id = flags.pop(), flags.pop(), flags.pop()
            has_pair = subject or predicate or object_id
            if not has_pair:
                invalid = part.offset
            flags.append(has_pair)
        else:
            flags.append(isinstance(part, BlankPair))
    return invalid


def _entry_count(content: bytes | memoryview) -> int:
    header = _Decoder(bytes(content), 0, ContentError)
    header.record(1)
    count = header.uint()
    if header.offset != len(content):
        raise ContentError("wrong_length", header.offset)
    return count


def decode_bindings(
    content: bytes | memoryview,
    payload: bytes | memoryview,
    *,
    registry_version: int = 1,
) -> tuple[Binding, ...]:
    """Check binding records before term and referent resolution."""
    count = _entry_count(content)
    decoder = _RecordsDecoder(bytes(payload), registry_version)
    return decoder.records(count, decoder.binding)


def decode_suppression(
    content: bytes | memoryview,
    payload: bytes | memoryview,
    *,
    registry_version: int = 1,
) -> tuple[SuppressionEntry, ...]:
    """Check suppression records before term resolution and visibility checks."""
    count = _entry_count(content)
    decoder = _RecordsDecoder(bytes(payload), registry_version)
    return decoder.records(count, decoder.suppression)

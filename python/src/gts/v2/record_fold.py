# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Fold bindings and suppression over resolved GTS v2 graph content."""

from collections.abc import Callable, Iterable, Mapping, Sequence, Set
from dataclasses import dataclass
from typing import Literal

from .dictionary import Blank, Iri, Term, TermDictionary, TermPool, Triple
from .records import (
    Binding,
    BlankPair,
    BlobReference,
    FrameReference,
    Leaf,
    LineTarget,
    Referent,
    SuppressionEntry,
    SuppressionKind,
    TripleLeaf,
)
from .rows import AnnotationRow, EmptyGraphRow, QuadRow, ReifierBindingRow, Row


@dataclass(frozen=True, slots=True)
class RecordLocation:
    segment: int
    position: int


@dataclass(frozen=True, slots=True)
class RecordFrame:
    location: RecordLocation
    entries: tuple[Binding, ...] | tuple[SuppressionEntry, ...]
    term_limit: int
    hidden_before: int = 0
    in_doubt: bool = False


@dataclass(frozen=True, slots=True)
class RecordScope:
    dictionaries: Mapping[int, TermDictionary]
    holders: Mapping[bytes, tuple[int, ...]]
    blobs: Set[BlobReference]
    frames: Set[bytes]
    states: Set[bytes]
    commits: Set[bytes]


@dataclass(frozen=True, slots=True)
class GroundRef:
    index: int


type ResolvedLeaf = int | GroundRef


@dataclass(frozen=True, slots=True)
class GroundTriple:
    subject: ResolvedLeaf
    predicate: int
    object: ResolvedLeaf


type GroundTerm = BlankPair | GroundTriple


@dataclass(frozen=True, slots=True)
class ResolvedBinding:
    term: ResolvedLeaf
    referent: Referent


@dataclass(frozen=True, slots=True)
class ResolvedSuppression:
    kind: SuppressionKind
    terms: tuple[ResolvedLeaf, ...] = ()
    graph: ResolvedLeaf | None = None
    referent: Referent | None = None


@dataclass(frozen=True, slots=True)
class RecordFinding:
    location: RecordLocation
    code: Literal[
        "forward_reference",
        "opaque_term",
        "binding_unanchored",
        "missing_dependency",
        "suppression_unmatched",
    ]
    reason: str | None = None


@dataclass(frozen=True, slots=True)
class RecordFold:
    bindings: frozenset[ResolvedBinding]
    suppression: frozenset[ResolvedSuppression]
    ground_terms: tuple[GroundTerm, ...]
    findings: tuple[RecordFinding, ...]
    hidden_lines: frozenset[Row]
    hidden_bindings: frozenset[ResolvedBinding]
    hidden_blobs: frozenset[BlobReference]

    @property
    def held_records(self) -> bool:
        return any(note.code == "opaque_term" for note in self.findings)


def _leaves(record: Binding | SuppressionEntry) -> tuple[Leaf, ...]:
    if isinstance(record, Binding):
        return (record.term,)
    target = record.target
    if isinstance(target, Binding):
        return (target.term,)
    if isinstance(target, LineTarget):
        return target.terms if target.graph is None else (*target.terms, target.graph)
    return () if isinstance(target, BlobReference) else (target,)


class _LeafResolver:
    def __init__(self, pool: TermPool, scope: RecordScope) -> None:
        self._pool = pool
        self._scope = scope
        self._ground_ids: dict[GroundTerm, GroundRef] = {}
        self.ground_terms: list[GroundTerm] = []

    def _ground(self, term: GroundTerm) -> GroundRef:
        ref = self._ground_ids.get(term)
        if ref is None:
            ref = GroundRef(len(self.ground_terms))
            self.ground_terms.append(term)
            self._ground_ids[term] = ref
        return ref

    def _blank(self, pair: BlankPair, ordinal: int) -> ResolvedLeaf:
        for holder in self._scope.holders.get(pair.frame, ()):
            if holder != ordinal:
                node = self._scope.dictionaries[holder].blank_id(pair.label)
                return self._ground(pair) if node is None else node
        return self._ground(pair)

    def _subject(self, node: ResolvedLeaf) -> bool:
        if isinstance(node, GroundRef):
            return isinstance(self.ground_terms[node.index], BlankPair)
        return isinstance(self._pool.term(node), Iri | Blank)

    def leaf(self, leaf: Leaf, frame: RecordFrame) -> ResolvedLeaf | None:
        ordinal = frame.location.segment
        dictionary = self._scope.dictionaries[ordinal]
        stack: list[ResolvedLeaf] = []
        for node in reversed(leaf):
            match node:
                case int():
                    value = dictionary.value_at(
                        node, hidden_before=frame.hidden_before, in_doubt=frame.in_doubt
                    )
                    if value is None:
                        return None
                    stack.append(value)
                case BlankPair():
                    stack.append(self._blank(node, ordinal))
                case TripleLeaf():
                    subject, predicate, object_id = (
                        stack.pop(),
                        stack.pop(),
                        stack.pop(),
                    )
                    if (
                        not self._subject(subject)
                        or isinstance(predicate, GroundRef)
                        or not isinstance(self._pool.term(predicate), Iri)
                    ):
                        return None
                    combination = (
                        self._pool._intern(Triple(subject, predicate, object_id))
                        if isinstance(subject, int) and isinstance(object_id, int)
                        else self._ground(GroundTriple(subject, predicate, object_id))
                    )
                    stack.append(combination)
        return stack.pop()


def _suppression(
    entry: SuppressionEntry, leaves: tuple[ResolvedLeaf, ...]
) -> ResolvedSuppression:
    target = entry.target
    if isinstance(target, BlobReference):
        return ResolvedSuppression(entry.kind, referent=target)
    if isinstance(target, Binding):
        return ResolvedSuppression(entry.kind, leaves, referent=target.referent)
    if isinstance(target, LineTarget) and target.graph is not None:
        return ResolvedSuppression(entry.kind, leaves[:-1], graph=leaves[-1])
    return ResolvedSuppression(entry.kind, leaves)


def _unresolved(
    referent: Referent,
    scope: RecordScope,
    resolve_state: Callable[[bytes], bool] | None,
) -> str | None:
    if isinstance(referent, BlobReference):
        return None if referent in scope.blobs else "blob"
    if isinstance(referent, FrameReference):
        return None if referent.identifier in scope.frames else "frame"
    if referent.identifier in scope.states or (
        resolve_state is not None and resolve_state(referent.identifier)
    ):
        return None
    return "state_unavailable" if referent.identifier in scope.commits else "state"


class _Occurrences:
    """Find targeted terms in a term DAG, visiting each reachable node once."""

    def __init__(
        self, pool: TermPool, ground: Sequence[GroundTerm], targets: set[int]
    ) -> None:
        self.pool = pool
        self.ground = ground
        self.targets = targets
        self.matched: set[int] = set()
        self._cache: dict[ResolvedLeaf, bool] = {}

    def contains(self, term_id: ResolvedLeaf) -> bool:
        if not self.targets:
            return False
        pending = [term_id]
        while pending:
            current = pending[-1]
            if current in self._cache:
                pending.pop()
                continue
            term: GroundTerm | Term
            if isinstance(current, GroundRef):
                term = self.ground[current.index]
            else:
                term = self.pool.term(current)
            direct = False
            if isinstance(current, int) and current in self.targets:
                direct = True
                self.matched.add(current)
            if isinstance(term, Triple | GroundTriple):
                components = (term.subject, term.predicate, term.object)
                unseen = [child for child in components if child not in self._cache]
                if unseen:
                    pending.extend(unseen)
                    continue
                direct |= any(self._cache[child] for child in components)
            self._cache[current] = direct
            pending.pop()
        return self._cache[term_id]


def exact_line_target(row: Row) -> ResolvedSuppression | None:
    """Return the resolved entry that targets only this kind of line."""
    terms: tuple[int, ...]
    match row:
        case QuadRow(subject, predicate, object_id, _):
            kind = SuppressionKind.QUAD
            terms = (subject, predicate, object_id)
        case ReifierBindingRow(reifier, triple, _):
            kind = SuppressionKind.REIFIER_BINDING
            terms = (reifier, triple)
        case AnnotationRow(_, reifier, predicate, object_id, _):
            kind = SuppressionKind.ANNOTATION
            terms = (reifier, predicate, object_id)
        case EmptyGraphRow():
            return None
    return ResolvedSuppression(kind, terms, graph=row.graph)


def _line_targets(
    row: Row,
) -> tuple[tuple[ResolvedLeaf, ...], tuple[ResolvedSuppression, ...]]:
    graph: tuple[int, ...] = () if row.graph is None else (row.graph,)
    options = [ResolvedSuppression(SuppressionKind.GRAPH, graph)] if graph else []
    exact = exact_line_target(row)
    if exact is None:
        return graph, tuple(options)
    if isinstance(row, AnnotationRow | ReifierBindingRow):
        options.append(ResolvedSuppression(SuppressionKind.REIFIER, (row.reifier,)))
    options.append(exact)
    return (*exact.terms, *graph), tuple(options)


def _hidden(
    suppressions: set[ResolvedSuppression],
    lines: Iterable[Row],
    bindings: set[ResolvedBinding],
    blobs: Set[BlobReference],
    pool: TermPool,
    ground: Sequence[GroundTerm],
) -> tuple[
    set[Row], set[ResolvedBinding], set[BlobReference], set[ResolvedSuppression]
]:
    if not suppressions:
        return set(), set(), set(), set()
    matched: set[ResolvedSuppression] = set()
    hidden_lines: set[Row] = set()
    hidden_bindings: set[ResolvedBinding] = set()
    hidden_blobs: set[BlobReference] = set()
    terms = {
        entry.terms[0]: entry
        for entry in suppressions
        if entry.kind == SuppressionKind.TERM and isinstance(entry.terms[0], int)
    }
    occurrences = _Occurrences(pool, ground, set(terms))
    for line in lines:
        slots, options = _line_targets(line)
        hits = suppressions.intersection(options)
        matched.update(hits)
        # Check every slot so hidden lines can still witness more than one rule.
        if sum(occurrences.contains(slot) for slot in slots) or hits:
            hidden_lines.add(line)
    for binding in bindings:
        if isinstance(binding.term, int):
            exact = ResolvedSuppression(
                SuppressionKind.BINDING, (binding.term,), referent=binding.referent
            )
            if exact in suppressions:
                matched.add(exact)
                hidden_bindings.add(binding)
        if occurrences.contains(binding.term):
            hidden_bindings.add(binding)
    for entry in suppressions:
        if (
            entry.kind == SuppressionKind.BLOB
            and isinstance(entry.referent, BlobReference)
            and entry.referent in blobs
        ):
            hidden_blobs.add(entry.referent)
            matched.add(entry)
    matched.update(terms[term_id] for term_id in occurrences.matched)
    return hidden_lines, hidden_bindings, hidden_blobs, matched


def fold_records(
    frames: Iterable[RecordFrame],
    scope: RecordScope,
    pool: TermPool,
    lines: Iterable[Row],
    *,
    resolve_state: Callable[[bytes], bool] | None = None,
    check_referents: bool = True,
) -> RecordFold:
    """Resolve file-order records against the final graph and declared referents."""
    resolver = _LeafResolver(pool, scope)
    once_per_frame: dict[RecordFinding, None] = {}
    missing: dict[Referent, RecordFinding] = {}
    bindings: set[ResolvedBinding] = set()
    suppressions: dict[ResolvedSuppression, dict[RecordLocation, None]] = {}
    for frame in frames:
        where = frame.location
        for record in frame.entries:
            raw_leaves = _leaves(record)
            if any(
                isinstance(part, int) and part >= frame.term_limit
                for leaf in raw_leaves
                for part in leaf
            ):
                once_per_frame[RecordFinding(where, "forward_reference")] = None
                continue
            leaves: list[ResolvedLeaf] = []
            for leaf in raw_leaves:
                resolved = resolver.leaf(leaf, frame)
                if resolved is None:
                    once_per_frame[RecordFinding(where, "opaque_term")] = None
                    break
                leaves.append(resolved)
            else:
                if isinstance(record, SuppressionEntry):
                    entry = _suppression(record, tuple(leaves))
                    suppressions.setdefault(entry, {})[where] = None
                    continue
                term = leaves[0]
                bindings.add(ResolvedBinding(term, record.referent))
                if isinstance(term, GroundRef):
                    once_per_frame[RecordFinding(where, "binding_unanchored")] = None
                reason = (
                    _unresolved(record.referent, scope, resolve_state)
                    if check_referents
                    else None
                )
                if reason is not None:
                    missing.setdefault(
                        record.referent,
                        RecordFinding(where, "missing_dependency", reason),
                    )

    hidden_lines, hidden_bindings, hidden_blobs, matched = _hidden(
        set(suppressions), lines, bindings, scope.blobs, pool, resolver.ground_terms
    )
    for entry, positions in suppressions.items():
        if entry not in matched:
            for where in positions:
                once_per_frame[
                    RecordFinding(
                        where, "suppression_unmatched", entry.kind.name.lower()
                    )
                ] = None
    return RecordFold(
        frozenset(bindings),
        frozenset(suppressions),
        tuple(resolver.ground_terms),
        (*once_per_frame, *missing.values()),
        frozenset(hidden_lines),
        frozenset(hidden_bindings),
        frozenset(hidden_blobs),
    )

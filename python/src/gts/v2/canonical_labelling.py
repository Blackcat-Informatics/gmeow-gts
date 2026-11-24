# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Judge the labelling level of a GTS v2 canonical claim (P7 §4.6)."""

from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Literal

import cbor2

from .dictionary import Blank, TermPool, Triple
from .fold_labelling import LabelledFold
from .record_fold import (
    GroundRef,
    GroundTerm,
    GroundTriple,
    RecordFold,
    ResolvedBinding,
    ResolvedLeaf,
    ResolvedSuppression,
)
from .rows import Row, remap_row
from .terms import TypedValue


@dataclass(frozen=True, slots=True)
class BlankOrigin:
    """A readable blank dictionary entry and its terms frame's item position."""

    term: int
    position: int


@dataclass(frozen=True, slots=True)
class LabellingDifference:
    """The first difference found on the canonical check's labelling level."""

    position: int
    check: Literal["labelling", "content_digest"]


class _MissingImage(Exception):
    """A term has no image in the fold under the stored-label permutation."""


class _Images:
    def __init__(
        self, pool: TermPool, ground: tuple[GroundTerm, ...], blanks: Mapping[int, int]
    ) -> None:
        self.pool = pool
        self.ground = ground
        self._known_ground = {
            term: GroundRef(index) for index, term in enumerate(ground)
        }
        self._images: dict[ResolvedLeaf, ResolvedLeaf] = dict(blanks.items())

    def leaf(self, root: ResolvedLeaf) -> ResolvedLeaf:
        pending = [root]
        while pending:
            ref = pending[-1]
            if ref in self._images:
                pending.pop()
                continue
            value = (
                self.pool.term(ref) if isinstance(ref, int) else self.ground[ref.index]
            )
            if isinstance(value, Triple | GroundTriple):
                missing = [
                    child
                    for child in (value.subject, value.object)
                    if child not in self._images
                ]
                if missing:
                    pending.extend(missing)
                    continue
                subject = self._images[value.subject]
                object_id = self._images[value.object]
                image: ResolvedLeaf | None
                if isinstance(value, Triple):
                    assert isinstance(subject, int) and isinstance(object_id, int)
                    image = self.pool.lookup(
                        Triple(subject, value.predicate, object_id)
                    )
                else:
                    image = self._known_ground.get(
                        GroundTriple(subject, value.predicate, object_id)
                    )
                if image is None:
                    raise _MissingImage
            elif isinstance(value, Blank):
                raise _MissingImage
            else:
                image = ref
            self._images[ref] = image
            pending.pop()
        return self._images[root]

    def term(self, ref: int) -> int:
        image = self.leaf(ref)
        assert isinstance(image, int)
        return image

    def suppression(self, entry: ResolvedSuppression) -> ResolvedSuppression:
        return ResolvedSuppression(
            entry.kind,
            tuple(self.leaf(term) for term in entry.terms),
            None if entry.graph is None else self.leaf(entry.graph),
            entry.referent,
        )


def _preserves_fold(
    images: _Images,
    rows: frozenset[Row],
    records: RecordFold,
    values: frozenset[tuple[int, TypedValue]],
) -> bool:
    try:
        return (
            all(remap_row(row, images.term) in rows for row in rows)
            and all(
                ResolvedBinding(images.leaf(binding.term), binding.referent)
                in records.bindings
                for binding in records.bindings
            )
            and all(
                images.suppression(entry) in records.suppression
                for entry in records.suppression
            )
            and all((images.term(ref), value) in values for ref, value in values)
        )
    except _MissingImage:
        return False


def _label_difference(
    pool: TermPool,
    rows: frozenset[Row],
    records: RecordFold,
    values: frozenset[tuple[int, TypedValue]],
    positions: Mapping[int, int],
    dictionary: Iterable[BlankOrigin],
) -> LabellingDifference | None:
    stored: dict[bytes, int | None] = {}
    for ref in positions:
        blank = pool.term(ref)
        assert isinstance(blank, Blank)
        stored[blank.label] = None if blank.label in stored else ref
    images = {
        ref: stored.get(cbor2.dumps(position)) for ref, position in positions.items()
    }
    moved = {ref for ref, image in images.items() if image != ref}
    if not moved:
        return None
    defined = {ref: image for ref, image in images.items() if image is not None}
    if len(defined) == len(images) and _preserves_fold(
        _Images(pool, records.ground_terms, defined), rows, records, values
    ):
        return None
    for blank_term in dictionary:
        if blank_term.term in moved:
            return LabellingDifference(blank_term.position, "labelling")
    raise ValueError("canonical labelling requires the blank origins of the own fold")


def check_canonical_labelling(
    pool: TermPool,
    rows: frozenset[Row],
    records: RecordFold,
    values: frozenset[tuple[int, TypedValue]],
    labelled: LabelledFold,
    dictionary: Iterable[BlankOrigin],
    index_digest: tuple[int, bytes] | None,
) -> LabellingDifference | None:
    """Check an eligible own fold against its stored labels and last index digest.

    The caller supplies blank origins in dictionary order and selects the
    eligible index and capacity state before calling this check.
    """
    difference = _label_difference(
        pool, rows, records, values, labelled.blank_positions, dictionary
    )
    if index_digest is not None:
        position, digest = index_digest
        if digest != labelled.labelling.sequence.digest and (
            difference is None or position < difference.position
        ):
            return LabellingDifference(position, "content_digest")
    return difference

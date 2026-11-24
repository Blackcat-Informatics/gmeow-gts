# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Measure and lower the interned terms handed to L1 by a P3 fold."""

from collections.abc import Iterator

import cbor2
from blake3 import blake3

from .dictionary import (
    Blank,
    Iri,
    LanguageLiteral,
    Term,
    TermPool,
    Triple,
    TypedLiteral,
)
from .labelling import Blank as LabellingBlank
from .labelling import Iri as LabellingIri
from .labelling import LabellingTerms, Literal, RecordLine
from .labelling import Term as LabellingTerm

_RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
_NESTED_PAYLOAD = bytes.fromhex("8205848101810181018101")
_NESTED_CONTEXT = "gts2 nested term"


class TermNodeCollision(ValueError):
    """Distinct nested terms would receive the same L1 source identifier."""


def _datatype(term: LanguageLiteral) -> str:
    return _RDF + ("langString" if term.direction is None else "dirLangString")


def _text_length(text: str) -> int:
    return len(text.encode("utf-8"))


def _leaf_weight(term: Term) -> int:
    match term:
        case Blank():
            return 9
        case Iri(text):
            return 9 + _text_length(text)
        case TypedLiteral(lexical, datatype):
            return 19 + _text_length(lexical) + _text_length(datatype)
        case LanguageLiteral(lexical, language, direction):
            return (
                27
                + _text_length(lexical)
                + _text_length(language)
                + len(_datatype(term))
                + (direction is not None)
            )
    raise AssertionError("triple weights depend on their components")


class FoldTerms:
    """Track only terms reached from the RDF lines and named record slots."""

    def __init__(self, pool: TermPool) -> None:
        self._pool = pool
        self._weights: dict[int, int] = {}
        self._nested: set[int] = set()
        self._sources: dict[int, bytes] = {}
        self._lowered: dict[int, int] = {}
        self._components: dict[int, tuple[int, int, int]] = {}
        self._term_nodes: dict[int, int] = {}
        self.blank_sources: dict[int, bytes] = {}
        self.node_sources: dict[int, bytes] = {}

    def weight(self, ref: int) -> int:
        """Weigh one term and discover its nested terms without encoding them."""
        pending = [ref]
        while pending:
            current = pending[-1]
            if current in self._weights:
                pending.pop()
                continue
            term = self._pool.term(current)
            if not isinstance(term, Triple):
                self._weights[current] = _leaf_weight(term)
                pending.pop()
                continue
            children = (term.subject, term.predicate, term.object)
            unmeasured = [child for child in children if child not in self._weights]
            if unmeasured:
                pending.extend(unmeasured)
                continue
            weight = 1
            for child in children:
                if isinstance(self._pool.term(child), Triple):
                    self._nested.add(child)
                    weight += 9
                else:
                    weight += self._weights[child]
            self._weights[current] = weight
            pending.pop()
        return self._weights[ref]

    @property
    def nested_weight(self) -> int:
        # The fixed term node and record framing replace the triple tag.
        return sum(36 + self._weights[ref] for ref in self._nested)

    def lower(self) -> LabellingTerms:
        """Construct the lowered terms after admitting the fold's weight."""
        terms = LabellingTerms()
        seen: set[bytes] = set()
        # A term gets a weight after each of its components.
        for ref in self._weights:
            term = self._pool.term(ref)
            lowered: LabellingTerm
            match term:
                case Blank(ordinal, name):
                    source = cbor2.dumps([ordinal, name])
                    self.blank_sources[ref] = source
                    lowered = LabellingBlank(source)
                case Iri(text):
                    lowered = LabellingIri(text.encode())
                case TypedLiteral(lexical, datatype):
                    lowered = Literal(lexical.encode(), datatype.encode())
                case LanguageLiteral(lexical, language, direction):
                    lowered = Literal(
                        lexical.encode(),
                        _datatype(term).encode(),
                        language.encode(),
                        direction,
                    )
                case Triple(subject, predicate, object_id):
                    parts = (subject, predicate, object_id)
                    children = (
                        self._component(subject),
                        self._component(predicate),
                        self._component(object_id),
                    )
                    lowered = Triple(*children)
                    if ref in self._nested:
                        digest = blake3(
                            cbor2.dumps([self._source(child) for child in parts]),
                            derive_key_context=_NESTED_CONTEXT,
                        ).digest()
                        if digest in seen:
                            raise TermNodeCollision
                        seen.add(digest)
                        source = cbor2.dumps([digest])
                        self.node_sources[ref] = source
                        self._term_nodes[ref] = terms.add(LabellingBlank(source))
                        self._components[ref] = children
            self._lowered[ref] = terms.add(lowered)
        return terms

    def lowered(self, ref: int) -> int:
        return self._lowered[ref]

    def _component(self, ref: int) -> int:
        if isinstance(self._pool.term(ref), Triple):
            return self._term_nodes[ref]
        return self._lowered[ref]

    def _source(self, ref: int) -> bytes:
        cached = self._sources.get(ref)
        if cached is not None:
            return cached
        match self._pool.term(ref):
            case Triple():
                source = self.node_sources[ref]
            case Blank():
                digest = blake3(
                    self.blank_sources[ref], derive_key_context=_NESTED_CONTEXT
                ).digest()
                source = cbor2.dumps([0, digest])
            case Iri(text):
                source = cbor2.dumps(cbor2.dumps([1, text]))
            case TypedLiteral(lexical, datatype):
                source = cbor2.dumps(cbor2.dumps([2, lexical, 1, datatype]))
            case LanguageLiteral(lexical, language, direction):
                form = (None, "ltr", "rtl").index(direction) + 2
                source = cbor2.dumps(cbor2.dumps([2, lexical, form, language]))
        self._sources[ref] = source
        return source

    def records(self) -> Iterator[RecordLine]:
        for ref, components in self._components.items():
            yield RecordLine(_NESTED_PAYLOAD, (self._term_nodes[ref], *components))

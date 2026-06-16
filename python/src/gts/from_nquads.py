# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The ``nquads → gts`` transform: the inverse of the §14 fold projection.

Parses N-Quads(-star) text — the output of :func:`gts.nquads.to_nquads` — back
into a GTS file. Handles IRIs, blank nodes, literals (plain, language-tagged,
datatyped), named graphs, and the RDF 1.2 reifying style
(``<r> rdf:reifies <<( s p o )>>`` plus annotation triples). Round-trips the
fold projection: ``to_nquads(read(from_nquads(to_nquads(g)))) == to_nquads(g)``
for pure-graph inputs (suppressions, blobs and opaque frames are not expressible
in N-Quads and so are out of scope, as for the forward transform).
"""

from __future__ import annotations

from dataclasses import dataclass

from gts.model import Term, TermKind
from gts.writer import Writer

_RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"


class NQuadsParseError(ValueError):
    """Raised on malformed N-Quads(-star) input."""


@dataclass(frozen=True)
class _Atom:
    """A parsed atomic term, as an interning key."""

    kind: TermKind
    value: str
    lang: str | None = None
    datatype: str | None = None  # datatype IRI, when explicit


@dataclass(frozen=True)
class _Triple:
    """A parsed quoted triple ``<<( s p o )>>`` (RDF 1.2 triple term)."""

    s: _Node
    p: _Node
    o: _Node


_Node = _Atom | _Triple


def _unescape(lex: str) -> str:
    out: list[str] = []
    i = 0
    while i < len(lex):
        ch = lex[i]
        if ch == "\\" and i + 1 < len(lex):
            nxt = lex[i + 1]
            mapping = {"\\": "\\", '"': '"', "n": "\n", "r": "\r", "t": "\t"}
            if nxt in mapping:
                out.append(mapping[nxt])
                i += 2
                continue
            if nxt in ("u", "U"):
                width = 4 if nxt == "u" else 8
                hexs = lex[i + 2 : i + 2 + width]
                out.append(chr(int(hexs, 16)))
                i += 2 + width
                continue
        out.append(ch)
        i += 1
    return "".join(out)


class _Tokenizer:
    """Pull terms off one logical N-Quads line (quoted triples are one node)."""

    def __init__(self, line: str) -> None:
        self.s = line
        self.i = 0

    def _skip_ws(self) -> None:
        while self.i < len(self.s) and self.s[self.i] in " \t":
            self.i += 1

    def at_end(self) -> bool:
        self._skip_ws()
        return self.i >= len(self.s) or self.s[self.i] == "."

    def node(self) -> _Node:
        self._skip_ws()
        if self.i >= len(self.s):
            raise NQuadsParseError(f"unexpected end of line: {self.s!r}")
        ch = self.s[self.i]
        if self.s.startswith("<<(", self.i):
            return self._quoted_triple()
        if ch == "<":
            return _Atom(TermKind.IRI, self._iri())
        if ch == "_":
            return _Atom(TermKind.BNODE, self._bnode())
        if ch == '"':
            return self._literal()
        raise NQuadsParseError(f"unexpected token at {self.i} in {self.s!r}")

    def _iri(self) -> str:
        end = self.s.index(">", self.i)
        value = self.s[self.i + 1 : end]
        self.i = end + 1
        return value

    def _bnode(self) -> str:
        if not self.s.startswith("_:", self.i):
            raise NQuadsParseError(f"bad blank node in {self.s!r}")
        self.i += 2
        start = self.i
        while self.i < len(self.s) and self.s[self.i] not in " \t":
            self.i += 1
        return self.s[start : self.i]

    def _literal(self) -> _Atom:
        # opening quote
        self.i += 1
        buf: list[str] = []
        while self.i < len(self.s):
            ch = self.s[self.i]
            if ch == "\\":
                buf.append(self.s[self.i : self.i + 2])
                self.i += 2
                continue
            if ch == '"':
                self.i += 1
                break
            buf.append(ch)
            self.i += 1
        else:
            raise NQuadsParseError(f"unterminated literal in {self.s!r}")
        lex = _unescape("".join(buf))
        lang: str | None = None
        datatype: str | None = None
        if self.i < len(self.s) and self.s[self.i] == "@":
            self.i += 1
            start = self.i
            while self.i < len(self.s) and self.s[self.i] not in " \t":
                self.i += 1
            lang = self.s[start : self.i]
        elif self.s.startswith("^^", self.i):
            self.i += 2
            self._skip_ws()
            datatype = self._iri()
        return _Atom(TermKind.LITERAL, lex, lang, datatype)

    def _quoted_triple(self) -> _Triple:
        self.i += 3  # consume "<<("
        s = self.node()
        p = self.node()
        o = self.node()
        self._skip_ws()
        if not self.s.startswith(")>>", self.i):
            raise NQuadsParseError(f"unterminated quoted triple in {self.s!r}")
        self.i += 3
        return _Triple(s, p, o)


class _Interner:
    """Assigns stable append-order term-ids; emits the term table for the Writer."""

    def __init__(self) -> None:
        self._ids: dict[object, int] = {}
        self.terms: list[Term] = []

    def atom(self, a: _Atom) -> int:
        key: object
        if a.kind is TermKind.LITERAL:
            dt_id = self.atom(_Atom(TermKind.IRI, a.datatype)) if a.datatype else None
            key = ("lit", a.value, a.lang, a.datatype)
            if key in self._ids:
                return self._ids[key]
            tid = len(self.terms)
            self.terms.append(
                Term(TermKind.LITERAL, a.value, datatype=dt_id, lang=a.lang)
            )
            self._ids[key] = tid
            return tid
        key = (int(a.kind), a.value)
        if key in self._ids:
            return self._ids[key]
        tid = len(self.terms)
        self.terms.append(Term(a.kind, a.value))
        self._ids[key] = tid
        return tid

    def node(self, n: _Node, reifiers: dict[int, tuple[int, int, int]]) -> int:
        if isinstance(n, _Atom):
            return self.atom(n)
        # A bare quoted triple as a quad component: intern as a TRIPLE term with
        # its own anonymous reifier binding.
        s = self.node(n.s, reifiers)
        p = self.node(n.p, reifiers)
        o = self.node(n.o, reifiers)
        key = ("triple", s, p, o)
        if key in self._ids:
            return self._ids[key]
        rid = len(self.terms)
        self.terms.append(Term(TermKind.TRIPLE, reifier=rid))
        self._ids[key] = rid
        reifiers[rid] = (s, p, o)
        return rid


def from_nquads(text: str) -> bytes:
    """Parse N-Quads(-star) text into a canonical GTS file (bytes)."""
    # Pass 1: tokenize every non-empty statement.
    statements: list[list[_Node]] = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        tok = _Tokenizer(line)
        nodes: list[_Node] = []
        while not tok.at_end():
            nodes.append(tok.node())
        if len(nodes) not in (3, 4):
            raise NQuadsParseError(f"expected 3 or 4 terms, got {len(nodes)}: {line!r}")
        statements.append(nodes)

    interner = _Interner()
    reifiers: dict[int, tuple[int, int, int]] = {}
    quads: list[tuple[int, int, int, int | None]] = []

    for nodes in statements:
        s, p, o = nodes[0], nodes[1], nodes[2]
        gname = nodes[3] if len(nodes) == 4 else None

        # Reifier binding: <r> rdf:reifies <<( s p o )>> . Bind the reifier to the
        # subject term so the projection re-emits exactly this line (rather than
        # interning the quoted triple as a self-referential TRIPLE term, which
        # would also be emitted by the reifies loop).
        if (
            isinstance(p, _Atom)
            and p.value == _RDF_REIFIES
            and isinstance(o, _Triple)
            and isinstance(s, _Atom)
            and gname is None
        ):
            rid = interner.atom(s)
            reifiers[rid] = (
                interner.node(o.s, reifiers),
                interner.node(o.p, reifiers),
                interner.node(o.o, reifiers),
            )
            continue

        # Everything else is a quad. Annotations (reifier, predicate, value) and
        # base quads project to identical N-Quads, so they need no distinction
        # here — both round-trip through the quads frame.
        sid = interner.node(s, reifiers)
        pid = interner.node(p, reifiers)
        oid = interner.node(o, reifiers)
        gid = interner.node(gname, reifiers) if gname is not None else None
        quads.append((sid, pid, oid, gid))

    w = Writer(profile="dist")
    if interner.terms:
        w.add_terms(interner.terms)
    if quads:
        w.add_quads(quads)
    if reifiers:
        w.add_reifies(reifiers)
    return w.to_bytes()

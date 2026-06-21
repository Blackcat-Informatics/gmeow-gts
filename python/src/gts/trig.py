# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""TriG import/export for folded GTS graphs.

The exporter preserves the same RDF 1.2 content as :func:`gts.nquads.to_nquads`
while grouping named graph quads into TriG graph blocks. The parser accepts the
emitted form plus common prefixes and graph blocks, then delegates to
``from_nquads`` so writer semantics stay shared.
"""

from __future__ import annotations

from dataclasses import dataclass

from gts.from_nquads import NQuadsParseError, from_nquads
from gts.model import Graph
from gts.nquads import term_token

_RDF_NS = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
_RDF_REIFIES = f"{_RDF_NS}reifies"
_RDF_TYPE = f"{_RDF_NS}type"


class TriGParseError(ValueError):
    """Raised when TriG input is malformed or uses unsupported shorthand."""


def _escape_literal(value: str) -> str:
    out: list[str] = []
    for ch in value:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif ord(ch) < 0x20:
            out.append(f"\\u{ord(ch):04X}")
        else:
            out.append(ch)
    return "".join(out)


def _render(g: Graph, tid: int) -> str:
    token = term_token(g, tid)
    if token == f"<{_RDF_REIFIES}>":
        return "rdf:reifies"
    return token


def _close_graph(lines: list[str], open_graph: str | None) -> str | None:
    if open_graph is not None:
        lines.append("}")
    return None


def to_trig(g: Graph) -> str:
    """Serialise a folded :class:`Graph` to TriG text."""
    if not g.quads and not g.reifiers and not g.annotations:
        return ""

    lines = [f"@prefix rdf: <{_RDF_NS}> .", ""]
    open_graph: str | None = None
    for s, p, o, graph_name in g.quads:
        triple = f"{_render(g, s)} {_render(g, p)} {_render(g, o)} ."
        if graph_name is None:
            open_graph = _close_graph(lines, open_graph)
            lines.append(triple)
            continue
        graph = _render(g, graph_name)
        if open_graph != graph:
            open_graph = _close_graph(lines, open_graph)
            lines.append(f"{graph} {{")
            open_graph = graph
        lines.append(f"  {triple}")

    open_graph = _close_graph(lines, open_graph)
    assert open_graph is None

    for rid, spo in g.reifiers.items():
        quoted = (
            f"<<( {_render(g, spo[0])} {_render(g, spo[1])} {_render(g, spo[2])} )>>"
        )
        lines.append(f"{_render(g, rid)} rdf:reifies {quoted} .")
    for r, p, v in g.annotations:
        lines.append(f"{_render(g, r)} {_render(g, p)} {_render(g, v)} .")
    return "\n".join(lines) + "\n"


@dataclass(frozen=True)
class _Iri:
    value: str


@dataclass(frozen=True)
class _Bnode:
    value: str


@dataclass(frozen=True)
class _Literal:
    value: str
    lang: str | None = None
    direction: str | None = None
    datatype: str | None = None


@dataclass(frozen=True)
class _Triple:
    s: _Node
    p: _Node
    o: _Node


_Node = _Iri | _Bnode | _Literal | _Triple


def _token(node: _Node) -> str:
    if isinstance(node, _Iri):
        return f"<{node.value}>"
    if isinstance(node, _Bnode):
        return f"_:{node.value}"
    if isinstance(node, _Literal):
        lit = f'"{_escape_literal(node.value)}"'
        if node.lang is not None:
            if node.direction is not None:
                return f"{lit}@{node.lang}--{node.direction}"
            return f"{lit}@{node.lang}"
        if node.datatype is not None:
            return f"{lit}^^<{node.datatype}>"
        return lit
    return f"<<( {_token(node.s)} {_token(node.p)} {_token(node.o)} )>>"


class _Parser:
    def __init__(self, text: str) -> None:
        self.text = text
        self.i = 0
        self.prefixes = {"rdf": _RDF_NS}
        self.nquads: list[str] = []

    def parse(self) -> str:
        while not self._eof():
            self._skip()
            if self._eof():
                break
            if self._consume("@prefix"):
                self._prefix_directive(require_dot=True)
                continue
            if self._consume_keyword("PREFIX"):
                self._prefix_directive(require_dot=False)
                continue
            if self._consume_keyword("GRAPH"):
                self._graph_block(self._term())
                continue
            first = self._term()
            self._skip()
            if self._consume_char("{"):
                self._graph_block_after_open(first)
            else:
                self._statement_after_subject(first, None)
        return "\n".join(self.nquads) + ("\n" if self.nquads else "")

    def _eof(self) -> bool:
        self._skip()
        return self.i >= len(self.text)

    def _skip(self) -> None:
        while True:
            while self.i < len(self.text) and self.text[self.i].isspace():
                self.i += 1
            if self.i < len(self.text) and self.text[self.i] == "#":
                while self.i < len(self.text) and self.text[self.i] != "\n":
                    self.i += 1
                continue
            break

    def _consume(self, value: str) -> bool:
        self._skip()
        if self.text.startswith(value, self.i):
            self.i += len(value)
            return True
        return False

    def _consume_keyword(self, keyword: str) -> bool:
        self._skip()
        end = self.i + len(keyword)
        if self.text[self.i : end].lower() != keyword.lower():
            return False
        nxt = self.text[end : end + 1]
        if nxt and not (nxt.isspace() or nxt in '{}<_"'):
            return False
        self.i = end
        return True

    def _consume_char(self, ch: str) -> bool:
        self._skip()
        if self.i < len(self.text) and self.text[self.i] == ch:
            self.i += 1
            return True
        return False

    def _expect_char(self, ch: str, context: str) -> None:
        if not self._consume_char(ch):
            raise TriGParseError(f"expected {ch!r} {context} at byte {self.i}")

    def _prefix_directive(self, *, require_dot: bool) -> None:
        label = self._prefix_label()
        iri = self._iri()
        self.prefixes[label] = iri
        if require_dot:
            self._expect_char(".", "after @prefix directive")
        else:
            self._consume_char(".")

    def _prefix_label(self) -> str:
        self._skip()
        start = self.i
        while self.i < len(self.text):
            ch = self.text[self.i]
            if ch == ":":
                label = self.text[start : self.i]
                self.i += 1
                return label
            if ch.isascii() and (ch.isalnum() or ch in "_-"):
                self.i += 1
                continue
            break
        raise TriGParseError(f"expected prefix label at byte {start}")

    def _term(self) -> _Node:
        self._skip()
        if self.text.startswith("<<(", self.i):
            return self._quoted_triple()
        if self.i >= len(self.text):
            raise TriGParseError("unexpected end of TriG input")
        ch = self.text[self.i]
        if ch == "<":
            return _Iri(self._iri())
        if ch == "_":
            return _Bnode(self._bnode())
        if ch == '"':
            return self._literal()
        return _Iri(self._prefixed_name())

    def _predicate(self) -> _Node:
        self._skip()
        if self._consume_keyword("a"):
            return _Iri(_RDF_TYPE)
        return self._term()

    def _iri(self) -> str:
        self._skip()
        if self.i >= len(self.text) or self.text[self.i] != "<":
            raise TriGParseError(f"expected IRI at byte {self.i}")
        self.i += 1
        start = self.i
        end = self.text.find(">", self.i)
        if end < 0:
            raise TriGParseError(f"unterminated IRI starting at byte {start - 1}")
        self.i = end + 1
        return self.text[start:end]

    def _bnode(self) -> str:
        self._skip()
        if not self.text.startswith("_:", self.i):
            raise TriGParseError(f"expected blank node at byte {self.i}")
        self.i += 2
        start = self.i
        while self.i < len(self.text):
            ch = self.text[self.i]
            if ch.isascii() and (ch.isalnum() or ch in "_.-"):
                self.i += 1
                continue
            break
        if self.i > start and self.text[self.i - 1] == ".":
            self.i -= 1
        if self.i == start:
            raise TriGParseError("empty blank node label")
        return self.text[start : self.i]

    def _literal(self) -> _Literal:
        self._skip()
        self.i += 1
        out: list[str] = []
        while self.i < len(self.text):
            ch = self.text[self.i]
            self.i += 1
            if ch == "\\":
                out.append(self._escape())
            elif ch == '"':
                break
            else:
                out.append(ch)
        else:
            raise TriGParseError("unterminated literal")

        lang: str | None = None
        direction: str | None = None
        datatype: str | None = None
        if self.i < len(self.text) and self.text[self.i] == "@":
            self.i += 1
            start = self.i
            while self.i < len(self.text):
                ch = self.text[self.i]
                if ch.isascii() and (ch.isalnum() or ch == "-"):
                    self.i += 1
                    continue
                break
            if self.i == start:
                raise TriGParseError("empty language tag")
            raw_lang = self.text[start : self.i]
            if "--" in raw_lang:
                base, raw_direction = raw_lang.rsplit("--", 1)
                if not base or raw_direction not in ("ltr", "rtl"):
                    raise TriGParseError("invalid literal base direction")
                lang = base
                direction = raw_direction
            else:
                lang = raw_lang
        elif self.text.startswith("^^", self.i):
            self.i += 2
            datatype = self._datatype_iri()
        return _Literal("".join(out), lang, direction, datatype)

    def _datatype_iri(self) -> str:
        self._skip()
        if self.i < len(self.text) and self.text[self.i] == "<":
            return self._iri()
        return self._prefixed_name()

    def _escape(self) -> str:
        if self.i >= len(self.text):
            raise TriGParseError("bad escape at end of literal")
        ch = self.text[self.i]
        self.i += 1
        mapping = {
            "\\": "\\",
            '"': '"',
            "b": "\u0008",
            "f": "\u000c",
            "n": "\n",
            "r": "\r",
            "t": "\t",
        }
        if ch in mapping:
            return mapping[ch]
        if ch in ("u", "U"):
            width = 4 if ch == "u" else 8
            raw = self.text[self.i : self.i + width]
            if len(raw) != width or not all(c in "0123456789abcdefABCDEF" for c in raw):
                raise TriGParseError(f"bad unicode escape \\{ch}{raw}")
            self.i += width
            try:
                return chr(int(raw, 16))
            except ValueError as exc:
                raise TriGParseError(f"invalid unicode scalar \\{ch}{raw}") from exc
        raise TriGParseError(f"unsupported escape \\{ch}")

    def _quoted_triple(self) -> _Triple:
        self.i += 3
        s = self._term()
        p = self._predicate()
        o = self._term()
        self._skip()
        if not self.text.startswith(")>>", self.i):
            raise TriGParseError("unterminated quoted triple")
        self.i += 3
        return _Triple(s, p, o)

    def _prefixed_name(self) -> str:
        self._skip()
        start = self.i
        while self.i < len(self.text):
            ch = self.text[self.i]
            if ch.isspace() or ch in "{}.;,)":
                break
            self.i += 1
        if self.i == start:
            raise TriGParseError(f"expected term at byte {self.i}")
        name = self.text[start : self.i]
        prefix, sep, local = name.partition(":")
        if not sep:
            raise TriGParseError(
                f"unsupported bare token {name!r}; use an IRI or prefix"
            )
        if prefix not in self.prefixes:
            raise TriGParseError(f"unknown prefix {prefix!r}")
        return f"{self.prefixes[prefix]}{local}"

    def _graph_block(self, graph: _Node) -> None:
        self._expect_char("{", "to open graph block")
        self._graph_block_after_open(graph)

    def _graph_block_after_open(self, graph: _Node) -> None:
        if not isinstance(graph, _Iri | _Bnode):
            raise TriGParseError("graph block name must be an IRI or blank node")
        while not self._consume_char("}"):
            if self._eof():
                raise TriGParseError("unterminated graph block")
            self._statement_after_subject(self._term(), graph)

    def _statement_after_subject(self, subject: _Node, graph: _Node | None) -> None:
        predicate = self._predicate()
        obj = self._term()
        self._skip()
        if self.i < len(self.text) and self.text[self.i] in ";,":
            raise TriGParseError(
                "TriG predicate/object shorthand is not supported; "
                "write one statement per line"
            )
        self._expect_char(".", "to terminate statement")
        line = f"{_token(subject)} {_token(predicate)} {_token(obj)}"
        if graph is not None:
            line = f"{line} {_token(graph)}"
        self.nquads.append(f"{line} .")


def from_trig(text: str) -> bytes:
    """Parse TriG text into a canonical GTS file."""
    try:
        return from_nquads(_Parser(text).parse())
    except NQuadsParseError as exc:
        raise TriGParseError(str(exc)) from exc

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Regenerate the vectors of the queryable snapshot proposal.

The proposal is docs/proposals/GIP-queryable-snapshots.md. This script writes
one public source file, one snapshot object built from it by the construction
of the proposal, and the expected outcomes of a certification and three
queries. The v2 engine of this package writes, compacts and reads the source;
the snapshot construction and the query procedure follow the proposal text.
Reproducible:

    uv run python scripts/gen_snapshot_vectors.py
    git diff --exit-code ../vectors/v2/proposals/queryable-snapshots
"""

from __future__ import annotations

import hashlib
import json
from collections.abc import Iterator
from dataclasses import dataclass
from itertools import pairwise
from pathlib import Path
from typing import Any

import cbor2

from gts.v2 import read_content
from gts.v2.compaction import Written
from gts.v2.content import ContentFold
from gts.v2.dictionary import Blank, Iri, LanguageLiteral, Term, Triple, TypedLiteral
from gts.v2.frame import Frame
from gts.v2.header import Header
from gts.v2.rebuild import unit_digest
from gts.v2.rows import EmptyGraphRow, QuadRow, encode_rows
from gts.v2.terms import IriTerm, LiteralTerm, WireTerm, encode_terms
from gts.v2.writer import SegmentWriter

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "vectors" / "v2" / "proposals" / "queryable-snapshots"

# Frame types of P2 used by the source.
_TERMS, _COMMIT, _INDEX, _PARITY = 1, 14, 15, 16

# Constants of the proposal.
SNAPSHOT_VERSION = 1
HASH_BLAKE3_256 = 1
LAW_P3_10A = 1
VARIANT_VISIBLE = 1
AUDIENCE_PUBLIC = 1
ORDER_SPOG = 1
BLOCK_MANIFEST, BLOCK_DIRECTORY, BLOCK_PAGE = 1, 2, 3
TERM_IRI, TERM_LITERAL, TERM_BLANK, TERM_TRIPLE = 1, 2, 3, 4
FORM_TYPED = 1
FORMS = {None: 2, "ltr": 3, "rtl": 4}
RESULT_EXACT, RESULT_ABSENT, RESULT_INCOMPLETE = 1, 2, 3
MISSING_WITHHELD = 1
TRAILER_TAG = "gts2 snapshot"
TRAILER_LENGTH = 33

# The layout parameters of this snapshot.
PAGE_ROWS = 3
FANOUT = 16

# The source dataset. IRIs are written as local names under _E.
_E = "https://example.org/"
_XSD_STRING = "http://www.w3.org/2001/XMLSchema#string"
_QUADS: list[tuple[str, str, tuple[str, ...], str | None]] = [
    ("a", "p1", ("iri", "b"), None),
    ("a", "p2", ("typed", "x"), None),
    ("b", "p1", ("iri", "c"), None),
    ("b", "p1", ("iri", "d"), "g"),
    ("b", "p2", ("lang", "y", "en"), "g"),
    ("b", "p3", ("iri", "e"), None),
    ("d", "p1", ("iri", "a"), None),
    ("d", "p2", ("typed", "z"), None),
    ("e", "p1", ("iri", "a"), "g"),
]
_EMPTY_GRAPHS = ["h"]

_ENTRY_NOTE = (
    "one public state, one snapshot of its visible projection in the order spog, and "
    "the strongest result of three subject lookups; blocks are named by their unit "
    "digest"
)
_CERTIFY_NOTE = (
    "the construction, applied to the state that the last commit of the source names "
    "with the projection visible, the audience public and the layout [3, 16], gives "
    "this snapshot identity; a snapshot object with another identity is not "
    "certified for these inputs"
)
_HIT_NOTE = (
    "the subject b, whose rows continue from the first page into the second: both "
    "pages decide, every match is returned once, and the result is exact"
)
_ABSENT_NOTE = (
    "the subject c, which no quad holds and which sorts between the subjects b and "
    "d: the second page alone covers the interval of c and holds no match, so the "
    "result is a verified absence"
)
_WITHHELD_NOTE = (
    "the query of absent-neighbour with the second page withheld by the holder: no "
    "other block covers the interval of c, so the result is incomplete and names "
    "the identity of that page; it is never a verified absence"
)

_INDEX_TEXT = """# Vectors of the queryable snapshot proposal

These vectors belong to the proposal
[queryable snapshots](../../../../docs/proposals/GIP-queryable-snapshots.md).
`python/scripts/gen_snapshot_vectors.py` writes every file of this directory.

| vector | outcome | notes |
|---|---|---|
| `snapshot/spog-public` | built and matches | `source.bin` is a canonical file of one \
segment; `snapshot.bin` is the snapshot of the state that its last commit names, in \
the order `spog`, with pages of three rows. Cases: `certify`, `hit`, \
`absent-neighbour`, `absent-withheld` |
"""

type Value = tuple[Any, ...]


def build_source() -> bytes:
    """Write the dataset as one segment and compact it to its canonical file."""
    iris = sorted(
        {_E + quad[0] for quad in _QUADS}
        | {_E + quad[1] for quad in _QUADS}
        | {_E + quad[2][1] for quad in _QUADS if quad[2][0] == "iri"}
        | {_E + quad[3] for quad in _QUADS if quad[3] is not None}
        | {_E + name for name in _EMPTY_GRAPHS}
        | {_XSD_STRING}
    )
    ids: dict[object, int] = {text: index for index, text in enumerate(iris)}
    literals = sorted({quad[2] for quad in _QUADS if quad[2][0] != "iri"})
    wire: list[WireTerm] = [IriTerm(text.encode()) for text in iris]
    for literal in literals:
        ids[literal] = len(wire)
        if literal[0] == "typed":
            wire.append(LiteralTerm(literal[1].encode(), datatype=ids[_XSD_STRING]))
        else:
            wire.append(LiteralTerm(literal[1].encode(), language=literal[2]))
    rows = [
        QuadRow(
            ids[_E + s],
            ids[_E + p],
            ids[_E + o[1]] if o[0] == "iri" else ids[o],
            None if g is None else ids[_E + g],
        )
        for s, p, o, g in _QUADS
    ]
    writer = SegmentWriter(Header())
    writer.append(_TERMS, *encode_terms(wire))
    writer.append(*encode_rows(rows))
    writer.append(*encode_rows([EmptyGraphRow(ids[_E + g]) for g in _EMPTY_GRAPHS]))
    writer.commit()
    read = read_content(writer.to_bytes())
    if read.findings:
        raise ValueError(f"the written source has findings: {read.findings}")
    outcome = read.compact().outcome
    if not isinstance(outcome, Written):
        raise ValueError("compaction gave no file", outcome)
    return outcome.file


@dataclass(frozen=True)
class Source:
    state: bytes
    dataset_digest: bytes
    quads: list[Value]
    empty_graphs: list[Value]


def read_source(data: bytes) -> Source:
    """Read the state that the last commit names and project it (P3 10a)."""
    read = read_content(data)
    if read.findings or read.fold is None or len(read.wire.segments) != 1:
        raise ValueError("the source must be one segment read with no finding")
    if read.visibility is None or not read.visibility.complete:
        raise ValueError("the public state needs complete visibility")
    frames = read.wire.segments[0].frames
    last = read.last_commits.get(0)
    commit = next((f for f in frames if f.position == last), None)
    if commit is None or commit.frame_type != _COMMIT:
        raise ValueError("the source needs a last valid commit")
    if any(
        f.position > commit.position and f.frame_type not in (_INDEX, _PARITY)
        for f in frames
    ):
        raise ValueError("the state must be the whole fold of the file")
    if not isinstance(commit.envelope, Frame):
        raise ValueError("the commit has no envelope")
    fold = read.fold
    if (
        fold.records.bindings
        or fold.records.hidden_lines
        or fold.blob_declarations
        or fold.values
    ):
        raise ValueError("the source must hold visible quad and empty-graph lines only")
    if fold.content_digest is None:
        raise ValueError("the source fold has no content digest")
    quads: list[Value] = []
    empty: list[Value] = []
    for row in fold.visible_dataset.lines:
        if isinstance(row, QuadRow):
            quads.append(
                (
                    term(fold, row.subject),
                    term(fold, row.predicate),
                    term(fold, row.object),
                    (1,) if row.graph is None else (2, term(fold, row.graph)),
                )
            )
        elif isinstance(row, EmptyGraphRow):
            empty.append(term(fold, row.graph))
        else:
            raise ValueError(f"unexpected line {row}")
    return Source(commit.envelope.identifier, fold.content_digest, quads, empty)


def term(fold: ContentFold, term_id: int) -> Value:
    """Write a term of the projection as a snapshot term."""
    value: Term = fold.terms.term(term_id)
    match value:
        case Iri(text):
            return (TERM_IRI, text)
        case TypedLiteral(lexical, datatype):
            return (TERM_LITERAL, lexical, FORM_TYPED, datatype)
        case LanguageLiteral(lexical, language, direction):
            return (TERM_LITERAL, lexical, FORMS[direction], language)
        case Triple(s, p, o):
            return (TERM_TRIPLE, term(fold, s), term(fold, p), term(fold, o))
        case Blank():
            raise ValueError("this fixture holds no blank node")
    raise ValueError(f"unknown term {value}")


# The key order of the proposal.


def term_key(value: Value) -> Value:
    tag = value[0]
    if tag == TERM_IRI:
        return (tag, value[1].encode())
    if tag == TERM_LITERAL:
        return (tag, value[1].encode(), value[2], value[3].encode())
    if tag == TERM_BLANK:
        return (tag, value[1])
    return (tag, depth(value), *(term_key(part) for part in value[1:4]))


def depth(value: Value) -> int:
    if value[0] != TERM_TRIPLE:
        return 0
    return 1 + max(depth(part) for part in value[1:4])


def component_key(index: int, value: Value) -> Value:
    if index < 3:
        return term_key(value)
    return (0,) if value[0] == 1 else (1, term_key(value[1]))


def quad_key(quad: Value) -> Value:
    return tuple(component_key(index, value) for index, value in enumerate(quad))


def freeze(value: Any) -> Any:
    return tuple(freeze(v) for v in value) if isinstance(value, list) else value


# The construction.


@dataclass(frozen=True)
class Block:
    kind: str
    data: bytes
    identity: bytes
    offset: int
    lower: Value | None
    rows: int

    @property
    def located(self) -> Value:
        return (self.identity, self.offset, len(self.data))


@dataclass(frozen=True)
class Snapshot:
    data: bytes
    blocks: list[Block]
    manifest: Block


def build_snapshot(source: Source) -> Snapshot:
    quads = sorted(source.quads, key=quad_key)
    if len(set(quads)) != len(quads):
        raise ValueError("the projection is a set")
    out = bytearray()
    blocks: list[Block] = []

    def place(kind: str, value: Value, lower: Value | None, rows: int) -> Block:
        data = cbor2.dumps(value)
        block = Block(kind, data, unit_digest(data), len(out), lower, rows)
        out.extend(data)
        blocks.append(block)
        return block

    cut = [quads[i : i + PAGE_ROWS] for i in range(0, len(quads), PAGE_ROWS)] or [[]]
    level = [
        place("page", (BLOCK_PAGE, tuple(rows)), rows[0] if i else None, len(rows))
        for i, rows in enumerate(cut)
    ]
    height = 0
    while height == 0 or len(level) > 1:
        height += 1
        level = [
            place(
                "directory",
                (
                    BLOCK_DIRECTORY,
                    height,
                    tuple(
                        ((1,) if c.lower is None else (2, c.lower), c.located, c.rows)
                        for c in children
                    ),
                ),
                children[0].lower,
                sum(c.rows for c in children),
            )
            for children in (
                level[i : i + FANOUT] for i in range(0, len(level), FANOUT)
            )
        ]
    manifest = place(
        "manifest",
        (
            BLOCK_MANIFEST,
            SNAPSHOT_VERSION,
            (HASH_BLAKE3_256, source.state),
            (LAW_P3_10A, VARIANT_VISIBLE),
            (AUDIENCE_PUBLIC,),
            (len(quads), source.dataset_digest),
            tuple(sorted(source.empty_graphs, key=term_key)),
            (PAGE_ROWS, FANOUT),
            ((ORDER_SPOG, height, level[0].located),),
        ),
        None,
        len(quads),
    )
    out.extend(trailer(manifest.offset, len(manifest.data)))
    return Snapshot(bytes(out), blocks, manifest)


def trailer(offset: int, length: int) -> bytes:
    """The fixed-length trailer: its two uint are written with the 8-byte head."""
    data = (
        b"\x83"
        + cbor2.dumps(TRAILER_TAG)
        + b"\x1b"
        + offset.to_bytes(8, "big")
        + b"\x1b"
        + length.to_bytes(8, "big")
    )
    if len(data) != TRAILER_LENGTH:
        raise ValueError("the trailer has a fixed length")
    return data


# The query procedure.


@dataclass(frozen=True)
class Fetch:
    kind: str
    identity: bytes


@dataclass(frozen=True)
class Request:
    offset: int
    length: int
    served: bool
    identity: bytes | None


def prefix_cmp(key: Value, bound: Value) -> int:
    head = key[: len(bound)]
    return (head > bound) - (head < bound)


def decides(lower: Value | None, upper: Value | None, bound: Value) -> bool:
    """A node decides when its interval can hold a key with the bound prefix."""
    return (lower is None or prefix_cmp(lower, bound) <= 0) and (
        upper is None or prefix_cmp(upper, bound) >= 0
    )


@dataclass
class Reader:
    snapshot: bytes
    withheld: set[bytes]
    transcript: list[Request]

    def get(self, offset: int, length: int, identity: bytes | None) -> bytes | None:
        served = identity not in self.withheld
        self.transcript.append(Request(offset, length, served, identity))
        if not served:
            return None
        data = self.snapshot[offset : offset + length]
        if identity is not None and unit_digest(data) != identity:
            raise ValueError("these cases hold no identity failure")
        return data


def verify_encoding(data: bytes) -> None:
    """The structure check: one deterministic encoding of one block."""
    if cbor2.dumps(cbor2.loads(data)) != data:
        raise ValueError("structure")


def verify(
    block: Value, level: int, lower: Value | None, upper: Value | None, rows: int
) -> dict[bytes, int]:
    """The fence and count checks of step 4; return the rows each child must hold."""

    def inside(key: Value) -> bool:
        return (lower is None or key >= lower) and (upper is None or key < upper)

    if level == 0:
        keys = [quad_key(row) for row in block[1]]
        if block[0] != BLOCK_PAGE or any(a >= b for a, b in pairwise(keys)):
            raise ValueError("structure or fence")
        if not all(inside(k) for k in keys) or (lower is not None and keys[0] != lower):
            raise ValueError("fence")
        if not keys and (lower is not None or upper is not None):
            raise ValueError("fence")
        if len(keys) != rows:
            raise ValueError("count")
        return {}
    if block[0] != BLOCK_DIRECTORY or block[1] != level or not block[2]:
        raise ValueError("structure")
    fences = [None if e[0][0] == 1 else quad_key(e[0][1]) for e in block[2]]
    if fences[0] != lower or any(f is None for f in fences[1:]):
        raise ValueError("fence")
    present = [f for f in fences[1:] if f is not None]
    if any(a >= b for a, b in pairwise(present)) or not all(map(inside, present)):
        raise ValueError("fence")
    if lower is not None and present and present[0] <= lower:
        raise ValueError("fence")
    if sum(e[2] for e in block[2]) != rows:
        raise ValueError("count")
    return {e[1][0]: e[2] for e in block[2]}


def query(
    snapshot: bytes, pin: bytes, bound: Value, withheld: set[bytes]
) -> tuple[Value, list[Fetch], list[Request]]:
    """Run the procedure of the proposal: the result, its evidence, the requests."""
    reader = Reader(snapshot, withheld, [])
    end = reader.get(len(snapshot) - TRAILER_LENGTH, TRAILER_LENGTH, None)
    if end is None or end[1:15] != cbor2.dumps(TRAILER_TAG):
        raise ValueError("no trailer")
    located = (
        pin,
        int.from_bytes(end[16:24], "big"),
        int.from_bytes(end[25:33], "big"),
    )
    data = reader.get(*located[1:], pin)
    if data is None:
        return (RESULT_INCOMPLETE, (), ((pin, MISSING_WITHHELD),), ()), [], []
    evidence = [Fetch("manifest", pin)]
    manifest = freeze(cbor2.loads(data))
    verify_encoding(data)
    order, height, root = manifest[8][0]
    if order != ORDER_SPOG:
        raise ValueError("these cases use the order spog")
    bound_key = tuple(component_key(i, v) for i, v in enumerate(bound))
    rows: list[Value] = []
    missing: list[bytes] = []
    counts: dict[bytes, int] = {}
    frontier: list[tuple[Value, Value | None, Value | None, int]] = [
        (root, None, None, height)
    ]
    while frontier:
        below: list[tuple[Value, Value | None, Value | None, int]] = []
        for (identity, offset, length), lower, upper, level in frontier:
            data = reader.get(offset, length, identity)
            if data is None:
                missing.append(identity)
                continue
            block = freeze(cbor2.loads(data))
            verify_encoding(data)
            expected = manifest[5][0] if level == height else counts[identity]
            counts.update(verify(block, level, lower, upper, expected))
            evidence.append(Fetch("directory" if level else "page", identity))
            if level == 0:
                rows.extend(
                    r for r in block[1] if prefix_cmp(quad_key(r), bound_key) == 0
                )
                continue
            entries = block[2]
            for index, (fence, child, _rows) in enumerate(entries):
                child_lower = lower if fence[0] == 1 else quad_key(fence[1])
                after = entries[index + 1][0] if index + 1 < len(entries) else None
                child_upper = upper if after is None else quad_key(after[1])
                if decides(child_lower, child_upper, bound_key):
                    below.append((child, child_lower, child_upper, level - 1))
        frontier = below
    identities = tuple(fetch.identity for fetch in evidence)
    result: Value
    if missing:
        withheld_blocks = tuple((m, MISSING_WITHHELD) for m in missing)
        result = (RESULT_INCOMPLETE, tuple(rows), withheld_blocks, identities)
    elif rows:
        result = (RESULT_EXACT, tuple(rows), identities)
    else:
        result = (RESULT_ABSENT, identities)
    return result, evidence, reader.transcript


# Plain renderings for the description.


def plain_term(value: Value) -> str:
    if value[0] == TERM_IRI:
        return f"<{value[1]}>"
    if value[0] == TERM_LITERAL and value[2] == FORM_TYPED:
        return f'"{value[1]}"^^<{value[3]}>'
    if value[0] == TERM_LITERAL:
        return f'"{value[1]}"@{value[3]}'
    raise ValueError(f"no plain form for {value}")


def plain_quad(row: Value) -> list[str]:
    graph = "default" if row[3][0] == 1 else plain_term(row[3][1])
    return [plain_term(row[0]), plain_term(row[1]), plain_term(row[2]), graph]


def certify_case(source_bytes: bytes, source: Source, snapshot: Snapshot) -> object:
    return {
        "label": "certify",
        "note": _CERTIFY_NOTE,
        "source_file": "source.bin",
        "snapshot_file": "snapshot.bin",
        "inputs": {
            "projection": "visible",
            "audience": "public",
            "page_rows": PAGE_ROWS,
            "fanout": FANOUT,
        },
        "result": {
            "outcome": "certified",
            "source_state": source.state.hex(),
            "dataset_digest": source.dataset_digest.hex(),
            "quad_count": len(source.quads),
            "empty_graphs_plain": [
                plain_term(g) for g in sorted(source.empty_graphs, key=term_key)
            ],
            "snapshot_identity": snapshot.manifest.identity.hex(),
            "snapshot_length": len(snapshot.data),
            "manifest": snapshot.manifest.data.hex(),
        },
        "blocks": [
            {
                "kind": block.kind,
                "identity": block.identity.hex(),
                "offset": block.offset,
                "length": len(block.data),
                "rows": block.rows,
                "lower_plain": None if block.lower is None else plain_quad(block.lower),
            }
            for block in snapshot.blocks
        ],
        "quads_plain": [plain_quad(q) for q in sorted(source.quads, key=quad_key)],
    }


def query_case(
    snapshot: Snapshot, label: str, note: str, subject: str, withheld: set[bytes]
) -> object:
    bound: Value = ((TERM_IRI, _E + subject),)
    pin = snapshot.manifest.identity
    result, evidence, transcript = query(snapshot.data, pin, bound, withheld)
    rows = result[1] if result[0] in (RESULT_EXACT, RESULT_INCOMPLETE) else ()
    pattern = (ORDER_SPOG, tuple((2, v) for v in bound) + ((1,),) * (4 - len(bound)))
    return {
        "label": label,
        "note": note,
        "input_file": "snapshot.bin",
        "read": {
            "pin": pin.hex(),
            "withheld": sorted(identity.hex() for identity in withheld),
        },
        "query": {
            "pattern": cbor2.dumps(pattern).hex(),
            "bound_plain": [plain_term(v) for v in bound],
        },
        "result": cbor2.dumps(result).hex(),
        "rows_plain": [plain_quad(row) for row in rows],
        "evidence": [{"kind": f.kind, "identity": f.identity.hex()} for f in evidence],
        "transcript": [
            {
                "range": [r.offset, r.length],
                "http_range": f"bytes={r.offset}-{r.offset + r.length - 1}",
                "served": r.served,
                "block": None if r.identity is None else r.identity.hex(),
            }
            for r in transcript
        ],
    }


def cases(source_bytes: bytes, source: Source, snapshot: Snapshot) -> Iterator[object]:
    second_page = [b for b in snapshot.blocks if b.kind == "page"][1].identity
    yield certify_case(source_bytes, source, snapshot)
    yield query_case(snapshot, "hit", _HIT_NOTE, "b", set())
    yield query_case(snapshot, "absent-neighbour", _ABSENT_NOTE, "c", set())
    yield query_case(snapshot, "absent-withheld", _WITHHELD_NOTE, "c", {second_page})


def main() -> None:
    source_bytes = build_source()
    source = read_source(source_bytes)
    snapshot = build_snapshot(source)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "source.bin").write_bytes(source_bytes)
    (OUT / "snapshot.bin").write_bytes(snapshot.data)
    description = {
        "name": "snapshot/spog-public",
        "section": "GIP queryable snapshots",
        "status": "built and matches",
        "note": _ENTRY_NOTE,
        "cases": list(cases(source_bytes, source, snapshot)),
    }
    (OUT / "snapshot.json").write_text(
        json.dumps(description, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    (OUT / "INDEX.md").write_text(_INDEX_TEXT, encoding="utf-8")
    names = ("INDEX.md", "snapshot.bin", "snapshot.json", "source.bin")
    (OUT / "SHA256SUMS").write_text(
        "".join(
            f"{hashlib.sha256((OUT / name).read_bytes()).hexdigest()}  {name}\n"
            for name in names
        ),
        encoding="utf-8",
    )
    print((OUT / "SHA256SUMS").read_text(encoding="utf-8"), end="")


if __name__ == "__main__":
    main()

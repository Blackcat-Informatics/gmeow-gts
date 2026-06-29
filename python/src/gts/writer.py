# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""A GTS writer: build frames, maintain the id/prev chain, emit a CBOR Sequence.

This is the encoder counterpart to :mod:`gts.reader`. It drives the
conformance vectors and is the seed of the future ``RDF 1.2 → GTS`` producer.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence

import cbor2

from gts.codec import DEFAULT_CATALOG, Codec, encode_chain
from gts.crypto import Signer, encrypt0, sign_id
from gts.model import (
    AnnotationRow,
    Graph,
    Quad,
    ReifierRow,
    Suppression,
    Term,
    TermKind,
    is_literal_direction,
)
from gts.wire import (
    MAGIC,
    SELF_DESCRIBE_TAG,
    VERSION,
    canonical,
    content_id,
    digest_str,
    header_id,
)


def term_to_wire(t: Term) -> dict[str, object]:
    """Serialise a :class:`Term` to its wire map (dropping absent fields)."""
    out: dict[str, object] = {"k": int(t.kind)}
    if t.value is not None:
        out["v"] = t.value
    if t.datatype is not None:
        out["dt"] = t.datatype
    if t.lang is not None:
        out["l"] = t.lang
    if is_literal_direction(t.direction):
        out["dir"] = t.direction
    if t.reifier is not None:
        out["rf"] = t.reifier
    return out


def _private_use_language_tag(tag: str) -> bool:
    """True for private-use language tags such as GMEOW's ``x-gmeow-*``."""
    tag_lower = tag.lower()
    return tag_lower.startswith("x-") or "-x-" in tag_lower


def _validate_term_language_tags(terms: list[Term], section: str) -> None:
    """Enforce §13.1 language-tag discipline at write time.

    Canonical sections (e.g. a ``dist`` or ``ai-package`` graph payload) MAY
    carry internal private-use tags. Projection/docs sections MUST carry public
    BCP 47 tags only; a private-use tag leaking into a projection section is a
    hard failure, not a warning (§13.1, vector 20).
    """
    if section == "canonical":
        return
    for t in terms:
        if t.lang is not None and _private_use_language_tag(t.lang):
            msg = (
                f"private-use language tag {t.lang!r} is not allowed in "
                f"a projection/docs section (§13.1)"
            )
            raise ValueError(msg)


def _deterministic_term_remap(graph: Graph) -> tuple[dict[int, int], list[int]]:
    """Return ``old -> new`` ids and ``new -> old`` term order for a graph."""
    old_by_new = sorted(
        range(len(graph.terms)),
        key=lambda tid: (canonical(_term_identity(graph, tid, [])), tid),
    )
    old_to_new = {old: new for new, old in enumerate(old_by_new)}
    return old_to_new, old_by_new


def _term_identity(graph: Graph, tid: int, stack: list[int]) -> list[object]:
    """Semantic term identity used for deterministic authoring order."""
    if tid in stack:
        return ["cycle", tid]
    try:
        term = graph.terms[tid]
    except IndexError:
        return ["missing", tid]
    stack.append(tid)
    out: list[object]
    if term.kind is TermKind.IRI:
        out = ["iri", term.value]
    elif term.kind is TermKind.LITERAL:
        out = [
            "literal",
            term.value,
            graph.datatype_iri(term),
            term.lang,
            term.direction,
        ]
    elif term.kind is TermKind.BNODE:
        out = [
            "bnode",
            term.value if term.value else ["anonymous", tid],
        ]
    else:
        triple = graph.reifier(term.reifier) if term.reifier is not None else None
        if triple is None:
            out = ["triple", None, term.reifier]
        else:
            s, p, o = triple
            out = [
                "triple",
                _term_identity(graph, s, stack),
                _term_identity(graph, p, stack),
                _term_identity(graph, o, stack),
            ]
    stack.pop()
    return out


def _remap_id(old_to_new: dict[int, int], tid: int) -> int:
    return old_to_new.get(tid, tid)


def _remap_term(term: Term, old_to_new: dict[int, int]) -> Term:
    return Term(
        kind=term.kind,
        value=term.value,
        datatype=_remap_id(old_to_new, term.datatype)
        if term.datatype is not None
        else None,
        lang=term.lang,
        direction=term.direction,
        reifier=_remap_id(old_to_new, term.reifier)
        if term.reifier is not None
        else None,
    )


def _remap_suppression(
    suppression: Suppression, old_to_new: dict[int, int]
) -> Suppression:
    targets: list[Mapping[str, object]] = []
    for target in suppression.targets:
        kind = target.get("kind")
        remapped = dict(target)
        tid = remapped.get("id")
        q = remapped.get("q")
        if kind in ("term", "reifier") and isinstance(tid, int):
            remapped["id"] = _remap_id(old_to_new, tid)
        elif kind == "quad" and isinstance(q, list):
            remapped["q"] = [
                _remap_id(old_to_new, item) if isinstance(item, int) else item
                for item in q
            ]
        targets.append(remapped)
    return Suppression(
        targets=targets,
        reason=suppression.reason,
        by=_remap_id(old_to_new, suppression.by)
        if suppression.by is not None
        else None,
    )


def _suppression_key(suppression: Suppression) -> bytes:
    payload: dict[str, object] = {"targets": list(suppression.targets)}
    if suppression.reason is not None:
        payload["reason"] = suppression.reason
    if suppression.by is not None:
        payload["by"] = suppression.by
    return canonical(payload)


def _quad_key(quad: Quad) -> bytes:
    row = [quad[0], quad[1], quad[2]]
    if quad[3] is not None:
        row.append(quad[3])
    return canonical(row)


class Writer:
    """Accumulates a GTS log as a CBOR Sequence.

    Args:
        profile: The header ``"prof"`` value (§13).
        catalog: The transform catalog (id → :class:`Codec`).
        meta: Optional header metadata.
        magic_tag: Prefix the Header with the CBOR self-describe tag (§3).
    """

    def __init__(
        self,
        profile: str = "generic",
        catalog: dict[int, Codec] | None = None,
        meta: dict[str, object] | None = None,
        *,
        magic_tag: bool = True,
        signer: Signer | None = None,
        layout: str | None = None,
    ) -> None:
        """Initialise the writer and emit the Header (the chain genesis).

        If ``signer`` is given, every appended frame is COSE_Sign1-signed over its
        ``id`` (§9.2) — the basis of the ``evidence`` profile's chain of custody.
        ``layout`` writes the header layout-state claim (§3.3; ``"streamable"``
        is the only value this revision defines).
        """
        if layout is not None and layout != "streamable":
            # §5: "streamable" is the only layout this revision defines; a
            # typo'd claim would persist into the tamper-evident header.
            msg = f"unsupported layout claim {layout!r} (§3.3)"
            raise ValueError(msg)
        self._signer = signer
        self.catalog = catalog or dict(DEFAULT_CATALOG)
        self._name_to_id = {c.name: i for i, c in self.catalog.items()}
        header: dict[str, object] = {
            "gts": MAGIC,
            "v": VERSION,
            "prof": profile,
            "cat": {i: {"name": c.name, "cls": c.cls} for i, c in self.catalog.items()},
        }
        if layout is not None:
            header["layout"] = layout
        if meta is not None:
            header["meta"] = meta
        header_id_value = header_id(header)
        header["id"] = header_id_value
        self._prev = header_id_value
        first = cbor2.CBORTag(SELF_DESCRIBE_TAG, header) if magic_tag else header
        self._buf = bytearray(canonical(first))
        # Per-frame byte offsets and types, in append order — the raw material
        # of an `index` footer (§6.2): offsets enable random access/parallel
        # verify, types the "ti" locator map.
        self._offsets: list[int] = []
        self._types: list[str] = []

    @classmethod
    def deterministic(cls, graph: Graph, profile: str = "dist") -> Writer:
        """Build a deterministic single-segment writer from folded graph state.

        This high-level authoring path remaps terms by semantic value, emits
        authorable graph frames in a fixed order, and relies on deterministic
        CBOR for every hashed frame. It does not replay reader observations such
        as diagnostics, signatures, opaque nodes, or segment ledgers.
        """
        old_to_new, old_by_new = _deterministic_term_remap(graph)
        writer = cls(profile=profile)

        if old_by_new:
            writer.add_terms(
                [_remap_term(graph.terms[old], old_to_new) for old in old_by_new]
            )

        quads = [
            (
                _remap_id(old_to_new, s),
                _remap_id(old_to_new, p),
                _remap_id(old_to_new, o),
                _remap_id(old_to_new, g) if g is not None else None,
            )
            for s, p, o, g in graph.quads
        ]
        quads.sort(key=_quad_key)
        if quads:
            writer.add_quads(quads)

        reifiers = sorted(
            (
                _remap_id(old_to_new, rid),
                (
                    _remap_id(old_to_new, s),
                    _remap_id(old_to_new, p),
                    _remap_id(old_to_new, o),
                ),
                _remap_id(old_to_new, g) if g is not None else None,
            )
            for rid, (s, p, o), g in graph.reifiers
        )
        reifiers.sort(key=lambda row: (row[2], row[0], row[1][0], row[1][1], row[1][2]))
        if reifiers:
            writer.add_reifies(reifiers)

        annotations = sorted(
            (
                (
                    _remap_id(old_to_new, r),
                    _remap_id(old_to_new, p),
                    _remap_id(old_to_new, v),
                    _remap_id(old_to_new, g) if g is not None else None,
                )
                for r, p, v, g in graph.annotations
            ),
            key=lambda row: (row[3], row[0], row[1], row[2]),
        )
        if annotations:
            writer.add_annot(annotations)

        for digest, data in sorted(graph.blobs.items()):
            meta = graph.blob_meta.get(digest, {})
            mt = meta.get("mt")
            rep = meta.get("rep")
            writer.add_blob(
                data,
                mt=mt if isinstance(mt, str) else None,
                rep=rep if isinstance(rep, str) else None,
            )

        if graph.meta:
            writer.add_meta(dict(sorted(graph.meta.items())))

        suppressions = sorted(
            (
                _remap_suppression(suppression, old_to_new)
                for suppression in graph.suppressions
            ),
            key=_suppression_key,
        )
        for suppression in suppressions:
            writer.add_suppress(
                suppression.targets,
                reason=suppression.reason,
                by=suppression.by,
            )

        return writer

    @property
    def head(self) -> bytes:
        """The id the next appended frame must reference as ``"prev"``."""
        return self._prev

    def _chain_ids(self, chain: list[str] | None) -> list[int]:
        """Resolve codec names to file-local catalog ids."""
        return [self._name_to_id[name] for name in (chain or [])]

    def add_frame(
        self,
        frame_type: str,
        *,
        payload: object | None = None,
        raw: bytes | None = None,
        transform: list[str] | None = None,
        zstd_level: int | None = None,
        pub: object | None = None,
        to: list[dict[str, object]] | None = None,
        sig: bytes | None = None,
        encrypt: tuple[str, bytes] | None = None,
    ) -> bytes:
        """Append one frame and return its ``"id"``.

        ``payload`` (structured CBOR) and ``raw`` (blob bytes) are mutually exclusive
        payload sources. ``transform`` compresses/encodes the payload; ``encrypt``
        ``(kid, key)`` then seals it as a ``COSE_Encrypt0`` (the outermost transform)
        and records the recipient in ``"to"`` (§9.3). ``"d"`` becomes a byte string.
        ``zstd_level`` is a per-frame encoder option for ``zstd`` and
        ``zstd-rsyncable`` transforms; ``None`` preserves the previous default.

        Raises:
            ValueError: if both ``payload`` and ``raw`` are given, or if ``transform``/
                ``encrypt`` is requested with neither source.
        """
        if payload is not None and raw is not None:
            msg = "payload and raw are mutually exclusive"
            raise ValueError(msg)
        if (transform or encrypt) and payload is None and raw is None:
            msg = "transform/encrypt requires a payload or raw source"
            raise ValueError(msg)
        if zstd_level is not None and not any(
            name in ("zstd", "zstd-rsyncable") for name in (transform or [])
        ):
            msg = "zstd_level requires a zstd or zstd-rsyncable transform"
            raise ValueError(msg)
        frame: dict[str, object] = {"t": frame_type}
        if transform or encrypt is not None:
            data = raw if raw is not None else canonical(payload)
            x_ids: list[int] = []
            if transform:
                data = encode_chain(transform, data, zstd_level=zstd_level)
                x_ids += self._chain_ids(transform)
            if encrypt is not None:
                encrypt_id = self._name_to_id.get("cose-encrypt0")
                if encrypt_id is None:
                    msg = "encrypt requires a catalog entry for 'cose-encrypt0'"
                    raise ValueError(msg)
                kid, key = encrypt
                data = encrypt0(data, kid, key)
                x_ids.append(encrypt_id)
            frame["x"] = x_ids
            frame["d"] = data
        elif raw is not None:
            frame["d"] = raw
        elif payload is not None:
            frame["d"] = payload
        if pub is not None:
            frame["pub"] = pub
        recipients = list(to) if to is not None else []
        if encrypt is not None:
            recipients.append({"kid": encrypt[0]})
        if recipients:
            frame["to"] = recipients
        frame["prev"] = self._prev
        fid = content_id(frame)
        frame["id"] = fid
        if sig is None and self._signer is not None:
            sig = sign_id(fid, self._signer)
        if sig is not None:
            frame["sig"] = sig
        self._offsets.append(len(self._buf))
        self._types.append(frame_type)
        self._buf += canonical(frame)
        self._prev = fid
        return self._prev

    # -- convenience builders -------------------------------------------------

    def add_terms(
        self,
        terms: list[Term],
        *,
        transform: list[str] | None = None,
        zstd_level: int | None = None,
        section: str = "canonical",
    ) -> bytes:
        """Append a ``terms`` frame.

        Args:
            terms: Terms to serialize into the frame.
            transform: Optional transform chain applied to the frame.
            zstd_level: Optional per-frame zstd level for zstd transforms.
            section: ``"canonical"`` for graph payloads that may carry internal
                private-use language tags; ``"projection"`` for docs/derived-view
                sections that MUST use public BCP 47 tags only (§13.1).
        """
        _validate_term_language_tags(terms, section)
        return self.add_frame(
            "terms",
            payload=[term_to_wire(t) for t in terms],
            transform=transform,
            zstd_level=zstd_level,
        )

    def add_quads(
        self,
        quads: list[Quad],
        *,
        transform: list[str] | None = None,
        zstd_level: int | None = None,
    ) -> bytes:
        """Append a ``quads`` frame (drops a ``None`` graph slot)."""
        rows = [
            [q[0], q[1], q[2], *([q[3]] if q[3] is not None else [])] for q in quads
        ]
        return self.add_frame(
            "quads", payload=rows, transform=transform, zstd_level=zstd_level
        )

    def add_reifies(self, bindings: list[ReifierRow]) -> bytes:
        """Append a ``reifies`` frame."""
        payload = [
            [rid, *spo, *([graph_name] if graph_name is not None else [])]
            for rid, spo, graph_name in bindings
        ]
        return self.add_frame("reifies", payload=payload)

    def add_annot(self, rows: list[AnnotationRow]) -> bytes:
        """Append an ``annot`` frame (reifier, predicate, value rows)."""
        return self.add_frame(
            "annot",
            payload=[
                [r, p, v, *([graph_name] if graph_name is not None else [])]
                for r, p, v, graph_name in rows
            ],
        )

    def add_blob(
        self,
        data: bytes,
        *,
        mt: str | None = None,
        rep: str | None = None,
        transform: list[str] | None = None,
        zstd_level: int | None = None,
    ) -> bytes:
        """Append an inline ``blob`` frame; metadata goes in ``pub`` (§12).

        The decoded content digest is included in ``pub.digest`` so the reader
        can address the blob lazily without decompressing the frame first.
        """
        pub: dict[str, object] = {"digest": digest_str(data)}
        if mt is not None:
            pub["mt"] = mt
        if rep is not None:
            pub["rep"] = rep
        return self.add_frame(
            "blob", raw=data, transform=transform, zstd_level=zstd_level, pub=pub
        )

    def add_meta(self, meta: dict[str, object]) -> bytes:
        """Append a ``meta`` frame."""
        return self.add_frame("meta", payload=meta)

    def add_suppress(
        self,
        targets: Sequence[Mapping[str, object]],
        *,
        reason: str | None = None,
        by: int | None = None,
    ) -> bytes:
        """Append a ``suppress`` frame (§11)."""
        payload: dict[str, object] = {"targets": [dict(t) for t in targets]}
        if reason is not None:
            payload["reason"] = reason
        if by is not None:
            payload["by"] = by
        return self.add_frame("suppress", payload=payload)

    def add_index(self) -> bytes:
        """Append an ``index`` footer covering every frame appended so far (§6.2).

        ``count``/``head`` delimit the covered region (the streamable boundary,
        §3.3); ``off`` carries each covered frame's byte offset from the start
        of this writer's output; ``ti`` locates frames by type (0-based frame
        positions). A later ``add_index`` covers the earlier one too — the last
        index wins (§6.2).
        """
        ti: dict[str, list[int]] = {}
        for pos, ftype in enumerate(self._types):
            ti.setdefault(ftype, []).append(pos)
        payload: dict[str, object] = {
            "count": len(self._types),
            "head": self._prev,
        }
        if self._offsets:  # "off"/"ti" are [+ uint]-shaped — omit when empty
            payload["off"] = list(self._offsets)
            payload["ti"] = ti
        return self.add_frame("index", payload=payload)

    def to_bytes(self) -> bytes:
        """Return the complete GTS file."""
        return bytes(self._buf)

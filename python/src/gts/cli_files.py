# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Files, blob, composition, and compact commands for the Python CLI."""

from __future__ import annotations

import sys
from pathlib import Path

from gts.cli_common import _has_problems, _load, _write_out
from gts.model import Graph
from gts.reader import read, read_segments


def _all_quads_suppressed(g: Graph) -> bool:
    """True iff the fold has quads and EVERY one is hidden by a suppression.

    A quad is hidden by a direct quad target or a term target on any of its
    components (§11) — the union graph is value-interned, so id matching IS
    value matching.
    """
    if not g.quads or not g.suppressions:
        return False
    term_sup: set[int] = set()
    quad_sup: set[tuple[int, ...]] = set()
    for sup in g.suppressions:
        for target in sup.targets:
            kind = target.get("kind")
            tid = target.get("id")
            if kind in ("term", "reifier") and isinstance(tid, int):
                term_sup.add(tid)
            elif kind == "quad":
                q = target.get("q")
                if isinstance(q, list) and all(isinstance(x, int) for x in q):
                    quad_sup.add(tuple(q))
    return all(
        (s, p, o) in quad_sup
        or (gq is not None and (s, p, o, gq) in quad_sup)
        or s in term_sup
        or p in term_sup
        or o in term_sup
        or (gq is not None and gq in term_sup)
        for s, p, o, gq in g.quads
    )


def _cmd_ls(path: str) -> int:
    """List inline blobs: digest, size, declared media type (tar's ``t``)."""
    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    for digest, data in g.blobs.items():
        mt = g.blob_meta.get(digest, {}).get("mt")
        mt_text = mt if isinstance(mt, str) else "-"
        print(f"{digest}  {len(data):>10}  {mt_text}")
    return 0


def _normalize_digest(digest: str) -> str:
    return digest if digest.startswith("blake3:") else f"blake3:{digest}"


def _suppressed_blob_digests(g: Graph) -> set[str]:
    """Digests hidden by ``{"kind": "blob", "digest": …}`` targets (§11)."""
    out: set[str] = set()
    for sup in g.suppressions:
        for target in sup.targets:
            if target.get("kind") != "blob":
                continue
            d = target.get("digest")
            if isinstance(d, bytes):
                out.add(f"blake3:{d.hex()}")
            elif isinstance(d, str):
                out.add(_normalize_digest(d))
    return out


def _cmd_extract(
    path: str,
    digest: str,
    out: str | None,
    mt: str | None,
    include_suppressed: bool,
) -> int:
    """Extract one blob by content digest (tar's ``x``), refuse-don't-trust.

    Verifies the bytes against the requested digest on the way out, honours
    blob suppression (§11) unless overridden, and treats ``--mt`` as an
    ASSERTION against the blob's declared media type — never a conversion.
    """
    g = read(_load(path))
    digest = _normalize_digest(digest)
    data = g.blobs.get(digest)
    if data is None:
        print(f"gts: no inline blob {digest} in {path}", file=sys.stderr)
        return 1
    if digest in _suppressed_blob_digests(g) and not include_suppressed:
        print(
            f"gts: refusing {digest}: suppressed (§11); "
            "pass --include-suppressed to extract anyway",
            file=sys.stderr,
        )
        return 1
    if mt is not None:
        declared = g.blob_meta.get(digest, {}).get("mt")
        if declared != mt:
            print(
                f"gts: refusing {digest}: declared media type "
                f"{declared!r} does not match asserted {mt!r}",
                file=sys.stderr,
            )
            return 1
    from gts.wire import digest_str

    if digest_str(data) != digest:
        print(
            f"gts: integrity failure: {digest} bytes re-hash differently",
            file=sys.stderr,
        )
        return 1
    return _write_out(out, data)


def _cmd_cat(paths: list[str], out: str | None) -> int:
    """The validating composer (§14.1): refuse-don't-trust, then ``cat``."""
    if len(paths) < 2:
        print("gts: cat needs at least two inputs", file=sys.stderr)
        return 2
    combined = bytearray()
    for path in paths:
        data = _load(path)
        segments, torn, fatal = read_segments(data)
        if _has_problems(segments, torn, fatal):
            print(f"gts: refusing {path}: not a clean GTS input", file=sys.stderr)
            return 1
        # §14.1: a segment that contributes NOTHING (no quads, blobs, reifier
        # bindings, annotations, or suppressions) is almost always a wiring
        # bug — never a real package. Refuse, don't trust.
        for idx, seg in enumerate(segments):
            contributes = bool(
                seg.quads
                or seg.blobs
                or seg.reifiers
                or seg.annotations
                or seg.suppressions
            )
            if not contributes:
                print(
                    f"gts: refusing {path}: segment {idx} folds to nothing "
                    "(no quads/blobs/reifies/annot/suppress) — wiring bug?",
                    file=sys.stderr,
                )
                return 1
        combined += data

    # §14.1: refuse an output in which suppressions would hide every quad.
    folded = read(bytes(combined))
    if _all_quads_suppressed(folded):
        print(
            "gts: refusing composition: suppressions hide every quad in the "
            "folded output",
            file=sys.stderr,
        )
        return 1

    return _write_out(out, bytes(combined))


def _cmd_compact(
    path: str,
    out: str,
    *,
    streamable: bool,
    seal_original: bool,
    timestamp: str | None,
) -> int:
    """Rewrite a GTS file into the streamable layout state (§10.1, §14.1)."""
    if not streamable:
        # The verb is reserved for layout rewrites; a future --snapshot mode
        # (§10) would land here. Without a mode the request is ambiguous.
        print("gts: compact requires --streamable", file=sys.stderr)
        return 2
    from datetime import UTC, datetime

    from gts.compact import CompactRefusedError, compact_streamable

    ts = timestamp or datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    try:
        data = compact_streamable(
            _load(path), timestamp=ts, seal_original=seal_original
        )
    except CompactRefusedError as exc:
        print(f"gts: refusing compact: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


def _cmd_pack(sources: list[str], out: str) -> int:
    """Pack files/directories into a files-profile GTS archive (tar's ``c``)."""
    from gts.files import pack

    try:
        data = pack([Path(s) for s in sources])
    except (OSError, ValueError) as exc:
        print(f"gts: refusing pack: {exc}", file=sys.stderr)
        return 1
    return _write_out(out, data)


def _cmd_unpack(path: str, dest: str | None, include_suppressed: bool) -> int:
    """Unpack a files-profile GTS archive (tar's ``x``), verifying digests."""
    from gts.files import unpack

    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    if g.diagnostics or not g.segment_heads:
        print("gts: refusing unpack: archive did not read cleanly", file=sys.stderr)
        return 1
    try:
        unpack(g, Path(dest or "."), include_suppressed=include_suppressed)
    except (OSError, ValueError) as exc:
        print(f"gts: refusing unpack: {exc}", file=sys.stderr)
        return 1
    return 0


def _cmd_diff(path: str, directory: str) -> int:
    """Compare an archive to a directory by content digest (tar's ``d``)."""
    from gts.files import diff

    g = read(_load(path))
    for d in g.diagnostics:
        print(f"gts: diagnostic {d.code}: {d.detail}", file=sys.stderr)
    if g.diagnostics or not g.segment_heads:
        print("gts: refusing diff: archive did not read cleanly", file=sys.stderr)
        return 1
    try:
        lines = diff(g, Path(directory))
    except (OSError, ValueError) as exc:
        print(f"gts: refusing diff: {exc}", file=sys.stderr)
        return 1
    for line in lines:
        print(line)
    return 1 if lines else 0

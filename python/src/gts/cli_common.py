# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Shared helpers for the internal Python CLI command modules."""

from __future__ import annotations

import sys
from pathlib import Path

from gts.model import Graph


def _load(path: str) -> bytes:
    try:
        return Path(path).read_bytes()
    except OSError as exc:
        print(f"gts: cannot read {path}: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc


def _print_ledger(path: str, segments: list[Graph], torn: int | None) -> None:
    """Print the per-segment composition ledger (§14.1 "SHOULD report")."""
    suffix = f", TORN at byte {torn}" if torn is not None else ""
    print(f"{path}: {len(segments)} segment(s){suffix}")
    for idx, seg in enumerate(segments):
        head = seg.segment_heads[0].hex() if seg.segment_heads else "<none>"
        profile = seg.segment_profiles[0] if seg.segment_profiles else "<none>"
        signers = sum(1 for s in seg.signatures if s.status != "invalid")
        print(
            f"  segment {idx}: head {head} profile {profile} "
            f"terms {len(seg.terms)} quads {len(seg.quads)} "
            f"reifies {len(seg.reifiers)} annot {len(seg.annotations)} "
            f"blobs {len(seg.blobs)} suppress {len(seg.suppressions)} "
            f"opaque {len(seg.opaque)} sigs {signers}"
        )
        layout = seg.segment_streamable[0] if seg.segment_streamable else None
        if layout is not None and layout.claimed:
            head_hex = layout.head.hex() if layout.head is not None else "<none>"
            tail = f", accretive tail {layout.tail} frame(s)" if layout.tail else ""
            print(
                f"    layout: streamable through frame {layout.covered} "
                f"(head {head_hex}){tail}"
            )
        for o in seg.opaque:
            print(f"    opaque: {o.frame_type} ({o.reason})")
        for d in seg.diagnostics:
            where = f" [item {d.frame_index}]" if d.frame_index is not None else ""
            print(f"    diagnostic {d.code}: {d.detail}{where}")


def _has_problems(
    segments: list[Graph], torn: int | None, fatal: object | None
) -> bool:
    return (
        fatal is not None
        or torn is not None
        or any(seg.diagnostics for seg in segments)
    )


def _write_out(out: str | None, data: bytes) -> int:
    """Write to a path or stdout; IO failure is exit 2, never a traceback."""
    try:
        if out is not None:
            Path(out).write_bytes(data)
        else:
            sys.stdout.buffer.write(data)
    except OSError as exc:  # includes BrokenPipeError
        print(f"gts: cannot write {out or 'stdout'}: {exc}", file=sys.stderr)
        return 2
    return 0

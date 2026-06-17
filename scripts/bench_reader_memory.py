#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Benchmark full-reader memory against a frame-scan baseline."""

from __future__ import annotations

import argparse
import gc
import io
import json
import sys
import time
import tracemalloc
from collections.abc import Mapping
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PY_SRC = ROOT / "python" / "src"
if str(PY_SRC) not in sys.path:
    sys.path.insert(0, str(PY_SRC))


@dataclass
class BenchmarkRow:
    path: str
    bytes: int
    mode: str
    items: int | None
    frames: int | None
    terms: int | None
    quads: int | None
    blobs: int | None
    peak_kib: float | None
    elapsed_ms: float | None
    note: str


def measure(fn: Any) -> tuple[Any, float, float]:
    gc.collect()
    tracemalloc.start()
    try:
        start = time.perf_counter()
        result = fn()
        elapsed_ms = (time.perf_counter() - start) * 1000.0
        _, peak = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()
    return result, peak / 1024.0, elapsed_ms


def is_header_item(item: object) -> bool:
    import cbor2

    if isinstance(item, cbor2.CBORTag):
        item = item.value
    return isinstance(item, Mapping) and "gts" in item and "t" not in item


def scan_items(data: bytes) -> dict[str, int]:
    import cbor2

    stream = io.BytesIO(data)
    decoder = cbor2.CBORDecoder(stream)
    items = 0
    frames = 0
    headers = 0
    while stream.tell() < len(data):
        try:
            item = decoder.decode()
        except (EOFError, cbor2.CBORDecodeEOF):
            break
        items += 1
        if is_header_item(item):
            headers += 1
        else:
            frames += 1
    return {"items": items, "frames": frames, "headers": headers}


def full_read(data: bytes) -> dict[str, int]:
    from gts.reader import read

    graph = read(data)
    return {
        "items": None,
        "frames": None,
        "terms": len(graph.terms),
        "quads": len(graph.quads),
        "blobs": len(graph.blobs),
    }


def row_from_result(
    path: Path,
    size: int,
    mode: str,
    result: dict[str, int | None],
    peak_kib: float,
    elapsed_ms: float,
    note: str,
) -> BenchmarkRow:
    return BenchmarkRow(
        path=str(path),
        bytes=size,
        mode=mode,
        items=result.get("items"),
        frames=result.get("frames"),
        terms=result.get("terms"),
        quads=result.get("quads"),
        blobs=result.get("blobs"),
        peak_kib=round(peak_kib, 1),
        elapsed_ms=round(elapsed_ms, 2),
        note=note,
    )


def benchmark(path: Path) -> list[BenchmarkRow]:
    data = path.read_bytes()
    full, full_peak, full_elapsed = measure(lambda: full_read(data))
    scan, scan_peak, scan_elapsed = measure(lambda: scan_items(data))
    return [
        row_from_result(
            path,
            len(data),
            "full-reader",
            full,
            full_peak,
            full_elapsed,
            "materializes Graph",
        ),
        row_from_result(
            path,
            len(data),
            "frame-scan",
            scan,
            scan_peak,
            scan_elapsed,
            "CBOR item scan only; not a streaming fold",
        ),
        BenchmarkRow(
            path=str(path),
            bytes=len(data),
            mode="streaming-fold",
            items=None,
            frames=None,
            terms=None,
            quads=None,
            blobs=None,
            peak_kib=None,
            elapsed_ms=None,
            note="deferred: no current engine claims a streaming sink API",
        ),
    ]


def format_cell(value: object) -> str:
    if value is None:
        return "n/a"
    return str(value)


def print_markdown(rows: list[BenchmarkRow]) -> None:
    headers = [
        "path",
        "bytes",
        "mode",
        "items",
        "frames",
        "terms",
        "quads",
        "blobs",
        "peak_kib",
        "elapsed_ms",
        "note",
    ]
    print("| " + " | ".join(headers) + " |")
    print("|" + "|".join("---" for _ in headers) + "|")
    for row in rows:
        data = asdict(row)
        print("| " + " | ".join(format_cell(data[h]) for h in headers) + " |")


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Benchmark current full-reader memory separately from a frame-scan "
            "baseline. True streaming fold remains deferred."
        )
    )
    parser.add_argument("files", nargs="+", type=Path)
    parser.add_argument("--json", action="store_true", help="emit JSON rows")
    args = parser.parse_args()

    rows: list[BenchmarkRow] = []
    for path in args.files:
        rows.extend(benchmark(path))

    if args.json:
        print(json.dumps([asdict(row) for row in rows], indent=2, sort_keys=True))
    else:
        print_markdown(rows)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

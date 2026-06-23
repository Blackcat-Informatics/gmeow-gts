#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Benchmark full-reader memory against a frame-scan baseline."""

from __future__ import annotations

import argparse
import gc
import io
import json
import shutil
import subprocess
import sys
import time
import tracemalloc
from collections.abc import Mapping
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PY_SRC = ROOT / "python" / "src"
RUST_EXAMPLE_TIMEOUT_SECONDS = 30
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
    peak_kib: float | None,
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
        peak_kib=round(peak_kib, 1) if peak_kib is not None else None,
        elapsed_ms=round(elapsed_ms, 2),
        note=note,
    )


def rust_example_path() -> Path:
    suffix = ".exe" if sys.platform == "win32" else ""
    return (
        ROOT
        / "rust"
        / "target"
        / "debug"
        / "examples"
        / f"streaming_sink_bench{suffix}"
    )


def rust_streaming_fold(
    path: Path,
) -> tuple[dict[str, int | None], float | None, float]:
    cargo = shutil.which("cargo")
    if cargo is None:
        raise RuntimeError("cargo not found on PATH")
    subprocess.run(
        [
            cargo,
            "build",
            "--quiet",
            "--manifest-path",
            str(ROOT / "rust" / "Cargo.toml"),
            "--example",
            "streaming_sink_bench",
        ],
        cwd=ROOT,
        check=True,
    )
    exe = rust_example_path()
    start = time.perf_counter()
    try:
        completed = subprocess.run(
            [str(exe), str(path.resolve())],
            cwd=ROOT,
            check=True,
            text=True,
            capture_output=True,
            timeout=RUST_EXAMPLE_TIMEOUT_SECONDS,
        )
        result = json.loads(completed.stdout)
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(
            f"Rust streaming sink timed out after {RUST_EXAMPLE_TIMEOUT_SECONDS}s"
        ) from exc
    except json.JSONDecodeError as exc:
        raise RuntimeError("Rust streaming sink emitted invalid JSON") from exc
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    peak = result.get("peak_kib")
    peak_kib = float(peak) if isinstance(peak, (int, float)) else None
    return result, peak_kib, elapsed_ms


def typescript_streaming_fold(
    path: Path,
) -> tuple[dict[str, int | None], float | None, float]:
    npm = shutil.which("npm")
    node = shutil.which("node")
    if npm is None or node is None:
        raise RuntimeError(
            "npm and node are required for TypeScript streaming evidence"
        )
    subprocess.run(
        [npm, "run", "build", "--silent"],
        cwd=ROOT / "ts",
        check=True,
    )
    script = r"""
import { createReadStream } from "node:fs";
import { Readable } from "node:stream";
import { foldStreamToSink } from "./ts/dist/browser.js";

const path = process.argv[1];
const counts = new Map();
const stream = Readable.toWeb(createReadStream(path));
const result = await foldStreamToSink(stream, {
  allowSegments: true,
  onEvent(event) {
    counts.set(event.kind, (counts.get(event.kind) ?? 0) + 1);
  },
});
const rss = process.memoryUsage().rss / 1024;
console.log(JSON.stringify({
  items: null,
  frames: null,
  terms: counts.get("term") ?? 0,
  quads: counts.get("quad") ?? 0,
  blobs: counts.get("blob") ?? 0,
  reifiers: counts.get("reifier") ?? 0,
  annotations: counts.get("annotation") ?? 0,
  suppressions: counts.get("suppression") ?? 0,
  opaque: counts.get("opaque") ?? 0,
  signatures: counts.get("signature") ?? 0,
  diagnostics: result.diagnostics.length,
  segment_heads: result.segmentHeads.length,
  streamable_layouts: result.segmentStreamable.length,
  peak_kib: rss
}));
"""
    start = time.perf_counter()
    completed = subprocess.run(
        [node, "--input-type=module", "--eval", script, str(path.resolve())],
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
        timeout=RUST_EXAMPLE_TIMEOUT_SECONDS,
    )
    result = json.loads(completed.stdout)
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    peak = result.get("peak_kib")
    peak_kib = float(peak) if isinstance(peak, (int, float)) else None
    return result, peak_kib, elapsed_ms


def benchmark(path: Path) -> list[BenchmarkRow]:
    data = path.read_bytes()
    full, full_peak, full_elapsed = measure(lambda: full_read(data))
    scan, scan_peak, scan_elapsed = measure(lambda: scan_items(data))
    rows = [
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
    ]
    try:
        rust, rust_peak, rust_elapsed = rust_streaming_fold(path)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as exc:
        rows.append(
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
                note=f"Rust streaming sink unavailable: {exc}",
            )
        )
    else:
        rows.append(
            row_from_result(
                path,
                len(data),
                "streaming-fold",
                rust,
                rust_peak,
                rust_elapsed,
                "Rust read_to_sink_from_reader; peak_kib is VmHWM when available",
            )
        )
    try:
        ts, ts_peak, ts_elapsed = typescript_streaming_fold(path)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as exc:
        rows.append(
            BenchmarkRow(
                path=str(path),
                bytes=len(data),
                mode="typescript-streaming-fold",
                items=None,
                frames=None,
                terms=None,
                quads=None,
                blobs=None,
                peak_kib=None,
                elapsed_ms=None,
                note=f"TypeScript sink-only fold unavailable: {exc}",
            )
        )
    else:
        rows.append(
            row_from_result(
                path,
                len(data),
                "typescript-streaming-fold",
                ts,
                ts_peak,
                ts_elapsed,
                "TypeScript foldStreamToSink; peak_kib is Node RSS",
            )
        )
    return rows


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
            "baseline and the Rust streaming sink API."
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

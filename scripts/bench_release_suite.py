#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Run the release benchmark suite and emit JSON plus Markdown reports."""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import os
import platform
import re
import shlex
import shutil
import statistics
import subprocess
import sys
import time
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENGINES = "rust,python,go,ts"
DEFAULT_VECTORS = (
    "vectors/01-minimal.gts,"
    "vectors/23-files-profile-tree.gts,"
    "vectors/25b-streamable-compacted.gts"
)
DEFAULT_STREAM_VECTOR = "vectors/25b-streamable-compacted.gts"
TRUNCATE_STDERR = 2000


@dataclass(frozen=True)
class ProcessResult:
    command: list[str]
    cwd: str
    returncode: int
    elapsed_ms: float
    stdout: bytes
    stderr: bytes
    timed_out: bool = False


@dataclass(frozen=True)
class SetupResult:
    engine: str
    status: str
    command: str
    elapsed_ms: float | None
    note: str


@dataclass(frozen=True)
class EngineRuntime:
    name: str
    command: list[str]
    cwd: Path
    supports_from_nq: bool


@dataclass(frozen=True)
class InputRow:
    kind: str
    name: str
    path: str
    bytes: int
    sha256: str
    files: int | None = None


@dataclass(frozen=True)
class OutputRow:
    kind: str
    bytes: int | None
    sha256: str | None
    files: int | None = None


@dataclass(frozen=True)
class BenchmarkRow:
    engine: str
    operation: str
    input: str
    input_bytes: int | None
    status: str
    iterations: int
    median_ms: float | None
    min_ms: float | None
    max_ms: float | None
    command: str
    output: OutputRow | None
    stdout_bytes: int | None
    stdout_sha256: str | None
    note: str


@dataclass(frozen=True)
class MemoryRow:
    engine: str
    mode: str
    input: str
    input_bytes: int | None
    status: str
    elapsed_ms: float | None
    peak_kib: float | None
    command: str
    note: str


def split_csv(value: str) -> list[str]:
    parts = re.split(r"[\s,]+", value.strip())
    return [part for part in parts if part]


def shlex_join(command: Iterable[str]) -> str:
    return shlex.join(str(part) for part in command)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_digest(root: Path) -> tuple[int, int, str]:
    digest = hashlib.sha256()
    files = 0
    total_bytes = 0
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        rel = path.relative_to(root).as_posix()
        data = path.read_bytes()
        files += 1
        total_bytes += len(data)
        digest.update(rel.encode("utf-8"))
        digest.update(b"\0")
        digest.update(hashlib.sha256(data).digest())
        digest.update(b"\0")
    return files, total_bytes, digest.hexdigest()


def run_process(
    command: list[str],
    *,
    cwd: Path = ROOT,
    env: dict[str, str] | None = None,
    timeout: int,
) -> ProcessResult:
    started = time.perf_counter()
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        elapsed_ms = (time.perf_counter() - started) * 1000.0
        return ProcessResult(
            command=command,
            cwd=str(cwd),
            returncode=completed.returncode,
            elapsed_ms=round(elapsed_ms, 2),
            stdout=completed.stdout,
            stderr=completed.stderr,
        )
    except subprocess.TimeoutExpired as exc:
        elapsed_ms = (time.perf_counter() - started) * 1000.0
        return ProcessResult(
            command=command,
            cwd=str(cwd),
            returncode=124,
            elapsed_ms=round(elapsed_ms, 2),
            stdout=exc.stdout or b"",
            stderr=exc.stderr or b"",
            timed_out=True,
        )


def stderr_note(result: ProcessResult) -> str:
    stderr = result.stderr.decode("utf-8", errors="replace").strip()
    stdout = result.stdout.decode("utf-8", errors="replace").strip()
    detail = stderr or stdout
    if len(detail) > TRUNCATE_STDERR:
        detail = detail[-TRUNCATE_STDERR:]
    if result.timed_out:
        return f"timed out after {result.elapsed_ms:.2f} ms; {detail}".strip()
    return detail


def version_from(command: list[str], timeout: int = 20) -> str:
    binary = shutil.which(command[0])
    if binary is None:
        return "not found"
    result = run_process([binary, *command[1:]], timeout=timeout)
    text = (result.stdout or result.stderr).decode("utf-8", errors="replace").strip()
    if result.returncode != 0:
        return f"error {result.returncode}: {text}"
    return text.splitlines()[0] if text else "ok"


def git_value(args: list[str]) -> str:
    result = run_process(["git", *args], timeout=20)
    if result.returncode != 0:
        return f"unavailable: {stderr_note(result)}"
    return result.stdout.decode("utf-8", errors="replace").strip()


def linux_memory_kib() -> int | None:
    meminfo = Path("/proc/meminfo")
    if not meminfo.exists():
        return None
    for line in meminfo.read_text(encoding="utf-8").splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1])
    return None


def command_environment() -> dict[str, Any]:
    manifest = ROOT / "vectors" / "manifest.json"
    return {
        "generated_at": datetime.now(UTC).isoformat(),
        "command_line": shlex_join(sys.argv),
        "repo_commit": git_value(["rev-parse", "HEAD"]),
        "spec_commit": git_value(["log", "-n1", "--format=%H", "--", "docs/GTS-SPEC.md"]),
        "spec_blob": git_value(["rev-parse", "HEAD:docs/GTS-SPEC.md"]),
        "corpus_commit": git_value(["log", "-n1", "--format=%H", "--", "vectors"]),
        "corpus_manifest_sha256": sha256_file(manifest) if manifest.exists() else None,
        "platform": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "python": sys.version.split()[0],
        "cpu_count": os.cpu_count(),
        "memory_kib": linux_memory_kib(),
        "tool_versions": {
            "git": version_from(["git", "--version"]),
            "rustc": version_from(["rustc", "--version"]),
            "cargo": version_from(["cargo", "--version"]),
            "go": version_from(["go", "version"]),
            "node": version_from(["node", "--version"]),
            "npm": version_from(["npm", "--version"]),
            "uv": version_from(["uv", "--version"]),
        },
    }


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def prepare_fixtures(out_dir: Path) -> dict[str, Path]:
    fixture_root = out_dir / "fixtures"
    if fixture_root.exists():
        shutil.rmtree(fixture_root)
    fixture_root.mkdir(parents=True)

    nq = fixture_root / "write-bench.nq"
    lines = []
    for idx in range(256):
        subject = f"<https://example.invalid/gts/bench/s{idx:04d}>"
        predicate = f"<https://example.invalid/gts/bench/p{idx % 8}>"
        value = f'"release benchmark value {idx:04d}"'
        lines.append(f"{subject} {predicate} {value} .")
    write_text(nq, "\n".join(lines) + "\n")

    pack_dir = fixture_root / "pack-tree"
    write_text(pack_dir / "a.txt", "hello gts\n" * 64)
    write_text(pack_dir / "sub" / "b.txt", "second file\n" * 128)
    write_text(pack_dir / "sub" / "dedup.txt", "shared content\n" * 96)
    write_text(pack_dir / "dedup-root.txt", "shared content\n" * 96)
    (pack_dir / "binary.bin").write_bytes(bytes(range(256)) * 8)
    for path in pack_dir.rglob("*"):
        if path.is_file():
            os.chmod(path, 0o644)
    return {"nq": nq, "pack_dir": pack_dir}


def input_rows(vectors: list[Path], fixtures: dict[str, Path]) -> list[InputRow]:
    rows: list[InputRow] = []
    for path in vectors:
        rows.append(
            InputRow(
                kind="conformance-vector",
                name=path.name,
                path=str(path.relative_to(ROOT) if path.is_relative_to(ROOT) else path),
                bytes=path.stat().st_size,
                sha256=sha256_file(path),
            )
        )
    nq = fixtures["nq"]
    rows.append(
        InputRow(
            kind="write-fixture",
            name=nq.name,
            path=str(nq),
            bytes=nq.stat().st_size,
            sha256=sha256_file(nq),
        )
    )
    pack_dir = fixtures["pack_dir"]
    files, total_bytes, digest = tree_digest(pack_dir)
    rows.append(
        InputRow(
            kind="pack-fixture",
            name=pack_dir.name,
            path=str(pack_dir),
            bytes=total_bytes,
            sha256=digest,
            files=files,
        )
    )
    return rows


def setup_engine(engine: str, out_dir: Path, timeout: int) -> tuple[EngineRuntime | None, list[SetupResult]]:
    results: list[SetupResult] = []

    def run_setup(command: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None) -> bool:
        result = run_process(command, cwd=cwd, env=env, timeout=timeout)
        status = "ok" if result.returncode == 0 else "failed"
        results.append(
            SetupResult(
                engine=engine,
                status=status,
                command=shlex_join(command),
                elapsed_ms=result.elapsed_ms,
                note="" if status == "ok" else stderr_note(result),
            )
        )
        return status == "ok"

    if engine == "python":
        if shutil.which("uv") is None:
            results.append(SetupResult(engine, "failed", "uv", None, "uv not found"))
            return None, results
        if not run_setup(["uv", "sync", "--quiet"], cwd=ROOT / "python"):
            return None, results
        return EngineRuntime(engine, ["uv", "run", "--quiet", "gts"], ROOT / "python", True), results

    if engine == "rust":
        if shutil.which("cargo") is None:
            results.append(SetupResult(engine, "failed", "cargo", None, "cargo not found"))
            return None, results
        if not run_setup(
            ["cargo", "build", "--quiet", "--manifest-path", str(ROOT / "rust" / "Cargo.toml")]
        ):
            return None, results
        suffix = ".exe" if sys.platform == "win32" else ""
        return (
            EngineRuntime(
                engine,
                [str(ROOT / "rust" / "target" / "debug" / f"gts{suffix}")],
                ROOT,
                True,
            ),
            results,
        )

    if engine == "go":
        if shutil.which("go") is None:
            results.append(SetupResult(engine, "failed", "go", None, "go not found"))
            return None, results
        bin_dir = out_dir / "bin"
        bin_dir.mkdir(parents=True, exist_ok=True)
        suffix = ".exe" if sys.platform == "win32" else ""
        binary = bin_dir / f"gts-go{suffix}"
        env = os.environ.copy()
        env["CGO_ENABLED"] = "0"
        if not run_setup(["go", "build", "-o", str(binary), "./cmd/gts"], cwd=ROOT / "go", env=env):
            return None, results
        return EngineRuntime(engine, [str(binary)], ROOT, False), results

    if engine == "ts":
        if shutil.which("npm") is None or shutil.which("node") is None:
            results.append(
                SetupResult(engine, "failed", "node/npm", None, "node or npm not found")
            )
            return None, results
        if not run_setup(["npm", "ci", "--silent"], cwd=ROOT / "ts"):
            return None, results
        if not run_setup(["npm", "run", "build", "--silent"], cwd=ROOT / "ts"):
            return None, results
        return EngineRuntime(engine, ["node", str(ROOT / "ts" / "dist" / "bin" / "gts.js")], ROOT, False), results

    results.append(SetupResult(engine, "failed", engine, None, "unknown engine"))
    return None, results


def output_file(path: Path) -> OutputRow:
    if not path.exists():
        return OutputRow("file", None, None)
    return OutputRow("file", path.stat().st_size, sha256_file(path))


def output_tree(path: Path) -> OutputRow:
    if not path.exists():
        return OutputRow("tree", None, None, None)
    files, total_bytes, digest = tree_digest(path)
    return OutputRow("tree", total_bytes, digest, files)


CommandFactory = Callable[[int], tuple[list[str], Path, dict[str, str] | None]]


def measure_command(
    *,
    engine: str,
    operation: str,
    input_name: str,
    input_bytes: int | None,
    iterations: int,
    command_factory: CommandFactory,
    output_factory: Callable[[], OutputRow | None],
    timeout: int,
) -> BenchmarkRow:
    elapsed: list[float] = []
    command_text = ""
    stdout_bytes: int | None = None
    stdout_sha256: str | None = None
    note = ""
    status = "ok"

    for idx in range(iterations):
        command, cwd, env = command_factory(idx)
        if not command_text:
            command_text = f"(cd {shlex.quote(str(cwd))} && {shlex_join(command)})"
        result = run_process(command, cwd=cwd, env=env, timeout=timeout)
        if result.returncode != 0:
            status = "failed"
            note = f"iteration {idx + 1} exited {result.returncode}: {stderr_note(result)}"
            break
        elapsed.append(result.elapsed_ms)
        stdout_bytes = len(result.stdout)
        stdout_sha256 = sha256_bytes(result.stdout) if result.stdout else None

    output = output_factory() if status == "ok" else None
    return BenchmarkRow(
        engine=engine,
        operation=operation,
        input=input_name,
        input_bytes=input_bytes,
        status=status,
        iterations=len(elapsed),
        median_ms=round(statistics.median(elapsed), 2) if elapsed else None,
        min_ms=round(min(elapsed), 2) if elapsed else None,
        max_ms=round(max(elapsed), 2) if elapsed else None,
        command=command_text,
        output=output,
        stdout_bytes=stdout_bytes,
        stdout_sha256=stdout_sha256,
        note=note,
    )


def skipped_row(engine: str, operation: str, input_name: str, note: str) -> BenchmarkRow:
    return BenchmarkRow(
        engine=engine,
        operation=operation,
        input=input_name,
        input_bytes=None,
        status="skipped",
        iterations=0,
        median_ms=None,
        min_ms=None,
        max_ms=None,
        command="n/a",
        output=None,
        stdout_bytes=None,
        stdout_sha256=None,
        note=note,
    )


def run_cli_benchmarks(
    runtimes: dict[str, EngineRuntime],
    engines: list[str],
    vectors: list[Path],
    fixtures: dict[str, Path],
    out_dir: Path,
    iterations: int,
    timeout: int,
) -> list[BenchmarkRow]:
    rows: list[BenchmarkRow] = []
    products = out_dir / "products"
    products.mkdir(parents=True, exist_ok=True)

    for engine in engines:
        runtime = runtimes.get(engine)
        if runtime is None:
            rows.append(skipped_row(engine, "all", "all", "engine setup failed"))
            continue

        for vector in vectors:
            vector_bytes = vector.stat().st_size
            vector_name = vector.name
            rows.append(
                measure_command(
                    engine=engine,
                    operation="read-info",
                    input_name=vector_name,
                    input_bytes=vector_bytes,
                    iterations=iterations,
                    command_factory=lambda _idx, rt=runtime, path=vector: (
                        [*rt.command, "info", str(path)],
                        rt.cwd,
                        None,
                    ),
                    output_factory=lambda: None,
                    timeout=timeout,
                )
            )
            rows.append(
                measure_command(
                    engine=engine,
                    operation="fold",
                    input_name=vector_name,
                    input_bytes=vector_bytes,
                    iterations=iterations,
                    command_factory=lambda _idx, rt=runtime, path=vector: (
                        [*rt.command, "fold", str(path)],
                        rt.cwd,
                        None,
                    ),
                    output_factory=lambda: None,
                    timeout=timeout,
                )
            )

        if runtime.supports_from_nq:
            nq = fixtures["nq"]
            out_file = products / f"{engine}-from-nq.gts"

            def write_command(
                idx: int, rt: EngineRuntime = runtime, source: Path = nq, base: Path = out_file
            ) -> tuple[list[str], Path, dict[str, str] | None]:
                target = base.with_name(f"{base.stem}-{idx}{base.suffix}")
                if target.exists():
                    target.unlink()
                return [*rt.command, "from-nq", str(source), "-o", str(target)], rt.cwd, None

            rows.append(
                measure_command(
                    engine=engine,
                    operation="write-from-nq",
                    input_name=nq.name,
                    input_bytes=nq.stat().st_size,
                    iterations=iterations,
                    command_factory=write_command,
                    output_factory=lambda path=out_file.with_name(
                        f"{out_file.stem}-{iterations - 1}{out_file.suffix}"
                    ): output_file(path),
                    timeout=timeout,
                )
            )
        else:
            rows.append(
                skipped_row(
                    engine,
                    "write-from-nq",
                    fixtures["nq"].name,
                    "current CLI parity exposes from-nq only for Python and Rust",
                )
            )

        pack_dir = fixtures["pack_dir"]
        archive = products / f"{engine}-pack.gts"

        def pack_command(
            idx: int, rt: EngineRuntime = runtime, source: Path = pack_dir, base: Path = archive
        ) -> tuple[list[str], Path, dict[str, str] | None]:
            target = base.with_name(f"{base.stem}-{idx}{base.suffix}")
            if target.exists():
                target.unlink()
            return [*rt.command, "pack", str(source), "-o", str(target)], rt.cwd, None

        rows.append(
            measure_command(
                engine=engine,
                operation="pack",
                input_name=pack_dir.name,
                input_bytes=tree_digest(pack_dir)[1],
                iterations=iterations,
                command_factory=pack_command,
                output_factory=lambda path=archive.with_name(
                    f"{archive.stem}-{iterations - 1}{archive.suffix}"
                ): output_file(path),
                timeout=timeout,
            )
        )

        archive_for_unpack = archive.with_name(f"{archive.stem}-{iterations - 1}{archive.suffix}")
        if archive_for_unpack.exists():

            def unpack_command(
                idx: int,
                rt: EngineRuntime = runtime,
                source: Path = archive_for_unpack,
                product_root: Path = products,
                engine_name: str = engine,
            ) -> tuple[list[str], Path, dict[str, str] | None]:
                target = product_root / f"{engine_name}-unpack-{idx}"
                if target.exists():
                    shutil.rmtree(target)
                target.mkdir(parents=True)
                return [*rt.command, "unpack", str(source), "-C", str(target)], rt.cwd, None

            rows.append(
                measure_command(
                    engine=engine,
                    operation="unpack",
                    input_name=archive_for_unpack.name,
                    input_bytes=archive_for_unpack.stat().st_size,
                    iterations=iterations,
                    command_factory=unpack_command,
                    output_factory=lambda path=products
                    / f"{engine}-unpack-{iterations - 1}": output_tree(path),
                    timeout=timeout,
                )
            )
        else:
            rows.append(skipped_row(engine, "unpack", archive.name, "pack output missing"))

    return rows


def run_memory_helper(
    *,
    engines: list[str],
    stream_vector: Path,
    timeout: int,
) -> list[MemoryRow]:
    rows: list[MemoryRow] = []
    if "python" not in engines and "rust" not in engines:
        return rows

    command = [
        "uv",
        "run",
        "--quiet",
        "python",
        str(ROOT / "scripts" / "bench_reader_memory.py"),
        "--json",
        str(stream_vector),
    ]
    if shutil.which("uv") is None:
        rows.append(
            MemoryRow(
                engine="python/rust",
                mode="streaming-memory",
                input=stream_vector.name,
                input_bytes=stream_vector.stat().st_size,
                status="skipped",
                elapsed_ms=None,
                peak_kib=None,
                command=shlex_join(command),
                note="uv not found",
            )
        )
        return rows

    result = run_process(command, cwd=ROOT / "python", timeout=timeout)
    command_text = f"(cd {shlex.quote(str(ROOT / 'python'))} && {shlex_join(command)})"
    if result.returncode != 0:
        rows.append(
            MemoryRow(
                engine="python/rust",
                mode="streaming-memory",
                input=stream_vector.name,
                input_bytes=stream_vector.stat().st_size,
                status="failed",
                elapsed_ms=result.elapsed_ms,
                peak_kib=None,
                command=command_text,
                note=stderr_note(result),
            )
        )
        return rows

    try:
        payload = json.loads(result.stdout.decode("utf-8"))
    except json.JSONDecodeError as exc:
        rows.append(
            MemoryRow(
                engine="python/rust",
                mode="streaming-memory",
                input=stream_vector.name,
                input_bytes=stream_vector.stat().st_size,
                status="failed",
                elapsed_ms=result.elapsed_ms,
                peak_kib=None,
                command=command_text,
                note=f"invalid JSON from bench_reader_memory.py: {exc}",
            )
        )
        return rows

    mode_engine = {
        "full-reader": "python",
        "frame-scan": "cbor-frame-scan",
        "streaming-fold": "rust",
    }
    for item in payload:
        mode = str(item.get("mode", "unknown"))
        engine = mode_engine.get(mode, mode)
        if engine not in engines and engine != "cbor-frame-scan":
            continue
        status = "ok" if item.get("elapsed_ms") is not None else "skipped"
        rows.append(
            MemoryRow(
                engine=engine,
                mode=mode,
                input=Path(str(item.get("path", stream_vector))).name,
                input_bytes=int(item.get("bytes") or stream_vector.stat().st_size),
                status=status,
                elapsed_ms=float(item["elapsed_ms"]) if item.get("elapsed_ms") is not None else None,
                peak_kib=float(item["peak_kib"]) if item.get("peak_kib") is not None else None,
                command=command_text,
                note=str(item.get("note", "")),
            )
        )
    return rows


def parse_go_benchmarks(output: str) -> list[dict[str, str | float]]:
    parsed: list[dict[str, str | float]] = []
    for line in output.splitlines():
        if not line.startswith("Benchmark"):
            continue
        parts = line.split()
        if len(parts) < 4:
            continue
        row: dict[str, str | float] = {"name": parts[0], "iterations": parts[1]}
        for idx, part in enumerate(parts):
            if part == "ns/op" and idx > 0:
                row["ns_per_op"] = float(parts[idx - 1])
            if part == "B/op" and idx > 0:
                row["bytes_per_op"] = float(parts[idx - 1])
            if part == "allocs/op" and idx > 0:
                row["allocs_per_op"] = float(parts[idx - 1])
        parsed.append(row)
    return parsed


def run_go_memory_benchmarks(timeout: int) -> list[MemoryRow]:
    command = [
        "go",
        "test",
        "./reader",
        "-run",
        "^$",
        "-bench",
        "Benchmark(ReadFull|ReadToSink)CorpusVector",
        "-benchmem",
        "-count",
        "1",
    ]
    env = os.environ.copy()
    env["CGO_ENABLED"] = "0"
    result = run_process(command, cwd=ROOT / "go", env=env, timeout=timeout)
    command_text = f"(cd {shlex.quote(str(ROOT / 'go'))} && CGO_ENABLED=0 {shlex_join(command)})"
    if result.returncode != 0:
        return [
            MemoryRow(
                engine="go",
                mode="streaming-memory",
                input=DEFAULT_STREAM_VECTOR,
                input_bytes=None,
                status="failed",
                elapsed_ms=result.elapsed_ms,
                peak_kib=None,
                command=command_text,
                note=stderr_note(result),
            )
        ]
    rows: list[MemoryRow] = []
    output = result.stdout.decode("utf-8", errors="replace")
    for parsed in parse_go_benchmarks(output):
        mode = str(parsed["name"])
        details = []
        if "ns_per_op" in parsed:
            details.append(f"{parsed['ns_per_op']} ns/op")
        if "bytes_per_op" in parsed:
            details.append(f"{parsed['bytes_per_op']} B/op")
        if "allocs_per_op" in parsed:
            details.append(f"{parsed['allocs_per_op']} allocs/op")
        rows.append(
            MemoryRow(
                engine="go",
                mode=mode,
                input=Path(DEFAULT_STREAM_VECTOR).name,
                input_bytes=(ROOT / DEFAULT_STREAM_VECTOR).stat().st_size,
                status="ok",
                elapsed_ms=result.elapsed_ms,
                peak_kib=None,
                command=command_text,
                note="; ".join(details),
            )
        )
    return rows


def run_memory_benchmarks(
    *,
    engines: list[str],
    stream_vector: Path,
    timeout: int,
    include_streaming: bool,
) -> list[MemoryRow]:
    if not include_streaming:
        return []
    rows = run_memory_helper(engines=engines, stream_vector=stream_vector, timeout=timeout)
    if "go" in engines:
        rows.extend(run_go_memory_benchmarks(timeout=timeout))
    if "ts" in engines:
        rows.append(
            MemoryRow(
                engine="ts",
                mode="browser-foldStream-memory",
                input=stream_vector.name,
                input_bytes=stream_vector.stat().st_size,
                status="skipped",
                elapsed_ms=None,
                peak_kib=None,
                command="manual browser harness required",
                note=(
                    "TypeScript memory evidence is gathered in a browser runtime; "
                    "record the browser harness output in the release report."
                ),
            )
        )
    return rows


def table(headers: list[str], rows: list[list[Any]]) -> str:
    rendered = ["| " + " | ".join(headers) + " |", "|" + "|".join("---" for _ in headers) + "|"]
    for row in rows:
        rendered.append("| " + " | ".join(cell(value) for value in row) + " |")
    return "\n".join(rendered)


def cell(value: Any) -> str:
    if value is None:
        return "n/a"
    text = str(value)
    return text.replace("|", "\\|").replace("\n", "<br>")


def ms(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.2f}"


def render_markdown(payload: dict[str, Any]) -> str:
    metadata = payload["metadata"]
    inputs = [InputRow(**row) for row in payload["inputs"]]
    setup = [SetupResult(**row) for row in payload["setup"]]
    benchmarks = [
        BenchmarkRow(
            **{
                **row,
                "output": OutputRow(**row["output"]) if row.get("output") else None,
            }
        )
        for row in payload["benchmarks"]
    ]
    memory = [MemoryRow(**row) for row in payload["streaming_memory"]]

    lines = [
        "# GTS Release Benchmark Report",
        "",
        "Generated by `scripts/bench_release_suite.py`.",
        "",
        "## Run Metadata",
        "",
        table(
            ["field", "value"],
            [
                ["generated_at", metadata["generated_at"]],
                ["command_line", metadata["command_line"]],
                ["repo_commit", metadata["repo_commit"]],
                ["spec_commit", metadata["spec_commit"]],
                ["spec_blob", metadata["spec_blob"]],
                ["corpus_commit", metadata["corpus_commit"]],
                ["corpus_manifest_sha256", metadata["corpus_manifest_sha256"]],
            ],
        ),
        "",
        "## Platform",
        "",
        table(
            ["field", "value"],
            [
                ["platform", metadata["platform"]],
                ["machine", metadata["machine"]],
                ["processor", metadata["processor"]],
                ["cpu_count", metadata["cpu_count"]],
                ["memory_kib", metadata["memory_kib"]],
                ["python", metadata["python"]],
            ],
        ),
        "",
        "## Tool Versions",
        "",
        table(
            ["tool", "version"],
            [[name, value] for name, value in metadata["tool_versions"].items()],
        ),
        "",
        "## Inputs",
        "",
        table(
            ["kind", "name", "bytes", "files", "sha256", "path"],
            [[row.kind, row.name, row.bytes, row.files, row.sha256, row.path] for row in inputs],
        ),
        "",
        "## Engine Setup",
        "",
        table(
            ["engine", "status", "elapsed_ms", "command", "note"],
            [[row.engine, row.status, ms(row.elapsed_ms), row.command, row.note] for row in setup],
        ),
        "",
        "## CLI Benchmarks",
        "",
        table(
            [
                "engine",
                "operation",
                "input",
                "input_bytes",
                "status",
                "iterations",
                "median_ms",
                "min_ms",
                "max_ms",
                "output",
                "note",
            ],
            [
                [
                    row.engine,
                    row.operation,
                    row.input,
                    row.input_bytes,
                    row.status,
                    row.iterations,
                    ms(row.median_ms),
                    ms(row.min_ms),
                    ms(row.max_ms),
                    output_cell(row.output),
                    row.note,
                ]
                for row in benchmarks
            ],
        ),
        "",
        "## Streaming Memory",
        "",
        table(
            ["engine", "mode", "input", "status", "elapsed_ms", "peak_kib", "note"],
            [
                [
                    row.engine,
                    row.mode,
                    row.input,
                    row.status,
                    ms(row.elapsed_ms),
                    row.peak_kib,
                    row.note,
                ]
                for row in memory
            ],
        ),
        "",
        "## Command Lines",
        "",
        table(
            ["engine", "operation", "input", "command"],
            sorted(
                {
                    (row.engine, row.operation, row.input, row.command)
                    for row in benchmarks
                    if row.command != "n/a"
                }
            ),
        ),
        "",
        "## Release-Note Excerpt Template",
        "",
        (
            "Benchmarks were run on `<platform summary>` at `<repo_commit>` "
            "against spec commit `<spec_commit>` and corpus commit `<corpus_commit>`. "
            "Use the median values above for cited read/fold/write/pack/unpack numbers, "
            "and cite the streaming-memory rows separately because they measure RSS or "
            "runtime allocation behavior rather than CLI wall time."
        ),
        "",
    ]
    return "\n".join(lines)


def output_cell(output: OutputRow | None) -> str:
    if output is None:
        return "n/a"
    bits = [output.kind]
    if output.bytes is not None:
        bits.append(f"{output.bytes} bytes")
    if output.files is not None:
        bits.append(f"{output.files} files")
    if output.sha256:
        bits.append(output.sha256)
    return "; ".join(bits)


def validate_paths(paths: list[str], *, kind: str) -> list[Path]:
    resolved: list[Path] = []
    for raw in paths:
        path = (ROOT / raw).resolve() if not Path(raw).is_absolute() else Path(raw)
        if not path.exists():
            raise SystemExit(f"{kind} does not exist: {raw}")
        resolved.append(path)
    return resolved


def has_failures(payload: dict[str, Any]) -> bool:
    setup_failed = any(row["status"] == "failed" for row in payload["setup"])
    benchmark_failed = any(row["status"] == "failed" for row in payload["benchmarks"])
    memory_failed = any(row["status"] == "failed" for row in payload["streaming_memory"])
    return setup_failed or benchmark_failed or memory_failed


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run release-quality GTS benchmarks across read, fold, write, "
            "pack, unpack, and streaming-memory paths."
        )
    )
    parser.add_argument(
        "--engines",
        default=DEFAULT_ENGINES,
        help="comma or whitespace separated engines: rust, python, go, ts",
    )
    parser.add_argument(
        "--vectors",
        default=DEFAULT_VECTORS,
        help="comma or whitespace separated .gts corpus files for read/fold benchmarks",
    )
    parser.add_argument(
        "--stream-vector",
        default=DEFAULT_STREAM_VECTOR,
        help="GTS file used by streaming-memory helpers",
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=3,
        help="iterations per CLI benchmark command",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("dist/benchmarks/release-suite"),
        help="directory for generated JSON, Markdown, fixtures, and products",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=180,
        help="per-command timeout in seconds",
    )
    parser.add_argument(
        "--skip-streaming-memory",
        action="store_true",
        help="skip bench_reader_memory.py and Go benchmem commands",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit 1 when a selected setup step or benchmark command fails",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.iterations < 1:
        raise SystemExit("--iterations must be >= 1")
    engines = split_csv(args.engines)
    unknown = sorted(set(engines) - {"rust", "python", "go", "ts"})
    if unknown:
        raise SystemExit(f"unknown engine(s): {', '.join(unknown)}")

    out_dir = args.out_dir if args.out_dir.is_absolute() else ROOT / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    fixtures = prepare_fixtures(out_dir)
    vectors = validate_paths(split_csv(args.vectors), kind="vector")
    stream_vector = validate_paths([args.stream_vector], kind="stream vector")[0]

    setup: list[SetupResult] = []
    runtimes: dict[str, EngineRuntime] = {}
    for engine in engines:
        runtime, engine_setup = setup_engine(engine, out_dir, args.timeout)
        setup.extend(engine_setup)
        if runtime is not None:
            runtimes[engine] = runtime

    metadata = command_environment()
    rows = run_cli_benchmarks(
        runtimes=runtimes,
        engines=engines,
        vectors=vectors,
        fixtures=fixtures,
        out_dir=out_dir,
        iterations=args.iterations,
        timeout=args.timeout,
    )
    memory_rows = run_memory_benchmarks(
        engines=engines,
        stream_vector=stream_vector,
        timeout=args.timeout,
        include_streaming=not args.skip_streaming_memory,
    )

    payload = {
        "metadata": metadata,
        "inputs": [dataclasses.asdict(row) for row in input_rows(vectors, fixtures)],
        "setup": [dataclasses.asdict(row) for row in setup],
        "benchmarks": [dataclasses.asdict(row) for row in rows],
        "streaming_memory": [dataclasses.asdict(row) for row in memory_rows],
    }

    json_path = out_dir / "release-benchmark-report.json"
    md_path = out_dir / "release-benchmark-report.md"
    json_path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    md_path.write_text(render_markdown(payload), encoding="utf-8")
    print(f"wrote {json_path}")
    print(f"wrote {md_path}")
    if args.strict and has_failures(payload):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

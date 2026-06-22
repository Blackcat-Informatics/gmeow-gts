#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Verify a published GTS release family from public release surfaces."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Iterable


DEFAULT_REPO = "Blackcat-Informatics/gmeow-gts"
DEFAULT_REPO_URL = f"https://github.com/{DEFAULT_REPO}"
DEFAULT_PYPI_PROJECT = "gmeow-gts"
DEFAULT_NPM_PACKAGE = "@blackcatinformatics/gmeow-gts"
DEFAULT_RUST_CRATE = "gmeow-gts"
DEFAULT_VISUAL_HASHING_CRATE = "visual-hashing"
SPDX_PREDICATE = "https://spdx.dev/Document/v2.3"
USER_AGENT = "gmeow-gts-release-verifier/1.0"


@dataclass
class CheckResult:
    surface: str
    check: str
    status: str
    detail: str


class Recorder:
    def __init__(self) -> None:
        self.results: list[CheckResult] = []

    def pass_(self, surface: str, check: str, detail: str = "ok") -> None:
        self._add(surface, check, "PASS", detail)

    def warn(self, surface: str, check: str, detail: str) -> None:
        self._add(surface, check, "WARN", detail)

    def fail(self, surface: str, check: str, detail: str) -> None:
        self._add(surface, check, "FAIL", detail)

    def _add(self, surface: str, check: str, status: str, detail: str) -> None:
        self.results.append(CheckResult(surface, check, status, one_line(detail)))
        print(f"{status}: {surface}: {check}: {one_line(detail)}", flush=True)

    def has_failures(self) -> bool:
        return any(result.status == "FAIL" for result in self.results)


def one_line(value: str, limit: int = 500) -> str:
    text = " ".join(str(value).split())
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def request(url: str, *, headers: dict[str, str] | None = None) -> urllib.request.Request:
    merged = {"User-Agent": USER_AGENT}
    if headers:
        merged.update(headers)
    return urllib.request.Request(url, headers=merged)


def fetch_json(url: str, *, headers: dict[str, str] | None = None, attempts: int = 3) -> Any:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(request(url, headers=headers), timeout=30) as response:
                return json.load(response)
        except (OSError, urllib.error.URLError, json.JSONDecodeError) as exc:
            last_error = exc
            if attempt < attempts:
                time.sleep(attempt)
    raise RuntimeError(f"failed to fetch JSON from {url}: {last_error}") from last_error


def download(url: str, destination: Path, *, headers: dict[str, str] | None = None) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    last_error: Exception | None = None
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(request(url, headers=headers), timeout=60) as response:
                with destination.open("wb") as handle:
                    shutil.copyfileobj(response, handle)
            return
        except (OSError, urllib.error.URLError) as exc:
            last_error = exc
            if destination.exists():
                try:
                    destination.unlink()
                except OSError:
                    # Best-effort cleanup; the original download error is reported below.
                    pass
            if attempt < 3:
                time.sleep(attempt)
    raise RuntimeError(f"failed to download {url}: {last_error}") from last_error


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sri_matches(path: Path, integrity: str) -> bool:
    algorithm, encoded = integrity.split("-", 1)
    digest = hashlib.new(algorithm)
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return base64.b64encode(digest.digest()).decode("ascii") == encoded


def require_tool(name: str, recorder: Recorder, surface: str) -> str | None:
    path = shutil.which(name)
    if path:
        recorder.pass_(surface, f"{name} available", path)
        return path
    recorder.fail(surface, f"{name} available", f"{name} not found in PATH")
    return None


def run_command(
    recorder: Recorder,
    surface: str,
    check: str,
    command: list[str],
    *,
    cwd: Path | None = None,
    allow_legacy_gap: bool = False,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    pretty = " ".join(command)
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if result.returncode == 0:
        recorder.pass_(surface, check, result.stdout.strip() or pretty)
    elif allow_legacy_gap:
        recorder.warn(surface, check, f"legacy gap allowed: {result.stdout.strip() or pretty}")
    else:
        recorder.fail(surface, check, f"exit {result.returncode}: {result.stdout.strip() or pretty}")
    return result


def run_json_command(
    recorder: Recorder,
    surface: str,
    check: str,
    command: list[str],
    *,
    cwd: Path | None = None,
) -> Any | None:
    result = run_command(recorder, surface, check, command, cwd=cwd)
    if result.returncode != 0:
        return None
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        recorder.fail(surface, f"{check} JSON parse", str(exc))
        return None


def pypi_attestations_command() -> list[str] | None:
    if shutil.which("pypi-attestations"):
        return ["pypi-attestations"]
    if shutil.which("uvx"):
        return ["uvx", "--from", "pypi-attestations", "pypi-attestations"]
    return None


def verify_pypi(args: argparse.Namespace, recorder: Recorder, out_dir: Path) -> list[Path]:
    surface = "PyPI"
    pypi_dir = out_dir / "python"
    artifacts: list[Path] = []
    pypi_json_url = f"https://pypi.org/pypi/{args.pypi_project}/{args.version}/json"
    try:
        metadata = fetch_json(pypi_json_url)
        if not isinstance(metadata, dict):
            raise RuntimeError("metadata is not a dictionary")
        recorder.pass_(surface, "release metadata", pypi_json_url)
    except RuntimeError as exc:
        recorder.fail(surface, "release metadata", str(exc))
        return artifacts

    urls = metadata.get("urls") or []
    if not isinstance(urls, list):
        recorder.fail(surface, "release file metadata", "urls is not a list")
        return artifacts
    wanted_types = {"bdist_wheel", "sdist"}
    found_types = {entry.get("packagetype") for entry in urls if isinstance(entry, dict)}
    missing_types = wanted_types - found_types
    if missing_types:
        recorder.fail(surface, "wheel and sdist present", f"missing {', '.join(sorted(missing_types))}")
    else:
        recorder.pass_(surface, "wheel and sdist present", ", ".join(sorted(found_types & wanted_types)))

    attest = pypi_attestations_command()
    if not attest:
        recorder.fail(surface, "pypi-attestations available", "install pypi-attestations or uvx")

    for entry in urls:
        if not isinstance(entry, dict):
            recorder.fail(surface, "release file metadata", "file entry is not a dictionary")
            continue
        filename = entry.get("filename")
        file_url = entry.get("url")
        if not isinstance(filename, str) or not isinstance(file_url, str):
            recorder.fail(surface, "release file metadata", "filename or URL missing")
            continue
        destination = pypi_dir / filename
        try:
            download(file_url, destination)
            artifacts.append(destination)
            recorder.pass_(surface, f"download {filename}", file_url)
        except RuntimeError as exc:
            recorder.fail(surface, f"download {filename}", str(exc))
            continue

        digests = entry.get("digests") or {}
        if not isinstance(digests, dict):
            recorder.fail(surface, f"hash {filename}", "digests is not a dictionary")
            expected_sha256 = None
        else:
            expected_sha256 = digests.get("sha256")
        actual_sha256 = sha256_file(destination)
        if expected_sha256 == actual_sha256:
            recorder.pass_(surface, f"hash {filename}", actual_sha256)
        else:
            recorder.fail(surface, f"hash {filename}", f"expected {expected_sha256}, got {actual_sha256}")

        if attest:
            run_command(
                recorder,
                surface,
                f"PyPI provenance {filename}",
                [*attest, "verify", "pypi", "--repository", args.repository_url, file_url],
            )
    return artifacts


def verify_npm(args: argparse.Namespace, recorder: Recorder, out_dir: Path) -> list[Path]:
    surface = "npm"
    npm_dir = out_dir / "npm"
    artifacts: list[Path] = []
    encoded_package = urllib.parse.quote(args.npm_package, safe="")
    try:
        packument = fetch_json(f"https://registry.npmjs.org/{encoded_package}")
        if not isinstance(packument, dict) or not isinstance(packument.get("versions"), dict):
            raise RuntimeError("invalid registry metadata structure")
        version_data = packument["versions"][args.version]
        if not isinstance(version_data, dict):
            raise RuntimeError("invalid version metadata structure")
        recorder.pass_(surface, "registry metadata", args.npm_package)
    except (RuntimeError, KeyError, TypeError) as exc:
        recorder.fail(surface, "registry metadata", str(exc))
        return artifacts

    dist = version_data.get("dist") or {}
    if not isinstance(dist, dict):
        recorder.fail(surface, "tarball integrity metadata", "dist metadata is not a dictionary")
        return artifacts
    tarball_url = dist.get("tarball")
    integrity = dist.get("integrity")
    if not tarball_url or not integrity:
        recorder.fail(surface, "tarball integrity metadata", "dist.tarball or dist.integrity missing")
        return artifacts
    recorder.pass_(surface, "tarball integrity metadata", integrity)

    if dist.get("signatures"):
        recorder.pass_(surface, "registry signature metadata", f"{len(dist['signatures'])} signature(s)")
    else:
        recorder.fail(surface, "registry signature metadata", "dist.signatures missing")

    attestations = dist.get("attestations") or {}
    if not isinstance(attestations, dict):
        recorder.fail(surface, "provenance endpoint", "dist.attestations is not a dictionary")
        attestation_url = None
    else:
        attestation_url = attestations.get("url")
    if attestation_url:
        try:
            fetch_json(attestation_url)
            recorder.pass_(surface, "provenance endpoint", attestation_url)
        except RuntimeError as exc:
            recorder.fail(surface, "provenance endpoint", str(exc))
    else:
        recorder.fail(surface, "provenance endpoint", "dist.attestations.url missing")

    tarball = npm_dir / Path(urllib.parse.urlparse(tarball_url).path).name
    try:
        download(tarball_url, tarball)
        artifacts.append(tarball)
        recorder.pass_(surface, f"download {tarball.name}", tarball_url)
    except RuntimeError as exc:
        recorder.fail(surface, f"download {tarball.name}", str(exc))
        return artifacts

    try:
        integrity_ok = sri_matches(tarball, integrity)
    except ValueError as exc:
        recorder.fail(surface, f"integrity {tarball.name}", f"invalid dist.integrity: {exc}")
        integrity_ok = None
    if integrity_ok:
        recorder.pass_(surface, f"integrity {tarball.name}", integrity)
    elif integrity_ok is False:
        recorder.fail(surface, f"integrity {tarball.name}", "downloaded tarball does not match dist.integrity")

    if not require_tool("npm", recorder, surface):
        return artifacts

    audit_dir = npm_dir / "audit-project"
    if audit_dir.exists():
        shutil.rmtree(audit_dir)
    audit_dir.mkdir(parents=True)
    run_command(recorder, surface, "npm init audit project", ["npm", "init", "-y"], cwd=audit_dir)
    install_result = run_command(
        recorder,
        surface,
        "install package for signature audit",
        [
            "npm",
            "install",
            "--ignore-scripts",
            "--no-audit",
            "--fund=false",
            "--save-exact",
            f"{args.npm_package}@{args.version}",
        ],
        cwd=audit_dir,
    )
    if install_result.returncode == 0:
        run_command(recorder, surface, "registry signatures and provenance", ["npm", "audit", "signatures"], cwd=audit_dir)
    return artifacts


def crate_checksum(crate: str, version: str) -> str:
    metadata = fetch_json(f"https://crates.io/api/v1/crates/{crate}/{version}")
    if not isinstance(metadata, dict):
        raise RuntimeError("crates.io metadata is not a dictionary")
    version_info = metadata.get("version")
    if not isinstance(version_info, dict) or not version_info.get("checksum"):
        raise RuntimeError(f"checksum not found in crates.io response for {crate}@{version}")
    return version_info["checksum"]


def verify_crate(
    args: argparse.Namespace,
    recorder: Recorder,
    out_dir: Path,
    crate: str,
    version: str,
) -> list[Path]:
    surface = f"crates.io {crate}"
    crate_dir = out_dir / "rust"
    artifacts: list[Path] = []
    download_url = f"https://crates.io/api/v1/crates/{crate}/{version}/download"
    destination = crate_dir / f"{crate}-{version}.crate"

    try:
        expected_checksum = crate_checksum(crate, version)
        recorder.pass_(surface, "registry checksum metadata", expected_checksum)
    except RuntimeError as exc:
        recorder.fail(surface, "registry checksum metadata", str(exc))
        expected_checksum = ""

    try:
        download(download_url, destination)
        artifacts.append(destination)
        recorder.pass_(surface, f"download {destination.name}", download_url)
    except RuntimeError as exc:
        recorder.fail(surface, f"download {destination.name}", str(exc))
        return artifacts

    actual_checksum = sha256_file(destination)
    if expected_checksum and expected_checksum == actual_checksum:
        recorder.pass_(surface, f"checksum {destination.name}", actual_checksum)
    elif expected_checksum:
        recorder.fail(surface, f"checksum {destination.name}", f"expected {expected_checksum}, got {actual_checksum}")
    return artifacts


def parse_checksums(checksum_file: Path) -> dict[str, str]:
    checksums: dict[str, str] = {}
    for line in checksum_file.read_text(encoding="utf-8").splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        name = parts[1].removeprefix("./")
        checksums[name] = parts[0]
    return checksums


def verify_go(args: argparse.Namespace, recorder: Recorder, out_dir: Path) -> list[Path]:
    surface = "Go release"
    go_dir = out_dir / "go-release"
    artifacts: list[Path] = []
    if not require_tool("gh", recorder, surface):
        return artifacts

    release = run_json_command(
        recorder,
        surface,
        "release metadata",
        [
            "gh",
            "release",
            "view",
            args.go_tag,
            "--repo",
            args.repo,
            "--json",
            "tagName,isDraft,isImmutable,isPrerelease,publishedAt,assets",
        ],
    )
    if not release:
        return artifacts
    if not isinstance(release, dict):
        recorder.fail(surface, "release metadata", "invalid metadata structure returned by gh")
        return artifacts

    if release.get("isDraft"):
        recorder.fail(surface, "published release", f"{args.go_tag} is still a draft")
    else:
        recorder.pass_(surface, "published release", str(release.get("publishedAt")))

    if release.get("isImmutable"):
        recorder.pass_(surface, "immutable release", args.go_tag)
    elif args.allow_legacy_release_gaps:
        recorder.warn(surface, "immutable release", "legacy release predates immutable-release enforcement")
    else:
        recorder.fail(surface, "immutable release", "release is mutable")

    assets = release.get("assets") or []
    if not isinstance(assets, list):
        recorder.fail(surface, "release assets metadata", "assets is not a list")
        assets = []
    asset_names = {
        asset["name"]
        for asset in assets
        if isinstance(asset, dict) and isinstance(asset.get("name"), str)
    }
    required_assets = {"checksums.txt", "sbom-go-gts.spdx.json"}
    archive_assets = {name for name in asset_names if name.endswith((".tar.gz", ".zip"))}
    missing_assets = required_assets - asset_names
    if missing_assets:
        recorder.fail(surface, "required assets", f"missing {', '.join(sorted(missing_assets))}")
    elif archive_assets:
        recorder.pass_(surface, "required assets", f"{len(archive_assets)} archives plus checksums/SBOM")
    else:
        recorder.fail(surface, "required assets", "no Go archives found")

    if go_dir.exists():
        shutil.rmtree(go_dir)
    go_dir.mkdir(parents=True)
    result = run_command(
        recorder,
        surface,
        "download release assets",
        ["gh", "release", "download", args.go_tag, "--repo", args.repo, "--dir", str(go_dir), "--clobber"],
    )
    if result.returncode != 0:
        return artifacts

    artifacts = sorted(path for path in go_dir.iterdir() if path.is_file())
    checksum_file = go_dir / "checksums.txt"
    if checksum_file.exists():
        checksums = parse_checksums(checksum_file)
        checked = 0
        for name, expected in checksums.items():
            candidate = go_dir / name
            if not candidate.exists():
                recorder.fail(surface, f"checksum target {name}", "listed in checksums.txt but not downloaded")
                continue
            actual = sha256_file(candidate)
            checked += 1
            if actual == expected:
                recorder.pass_(surface, f"checksum {name}", actual)
            else:
                recorder.fail(surface, f"checksum {name}", f"expected {expected}, got {actual}")
        if not checked:
            recorder.fail(surface, "checksums.txt contents", "no checksum entries parsed")
    else:
        recorder.fail(surface, "checksums.txt download", "checksums.txt not downloaded")

    run_command(
        recorder,
        surface,
        "immutable release attestation",
        ["gh", "release", "verify", args.go_tag, "--repo", args.repo],
        allow_legacy_gap=args.allow_legacy_release_gaps,
    )
    for artifact in artifacts:
        run_command(
            recorder,
            surface,
            f"release asset attestation {artifact.name}",
            ["gh", "release", "verify-asset", args.go_tag, str(artifact), "--repo", args.repo],
            allow_legacy_gap=args.allow_legacy_release_gaps,
        )
    return artifacts


def verify_capi(args: argparse.Namespace, recorder: Recorder, out_dir: Path) -> list[Path]:
    surface = "C ABI release"
    capi_dir = out_dir / "capi-release"
    capi_version = args.capi_tag.removeprefix("capi-v")
    artifacts: list[Path] = []
    if not require_tool("gh", recorder, surface):
        return artifacts

    release = run_json_command(
        recorder,
        surface,
        "release metadata",
        [
            "gh",
            "release",
            "view",
            args.capi_tag,
            "--repo",
            args.repo,
            "--json",
            "tagName,isDraft,isImmutable,isPrerelease,publishedAt,assets",
        ],
    )
    if not release:
        return artifacts
    if not isinstance(release, dict):
        recorder.fail(surface, "release metadata", "invalid metadata structure returned by gh")
        return artifacts

    if release.get("isDraft"):
        recorder.fail(surface, "published release", f"{args.capi_tag} is still a draft")
    else:
        recorder.pass_(surface, "published release", str(release.get("publishedAt")))

    if release.get("isImmutable"):
        recorder.pass_(surface, "immutable release", args.capi_tag)
    elif args.allow_legacy_release_gaps:
        recorder.warn(surface, "immutable release", "legacy release predates immutable-release enforcement")
    else:
        recorder.fail(surface, "immutable release", "release is mutable")

    assets = release.get("assets") or []
    if not isinstance(assets, list):
        recorder.fail(surface, "release assets metadata", "assets is not a list")
        assets = []
    asset_names = {
        asset["name"]
        for asset in assets
        if isinstance(asset, dict) and isinstance(asset.get("name"), str)
    }
    required_assets = {"checksums.txt", "sbom-gmeow-gts-capi.spdx.json"}
    archive_assets = {
        name
        for name in asset_names
        if name.startswith(f"gmeow-gts-capi_{capi_version}_") and name.endswith(".tar.gz")
    }
    missing_assets = required_assets - asset_names
    if missing_assets:
        recorder.fail(surface, "required assets", f"missing {', '.join(sorted(missing_assets))}")
    elif archive_assets:
        recorder.pass_(surface, "required assets", f"{len(archive_assets)} archives plus checksums/SBOM")
    else:
        recorder.fail(surface, "required assets", f"no C ABI archives found for {capi_version}")

    if capi_dir.exists():
        shutil.rmtree(capi_dir)
    capi_dir.mkdir(parents=True)
    result = run_command(
        recorder,
        surface,
        "download release assets",
        ["gh", "release", "download", args.capi_tag, "--repo", args.repo, "--dir", str(capi_dir), "--clobber"],
    )
    if result.returncode != 0:
        return artifacts

    artifacts = sorted(path for path in capi_dir.iterdir() if path.is_file())
    checksum_file = capi_dir / "checksums.txt"
    if checksum_file.exists():
        checksums = parse_checksums(checksum_file)
        checked = 0
        for name, expected in checksums.items():
            candidate = capi_dir / name
            if not candidate.exists():
                recorder.fail(surface, f"checksum target {name}", "listed in checksums.txt but not downloaded")
                continue
            actual = sha256_file(candidate)
            checked += 1
            if actual == expected:
                recorder.pass_(surface, f"checksum {name}", actual)
            else:
                recorder.fail(surface, f"checksum {name}", f"expected {expected}, got {actual}")
        if not checked:
            recorder.fail(surface, "checksums.txt contents", "no checksum entries parsed")
    else:
        recorder.fail(surface, "checksums.txt download", "checksums.txt not downloaded")

    run_command(
        recorder,
        surface,
        "immutable release attestation",
        ["gh", "release", "verify", args.capi_tag, "--repo", args.repo],
        allow_legacy_gap=args.allow_legacy_release_gaps,
    )
    for artifact in artifacts:
        run_command(
            recorder,
            surface,
            f"release asset attestation {artifact.name}",
            ["gh", "release", "verify-asset", args.capi_tag, str(artifact), "--repo", args.repo],
            allow_legacy_gap=args.allow_legacy_release_gaps,
        )
    return artifacts


def verify_github_attestations(
    args: argparse.Namespace,
    recorder: Recorder,
    artifacts: Iterable[tuple[str, Path, bool]],
) -> None:
    if not require_tool("gh", recorder, "GitHub attestations"):
        return
    for surface, artifact, expect_sbom in artifacts:
        run_command(
            recorder,
            surface,
            f"SLSA provenance {artifact.name}",
            ["gh", "attestation", "verify", str(artifact), "--repo", args.repo],
            allow_legacy_gap=args.allow_legacy_release_gaps,
        )
        if expect_sbom:
            run_command(
                recorder,
                surface,
                f"SPDX SBOM predicate {artifact.name}",
                [
                    "gh",
                    "attestation",
                    "verify",
                    str(artifact),
                    "--repo",
                    args.repo,
                    "--predicate-type",
                    SPDX_PREDICATE,
                ],
                allow_legacy_gap=args.allow_legacy_release_gaps,
            )


def artifact_attestation_plan(
    pypi_artifacts: list[Path],
    npm_artifacts: list[Path],
    crate_artifacts: list[Path],
    go_artifacts: list[Path],
    capi_artifacts: list[Path],
) -> list[tuple[str, Path, bool]]:
    plan: list[tuple[str, Path, bool]] = []
    plan.extend(("PyPI", artifact, True) for artifact in pypi_artifacts)
    plan.extend(("npm", artifact, True) for artifact in npm_artifacts)
    plan.extend(("crates.io", artifact, True) for artifact in crate_artifacts)
    for artifact in go_artifacts:
        if artifact.name == "checksums.txt" or artifact.name == "sbom-go-gts.spdx.json":
            plan.append(("Go release", artifact, False))
        elif artifact.suffix == ".zip" or artifact.name.endswith(".tar.gz"):
            plan.append(("Go release", artifact, True))
    for artifact in capi_artifacts:
        if artifact.name == "checksums.txt" or artifact.name == "sbom-gmeow-gts-capi.spdx.json":
            plan.append(("C ABI release", artifact, False))
        elif artifact.name.endswith(".tar.gz"):
            plan.append(("C ABI release", artifact, True))
    return plan


def markdown_escape(value: str) -> str:
    return value.replace("|", "\\|")


def write_summary(args: argparse.Namespace, recorder: Recorder, out_dir: Path) -> None:
    summary_json = out_dir / "release-verification-summary.json"
    summary_md = out_dir / "release-verification-summary.md"
    summary = {
        "version": args.version,
        "visual_hashing_version": args.visual_hashing_version,
        "repo": args.repo,
        "go_tag": args.go_tag,
        "capi_tag": args.capi_tag,
        "allow_legacy_release_gaps": args.allow_legacy_release_gaps,
        "results": [asdict(result) for result in recorder.results],
    }
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    lines = [
        "# GTS Release Verification Summary",
        "",
        f"- Version: `{args.version}`",
        f"- Go tag: `{args.go_tag}`",
        f"- C ABI tag: `{args.capi_tag}`",
        f"- visual-hashing version: `{args.visual_hashing_version}`",
        f"- Repository: `{args.repo}`",
        f"- Legacy gap override: `{str(args.allow_legacy_release_gaps).lower()}`",
        "",
        "| Surface | Check | Status | Detail |",
        "|---|---|---|---|",
    ]
    for result in recorder.results:
        lines.append(
            "| "
            + " | ".join(
                [
                    markdown_escape(result.surface),
                    markdown_escape(result.check),
                    result.status,
                    markdown_escape(result.detail),
                ]
            )
            + " |"
        )
    summary_md.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Summary written to {summary_md}", flush=True)
    print(f"Machine-readable summary written to {summary_json}", flush=True)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Verify public artifacts and attestations for a GTS release family."
    )
    parser.add_argument("--version", required=True, help="GTS package version, such as 0.9.0.")
    parser.add_argument(
        "--visual-hashing-version",
        required=True,
        help="visual-hashing crate version to verify for this release family.",
    )
    parser.add_argument("--repo", default=DEFAULT_REPO, help="GitHub repository, owner/name.")
    parser.add_argument(
        "--repository-url",
        default=DEFAULT_REPO_URL,
        help="Repository URL expected by PyPI attestations.",
    )
    parser.add_argument("--pypi-project", default=DEFAULT_PYPI_PROJECT)
    parser.add_argument("--npm-package", default=DEFAULT_NPM_PACKAGE)
    parser.add_argument("--rust-crate", default=DEFAULT_RUST_CRATE)
    parser.add_argument("--visual-hashing-crate", default=DEFAULT_VISUAL_HASHING_CRATE)
    parser.add_argument("--go-tag", help="Go release tag. Defaults to go-v<version>.")
    parser.add_argument("--capi-tag", help="C ABI release tag. Defaults to capi-v<version>.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        help="Directory for downloaded artifacts and summaries. Defaults under dist/release-verification/.",
    )
    parser.add_argument(
        "--allow-legacy-release-gaps",
        action="store_true",
        help=(
            "Downgrade missing post-0.9.0 hardening evidence to warnings. "
            "Do not use for new releases."
        ),
    )
    args = parser.parse_args(argv)
    args.go_tag = args.go_tag or f"go-v{args.version}"
    args.capi_tag = args.capi_tag or f"capi-v{args.version}"
    args.out_dir = args.out_dir or Path("dist") / "release-verification" / args.version
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    recorder = Recorder()
    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    pypi_artifacts = verify_pypi(args, recorder, out_dir)
    npm_artifacts = verify_npm(args, recorder, out_dir)
    crate_artifacts = verify_crate(args, recorder, out_dir, args.rust_crate, args.version)
    crate_artifacts.extend(
        verify_crate(args, recorder, out_dir, args.visual_hashing_crate, args.visual_hashing_version)
    )
    go_artifacts = verify_go(args, recorder, out_dir)
    capi_artifacts = verify_capi(args, recorder, out_dir)
    verify_github_attestations(
        args,
        recorder,
        artifact_attestation_plan(pypi_artifacts, npm_artifacts, crate_artifacts, go_artifacts, capi_artifacts),
    )
    write_summary(args, recorder, out_dir)
    return 1 if recorder.has_failures() else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

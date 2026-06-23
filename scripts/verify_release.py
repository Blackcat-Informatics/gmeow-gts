#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Verify a published GTS release family from public release surfaces."""

from __future__ import annotations

import argparse
import base64
import hashlib
import http.client
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
from typing import Any, Iterable, Sequence


DEFAULT_REPO = "Blackcat-Informatics/gmeow-gts"
DEFAULT_REPO_URL = f"https://github.com/{DEFAULT_REPO}"
DEFAULT_PYPI_PROJECT = "gmeow-gts"
DEFAULT_NPM_PACKAGE = "@blackcatinformatics/gmeow-gts"
DEFAULT_RUST_CRATE = "gmeow-gts"
DEFAULT_VISUAL_HASHING_CRATE = "visual-hashing"
DEFAULT_CAPI_CRATE = "gmeow-gts-capi"
DEFAULT_NUGET_PACKAGE = "Gmeow.Gts"
DEFAULT_PACKAGIST_PACKAGE = "blackcatinformatics/gmeow-gts"
DEFAULT_LUAROCKS_PACKAGE = "gmeow-gts"
DEFAULT_RUBYGEMS_PACKAGE = "gmeow-gts"
DEFAULT_RUNIVERSE_OWNER = "blackcat-informatics"
DEFAULT_R_PACKAGE = "gmeowgts"
DEFAULT_JULIA_PACKAGE = "GmeowGTS"
DEFAULT_JULIA_UUID = "2d7fe44c-1957-4481-aa09-d6d0150c36ae"
SPDX_PREDICATE = "https://spdx.dev/Document/v2.3"
USER_AGENT = "gmeow-gts-release-verifier/1.0"
PERMANENT_MISSING_HTTP_STATUSES = {404, 410}


class FetchError(RuntimeError):
    def __init__(self, message: str, *, status_code: int | None = None) -> None:
        super().__init__(message)
        self.status_code = status_code


@dataclass
class CheckResult:
    surface: str
    check: str
    status: str
    release_status: str
    detail: str


class Recorder:
    def __init__(self, *, emit: bool = True) -> None:
        self.results: list[CheckResult] = []
        self.emit = emit

    def pass_(
        self,
        surface: str,
        check: str,
        detail: str = "ok",
        *,
        release_status: str = "ok",
    ) -> None:
        self._add(surface, check, "PASS", release_status, detail)

    def warn(
        self,
        surface: str,
        check: str,
        detail: str,
        *,
        release_status: str = "warning",
    ) -> None:
        self._add(surface, check, "WARN", release_status, detail)

    def fail(
        self,
        surface: str,
        check: str,
        detail: str,
        *,
        release_status: str = "failed",
    ) -> None:
        self._add(surface, check, "FAIL", release_status, detail)

    def published(self, surface: str, check: str, detail: str = "published") -> None:
        self.pass_(surface, check, detail, release_status="published")

    def pending(self, surface: str, check: str, detail: str) -> None:
        self.warn(surface, check, detail, release_status="pending")

    def metadata_mismatch(self, surface: str, check: str, detail: str) -> None:
        self.fail(surface, check, detail, release_status="metadata-mismatch")

    def missing(self, surface: str, check: str, detail: str) -> None:
        self.fail(surface, check, detail, release_status="missing")

    def _add(
        self,
        surface: str,
        check: str,
        status: str,
        release_status: str,
        detail: str,
    ) -> None:
        clean_detail = one_line(detail)
        self.results.append(
            CheckResult(surface, check, status, release_status, clean_detail)
        )
        if self.emit:
            print(
                f"{status}[{release_status}]: {surface}: {check}: {clean_detail}",
                flush=True,
            )

    def has_failures(self) -> bool:
        return any(result.status == "FAIL" for result in self.results)


def one_line(value: str, limit: int = 500) -> str:
    text = " ".join(str(value).split())
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def normalize_url(value: str) -> str:
    text = value.strip()
    if text.startswith("git+"):
        text = text[4:]
    parsed = urllib.parse.urlsplit(text)
    if parsed.scheme == "git" and parsed.netloc.casefold() == "github.com":
        text = urllib.parse.urlunsplit(
            ("https", parsed.netloc, parsed.path, parsed.query, parsed.fragment)
        )
    text = text.removesuffix(".git").rstrip("/")
    return text.casefold()


def expected_repo_urls(args: argparse.Namespace) -> set[str]:
    repo_url = args.repository_url or f"https://github.com/{args.repo}"
    return {
        normalize_url(repo_url),
        normalize_url(f"https://github.com/{args.repo}"),
        normalize_url(f"https://github.com/{args.repo}.git"),
    }


def expected_source_directory_urls(args: argparse.Namespace, directory: str) -> set[str]:
    repo_url = (args.repository_url or f"https://github.com/{args.repo}").rstrip("/")
    clean_directory = directory.strip("/")
    return {
        normalize_url(f"{repo_url}/tree/main/{clean_directory}"),
        normalize_url(f"{repo_url}/blob/main/{clean_directory}"),
        normalize_url(f"{repo_url}/{clean_directory}"),
    }


def check_metadata_link(
    recorder: Recorder,
    surface: str,
    check: str,
    actual: str | None,
    expected: Iterable[str],
) -> None:
    normalized_expected = {normalize_url(value) for value in expected if value}
    if not actual:
        recorder.missing(
            surface,
            check,
            f"metadata link missing; expected one of {sorted(normalized_expected)}",
        )
        return
    normalized_actual = normalize_url(actual)
    if normalized_actual in normalized_expected:
        recorder.published(surface, check, actual)
    else:
        recorder.metadata_mismatch(
            surface,
            check,
            f"expected one of {sorted(normalized_expected)}, got {actual}",
        )


def check_any_metadata_link(
    recorder: Recorder,
    surface: str,
    check: str,
    actual_values: Iterable[Any],
    expected: Iterable[str],
) -> None:
    actual = [value for value in actual_values if isinstance(value, str) and value]
    normalized_expected = {normalize_url(value) for value in expected if value}
    for value in actual:
        if normalize_url(value) in normalized_expected:
            recorder.published(surface, check, value)
            return
    if actual:
        recorder.metadata_mismatch(
            surface,
            check,
            f"expected one of {sorted(normalized_expected)}, got {actual}",
        )
    else:
        recorder.missing(
            surface,
            check,
            f"metadata link missing; expected one of {sorted(normalized_expected)}",
        )


def record_registry_fetch_error(
    recorder: Recorder, surface: str, check: str, exc: BaseException
) -> None:
    if (
        isinstance(exc, FetchError)
        and exc.status_code in PERMANENT_MISSING_HTTP_STATUSES
    ):
        recorder.missing(surface, check, str(exc))
    else:
        recorder.pending(surface, check, str(exc))


def repository_directory_matches(actual: Any, expected: str) -> bool:
    return isinstance(actual, str) and actual.strip("/").casefold() == expected.strip(
        "/"
    ).casefold()


def collect_project_urls(info: dict[str, Any]) -> list[str]:
    values: list[str] = []
    for key in ("home_page", "project_url"):
        value = info.get(key)
        if isinstance(value, str):
            values.append(value)
    project_urls = info.get("project_urls")
    if isinstance(project_urls, dict):
        values.extend(value for value in project_urls.values() if isinstance(value, str))
    return values


def npm_repository_url(repository: Any) -> str | None:
    if isinstance(repository, str):
        return repository
    if isinstance(repository, dict) and isinstance(repository.get("url"), str):
        return repository["url"]
    return None


def request(
    url: str, *, headers: dict[str, str] | None = None
) -> urllib.request.Request:
    merged = {"User-Agent": USER_AGENT}
    if headers:
        merged.update(headers)
    return urllib.request.Request(url, headers=merged)


def raise_fetch_error(kind: str, url: str, last_error: Exception | None) -> None:
    message = f"failed to fetch {kind} from {url}: {last_error}"
    if isinstance(last_error, FetchError):
        raise FetchError(message, status_code=last_error.status_code) from last_error
    raise RuntimeError(message) from last_error


def fetch_json(
    url: str, *, headers: dict[str, str] | None = None, attempts: int = 3
) -> Any:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(
                request(url, headers=headers), timeout=30
            ) as response:
                return json.load(response)
        except urllib.error.HTTPError as exc:
            last_error = FetchError(
                f"HTTP {exc.code} {exc.reason}", status_code=exc.code
            )
            if exc.code in PERMANENT_MISSING_HTTP_STATUSES:
                break
            if attempt < attempts:
                time.sleep(attempt)
        except (
            OSError,
            urllib.error.URLError,
            json.JSONDecodeError,
            http.client.HTTPException,
        ) as exc:
            last_error = exc
            if attempt < attempts:
                time.sleep(attempt)
    raise_fetch_error("JSON", url, last_error)


def fetch_text(
    url: str, *, headers: dict[str, str] | None = None, attempts: int = 3
) -> str:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(
                request(url, headers=headers), timeout=30
            ) as response:
                return response.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            last_error = FetchError(
                f"HTTP {exc.code} {exc.reason}", status_code=exc.code
            )
            if exc.code in PERMANENT_MISSING_HTTP_STATUSES:
                break
            if attempt < attempts:
                time.sleep(attempt)
        except (
            OSError,
            UnicodeDecodeError,
            urllib.error.URLError,
            http.client.HTTPException,
        ) as exc:
            last_error = exc
            if attempt < attempts:
                time.sleep(attempt)
    raise_fetch_error("text", url, last_error)


def download(
    url: str, destination: Path, *, headers: dict[str, str] | None = None
) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    last_error: Exception | None = None
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(
                request(url, headers=headers), timeout=60
            ) as response:
                with destination.open("wb") as handle:
                    shutil.copyfileobj(response, handle)
            return
        except (OSError, urllib.error.URLError, http.client.HTTPException) as exc:
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
        recorder.warn(
            surface, check, f"legacy gap allowed: {result.stdout.strip() or pretty}"
        )
    else:
        recorder.fail(
            surface,
            check,
            f"exit {result.returncode}: {result.stdout.strip() or pretty}",
        )
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


def verify_pypi(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    surface = "PyPI"
    pypi_dir = out_dir / "python"
    artifacts: list[Path] = []
    pypi_json_url = f"https://pypi.org/pypi/{args.pypi_project}/{args.version}/json"
    try:
        metadata = fetch_json(pypi_json_url)
        if not isinstance(metadata, dict):
            raise RuntimeError("metadata is not a dictionary")
        recorder.published(surface, "release metadata", pypi_json_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "release metadata", exc)
        return artifacts
    info = metadata.get("info")
    if isinstance(info, dict):
        check_any_metadata_link(
            recorder,
            surface,
            "repository/homepage metadata",
            collect_project_urls(info),
            expected_repo_urls(args),
        )
    else:
        recorder.metadata_mismatch(surface, "repository/homepage metadata", "info missing")

    urls = metadata.get("urls") or []
    if not isinstance(urls, list):
        recorder.metadata_mismatch(surface, "release file metadata", "urls is not a list")
        return artifacts
    wanted_types = {"bdist_wheel", "sdist"}
    found_types = {
        entry.get("packagetype") for entry in urls if isinstance(entry, dict)
    }
    missing_types = wanted_types - found_types
    if missing_types:
        recorder.missing(
            surface,
            "wheel and sdist present",
            f"missing {', '.join(sorted(missing_types))}",
        )
    else:
        recorder.published(
            surface,
            "wheel and sdist present",
            ", ".join(sorted(found_types & wanted_types)),
        )

    attest = pypi_attestations_command()
    if not attest:
        recorder.fail(
            surface, "pypi-attestations available", "install pypi-attestations or uvx"
        )

    for entry in urls:
        if not isinstance(entry, dict):
            recorder.metadata_mismatch(
                surface, "release file metadata", "file entry is not a dictionary"
            )
            continue
        filename = entry.get("filename")
        file_url = entry.get("url")
        if not isinstance(filename, str) or not isinstance(file_url, str):
            recorder.metadata_mismatch(
                surface, "release file metadata", "filename or URL missing"
            )
            continue
        destination = pypi_dir / filename
        try:
            download(file_url, destination)
            artifacts.append(destination)
            recorder.published(surface, f"download {filename}", file_url)
        except RuntimeError as exc:
            recorder.missing(surface, f"download {filename}", str(exc))
            continue

        digests = entry.get("digests") or {}
        if not isinstance(digests, dict):
            recorder.metadata_mismatch(
                surface, f"hash {filename}", "digests is not a dictionary"
            )
            expected_sha256 = None
        else:
            expected_sha256 = digests.get("sha256")
        actual_sha256 = sha256_file(destination)
        if expected_sha256 == actual_sha256:
            recorder.published(surface, f"hash {filename}", actual_sha256)
        else:
            recorder.metadata_mismatch(
                surface,
                f"hash {filename}",
                f"expected {expected_sha256}, got {actual_sha256}",
            )

        if attest:
            run_command(
                recorder,
                surface,
                f"PyPI provenance {filename}",
                [
                    *attest,
                    "verify",
                    "pypi",
                    "--repository",
                    args.repository_url,
                    file_url,
                ],
            )
    return artifacts


def verify_npm(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    surface = "npm"
    npm_dir = out_dir / "npm"
    artifacts: list[Path] = []
    encoded_package = urllib.parse.quote(args.npm_package, safe="")
    try:
        packument = fetch_json(f"https://registry.npmjs.org/{encoded_package}")
        if not isinstance(packument, dict) or not isinstance(
            packument.get("versions"), dict
        ):
            raise RuntimeError("invalid registry metadata structure")
        version_data = packument["versions"][args.version]
        if not isinstance(version_data, dict):
            raise RuntimeError("invalid version metadata structure")
        recorder.published(surface, "registry metadata", args.npm_package)
    except (RuntimeError, KeyError, TypeError) as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return artifacts
    check_metadata_link(
        recorder,
        surface,
        "repository metadata",
        npm_repository_url(version_data.get("repository")),
        expected_repo_urls(args),
    )
    repository = version_data.get("repository")
    if isinstance(repository, dict) and "directory" in repository:
        if repository_directory_matches(repository.get("directory"), "ts"):
            recorder.published(surface, "source directory metadata", "ts")
        else:
            recorder.metadata_mismatch(
                surface,
                "source directory metadata",
                f"expected ts, got {repository.get('directory')}",
            )

    dist = version_data.get("dist") or {}
    if not isinstance(dist, dict):
        recorder.metadata_mismatch(
            surface, "tarball integrity metadata", "dist metadata is not a dictionary"
        )
        return artifacts
    tarball_url = dist.get("tarball")
    integrity = dist.get("integrity")
    if not tarball_url or not integrity:
        recorder.metadata_mismatch(
            surface,
            "tarball integrity metadata",
            "dist.tarball or dist.integrity missing",
        )
        return artifacts
    recorder.published(surface, "tarball integrity metadata", integrity)

    if dist.get("signatures"):
        recorder.published(
            surface,
            "registry signature metadata",
            f"{len(dist['signatures'])} signature(s)",
        )
    else:
        recorder.missing(surface, "registry signature metadata", "dist.signatures missing")

    attestations = dist.get("attestations") or {}
    if not isinstance(attestations, dict):
        recorder.metadata_mismatch(
            surface, "provenance endpoint", "dist.attestations is not a dictionary"
        )
        attestation_url = None
    else:
        attestation_url = attestations.get("url")
    if attestation_url:
        try:
            fetch_json(attestation_url)
            recorder.published(surface, "provenance endpoint", attestation_url)
        except RuntimeError as exc:
            recorder.missing(surface, "provenance endpoint", str(exc))
    else:
        recorder.missing(surface, "provenance endpoint", "dist.attestations.url missing")

    tarball = npm_dir / Path(urllib.parse.urlparse(tarball_url).path).name
    try:
        download(tarball_url, tarball)
        artifacts.append(tarball)
        recorder.published(surface, f"download {tarball.name}", tarball_url)
    except RuntimeError as exc:
        recorder.missing(surface, f"download {tarball.name}", str(exc))
        return artifacts

    try:
        integrity_ok = sri_matches(tarball, integrity)
    except ValueError as exc:
        recorder.metadata_mismatch(
            surface, f"integrity {tarball.name}", f"invalid dist.integrity: {exc}"
        )
        integrity_ok = None
    if integrity_ok:
        recorder.published(surface, f"integrity {tarball.name}", integrity)
    elif integrity_ok is False:
        recorder.metadata_mismatch(
            surface,
            f"integrity {tarball.name}",
            "downloaded tarball does not match dist.integrity",
        )

    if not require_tool("npm", recorder, surface):
        return artifacts

    audit_dir = npm_dir / "audit-project"
    if audit_dir.exists():
        shutil.rmtree(audit_dir)
    audit_dir.mkdir(parents=True)
    run_command(
        recorder,
        surface,
        "npm init audit project",
        ["npm", "init", "-y"],
        cwd=audit_dir,
    )
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
        run_command(
            recorder,
            surface,
            "registry signatures and provenance",
            ["npm", "audit", "signatures"],
            cwd=audit_dir,
        )
    return artifacts


def crate_version_metadata(crate: str, version: str) -> dict[str, Any]:
    metadata = fetch_json(f"https://crates.io/api/v1/crates/{crate}/{version}")
    if not isinstance(metadata, dict):
        raise RuntimeError("crates.io metadata is not a dictionary")
    return metadata


def crate_repository_metadata(crate: str) -> dict[str, Any]:
    metadata = fetch_json(f"https://crates.io/api/v1/crates/{crate}")
    crate_info = metadata.get("crate") if isinstance(metadata, dict) else None
    if not isinstance(crate_info, dict):
        raise RuntimeError("crate repository metadata missing from crates.io response")
    return crate_info


def crate_checksum(metadata: dict[str, Any], crate: str, version: str) -> str:
    version_info = metadata.get("version")
    if not isinstance(version_info, dict) or not version_info.get("checksum"):
        raise RuntimeError(
            f"checksum not found in crates.io response for {crate}@{version}"
        )
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
        expected_checksum = crate_checksum(
            crate_version_metadata(crate, version), crate, version
        )
        recorder.published(surface, "registry metadata", crate)
        recorder.published(surface, "registry checksum metadata", expected_checksum)
    except RuntimeError as exc:
        record_registry_fetch_error(
            recorder, surface, "registry checksum metadata", exc
        )
        expected_checksum = ""

    try:
        crate_info = crate_repository_metadata(crate)
        check_metadata_link(
            recorder,
            surface,
            "repository metadata",
            crate_info.get("repository"),
            expected_repo_urls(args),
        )
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "repository metadata", exc)

    try:
        download(download_url, destination)
        artifacts.append(destination)
        recorder.published(surface, f"download {destination.name}", download_url)
    except RuntimeError as exc:
        recorder.missing(surface, f"download {destination.name}", str(exc))
        return artifacts

    actual_checksum = sha256_file(destination)
    if expected_checksum and expected_checksum == actual_checksum:
        recorder.published(surface, f"checksum {destination.name}", actual_checksum)
    elif expected_checksum:
        recorder.metadata_mismatch(
            surface,
            f"checksum {destination.name}",
            f"expected {expected_checksum}, got {actual_checksum}",
        )
    return artifacts


def version_present(values: Iterable[Any], version: str) -> bool:
    return any(str(value).casefold() == version.casefold() for value in values)


def verify_nuget(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    surface = f"NuGet {args.nuget_package}"
    nuget_dir = out_dir / "nuget"
    artifacts: list[Path] = []
    normalized = args.nuget_package.lower()
    metadata_url = f"https://api.nuget.org/v3-flatcontainer/{normalized}/index.json"
    try:
        metadata = fetch_json(metadata_url)
        versions = metadata.get("versions") if isinstance(metadata, dict) else None
        if not isinstance(versions, list):
            raise RuntimeError("versions list missing")
        recorder.published(surface, "registry metadata", metadata_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return artifacts

    if version_present(versions, args.version):
        recorder.published(surface, "package version", args.version)
    else:
        recorder.pending(surface, "package version", f"{args.version} not present")
        return artifacts

    registration_url = (
        "https://api.nuget.org/v3/registration5-semver1/"
        f"{normalized}/{args.version.lower()}.json"
    )
    try:
        registration = fetch_json(registration_url)
        catalog_entry = (
            registration.get("catalogEntry")
            if isinstance(registration, dict)
            else None
        )
        if not isinstance(catalog_entry, dict):
            raise RuntimeError("catalogEntry missing")
        check_any_metadata_link(
            recorder,
            surface,
            "repository/homepage metadata",
            (catalog_entry.get("repositoryUrl"), catalog_entry.get("projectUrl")),
            expected_repo_urls(args)
            | expected_source_directory_urls(args, "dotnet"),
        )
    except RuntimeError as exc:
        recorder.missing(surface, "repository/homepage metadata", str(exc))

    package_url = f"https://api.nuget.org/v3-flatcontainer/{normalized}/{args.version.lower()}/{normalized}.{args.version.lower()}.nupkg"
    destination = nuget_dir / f"{args.nuget_package}.{args.version}.nupkg"
    try:
        download(package_url, destination)
        artifacts.append(destination)
        recorder.published(surface, f"download {destination.name}", package_url)
    except RuntimeError as exc:
        recorder.missing(surface, f"download {destination.name}", str(exc))
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source-only wrapper; host must provide libgts",
    )
    return artifacts


def verify_packagist(args: argparse.Namespace, recorder: Recorder) -> None:
    surface = f"Packagist {args.packagist_package}"
    metadata_url = f"https://repo.packagist.org/p2/{args.packagist_package}.json"
    try:
        metadata = fetch_json(metadata_url)
        packages = metadata.get("packages") if isinstance(metadata, dict) else None
        versions = (
            packages.get(args.packagist_package) if isinstance(packages, dict) else None
        )
        if not isinstance(versions, list):
            raise RuntimeError("package versions missing")
        recorder.published(surface, "registry metadata", metadata_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return

    accepted_versions = {args.version, f"v{args.version}"}
    matching = [
        entry
        for entry in versions
        if isinstance(entry, dict)
        and (
            entry.get("version") in accepted_versions
            or str(entry.get("version_normalized", "")).startswith(f"{args.version}.")
        )
    ]
    if not matching:
        recorder.pending(surface, "package version", f"{args.version} not present")
        return
    recorder.published(surface, "package version", args.version)
    source = matching[0].get("source")
    if isinstance(source, dict):
        check_metadata_link(
            recorder,
            surface,
            "source repository metadata",
            source.get("url"),
            expected_repo_urls(args),
        )
        if source.get("reference"):
            recorder.published(surface, "source reference", str(source["reference"]))
        else:
            recorder.missing(surface, "source reference", "source reference missing")
    else:
        recorder.missing(surface, "source reference", "source metadata missing")
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source-only wrapper; host must provide libgts",
    )


def verify_luarocks(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    surface = f"LuaRocks {args.luarocks_package}"
    luarocks_dir = out_dir / "luarocks"
    artifacts: list[Path] = []
    rock_version = args.luarocks_version or f"{args.version}-1"
    manifest_url = "https://luarocks.org/manifest.json"
    try:
        manifest = fetch_json(manifest_url)
        repository = manifest.get("repository") if isinstance(manifest, dict) else None
        package_versions = (
            repository.get(args.luarocks_package)
            if isinstance(repository, dict)
            else None
        )
        if not isinstance(package_versions, dict):
            raise RuntimeError("package missing from root manifest")
        recorder.published(surface, "registry metadata", manifest_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return artifacts

    entries = package_versions.get(rock_version)
    if not isinstance(entries, list):
        recorder.pending(surface, "package version", f"{rock_version} not present")
        return artifacts
    recorder.published(surface, "package version", rock_version)
    arch_values = sorted(
        str(entry.get("arch", "unknown"))
        for entry in entries
        if isinstance(entry, dict)
    )
    if arch_values:
        recorder.published(surface, "published artifact types", ", ".join(arch_values))
    else:
        recorder.warn(
            surface, "published artifact types", "no artifact arch entries found"
        )

    rockspec_url = (
        f"https://luarocks.org/{args.luarocks_package}-{rock_version}.rockspec"
    )
    destination = luarocks_dir / f"{args.luarocks_package}-{rock_version}.rockspec"
    try:
        download(rockspec_url, destination)
        artifacts.append(destination)
        recorder.published(surface, f"download {destination.name}", rockspec_url)
    except RuntimeError as exc:
        recorder.missing(surface, f"download {destination.name}", str(exc))
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source-only wrapper; host must provide libgts",
    )
    return artifacts


def verify_rubygems(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    surface = f"RubyGems {args.rubygems_package}"
    ruby_dir = out_dir / "ruby"
    artifacts: list[Path] = []
    versions_url = f"https://rubygems.org/api/v1/versions/{args.rubygems_package}.json"
    try:
        versions = fetch_json(versions_url)
        if not isinstance(versions, list):
            raise RuntimeError("versions response is not a list")
        recorder.published(surface, "registry metadata", versions_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return artifacts

    if version_present(
        (entry.get("number") for entry in versions if isinstance(entry, dict)),
        args.version,
    ):
        recorder.published(surface, "package version", args.version)
    else:
        recorder.pending(surface, "package version", f"{args.version} not present")
        return artifacts

    metadata_url = f"https://rubygems.org/api/v2/rubygems/{args.rubygems_package}.json"
    try:
        gem_metadata = fetch_json(metadata_url)
        if not isinstance(gem_metadata, dict):
            raise RuntimeError("gem metadata is not a dictionary")
        check_any_metadata_link(
            recorder,
            surface,
            "repository/homepage metadata",
            (
                gem_metadata.get("source_code_uri"),
                gem_metadata.get("homepage_uri"),
            ),
            expected_repo_urls(args) | expected_source_directory_urls(args, "ruby"),
        )
    except RuntimeError as exc:
        recorder.missing(surface, "repository/homepage metadata", str(exc))

    gem_url = (
        f"https://rubygems.org/downloads/{args.rubygems_package}-{args.version}.gem"
    )
    destination = ruby_dir / f"{args.rubygems_package}-{args.version}.gem"
    try:
        download(gem_url, destination)
        artifacts.append(destination)
        recorder.published(surface, f"download {destination.name}", gem_url)
    except RuntimeError as exc:
        recorder.missing(surface, f"download {destination.name}", str(exc))
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source-only wrapper; host must provide libgts",
    )
    return artifacts


def parse_r_packages_index(text: str) -> dict[str, dict[str, str]]:
    packages: dict[str, dict[str, str]] = {}
    current: dict[str, str] = {}
    for line in text.splitlines() + [""]:
        if not line.strip():
            name = current.get("Package")
            if name:
                packages[name] = current
            current = {}
            continue
        if line.startswith((" ", "\t")):
            continue
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        current[key.strip()] = value.strip()
    return packages


def verify_runiverse(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    surface = f"r-universe {args.r_package}"
    r_dir = out_dir / "r"
    artifacts: list[Path] = []
    packages_url = (
        f"https://{args.r_universe_owner}.r-universe.dev/src/contrib/PACKAGES"
    )
    try:
        packages = parse_r_packages_index(fetch_text(packages_url))
        metadata = packages.get(args.r_package)
        if not metadata:
            raise RuntimeError("package missing from PACKAGES index")
        recorder.published(surface, "registry metadata", packages_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return artifacts

    published_version = metadata.get("Version")
    if published_version == args.version:
        recorder.published(surface, "package version", args.version)
    else:
        recorder.pending(
            surface,
            "package version",
            f"expected {args.version}, got {published_version}",
        )
        return artifacts
    check_any_metadata_link(
        recorder,
        surface,
        "repository/homepage metadata",
        (metadata.get("URL"), metadata.get("BugReports")),
        expected_repo_urls(args) | expected_source_directory_urls(args, "r"),
    )

    tarball_url = f"https://{args.r_universe_owner}.r-universe.dev/src/contrib/{args.r_package}_{args.version}.tar.gz"
    destination = r_dir / f"{args.r_package}_{args.version}.tar.gz"
    try:
        download(tarball_url, destination)
        artifacts.append(destination)
        recorder.published(surface, f"download {destination.name}", tarball_url)
    except RuntimeError as exc:
        recorder.missing(surface, f"download {destination.name}", str(exc))
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source package; host must provide libgts",
    )
    return artifacts


def julia_registry_path(package: str) -> str:
    return f"{package[0].upper()}/{package}"


def verify_julia_general(args: argparse.Namespace, recorder: Recorder) -> None:
    surface = f"Julia General {args.julia_package}"
    if not args.julia_package:
        recorder.metadata_mismatch(
            surface, "registry metadata", "Julia package name is empty"
        )
        return
    base = f"https://raw.githubusercontent.com/JuliaRegistries/General/master/{julia_registry_path(args.julia_package)}"
    package_url = f"{base}/Package.toml"
    versions_url = f"{base}/Versions.toml"
    try:
        package_toml = fetch_text(package_url)
        recorder.published(surface, "registry metadata", package_url)
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "registry metadata", exc)
        return

    if (
        f'name = "{args.julia_package}"' in package_toml
        and f'uuid = "{args.julia_uuid}"' in package_toml
    ):
        recorder.published(
            surface, "package identity", f"{args.julia_package} {args.julia_uuid}"
        )
    else:
        recorder.metadata_mismatch(surface, "package identity", "name or UUID mismatch")
    repo_line = next(
        (
            line.split("=", 1)[1].strip().strip('"')
            for line in package_toml.splitlines()
            if line.strip().startswith("repo =") and "=" in line
        ),
        None,
    )
    check_metadata_link(
        recorder,
        surface,
        "repository metadata",
        repo_line,
        expected_repo_urls(args) | expected_source_directory_urls(args, "julia"),
    )

    try:
        versions_toml = fetch_text(versions_url)
        if f'["{args.version}"]' in versions_toml:
            recorder.published(surface, "package version", args.version)
        else:
            recorder.pending(surface, "package version", f"{args.version} not present")
    except RuntimeError as exc:
        record_registry_fetch_error(recorder, surface, "package version", exc)
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source-only wrapper; host must provide libgts",
    )


def verify_swift_package(args: argparse.Namespace, recorder: Recorder) -> None:
    surface = "Swift Package Index GmeowGTS"
    tag = args.swift_tag or args.version
    if "/" not in args.repo:
        recorder.metadata_mismatch(
            surface, "semantic version tag", f"invalid repository format: {args.repo}"
        )
        return
    owner, repo_name = args.repo.split("/", 1)
    encoded_tag = urllib.parse.quote(tag, safe="")
    ref_url = (
        f"https://api.github.com/repos/{owner}/{repo_name}/git/ref/tags/{encoded_tag}"
    )
    try:
        ref_data = fetch_json(
            ref_url, headers={"Accept": "application/vnd.github+json"}
        )
        if not isinstance(ref_data, dict) or "object" not in ref_data:
            raise RuntimeError("invalid GitHub tag metadata")
        recorder.published(surface, "semantic version tag", tag)
    except RuntimeError as exc:
        recorder.pending(surface, "semantic version tag", str(exc))

    spi_url = f"https://swiftpackageindex.com/{owner}/{repo_name}"
    recorder.published(surface, "Swift Package Index URL", spi_url)
    recorder.pass_(
        surface,
        "native dependency expectation",
        "source-only wrapper; host must provide libgts",
    )


def verify_wrapper_packages(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
    artifacts: list[Path] = []
    artifacts.extend(
        verify_crate(args, recorder, out_dir, args.capi_crate, args.version)
    )
    artifacts.extend(verify_nuget(args, recorder, out_dir))
    verify_packagist(args, recorder)
    artifacts.extend(verify_luarocks(args, recorder, out_dir))
    verify_swift_package(args, recorder)
    artifacts.extend(verify_rubygems(args, recorder, out_dir))
    artifacts.extend(verify_runiverse(args, recorder, out_dir))
    verify_julia_general(args, recorder)
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


def verify_go(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
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
        recorder.metadata_mismatch(
            surface, "release metadata", "invalid metadata structure returned by gh"
        )
        return artifacts

    if release.get("isDraft"):
        recorder.metadata_mismatch(
            surface, "published release", f"{args.go_tag} is still a draft"
        )
    else:
        recorder.published(surface, "published release", str(release.get("publishedAt")))

    if release.get("isImmutable"):
        recorder.published(surface, "immutable release", args.go_tag)
    elif args.allow_legacy_release_gaps:
        recorder.warn(
            surface,
            "immutable release",
            "legacy release predates immutable-release enforcement",
        )
    else:
        recorder.metadata_mismatch(surface, "immutable release", "release is mutable")

    assets = release.get("assets") or []
    if not isinstance(assets, list):
        recorder.metadata_mismatch(
            surface, "release assets metadata", "assets is not a list"
        )
        assets = []
    asset_names = {
        asset["name"]
        for asset in assets
        if isinstance(asset, dict) and isinstance(asset.get("name"), str)
    }
    required_assets = {"checksums.txt", "sbom-go-gts.spdx.json"}
    archive_assets = {
        name for name in asset_names if name.endswith((".tar.gz", ".zip"))
    }
    missing_assets = required_assets - asset_names
    if missing_assets:
        recorder.missing(
            surface, "required assets", f"missing {', '.join(sorted(missing_assets))}"
        )
    elif archive_assets:
        recorder.published(
            surface,
            "required assets",
            f"{len(archive_assets)} archives plus checksums/SBOM",
        )
    else:
        recorder.missing(surface, "required assets", "no Go archives found")

    if go_dir.exists():
        shutil.rmtree(go_dir)
    go_dir.mkdir(parents=True)
    result = run_command(
        recorder,
        surface,
        "download release assets",
        [
            "gh",
            "release",
            "download",
            args.go_tag,
            "--repo",
            args.repo,
            "--dir",
            str(go_dir),
            "--clobber",
        ],
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
                recorder.missing(
                    surface,
                    f"checksum target {name}",
                    "listed in checksums.txt but not downloaded",
                )
                continue
            actual = sha256_file(candidate)
            checked += 1
            if actual == expected:
                recorder.published(surface, f"checksum {name}", actual)
            else:
                recorder.metadata_mismatch(
                    surface, f"checksum {name}", f"expected {expected}, got {actual}"
                )
        if not checked:
            recorder.metadata_mismatch(
                surface, "checksums.txt contents", "no checksum entries parsed"
            )
    else:
        recorder.missing(
            surface, "checksums.txt download", "checksums.txt not downloaded"
        )

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
            [
                "gh",
                "release",
                "verify-asset",
                args.go_tag,
                str(artifact),
                "--repo",
                args.repo,
            ],
            allow_legacy_gap=args.allow_legacy_release_gaps,
        )
    return artifacts


def verify_capi(
    args: argparse.Namespace, recorder: Recorder, out_dir: Path
) -> list[Path]:
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
        recorder.metadata_mismatch(
            surface, "release metadata", "invalid metadata structure returned by gh"
        )
        return artifacts

    if release.get("isDraft"):
        recorder.metadata_mismatch(
            surface, "published release", f"{args.capi_tag} is still a draft"
        )
    else:
        recorder.published(surface, "published release", str(release.get("publishedAt")))

    if release.get("isImmutable"):
        recorder.published(surface, "immutable release", args.capi_tag)
    elif args.allow_legacy_release_gaps:
        recorder.warn(
            surface,
            "immutable release",
            "legacy release predates immutable-release enforcement",
        )
    else:
        recorder.metadata_mismatch(surface, "immutable release", "release is mutable")

    assets = release.get("assets") or []
    if not isinstance(assets, list):
        recorder.metadata_mismatch(
            surface, "release assets metadata", "assets is not a list"
        )
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
        if name.startswith(f"gmeow-gts-capi_{capi_version}_")
        and name.endswith(".tar.gz")
    }
    missing_assets = required_assets - asset_names
    if missing_assets:
        recorder.missing(
            surface, "required assets", f"missing {', '.join(sorted(missing_assets))}"
        )
    elif archive_assets:
        recorder.published(
            surface,
            "required assets",
            f"{len(archive_assets)} archives plus checksums/SBOM",
        )
    else:
        recorder.missing(
            surface, "required assets", f"no C ABI archives found for {capi_version}"
        )

    if capi_dir.exists():
        shutil.rmtree(capi_dir)
    capi_dir.mkdir(parents=True)
    result = run_command(
        recorder,
        surface,
        "download release assets",
        [
            "gh",
            "release",
            "download",
            args.capi_tag,
            "--repo",
            args.repo,
            "--dir",
            str(capi_dir),
            "--clobber",
        ],
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
                recorder.missing(
                    surface,
                    f"checksum target {name}",
                    "listed in checksums.txt but not downloaded",
                )
                continue
            actual = sha256_file(candidate)
            checked += 1
            if actual == expected:
                recorder.published(surface, f"checksum {name}", actual)
            else:
                recorder.metadata_mismatch(
                    surface, f"checksum {name}", f"expected {expected}, got {actual}"
                )
        if not checked:
            recorder.metadata_mismatch(
                surface, "checksums.txt contents", "no checksum entries parsed"
            )
    else:
        recorder.missing(
            surface, "checksums.txt download", "checksums.txt not downloaded"
        )

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
            [
                "gh",
                "release",
                "verify-asset",
                args.capi_tag,
                str(artifact),
                "--repo",
                args.repo,
            ],
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
    wrapper_attested_artifacts: list[Path],
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
        if (
            artifact.name == "checksums.txt"
            or artifact.name == "sbom-gmeow-gts-capi.spdx.json"
        ):
            plan.append(("C ABI release", artifact, False))
        elif artifact.name.endswith(".tar.gz"):
            plan.append(("C ABI release", artifact, True))
    for artifact in wrapper_attested_artifacts:
        plan.append(("Wrapper package", artifact, True))
    return plan


def markdown_escape(value: str) -> str:
    return value.replace("|", "\\|")


def dry_run_checks(args: argparse.Namespace, recorder: Recorder) -> None:
    planned_surfaces: list[tuple[str, Sequence[str]]] = [
        (
            "PyPI",
            (
                "release metadata",
                "repository/homepage metadata",
                "wheel and sdist present",
            ),
        ),
        (
            "npm",
            (
                "registry metadata",
                "repository metadata",
                "tarball integrity metadata",
                "registry signature metadata",
                "provenance endpoint",
            ),
        ),
        (
            f"crates.io {args.rust_crate}",
            ("registry metadata", "repository metadata", "package version"),
        ),
        (
            f"crates.io {args.visual_hashing_crate}",
            ("registry metadata", "repository metadata", "package version"),
        ),
        ("Go release", ("release metadata", "published release", "required assets")),
        ("C ABI release", ("release metadata", "published release", "required assets")),
    ]
    if args.include_wrapper_packages:
        planned_surfaces.extend(
            [
                (
                    f"crates.io {args.capi_crate}",
                    ("registry metadata", "repository metadata", "package version"),
                ),
                (
                    f"NuGet {args.nuget_package}",
                    (
                        "registry metadata",
                        "package version",
                        "repository/homepage metadata",
                    ),
                ),
                (
                    f"Packagist {args.packagist_package}",
                    (
                        "registry metadata",
                        "package version",
                        "source repository metadata",
                    ),
                ),
                (
                    f"LuaRocks {args.luarocks_package}",
                    (
                        "registry metadata",
                        "package version",
                        "published artifact types",
                    ),
                ),
                (
                    "Swift Package Index GmeowGTS",
                    ("semantic version tag", "Swift Package Index URL"),
                ),
                (
                    f"RubyGems {args.rubygems_package}",
                    (
                        "registry metadata",
                        "package version",
                        "repository/homepage metadata",
                    ),
                ),
                (
                    f"r-universe {args.r_package}",
                    (
                        "registry metadata",
                        "package version",
                        "repository/homepage metadata",
                    ),
                ),
                (
                    f"Julia General {args.julia_package}",
                    ("registry metadata", "package identity", "package version"),
                ),
            ]
        )
    for surface, checks in planned_surfaces:
        for check in checks:
            recorder.pending(
                surface,
                check,
                "dry run: planned check not executed against live registry",
            )


def run_self_test() -> int:
    recorder = Recorder(emit=False)
    recorder.published("fixture", "package version", "1.2.3")
    recorder.pending("fixture", "package version", "registry lag")
    recorder.metadata_mismatch("fixture", "repository metadata", "wrong URL")
    recorder.missing("fixture", "download artifact", "artifact not found")
    statuses = [result.release_status for result in recorder.results]
    expected = ["published", "pending", "metadata-mismatch", "missing"]
    if statuses != expected:
        print(
            f"verify_release self-test: expected {expected}, got {statuses}",
            file=sys.stderr,
        )
        return 1
    if not recorder.has_failures():
        print("verify_release self-test: failure severity not detected", file=sys.stderr)
        return 1

    link_recorder = Recorder(emit=False)
    args = argparse.Namespace(repo=DEFAULT_REPO, repository_url=DEFAULT_REPO_URL)
    check_metadata_link(
        link_recorder,
        "fixture",
        "repository metadata",
        f"{DEFAULT_REPO_URL}.git",
        expected_repo_urls(args),
    )
    check_metadata_link(
        link_recorder,
        "fixture",
        "repository metadata",
        "https://example.invalid/wrong",
        expected_repo_urls(args),
    )
    link_statuses = [result.release_status for result in link_recorder.results]
    if link_statuses != ["published", "metadata-mismatch"]:
        print(
            "verify_release self-test: metadata link classification drifted",
            file=sys.stderr,
        )
        return 1

    fetch_recorder = Recorder(emit=False)
    record_registry_fetch_error(
        fetch_recorder,
        "fixture",
        "registry metadata",
        FetchError("HTTP 404 Not Found", status_code=404),
    )
    record_registry_fetch_error(
        fetch_recorder,
        "fixture",
        "registry metadata",
        FetchError("HTTP 500 Internal Server Error", status_code=500),
    )
    record_registry_fetch_error(
        fetch_recorder,
        "fixture",
        "registry metadata",
        RuntimeError("version not present"),
    )
    fetch_statuses = [result.release_status for result in fetch_recorder.results]
    if fetch_statuses != ["missing", "pending", "pending"]:
        print(
            "verify_release self-test: registry fetch classification drifted",
            file=sys.stderr,
        )
        return 1
    print("verify_release self-test: OK")
    return 0


def write_summary(args: argparse.Namespace, recorder: Recorder, out_dir: Path) -> None:
    summary_json = out_dir / "release-verification-summary.json"
    summary_md = out_dir / "release-verification-summary.md"
    summary = {
        "version": args.version,
        "visual_hashing_version": args.visual_hashing_version,
        "repo": args.repo,
        "go_tag": args.go_tag,
        "capi_tag": args.capi_tag,
        "dry_run": args.dry_run,
        "include_wrapper_packages": args.include_wrapper_packages,
        "allow_legacy_release_gaps": args.allow_legacy_release_gaps,
        "release_status_counts": {
            status: sum(
                1 for result in recorder.results if result.release_status == status
            )
            for status in sorted({result.release_status for result in recorder.results})
        },
        "results": [asdict(result) for result in recorder.results],
    }
    summary_json.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    lines = [
        "# GTS Release Verification Summary",
        "",
        f"- Version: `{args.version}`",
        f"- Go tag: `{args.go_tag}`",
        f"- C ABI tag: `{args.capi_tag}`",
        f"- visual-hashing version: `{args.visual_hashing_version}`",
        f"- Repository: `{args.repo}`",
        f"- Dry run: `{str(args.dry_run).lower()}`",
        f"- Wrapper package checks: `{str(args.include_wrapper_packages).lower()}`",
        f"- Legacy gap override: `{str(args.allow_legacy_release_gaps).lower()}`",
        "",
        "| Surface | Check | Severity | Release status | Detail |",
        "|---|---|---|---|---|",
    ]
    for result in recorder.results:
        lines.append(
            "| "
            + " | ".join(
                [
                    markdown_escape(result.surface),
                    markdown_escape(result.check),
                    result.status,
                    result.release_status,
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
    parser.add_argument("--version", help="GTS package version, such as 0.9.0.")
    parser.add_argument(
        "--visual-hashing-version",
        help="visual-hashing crate version to verify for this release family.",
    )
    parser.add_argument(
        "--repo", default=DEFAULT_REPO, help="GitHub repository, owner/name."
    )
    parser.add_argument(
        "--repository-url",
        default=DEFAULT_REPO_URL,
        help="Repository URL expected by PyPI attestations.",
    )
    parser.add_argument("--pypi-project", default=DEFAULT_PYPI_PROJECT)
    parser.add_argument("--npm-package", default=DEFAULT_NPM_PACKAGE)
    parser.add_argument("--rust-crate", default=DEFAULT_RUST_CRATE)
    parser.add_argument("--visual-hashing-crate", default=DEFAULT_VISUAL_HASHING_CRATE)
    parser.add_argument("--capi-crate", default=DEFAULT_CAPI_CRATE)
    parser.add_argument("--nuget-package", default=DEFAULT_NUGET_PACKAGE)
    parser.add_argument("--packagist-package", default=DEFAULT_PACKAGIST_PACKAGE)
    parser.add_argument("--luarocks-package", default=DEFAULT_LUAROCKS_PACKAGE)
    parser.add_argument(
        "--luarocks-version", help="LuaRocks version. Defaults to <version>-1."
    )
    parser.add_argument("--rubygems-package", default=DEFAULT_RUBYGEMS_PACKAGE)
    parser.add_argument("--r-universe-owner", default=DEFAULT_RUNIVERSE_OWNER)
    parser.add_argument("--r-package", default=DEFAULT_R_PACKAGE)
    parser.add_argument("--julia-package", default=DEFAULT_JULIA_PACKAGE)
    parser.add_argument("--julia-uuid", default=DEFAULT_JULIA_UUID)
    parser.add_argument(
        "--swift-tag", help="Swift semantic version tag. Defaults to <version>."
    )
    parser.add_argument("--go-tag", help="Go release tag. Defaults to go-v<version>.")
    parser.add_argument(
        "--capi-tag", help="C ABI release tag. Defaults to capi-v<version>."
    )
    parser.add_argument(
        "--include-wrapper-packages",
        action="store_true",
        help="Verify wrapper ecosystem registry packages in addition to core release artifacts.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help=(
            "Write a deterministic planned-check report with pending registry "
            "statuses and no live registry, download, or attestation calls."
        ),
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run deterministic release verifier status-classification self-tests.",
    )
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
    if args.self_test:
        return args
    if not args.version:
        parser.error("--version is required unless --self-test is used")
    if not args.visual_hashing_version:
        parser.error("--visual-hashing-version is required unless --self-test is used")
    args.go_tag = args.go_tag or f"go-v{args.version}"
    args.capi_tag = args.capi_tag or f"capi-v{args.version}"
    args.out_dir = args.out_dir or Path("dist") / "release-verification" / args.version
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    if args.self_test:
        return run_self_test()
    recorder = Recorder()
    out_dir = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    if args.dry_run:
        dry_run_checks(args, recorder)
        write_summary(args, recorder, out_dir)
        return 0

    pypi_artifacts = verify_pypi(args, recorder, out_dir)
    npm_artifacts = verify_npm(args, recorder, out_dir)
    crate_artifacts = verify_crate(
        args, recorder, out_dir, args.rust_crate, args.version
    )
    crate_artifacts.extend(
        verify_crate(
            args,
            recorder,
            out_dir,
            args.visual_hashing_crate,
            args.visual_hashing_version,
        )
    )
    go_artifacts = verify_go(args, recorder, out_dir)
    capi_artifacts = verify_capi(args, recorder, out_dir)
    wrapper_artifacts: list[Path] = []
    if args.include_wrapper_packages:
        wrapper_artifacts = verify_wrapper_packages(args, recorder, out_dir)
    verify_github_attestations(
        args,
        recorder,
        artifact_attestation_plan(
            pypi_artifacts,
            npm_artifacts,
            crate_artifacts,
            go_artifacts,
            capi_artifacts,
            [
                artifact
                for artifact in wrapper_artifacts
                if artifact.suffix == ".gem" or artifact.name.endswith(".crate")
            ],
        ),
    )
    write_summary(args, recorder, out_dir)
    return 1 if recorder.has_failures() else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))

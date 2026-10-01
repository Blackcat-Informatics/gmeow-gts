#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Keep the first-party offer, package metadata and literal Mulan texts coherent."""

import argparse
import hashlib
import io
import json
import re
import stat
import subprocess
import tarfile
import tempfile
import unittest
import warnings
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path, PurePosixPath

import tomllib

ROOT = Path(__file__).resolve().parents[1]
GRANT = "MIT OR Apache-2.0 OR MulanPSL-2.0"
ARCHIVE_GRANT = f"({GRANT}) AND (MIT OR Apache-2.0)"
MULAN_SHA256 = "eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c"
LICENSE_NAMES = ("LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN")


def audit_recipient_archive(path: Path) -> dict[str, object]:
    """Inspect real packed files, including effective LuaRocks copy behavior."""
    expected = {name: (ROOT / name).read_bytes() for name in LICENSE_NAMES}
    found: dict[str, list[tuple[str, bytes]]] = {name: [] for name in LICENSE_NAMES}

    def retain(name: str, size: int, regular: bool, read) -> None:
        member = PurePosixPath(name)
        if member.name not in expected:
            return
        if member.is_absolute() or ".." in member.parts or not regular:
            raise ValueError(f"{path}: invalid recipient license member {name}")
        if size != len(expected[member.name]):
            raise ValueError(f"{path}: changed recipient license size: {name}")
        found[member.name].append((name, read()))

    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            for member in archive.infolist():
                retain(
                    member.filename,
                    member.file_size,
                    not member.is_dir()
                    and not stat.S_ISLNK(member.external_attr >> 16),
                    lambda member=member: archive.read(member),
                )
    else:
        with tarfile.open(path) as archive:
            for member in archive.getmembers():

                def read(member=member) -> bytes:
                    source = archive.extractfile(member)
                    if source is None:
                        raise ValueError(f"{path}: unreadable license {member.name}")
                    with source:
                        return source.read()

                retain(member.name, member.size, member.isfile(), read)
    parents = set()
    members = {}
    for name, copies in found.items():
        if len(copies) != 1:
            raise ValueError(f"{path}: expected one {name}, found {len(copies)}")
        member, content = copies[0]
        if content != expected[name]:
            raise ValueError(f"{path}: changed recipient license bytes: {member}")
        parents.add(str(PurePosixPath(member).parent))
        members[member] = hashlib.sha256(content).hexdigest()
    if len(parents) != 1:
        raise ValueError(
            f"{path}: recipient license texts are split across directories"
        )
    return {
        "archive": str(path),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "recipient_license_sha256": members,
    }


def self_test() -> None:
    class RecipientTests(unittest.TestCase):
        def check(self, changes, valid=False):
            original = [
                (f"licenses/{name}", (ROOT / name).read_bytes())
                for name in LICENSE_NAMES
            ]
            entries = changes(original)
            with tempfile.TemporaryDirectory(prefix="gts-license-archive-") as temp:
                for kind in ("zip", "tar.gz"):
                    with self.subTest(kind=kind):
                        path = Path(temp) / f"recipient.{kind}"
                        if kind == "zip":
                            with warnings.catch_warnings():
                                warnings.simplefilter("ignore", UserWarning)
                                with zipfile.ZipFile(path, "w") as archive:
                                    for name, content in entries:
                                        archive.writestr(name, content)
                        else:
                            with tarfile.open(path, "w:gz") as archive:
                                for name, content in entries:
                                    member = tarfile.TarInfo(name)
                                    member.size = len(content)
                                    archive.addfile(member, io.BytesIO(content))
                        if valid:
                            self.assertEqual(
                                len(
                                    audit_recipient_archive(path)[
                                        "recipient_license_sha256"
                                    ]
                                ),
                                3,
                            )
                        else:
                            with self.assertRaises(ValueError):
                                audit_recipient_archive(path)

        def test_exact_installed_notices(self):
            self.check(lambda entries: entries, valid=True)

        def test_go_module_notices(self):
            self.check(
                lambda entries: [
                    (
                        "go.blackcatinformatics.ca/gts@v1.0.0-rc.1/"
                        + PurePosixPath(name).name,
                        content,
                    )
                    for name, content in entries
                ],
                valid=True,
            )

        def test_empty_archive(self):
            self.check(lambda _: [])

        def test_missing_license(self):
            self.check(lambda entries: entries[:-1])

        def test_changed_same_length_text(self):
            self.check(
                lambda entries: (
                    entries[:-1] + [(entries[-1][0], b"X" + entries[-1][1][1:])]
                )
            )

        def test_placeholder(self):
            self.check(lambda entries: entries[:-1] + [(entries[-1][0], b"")])

        def test_duplicate_member(self):
            self.check(lambda entries: entries + entries[-1:])

        def test_split_directories(self):
            self.check(
                lambda entries: (
                    entries[:-1]
                    + [("other/" + PurePosixPath(entries[-1][0]).name, entries[-1][1])]
                )
            )

        def test_traversal_member(self):
            self.check(
                lambda entries: (
                    entries[:-1] + [("../" + entries[-1][0], entries[-1][1])]
                )
            )

    result = unittest.TextTestRunner(verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(RecipientTests)
    )
    if not result.wasSuccessful():
        raise SystemExit(1)


def main() -> None:
    problems = []
    texts = [ROOT / "LICENSES/MulanPSL-2.0.txt"]
    texts.extend(ROOT.glob("**/LICENSE-MULAN"))
    for path in texts:
        if (
            ".worktrees" in path.relative_to(ROOT).parts
            or ".git" in path.relative_to(ROOT).parts
        ):
            continue
        if hashlib.sha256(path.read_bytes()).hexdigest() != MULAN_SHA256:
            problems.append(
                f"{path.relative_to(ROOT)}: changed controlling bilingual license"
            )
    for relative in ("rust/Cargo.toml", "rust/capi/Cargo.toml"):
        data = tomllib.loads((ROOT / relative).read_text())
        if data["package"]["license"] != ARCHIVE_GRANT:
            problems.append(
                f"{relative}: code and retained README grants must both be declared"
            )
    python = tomllib.loads((ROOT / "python/pyproject.toml").read_text())["project"]
    if python["license"] != ARCHIVE_GRANT:
        problems.append(
            "python/pyproject.toml: code and retained README grants must both be declared"
        )
    for relative in ("ts/package.json", "packaging/vcpkg/ports/gmeow-gts/vcpkg.json"):
        if json.loads((ROOT / relative).read_text())["license"] != GRANT:
            problems.append(f"{relative}: wrong first-party grant")
    if set(json.loads((ROOT / "php/composer.json").read_text())["license"]) != {
        "MIT",
        "Apache-2.0",
        "MulanPSL-2.0",
    }:
        problems.append("php/composer.json: wrong first-party license choices")
    dotnet = ET.parse(ROOT / "dotnet/Gmeow.Gts/Gmeow.Gts.csproj").getroot()
    if dotnet.findtext(".//PackageLicenseExpression") != GRANT:
        problems.append("dotnet/Gmeow.Gts/Gmeow.Gts.csproj: wrong first-party grant")
    for relative, declaration in (
        ("conanfile.py", f'license = "{GRANT}"'),
        (
            "ruby/gmeow-gts.gemspec",
            'spec.licenses = ["MIT", "Apache-2.0", "MulanPSL-2.0"]',
        ),
        ("lua/gmeow-gts-dev-1.rockspec", f'license = "{GRANT}"'),
        ("lua/gmeow-gts-1.0.0rc1-1.rockspec", f'license = "{GRANT}"'),
        (
            "r/DESCRIPTION",
            "License: MIT + file LICENSE | Apache License (>= 2) | file LICENSE-MULAN",
        ),
    ):
        if declaration not in (ROOT / relative).read_text():
            problems.append(f"{relative}: wrong wrapper license declaration")
    paths = (
        subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT)
        .decode()
        .split("\0")
    )
    for relative in paths:
        path = ROOT / relative
        if (
            not path.is_file()
            or path.suffix.lower() in (".md", ".json", ".lock", ".svg", ".rd")
            or relative.startswith(("LICENSE", "vectors/", "ietf/", "dist/"))
        ):
            continue
        try:
            head = "\n".join(path.read_text().splitlines()[:10])
        except UnicodeDecodeError:
            continue
        declared = re.search(r"SPDX-License-Identifier:\s*([^\n]+)", head)
        if "Blackcat Informatics" in head and declared:
            expression = (
                declared[1]
                .strip()
                .removesuffix("-->")
                .removesuffix("*/")
                .removesuffix('"')
                .strip()
            )
            if expression != GRANT:
                problems.append(f"{relative}: stale source grant {expression}")
    for relative in (
        "docs/GTS-SPEC.md",
        "docs/i18n/fr-CA/docs/GTS-SPEC.md",
        "docs/i18n/zh-Hans/docs/GTS-SPEC.md",
    ):
        if ("SPDX-License-" + f"Identifier: {GRANT}") not in "\n".join(
            (ROOT / relative).read_text().splitlines()[:10]
        ):
            problems.append(f"{relative}: spec grant is not aligned")
    for directory in (
        "rust",
        "rust/capi",
        "python",
        "ts",
        "go",
        "ruby",
        "julia",
        "r",
        "lua/licenses",
    ):
        for name in LICENSE_NAMES:
            path = ROOT / directory / name
            if (
                path.is_symlink()
                or not path.is_file()
                or path.read_bytes() != (ROOT / name).read_bytes()
            ):
                problems.append(f"{directory}/{name}: missing/changed recipient text")
    if problems:
        raise SystemExit("license coherence failed:\n" + "\n".join(problems))
    print(
        "license coherence: first-party metadata/headers and controlling Mulan texts verified; documentation/third-party grants preserved"
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--self-test", action="store_true")
    mode.add_argument("--recipient-archive", type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
    elif args.recipient_archive:
        print(json.dumps(audit_recipient_archive(args.recipient_archive), indent=2))
    else:
        self_test()
        main()

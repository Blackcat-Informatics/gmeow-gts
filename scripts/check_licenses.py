#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Keep the first-party offer, package metadata and literal Mulan texts coherent."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
GRANT = "MIT OR Apache-2.0 OR MulanPSL-2.0"
MULAN_SHA256 = "eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c"


def main() -> None:
    problems = []
    texts = [ROOT / "LICENSES/MulanPSL-2.0.txt"]
    texts.extend(ROOT.glob("**/LICENSE-MULAN"))
    for path in texts:
        if ".worktrees" in path.relative_to(ROOT).parts or ".git" in path.relative_to(ROOT).parts:
            continue
        if hashlib.sha256(path.read_bytes()).hexdigest() != MULAN_SHA256:
            problems.append(f"{path.relative_to(ROOT)}: changed controlling bilingual license")
    for relative in ("rust/Cargo.toml", "rust/capi/Cargo.toml"):
        data = tomllib.loads((ROOT / relative).read_text())
        if data["package"]["license"] != GRANT:
            problems.append(f"{relative}: wrong first-party grant")
    python = tomllib.loads((ROOT / "python/pyproject.toml").read_text())["project"]
    if python["license"] != f"({GRANT}) AND (MIT OR Apache-2.0)":
        problems.append("python/pyproject.toml: code and retained README grants must both be declared")
    for relative in ("ts/package.json", "packaging/vcpkg/ports/gmeow-gts/vcpkg.json"):
        if json.loads((ROOT / relative).read_text())["license"] != GRANT:
            problems.append(f"{relative}: wrong first-party grant")
    if set(json.loads((ROOT / "php/composer.json").read_text())["license"]) != {"MIT", "Apache-2.0", "MulanPSL-2.0"}:
        problems.append("php/composer.json: wrong first-party license choices")
    dotnet = ET.parse(ROOT / "dotnet/Gmeow.Gts/Gmeow.Gts.csproj").getroot()
    if dotnet.findtext(".//PackageLicenseExpression") != GRANT:
        problems.append("dotnet/Gmeow.Gts/Gmeow.Gts.csproj: wrong first-party grant")
    for relative, declaration in (
        ("conanfile.py", f'license = "{GRANT}"'),
        ("ruby/gmeow-gts.gemspec", 'spec.licenses = ["MIT", "Apache-2.0", "MulanPSL-2.0"]'),
        ("lua/gmeow-gts-dev-1.rockspec", f'license = "{GRANT}"'),
        ("lua/gmeow-gts-1.0.0rc1-1.rockspec", f'license = "{GRANT}"'),
        ("r/DESCRIPTION", "License: MIT + file LICENSE | Apache License (>= 2) | file LICENSE-MULAN"),
    ):
        if declaration not in (ROOT / relative).read_text():
            problems.append(f"{relative}: wrong wrapper license declaration")
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=ROOT).decode().split("\0")
    for relative in paths:
        path = ROOT / relative
        if not path.is_file() or path.suffix.lower() in (".md", ".json", ".lock", ".svg", ".rd") or relative.startswith(("LICENSE", "vectors/", "ietf/", "dist/")):
            continue
        try:
            head = "\n".join(path.read_text().splitlines()[:10])
        except UnicodeDecodeError:
            continue
        declared = re.search(r"SPDX-License-Identifier:\s*([^\n]+)", head)
        if "Blackcat Informatics" in head and declared:
            expression = declared[1].strip().removesuffix("-->").removesuffix("*/").removesuffix('"').strip()
            if expression != GRANT:
                problems.append(f"{relative}: stale source grant {expression}")
    for relative in ("docs/GTS-SPEC.md", "docs/i18n/fr-CA/docs/GTS-SPEC.md", "docs/i18n/zh-Hans/docs/GTS-SPEC.md"):
        if ("SPDX-License-" + f"Identifier: {GRANT}") not in "\n".join((ROOT / relative).read_text().splitlines()[:10]):
            problems.append(f"{relative}: spec grant is not aligned")
    for directory in ("rust", "rust/capi", "python", "ts", "ruby", "julia", "r", "lua/licenses"):
        for name in ("LICENSE-MIT", "LICENSE-APACHE", "LICENSE-MULAN"):
            path = ROOT / directory / name
            if not path.is_file() or path.read_bytes() != (ROOT / name).read_bytes():
                problems.append(f"{directory}/{name}: missing/changed recipient text")
    if problems:
        raise SystemExit("license coherence failed:\n" + "\n".join(problems))
    print("license coherence: first-party metadata/headers and controlling Mulan texts verified; documentation/third-party grants preserved")


if __name__ == "__main__":
    main()

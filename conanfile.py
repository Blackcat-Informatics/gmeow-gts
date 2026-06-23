# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

import json
import os
import re
import subprocess
from pathlib import Path

from conan import ConanFile
from conan.tools.files import copy, save


class GmeowGtsConan(ConanFile):
    name = "gmeow-gts"
    description = "Graph Transport Substrate C and C++ ABI package"
    license = "MIT OR Apache-2.0"
    homepage = "https://blackcatinformatics.ca/projects/gts"
    url = "https://github.com/Blackcat-Informatics/gmeow-gts"
    package_type = "library"
    settings = "os", "arch", "compiler", "build_type"
    options = {"shared": [True, False]}
    default_options = {"shared": True}
    _metadata = None
    exports_sources = (
        "cpp/include/**",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "LICENSES/**",
        "LICENSING.md",
        "README.md",
        "rust/Cargo.lock",
        "rust/Cargo.toml",
        "rust/capi/Cargo.lock",
        "rust/capi/Cargo.toml",
        "rust/capi/LICENSE-APACHE",
        "rust/capi/LICENSE-MIT",
        "rust/capi/README.md",
        "rust/capi/cmake/**",
        "rust/capi/examples/**",
        "rust/capi/gts.pc.in",
        "rust/capi/include/**",
        "rust/capi/src/**",
        "rust/src/**",
    )

    def build(self):
        cmd = "cargo build --manifest-path rust/capi/Cargo.toml --locked"
        if self._cargo_profile() == "release":
            cmd += " --release"
        self.run(cmd)

    def package(self):
        source = Path(self.source_folder)
        package = Path(self.package_folder)
        target = self._target_directory() / self._cargo_profile()
        version = self._metadata_value("version")
        abi_version = self._abi_version(source / "rust/capi/include/gts.h")

        copy(self, "gts.h", src=str(source / "rust/capi/include"), dst=str(package / "include"), keep_path=False)
        copy(self, "gts.hpp", src=str(source / "cpp/include/gts"), dst=str(package / "include/gts"), keep_path=False)
        copy(self, "GtsConfig.cmake", src=str(source / "rust/capi/cmake"), dst=str(package / "lib/cmake/Gts"), keep_path=False)
        copy(self, "README.md", src=str(source / "rust/capi"), dst=str(package), keep_path=False)
        copy(self, "LICENSE-MIT", src=str(source), dst=str(package / "licenses"), keep_path=False)
        copy(self, "LICENSE-APACHE", src=str(source), dst=str(package / "licenses"), keep_path=False)
        copy(self, "LICENSING.md", src=str(source), dst=str(package / "licenses"), keep_path=False)
        copy(self, "*", src=str(source / "LICENSES"), dst=str(package / "licenses/LICENSES"), keep_path=True)

        for pattern in ("libgts.so", "libgts.dylib", "libgts.a", "gts.lib", "gts.dll.lib"):
            copy(self, pattern, src=str(target), dst=str(package / "lib"), keep_path=False)
        for pattern in ("gts.dll", "gts.pdb"):
            copy(self, pattern, src=str(target), dst=str(package / "bin"), keep_path=False)

        save(self, str(package / "lib/pkgconfig/gts.pc"), self._pkg_config(version))
        save(self, str(package / "share/gts/VERSION"), f"{version}\n")
        save(self, str(package / "share/gts/ABI_VERSION"), f"{abi_version}\n")
        save(
            self,
            str(package / "share/gts/archive.json"),
            json.dumps(
                {
                    "schema": "gts-capi-archive-v1",
                    "package": "gmeow-gts",
                    "version": version,
                    "abi_version": int(abi_version),
                    "manager": "conan",
                },
                indent=2,
            )
            + "\n",
        )

    def package_info(self):
        self.cpp_info.libs = ["gts"]
        self.cpp_info.includedirs = ["include"]
        self.cpp_info.libdirs = ["lib"]
        self.cpp_info.bindirs = ["bin"]
        self.cpp_info.builddirs = [os.path.join("lib", "cmake", "Gts")]
        self.cpp_info.set_property("cmake_file_name", "Gts")
        self.cpp_info.set_property("cmake_target_name", "Gts::gts")
        self.cpp_info.set_property("pkg_config_name", "gts")

    def _target_directory(self):
        metadata = self._cargo_metadata()
        return Path(metadata["target_directory"])

    def _metadata_value(self, key):
        metadata = self._cargo_metadata()
        return metadata["packages"][0][key]

    def _cargo_profile(self):
        return "release" if str(self.settings.build_type) == "Release" else "debug"

    def _cargo_metadata(self):
        if self._metadata is None:
            output = subprocess.check_output(
                [
                    "cargo",
                    "metadata",
                    "--manifest-path",
                    "rust/capi/Cargo.toml",
                    "--no-deps",
                    "--format-version",
                    "1",
                ],
                cwd=self.source_folder,
                text=True,
            )
            self._metadata = json.loads(output)
        return self._metadata

    @staticmethod
    def _abi_version(header):
        for line in header.read_text(encoding="utf-8").splitlines():
            match = re.match(r"#define\s+GTS_ABI_VERSION\s+([0-9]+)", line)
            if match:
                return match.group(1)
        raise ValueError(f"could not determine GTS_ABI_VERSION from {header}")

    @staticmethod
    def _pkg_config(version):
        return f"""prefix=${{pcfiledir}}/../..
exec_prefix=${{prefix}}
libdir=${{exec_prefix}}/lib
includedir=${{prefix}}/include

Name: gts
Description: Graph Transport Substrate Rust C ABI
Version: {version}
Libs: -L${{libdir}} -lgts
Cflags: -I${{includedir}}
"""

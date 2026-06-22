<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Native C/C++ Packaging

The first-party C/C++ package name is `gmeow-gts`. The package exposes the
Rust-backed `libgts` C ABI, `gts.h`, the header-only C++ wrapper `gts/gts.hpp`,
pkg-config metadata, and the `Gts::gts` CMake target. It is not an independent
C++ engine.

Run both local package-manager checks from the repository root:

```bash
bash scripts/package_dry_run_native_managers.sh
```

The script runs:

- `conan create . --version <rust/capi version>`;
- a Conan CMake consumer using `find_package(Gts REQUIRED)`;
- a vcpkg overlay install from `packaging/vcpkg/ports`;
- a vcpkg CMake consumer using the same `Gts::gts` target.

## Conan

`conanfile.py` builds `rust/capi` with Cargo and packages the release `libgts`
artifacts plus headers, CMake config, pkg-config metadata, license files,
`README.md`, and `share/gts` version metadata.

For a direct local run:

```bash
version="$(cargo metadata --manifest-path rust/capi/Cargo.toml --no-deps --format-version 1 \
  | python3 -c "import json,sys; print(json.load(sys.stdin)['packages'][0]['version'])")"
conan profile detect --force
conan create . --version "${version}" --build=missing --settings=build_type=Release
```

ConanCenter submission should use the `gmeow-gts` name and preserve the same
package layout. Registry review may require recipe reshaping into
`conan-center-index` conventions, but the local package contents and consumer
contract should stay the same.

## vcpkg

The overlay port validates the local source tree. Set `GMEOW_GTS_SOURCE_PATH`
to the checkout being packaged:

```bash
GMEOW_GTS_SOURCE_PATH="$PWD" \
  vcpkg install gmeow-gts \
    --overlay-ports="$PWD/packaging/vcpkg/ports" \
    --triplet=x64-linux-dynamic
```

The current overlay validates the dynamic `libgts` layout because the CMake
config and C++ wrapper consumer use the shared ABI library. Upstream vcpkg
submission should replace the local `GMEOW_GTS_SOURCE_PATH` source hook with the
tagged `gmeow-gts` release source and the corresponding checksum.

## ABI And Runtime Loading

The package version tracks the Rust/C ABI crate version. Installed packages
write the runtime package version to `share/gts/VERSION` and the ABI version to
`share/gts/ABI_VERSION`. C and C++ consumers should compile against the shipped
headers and can compare `gts_abi_version()` or `gts::abi_version()` with
`GTS_ABI_VERSION` at startup.

Runtime library discovery remains platform-specific. For local package-manager
tests, set `LD_LIBRARY_PATH` on Linux, `DYLD_LIBRARY_PATH` on macOS, or add
`bin/` to `PATH` on Windows unless the package manager or consuming build
system installs `libgts` into a default loader path.

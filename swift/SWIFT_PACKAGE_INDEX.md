# Swift Package Manager Publication

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

This document records the Swift Package Manager and Swift Package Index release
path for the `GmeowGTS` C ABI wrapper.

## Publication Layout

The public Swift package is this repository:

```text
https://github.com/Blackcat-Informatics/gmeow-gts.git
```

SwiftPM and Swift Package Index require a valid `Package.swift` in the
repository root. The root manifest exposes the existing source tree:

- `swift/Sources/CGts/module.modulemap`
- `swift/Sources/CGts/include/gts.h`
- `swift/Sources/GmeowGTS/GmeowGTS.swift`
- `swift/Tests/GmeowGTSSmoke/main.swift`

The subdirectory manifest at `swift/Package.swift` remains available for local
development, but public versioned consumers should use the repository root URL.

## Runtime Requirements

The Swift package is source-only and links against an externally installed
`libgts`. It does not bundle native `libgts` archives.

The `CGts` system-library target uses:

```text
link "gts"
```

Consumers must make the shared library visible to the linker and dynamic
loader. From this repository, the validation path is:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
LIBRARY_PATH="$PWD/rust/capi/target/debug" \
LD_LIBRARY_PATH="$PWD/rust/capi/target/debug" \
swift run --package-path . \
  -Xlinker -L"$PWD/rust/capi/target/debug" \
  -Xlinker -rpath \
  -Xlinker "$PWD/rust/capi/target/debug" \
  GmeowGTSSmoke \
  vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts
```

On macOS, use `DYLD_LIBRARY_PATH` instead of `LD_LIBRARY_PATH`, and ensure the
shared library is available as `libgts.dylib`. On Linux, ensure it is available
as `libgts.so`.

## Consumer Dependency

After the first Swift publication tag exists, a consumer package can declare:

```swift
dependencies: [
    .package(
        url: "https://github.com/Blackcat-Informatics/gmeow-gts.git",
        from: "0.9.4"
    )
]
```

Targets that use the wrapper should depend on:

```swift
.product(name: "GmeowGTS", package: "gmeow-gts")
```

## Release Sequence

Swift Package Index requirements were checked on 2026-06-22 at
<https://swiftpackageindex.com/add-a-package>. The package must be public, have
a valid root `Package.swift`, use Swift 5.0 or later, have a semantic-version
release tag, produce valid `swift package dump-package` JSON with the latest
Swift toolchain, include protocol and `.git` in the submitted URL, and compile
without errors.

For the first Swift publication:

1. Merge the PR that adds the root `Package.swift`.
2. Tag the merge commit with the plain semantic version `0.9.4`.
3. Push the tag:

   ```sh
   git tag 0.9.4 <merge-commit>
   git push origin 0.9.4
   ```

4. Validate from the tag:

   ```sh
   git checkout 0.9.4
   diff -u rust/capi/include/gts.h swift/Sources/CGts/include/gts.h
   swift package dump-package --package-path .
   bash swift/scripts/smoke.sh
   ```

5. Submit this URL to Swift Package Index:

   ```text
   https://github.com/Blackcat-Informatics/gmeow-gts.git
   ```

6. After indexing, verify the package page reports the expected Swift and
   platform compatibility. Linux and macOS are expected; other Apple platforms
   depend on whether consumers provide a usable `libgts` for that platform.

# GTS Swift C ABI Wrapper

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

This package is a thin Swift Package Manager wrapper over the Rust-backed
`libgts` C ABI. It does not parse, fold, write, or verify GTS archives in
Swift; all GTS semantics come from the C ABI in `rust/capi/include/gts.h`.

## Requirements

- Swift 6.0 or newer.
- A built `libgts` shared library from `rust/capi`.

## Swift Package Manager

Public Swift Package Manager consumption uses the repository root
`Package.swift`, which exposes the existing sources under `swift/`. The
subdirectory manifest at `swift/Package.swift` remains for local validation.

After the first Swift publication tag is pushed, consumers can depend on this
repository with:

```swift
.package(
    url: "https://github.com/Blackcat-Informatics/gmeow-gts.git",
    from: "0.9.4"
)
```

and add the library product:

```swift
.product(name: "GmeowGTS", package: "gmeow-gts")
```

The wrapper is source-only. Consumers must install or otherwise expose `libgts`
separately; no `libgts` binaries are bundled in the Swift package.

Build the shared library from the repository root:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Then run the smoke executable with the library path available to the linker and
dynamic loader:

```sh
LIBRARY_PATH="$PWD/rust/capi/target/debug" \
LD_LIBRARY_PATH="$PWD/rust/capi/target/debug" \
swift run --package-path . \
  -Xlinker -L"$PWD/rust/capi/target/debug" \
  -Xlinker -rpath \
  -Xlinker "$PWD/rust/capi/target/debug" \
  GmeowGTSSmoke \
  vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts
```

On macOS use `DYLD_LIBRARY_PATH` instead of `LD_LIBRARY_PATH`. The package links
against `-lgts`, so the shared library must be named `libgts.so` on Linux or
`libgts.dylib` on macOS.

See [`SWIFT_PACKAGE_INDEX.md`](./SWIFT_PACKAGE_INDEX.md) for the maintainer
tagging and Swift Package Index submission process.

## API Shape

```swift
import Foundation
import GmeowGTS

let data = try Data(contentsOf: URL(fileURLWithPath: "vectors/01-minimal.gts"))

let metadata = try GTS.buildMetadataJSON()
let capabilities = try GTS.capabilitiesJSON()
let folded = try GTS.readJSON(data)
let verified = try GTS.verifyJSON(data)
let nquads = try GTS.toNQuads(data)
let roundTrip = try GTS.fromNQuads(nquads)
```

Files-profile helpers use `Data` for binary GTS payloads and ordinary Swift
`String` paths:

```swift
let packed = try GTS.filesPack(paths: ["/path/to/tree"])
let diff = try GTS.filesDiffJSON(packed, directory: "/path/to/tree")
let report = try GTS.filesUnpack(packed, to: "/tmp/unpacked")
```

Unpack policy flags are exposed as a Swift `OptionSet`:

```swift
let report = try GTS.filesUnpack(
    packed,
    to: "/tmp/unpacked",
    flags: [.includeSuppressed, .allowSymlinks]
)
```

## Ownership And Errors

The wrapper copies every returned `gts_buffer` into Swift-owned `Data` or
`String` values and then calls `gts_buffer_free`. If a C ABI call returns an
error handle, the wrapper copies the stable error code and message, releases the
handle with `gts_error_free`, and throws `GtsError`.

Callers never receive raw C pointers, buffer capacities, or Rust-owned memory.
`Data` is used for binary GTS byte streams, and `String` is used for UTF-8 JSON
and N-Quads text.

`GtsError` exposes:

- `operation`: C ABI operation name.
- `status`: stable C ABI status as `GtsStatus`.
- `code`: stable C ABI error code.
- `detail`: human-readable diagnostic detail.

## Threading And ABI Stability

`libgts` operations are reentrant. Each call owns its output buffer and error
handle independently, and the Swift wrapper frees them before returning or
throwing. Do not retain raw C pointers from the `CGts` module.

The wrapper targets `GTS_ABI_VERSION` 1. Check `GTS.abiVersion` and
`GTS.capabilitiesJSON()` when linking against a system-provided `libgts`.

## Validation

Run the smoke test from the repository root:

```sh
bash swift/scripts/smoke.sh
```

The script verifies that the Swift header mirror matches
`rust/capi/include/gts.h`, builds `libgts`, and exercises ABI metadata,
capabilities, read/fold, verify, N-Quads export/import, structured errors, and
files-profile pack/diff/unpack. If local Swift is missing on Linux, it uses the
pinned official Swift Docker image configured in `swift/scripts/smoke.sh`.

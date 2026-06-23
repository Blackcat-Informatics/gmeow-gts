<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS C ABI

`gmeow-gts-capi` builds `libgts` from the safe Rust GTS core. It is a distribution and interoperability surface for C-compatible runtimes; it is not a new GTS engine or a new CLI parity column.

## ABI model

- All byte inputs use pointer plus length.
- Path lists use NUL-terminated UTF-8 C strings because they are passed to path-based filesystem APIs.
- Rust graph structs are never exposed. Graph-shaped results are returned as stable JSON reports.
- Output buffers are returned as `gts_buffer` and must be released with `gts_buffer_free`.
- Errors are returned as opaque `gts_error *` handles and must be released with `gts_error_free`.
- Functions return `gts_status`; `GTS_STATUS_OK` means the output buffer is initialized.
- The ABI boundary catches Rust panics and reports `GTS_STATUS_PANIC`.
- Functions are reentrant. Returned buffers and errors are caller-owned and must not be shared mutably across threads.

Do not edit `gts_buffer.capacity`; it is part of the Rust allocation handle used by `gts_buffer_free`.

## Operations

- `gts_build_metadata_json`: ABI version, package version, build profile, and target metadata.
- `gts_capabilities_json`: ABI version, library version, and operation discovery.
- `gts_formats_json`: registry of supported RDF text codec ids, aliases, extensions, and media types.
- `gts_read_json`: fold/read report with counts, diagnostics, segment heads, signatures, streamable state, and blob summaries.
- `gts_verify_json`: verification report mirroring the Rust verifier result.
- `gts_to_format`: clean GTS bytes to a registered RDF text format.
- `gts_from_format`: registered RDF text format input to GTS bytes.
- `gts_to_nquads`: clean GTS bytes to N-Quads text.
- `gts_from_nquads`: N-Quads text to GTS bytes.
- `gts_files_pack`: path list to files-profile GTS bytes.
- `gts_files_unpack`: files-profile GTS bytes to a destination directory.
- `gts_files_diff_json`: files-profile GTS bytes compared to a directory.

The format registry covers N-Quads, N-Triples, Turtle, TriG, RDF/XML, and the
repository's deterministic JSON-LD-star profile (`application/ld+json`). It
accepts registry ids, common extensions, and media types with parameters, such
as `text/turtle; charset=utf-8`.

## Build

```sh
cargo build --manifest-path rust/capi/Cargo.toml --release
```

From the packaged crate root, use `cargo build --release`. The crate is
published as [`gmeow-gts-capi`](https://crates.io/crates/gmeow-gts-capi) and
depends on the matching `gmeow-gts` crate version.

The crate emits both shared and static native libraries where supported by the target:

- `libgts.so` / `libgts.dylib` / `gts.dll`
- `libgts.a` / `gts.lib`

The public header is checked in at `rust/capi/include/gts.h`. `gts.pc.in` and `cmake/GtsConfig.cmake` provide pkg-config and CMake integration metadata for packaging.

## Smoke Test

```sh
bash rust/capi/scripts/smoke.sh
```

The smoke test builds `libgts`, compiles `examples/smoke.c`, and exercises the
shared wrapper smoke matrix plus capability and format discovery,
registry-driven RDF format export/import, N-Quads compatibility export/import,
files pack/unpack/diff, and structured error handling.

## Distribution Archives

The first wrapper publication wave keeps language packages source-only. Native
`libgts` binaries are distributed separately through the C ABI archive lane:

```sh
archive="$(bash rust/capi/scripts/package.sh)"
bash rust/capi/scripts/verify-archive.sh "${archive}"
```

Each archive has a relocatable install layout:

```text
include/gts.h
include/gts/gts.hpp
lib/libgts.so | lib/libgts.dylib | bin/gts.dll
lib/libgts.a | lib/gts.lib             # when produced by the target
lib/pkgconfig/gts.pc
lib/cmake/Gts/GtsConfig.cmake
share/gts/VERSION
share/gts/ABI_VERSION
share/gts/archive.json
README.md
licenses/
```

Consumers can point `PKG_CONFIG_PATH` at `lib/pkgconfig` or set
`CMAKE_PREFIX_PATH` to the unpacked archive root and link `Gts::gts`. Runtime
library discovery remains platform-specific: set `LD_LIBRARY_PATH` on Linux,
`DYLD_LIBRARY_PATH` on macOS, or add `bin/` to `PATH` on Windows unless the
library is installed into a platform default search path.

The release tag lane is `capi-v*`. A tag such as `capi-v0.9.5` publishes the
`gmeow-gts-capi` source crate, builds C ABI archives, checksums, SBOM evidence,
and GitHub provenance attestations, then publishes an immutable GitHub Release.
Wrapper packages should depend on this native asset contract instead of
bundling `libgts`.

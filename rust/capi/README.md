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

- `gts_capabilities_json`: ABI version, library version, and operation discovery.
- `gts_read_json`: fold/read report with counts, diagnostics, segment heads, signatures, streamable state, and blob summaries.
- `gts_verify_json`: verification report mirroring the Rust verifier result.
- `gts_to_nquads`: clean GTS bytes to N-Quads text.
- `gts_from_nquads`: N-Quads text to GTS bytes.
- `gts_files_pack`: path list to files-profile GTS bytes.
- `gts_files_unpack`: files-profile GTS bytes to a destination directory.
- `gts_files_diff_json`: files-profile GTS bytes compared to a directory.

## Build

```sh
cargo build --manifest-path rust/capi/Cargo.toml --release
```

The crate emits both shared and static native libraries where supported by the target:

- `libgts.so` / `libgts.dylib` / `gts.dll`
- `libgts.a` / `gts.lib`

The public header is checked in at `rust/capi/include/gts.h`. `gts.pc.in` and `cmake/GtsConfig.cmake` provide pkg-config and CMake integration metadata for packaging.

## Smoke Test

```sh
bash rust/capi/scripts/smoke.sh
```

The smoke test builds `libgts`, compiles `examples/smoke.c`, and exercises read/fold JSON, verify JSON, N-Quads export/import, files pack/unpack/diff, and structured error handling.

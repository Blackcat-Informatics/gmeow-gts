<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS C++ Wrapper

The C++ wrapper is a header-only RAII layer over the Rust-backed `libgts` C ABI.
It consumes `rust/capi/include/gts.h` and links to `libgts`; it is not an
independent GTS engine and does not add a CLI parity column.

## Layout

- `include/gts/gts.hpp`: public C++17 wrapper.
- `examples/smoke.cpp`: smoke test covering the supported wrapper surface.
- `scripts/smoke.sh`: builds `libgts`, compiles the C++ smoke test, and runs it.

## Build And Link

Build the C ABI library first:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Compile consumers with both wrapper include roots and link to `libgts`:

```sh
c++ -std=c++17 \
  -Icpp/include \
  -Irust/capi/include \
  your_program.cpp \
  -Lrust/capi/target/debug -lgts
```

When running from an unpackaged checkout, make sure the dynamic loader can find
the built library:

```sh
LD_LIBRARY_PATH="$PWD/rust/capi/target/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ./your_program
```

Packagers can use the C ABI `pkg-config` and CMake metadata in `rust/capi/`.

## API Model

`gts/gts.hpp` exposes:

- ABI/version/build metadata;
- capabilities JSON;
- read/fold JSON;
- verify JSON;
- GTS to N-Quads;
- N-Quads to GTS bytes;
- files-profile pack/unpack/diff helpers;
- structured `gts::Error` exceptions for C ABI failures.

Public APIs return `std::string` for textual JSON/N-Quads results and
`std::vector<std::uint8_t>` for GTS bytes. Callers do not manipulate
`gts_buffer`, `gts_error`, raw capacity fields, or Rust-owned allocations.

## Ownership And Threading

The wrapper copies successful C ABI outputs into C++ values, then releases the
underlying `gts_buffer` with `gts_buffer_free`. Failed calls copy the C ABI
error code/message into `gts::Error`, then release the `gts_error` with
`gts_error_free`.

The underlying C ABI functions are reentrant. Returned C++ values are owned by
the caller and can be used according to normal C++ object rules.

## ABI Stability

The wrapper checks the same ABI surface declared by `gts.h`. `gts::abi_version()`
returns the runtime ABI version, and `GTS_ABI_VERSION` remains available from the
C header for compile-time checks.

## Smoke Test

```sh
bash cpp/scripts/smoke.sh
```

The smoke test builds `libgts`, compiles `examples/smoke.cpp`, and exercises
the shared wrapper smoke matrix: build metadata, capabilities, clean and
diagnostic read/fold JSON, verify JSON, N-Quads export/import, files
pack/unpack/diff, and structured error handling.

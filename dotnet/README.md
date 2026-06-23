<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS .NET Wrapper

`Gmeow.Gts` is a thin .NET P/Invoke wrapper over the Rust-backed `libgts` C ABI.
It consumes `rust/capi/include/gts.h` and links or loads `libgts`; it is not an
independent GTS engine and does not add a CLI parity column.

## Layout

- `Gmeow.Gts/`: NuGet-ready `net8.0` library.
- `Gmeow.Gts.Smoke/`: smoke executable for local and CI validation.
- `scripts/smoke.sh`: builds `libgts`, then runs the smoke executable.

## Build And Load

Build the C ABI library first:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Then build or run the .NET wrapper:

```sh
dotnet build dotnet/Gmeow.Gts/Gmeow.Gts.csproj
LD_LIBRARY_PATH="$PWD/rust/capi/target/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" \
  dotnet run --project dotnet/Gmeow.Gts.Smoke/Gmeow.Gts.Smoke.csproj -- \
  vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts
```

On Linux, P/Invoke resolves `gts` to `libgts.so`; on macOS it resolves
`libgts.dylib`; on Windows it resolves `gts.dll`. Packagers should ship the
native library next to the managed assembly or make it discoverable through the
platform dynamic-loader path.

## API Model

`Gmeow.Gts.Gts` exposes:

- ABI/version/build metadata;
- capabilities JSON;
- read/fold JSON;
- verify JSON;
- GTS to N-Quads;
- N-Quads to GTS bytes;
- files-profile pack/unpack/diff helpers;
- structured `GtsException` failures.

Public APIs return `string` for textual JSON/N-Quads results and `byte[]` for
GTS bytes. Callers do not manipulate `gts_buffer`, `gts_error`, raw capacity
fields, or Rust-owned allocations.

## Ownership And Threading

The wrapper copies successful C ABI outputs into managed .NET values, then
releases `gts_buffer` with `gts_buffer_free`. Failed calls copy the C ABI error
code/message into `GtsException`, then release `gts_error` with
`gts_error_free`.

The underlying C ABI functions are reentrant. Managed strings and byte arrays
returned by the wrapper are caller-owned and can be used according to normal .NET
object rules.

## ABI Stability

`Gts.AbiVersion` returns the runtime ABI version. The wrapper is scoped to the
stable `GTS_ABI_VERSION == 1` surface declared by `gts.h`.

## Smoke Test

```sh
bash dotnet/scripts/smoke.sh
```

The smoke test builds `libgts`, compiles the .NET projects, and exercises build
metadata, capabilities, read/fold JSON, verify JSON, N-Quads export/import,
files pack/unpack/diff, and structured error handling. If the host does not have
`dotnet`, the script uses the official .NET SDK container.

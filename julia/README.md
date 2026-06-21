# GmeowGTS.jl

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

`GmeowGTS.jl` is a thin Julia package over the Rust-backed `libgts` C ABI. It
does not implement GTS parsing, folding, writing, or verification in Julia; all
format semantics come from the checked-in C ABI in `rust/capi/include/gts.h`.

## Requirements

- Julia 1.10 or newer.
- A built `libgts` shared library from `rust/capi`.

Build the shared library from the repository root:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Run the smoke test against that library:

```sh
export GTS_LIBGTS="$PWD/rust/capi/target/debug/libgts.so"
export LD_LIBRARY_PATH="$PWD/rust/capi/target/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
GTS_JULIA_VECTOR="$PWD/vectors/01-minimal.gts" julia --project=julia -e 'using Pkg; Pkg.test()'
```

On macOS use `DYLD_LIBRARY_PATH` and `libgts.dylib`. On Windows make `gts.dll`
discoverable through `PATH` or set `GTS_LIBGTS` to the DLL path.

## API Shape

```julia
using GmeowGTS

input = read("vectors/01-minimal.gts")

metadata = build_metadata_json()
capabilities = capabilities_json()
folded = read_json(input)
verified = verify_json(input)
nquads = to_nquads(input)
round_trip = from_nquads(nquads)
```

Files-profile helpers use Julia-owned `Vector{UInt8}` payloads and ordinary
strings for paths:

```julia
packed = files_pack(["/path/to/tree"])
diff = files_diff_json(packed, "/path/to/tree")
report = files_unpack(packed, "/tmp/unpacked")
```

## Ownership And Errors

The wrapper copies every returned `gts_buffer` into a Julia `String` or
`Vector{UInt8}` and then calls `gts_buffer_free` in a `finally` block. If a C ABI
call returns an error handle, the wrapper copies the stable error code and
message, releases the handle with `gts_error_free`, and raises a structured
`GtsError`.

Callers never receive raw C pointers, buffer capacities, or Rust-owned memory.

`GtsError` exposes:

- `operation`: C ABI operation name.
- `status`: integer C ABI status code.
- `status_name`: symbolic C ABI status name.
- `code`: stable C ABI error code.
- `detail`: human-readable diagnostic detail.

```julia
try
    from_nquads("<https://example/s> <https://example/p> .\n")
catch error
    if error isa GtsError
        @info "GTS error" error.status_name error.code error.detail
    else
        rethrow()
    end
end
```

## Threading And ABI Stability

`libgts` operations are reentrant. Each wrapper call owns its output buffer and
error handle independently, and the wrapper frees them before returning or
throwing. Native library handle lookup is cached behind a Julia lock. Do not
share raw native pointers between Julia tasks or processes.

The wrapper targets `GTS_ABI_VERSION` 1. Check `abi_version()` and
`capabilities_json()` when loading a system-provided `libgts`.

## Validation

Run the smoke test from the repository root:

```sh
bash julia/scripts/smoke.sh
```

The script builds `libgts`, runs the Julia smoke test, and exercises ABI
metadata, capabilities, read/fold, verify, N-Quads export/import, structured
errors, and files-profile pack/diff/unpack. If local Julia is missing, it uses
the pinned fallback image defined in `julia/Dockerfile`.

Set `GTS_JULIA_FORCE_DOCKER=1` to run the Docker path even when Julia is
installed locally, which is useful when reproducing CI behavior.

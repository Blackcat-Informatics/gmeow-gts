# GTS Lua C ABI Wrapper

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

This package is a thin LuaJIT FFI wrapper over the Rust-backed `libgts` C ABI.
It does not parse, fold, write, or verify GTS archives in Lua; all GTS semantics
come from the checked-in C ABI in `rust/capi/include/gts.h`.

## Requirements

- LuaJIT 2.1 with the `ffi` module.
- A built `libgts` shared library from `rust/capi`.

Build the shared library from the repository root:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Point the wrapper at the library with `GTS_LIBGTS`:

```sh
export GTS_LIBGTS="$PWD/rust/capi/target/debug/libgts.so"
LUA_PATH="$PWD/lua/?.lua;$PWD/lua/?/init.lua;;" \
  luajit lua/tests/smoke.lua vectors/01-minimal.gts
```

On macOS use `libgts.dylib`; on Windows use `gts.dll`. If `GTS_LIBGTS` is not
set, the wrapper asks the platform dynamic loader for the default library name.

## API Shape

```lua
local gts = require("gmeow.gts").load()
local input = assert(io.open("vectors/01-minimal.gts", "rb")):read("*a")

local metadata = gts:build_metadata_json()
local capabilities = gts:capabilities_json()
local folded = gts:read_json(input)
local verified = gts:verify_json(input)
local nquads = gts:to_nquads(input)
local round_trip = gts:from_nquads(nquads)
```

Files-profile helpers use Lua strings for binary GTS payloads and ordinary path
strings for directories:

```lua
local packed = gts:files_pack({"/path/to/tree"})
local diff = gts:files_diff_json(packed, "/path/to/tree")
local report = gts:files_unpack(packed, "/tmp/unpacked")
```

## Ownership And Errors

The wrapper copies every returned `gts_buffer` into a Lua string and then calls
`gts_buffer_free`. If a C ABI call returns an error handle, the wrapper copies
the stable error code and message, releases the handle with `gts_error_free`,
and raises a structured Lua table.

Callers never receive raw FFI pointers, buffer capacities, or Rust-owned memory.
Lua strings are binary-safe and are used for both GTS byte streams and UTF-8
JSON/N-Quads text.

Raised error tables expose:

- `operation`: C ABI operation name.
- `status`: integer C ABI status code.
- `status_name`: symbolic C ABI status name.
- `code`: stable C ABI error code.
- `detail`: human-readable diagnostic detail.

Use `pcall` to catch structured errors:

```lua
local ok, err = pcall(function()
  return gts:from_nquads("<https://example/s> <https://example/p> .\n")
end)
if not ok then
  print(err.status_name, err.code, err.detail)
end
```

## Threading And ABI Stability

`libgts` operations are reentrant. Each call owns its output buffer and error
handle independently, and the Lua wrapper frees them before returning or
throwing. Do not share raw FFI values across Lua states.

The wrapper targets `GTS_ABI_VERSION` 1. Check `gts:abi_version()` and
`gts:capabilities_json()` when loading a system-provided `libgts`.

## Validation

Run the smoke test from the repository root:

```sh
bash lua/scripts/smoke.sh
```

The script builds `libgts` and exercises ABI metadata, capabilities, read/fold,
verify, N-Quads export/import, structured errors, and files-profile
pack/diff/unpack. If local LuaJIT is missing, it uses the pinned fallback image
defined in `lua/Dockerfile`.

# GTS Ruby C ABI Wrapper

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

`gmeow-gts` is a thin Ruby FFI wrapper over the Rust-backed `libgts` C ABI. It
does not parse, fold, write, or verify GTS archives in Ruby; all GTS semantics
come from the checked-in C ABI in `rust/capi/include/gts.h`.

## Requirements

- Ruby 3.1 or newer.
- The `ffi` gem.
- A built `libgts` shared library from `rust/capi`.

Build the shared library from the repository root:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Point the wrapper at the library with `GTS_LIBGTS`:

```sh
export GTS_LIBGTS="$PWD/rust/capi/target/debug/libgts.so"
ruby -I ruby/lib ruby/tests/smoke.rb \
  vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts
```

On macOS use `libgts.dylib`; on Windows use `gts.dll`. If `GTS_LIBGTS` is not
set, the wrapper asks the platform dynamic loader for the default library name.

## RubyGems Installation

The published `gmeow-gts` gem is source-only. It ships the Ruby FFI wrapper and
declares the `ffi` dependency, but it does not vendor `libgts`.

```sh
gem install gmeow-gts
export GTS_LIBGTS=/path/to/libgts.so
ruby -rgmeow/gts -e 'puts Gmeow::Gts.load.version'
```

On macOS set `GTS_LIBGTS` to the `.dylib`; on Windows set it to `gts.dll`.

## API Shape

```ruby
require "gmeow/gts"

gts = Gmeow::Gts.load
input = File.binread("vectors/01-minimal.gts")

metadata = gts.build_metadata_json
capabilities = gts.capabilities_json
folded = gts.read_json(input)
verified = gts.verify_json(input)
nquads = gts.to_nquads(input)
round_trip = gts.from_nquads(nquads)
```

Files-profile helpers use Ruby strings for binary GTS payloads and ordinary path
strings for directories:

```ruby
packed = gts.files_pack(["/path/to/tree"])
diff = gts.files_diff_json(packed, "/path/to/tree")
report = gts.files_unpack(packed, "/tmp/unpacked")
```

## Ownership And Errors

The wrapper copies every returned `gts_buffer` into a Ruby string and then calls
`gts_buffer_free`. If a C ABI call returns an error handle, the wrapper copies
the stable error code and message, releases the handle with `gts_error_free`,
and raises `Gmeow::Gts::Error`.

Callers never receive raw FFI pointers, buffer capacities, or Rust-owned memory.
Ruby strings are binary-safe and are used for both GTS byte streams and UTF-8
JSON/N-Quads text.

Raised errors expose:

- `operation`: C ABI operation name.
- `status`: integer C ABI status code.
- `status_name`: symbolic C ABI status name.
- `code`: stable C ABI error code.
- `detail`: human-readable diagnostic detail.

```ruby
begin
  gts.from_nquads("<https://example/s> <https://example/p> .\n")
rescue Gmeow::Gts::Error => error
  warn "#{error.status_name} #{error.code}: #{error.detail}"
end
```

## Threading And ABI Stability

`libgts` operations are reentrant. Each call owns its output buffer and error
handle independently, and the Ruby wrapper frees them before returning or
throwing. Do not share raw FFI values between Ruby runtimes.

The wrapper targets `GTS_ABI_VERSION` 1. Check `gts.abi_version` and
`gts.capabilities_json` when loading a system-provided `libgts`.

## Validation

Run the smoke test from the repository root:

```sh
bash ruby/scripts/smoke.sh
```

The script builds `libgts`, validates the gemspec, and exercises ABI metadata,
capabilities, read/fold, verify, N-Quads export/import, structured errors, and
files-profile pack/diff/unpack. It runs the smoke once from the checkout and
once through an installed local gem with the checkout removed from Ruby's load
path. If local Ruby with the `ffi` gem is missing, it uses the pinned fallback
image defined in `ruby/Dockerfile`.

# GTS PHP C ABI Wrapper

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

This package is a thin PHP FFI wrapper over the Rust-backed `libgts` C ABI. It
does not parse, fold, write, or verify GTS archives in PHP; all GTS semantics
come from the checked-in C ABI in `rust/capi/include/gts.h`.

## Requirements

- PHP 8.2 or newer.
- The PHP FFI extension with `ffi.enable=1` at runtime.
- A separately installed `libgts` shared library.

## Composer Install

The Composer package name is `blackcatinformatics/gmeow-gts`:

```sh
composer require blackcatinformatics/gmeow-gts
```

The package is source-only. It does not bundle `libgts`, does not implement GTS
in PHP, and requires PHP FFI plus the native C ABI library at runtime.

## Loading `libgts`

Install `libgts` from a C ABI release archive, a package-manager integration,
or a local build. For local development, build the shared library from the
repository root:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Point the wrapper at the library with `GTS_LIBGTS`:

```sh
export GTS_LIBGTS="$PWD/rust/capi/target/debug/libgts.so"
php -d ffi.enable=1 php/tests/smoke.php vectors/01-minimal.gts
```

On macOS use `libgts.dylib`; on Windows use `gts.dll`. If `GTS_LIBGTS` is not
set, the wrapper asks the platform dynamic loader for the default library name.

## API Shape

```php
use Gmeow\Gts\Gts;

$gts = Gts::load();               // or Gts::load('/path/to/libgts.so')
$bytes = file_get_contents('vectors/01-minimal.gts');

$metadata = $gts->buildMetadataJson();
$capabilities = $gts->capabilitiesJson();
$folded = $gts->readJson($bytes);
$verified = $gts->verifyJson($bytes);
$nquads = $gts->toNQuads($bytes);
$roundTripBytes = $gts->fromNQuads($nquads);
```

Files-profile helpers use PHP strings for binary GTS payloads and ordinary path
strings for directories:

```php
$packed = $gts->filesPack(['/path/to/tree']);
$diff = $gts->filesDiffJson($packed, '/path/to/tree');
$report = $gts->filesUnpack($packed, '/tmp/unpacked');
```

## Ownership And Errors

The wrapper copies every returned `gts_buffer` into a PHP string and then calls
`gts_buffer_free`. If a C ABI call returns an error handle, the wrapper copies
the stable error code and message, releases the handle with `gts_error_free`,
and throws `Gmeow\Gts\GtsException`.

Callers never receive raw FFI pointers, buffer capacities, or Rust-owned memory.
PHP strings are binary-safe and are used for both GTS byte streams and UTF-8
JSON/N-Quads text.

`GtsException` exposes:

- `operation`: C ABI operation name.
- `status`: integer C ABI status code.
- `errorCode`: stable C ABI error code.
- `detail`: human-readable diagnostic detail.

## Threading And ABI Stability

`libgts` operations are reentrant. Each call owns its output buffer and error
handle independently, and the PHP wrapper frees them before returning or
throwing. Do not share raw FFI values between threads or fibers.

The wrapper targets `GTS_ABI_VERSION` 1. Check `$gts->abiVersion()` and
`$gts->capabilitiesJson()` when loading a system-provided `libgts`.

## Validation

Run the smoke test from the repository root:

```sh
bash php/scripts/smoke.sh
```

The script validates `php/composer.json`, builds `libgts`, and exercises ABI
metadata, capabilities, read/fold, verify, N-Quads export/import, structured
errors, and files-profile pack/diff/unpack. If local PHP or Composer tooling is
missing, it uses pinned container fallbacks; the PHP fallback builds a tiny
FFI-enabled smoke image from `php/Dockerfile`.

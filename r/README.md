# GTS R C ABI Wrapper

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

`gmeowgts` is a thin R package over the Rust-backed `libgts` C ABI. It does not
parse, fold, write, or verify GTS archives in R; all GTS semantics come from the
checked-in C ABI in `rust/capi/include/gts.h`.

## Requirements

- R 4.3 or newer with native package compilation tools.
- A built or installed `libgts` shared library from the GTS C ABI.

Build the shared library from the repository root:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
```

Install the package against that library:

```sh
GTS_LIB_DIR="$PWD/rust/capi/target/debug" R CMD INSTALL r
```

When `GTS_LIB_DIR` is not set, package configuration tries the default linker
path followed by common system library locations such as `/usr/local/lib`,
`/usr/lib`, `/opt/homebrew/lib`, and `/opt/local/lib`. If no linkable `libgts`
is found, installation stops during `configure` with the searched paths and the
compiler output location.

At runtime, the platform dynamic loader must also find `libgts`:

```sh
export LD_LIBRARY_PATH="$PWD/rust/capi/target/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
Rscript r/tests/smoke.R \
  vectors/01-minimal.gts vectors/04-damaged-frame.gts vectors/28-empty-file.gts
```

On macOS use `DYLD_LIBRARY_PATH`; on Windows make `gts.dll` discoverable through
`PATH` or package the native library next to the package DLL.

## r-universe

The Blackcat Informatics universe is configured to build this package from the
`r/` subdirectory:

```r
options(repos = c(
  "blackcat-informatics" = "https://blackcat-informatics.r-universe.dev",
  CRAN = "https://cloud.r-project.org"
))
install.packages("gmeowgts")
```

Use the r-universe install path after the package page reports a successful
source build. The package remains source-only, so `libgts` must still be
installed in a system location or made discoverable with `GTS_LIB_DIR` during
source installation and with the platform loader at runtime.

## API Shape

```r
library(gmeowgts)

input <- readBin("vectors/01-minimal.gts", "raw", n = file.info("vectors/01-minimal.gts")$size)

metadata <- build_metadata_json()
capabilities <- capabilities_json()
folded <- read_json(input)
verified <- verify_json(input)
nquads <- to_nquads(input)
round_trip <- from_nquads(nquads)
```

Files-profile helpers use raw vectors for binary GTS payloads and ordinary
character strings for paths:

```r
packed <- files_pack(c("/path/to/tree"))
diff <- files_diff_json(packed, "/path/to/tree")
report <- files_unpack(packed, "/tmp/unpacked")
```

## Ownership And Errors

The C bridge copies every returned `gts_buffer` into an R character string or raw
vector and then calls `gts_buffer_free`. If a C ABI call returns an error handle,
the bridge copies the stable error code and message, releases the handle with
`gts_error_free`, and the R wrapper raises a structured `gmeowgts_error`
condition.

Callers never receive raw C pointers, buffer capacities, or Rust-owned memory.

Raised errors expose:

- `operation`: C ABI operation name.
- `status`: integer C ABI status code.
- `status_name`: symbolic C ABI status name.
- `code`: stable C ABI error code.
- `detail`: human-readable diagnostic detail.

```r
tryCatch(
  from_nquads("<https://example/s> <https://example/p> .\n"),
  gmeowgts_error = function(error) {
    message(error$status_name, " ", error$code, ": ", error$detail)
  }
)
```

## Threading And ABI Stability

`libgts` operations are reentrant. Each call owns its output buffer and error
handle independently, and the bridge frees them before returning or throwing. Do
not share raw native state between R processes.

The wrapper targets `GTS_ABI_VERSION` 1. Check `abi_version()` and
`capabilities_json()` when loading a system-provided `libgts`.

CRAN submission is tracked separately in
[`CRAN-READINESS.md`](./CRAN-READINESS.md); it is not a blocker for the first
r-universe publication.

## Validation

Run the smoke test from the repository root:

```sh
bash r/scripts/smoke.sh
```

The script builds `libgts`, verifies `r/src/gts.h` matches the canonical C ABI
header, installs the R package into a temporary library, and exercises ABI
metadata, capabilities, read/fold, verify, N-Quads export/import, structured
errors, and files-profile pack/diff/unpack. If local R is missing, it uses the
pinned fallback image defined in `r/Dockerfile`.

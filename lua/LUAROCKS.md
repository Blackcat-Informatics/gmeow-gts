# LuaRocks Publication

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

The Lua wrapper is published as the `gmeow-gts` LuaRocks package. It is a
source-only LuaJIT FFI wrapper and does not bundle native `libgts` binaries.
Users must provide `libgts` with `GTS_LIBGTS` or through the platform dynamic
loader.

## Package Name

The first stable upload uses:

- rockspec: `lua/gmeow-gts-0.9.4-1.rockspec`
- LuaRocks package: `gmeow-gts`
- LuaRocks version: `0.9.4-1`
- source tag: `lua-v0.9.4`

Live availability checked on 2026-06-22:

- `https://luarocks.org/search?q=gmeow-gts` returned `No modules`.
- `https://luarocks.org/modules/blackcatinformatics/gmeow-gts` returned 404.

Do not publish `lua/gmeow-gts-dev-1.rockspec` to the root manifest. It is only
for local development checks.

## Validation

Run the full wrapper package dry-run from the repository root:

```sh
bash scripts/package_dry_run_wrappers.sh
```

The Lua lane validates the stable rockspec, installs it into a temporary
LuaRocks tree, packs the rock artifact, and runs the smoke test through the
installed rock with `GTS_LIBGTS` set to the locally built C ABI library.

For a focused local check with a host LuaRocks install:

```sh
cargo build --manifest-path rust/capi/Cargo.toml
export GTS_LIBGTS="$PWD/rust/capi/target/debug/libgts.so"
export LD_LIBRARY_PATH="$PWD/rust/capi/target/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
rm -rf /tmp/gts-luarocks
luarocks lint lua/gmeow-gts-0.9.4-1.rockspec
luarocks make lua/gmeow-gts-0.9.4-1.rockspec --tree /tmp/gts-luarocks
luarocks --tree /tmp/gts-luarocks pack gmeow-gts 0.9.4-1
eval "$(luarocks --tree /tmp/gts-luarocks path --bin)"
luajit lua/tests/smoke.lua vectors/01-minimal.gts
```

Use `libgts.dylib` on macOS and `gts.dll` on Windows.

## Release

The `.github/workflows/release-luarocks.yaml` workflow publishes on `lua-v*`
tags. It verifies that the tag version matches the stable rockspec file name,
`version`, and `source.tag`, then runs LuaRocks lint/make/pack plus an
installed-rock smoke test before upload.

First stable upload:

```sh
git tag -s lua-v0.9.4
git push origin lua-v0.9.4
```

The repository secret `LUAROCKS_API_KEY` must be configured before the tag is
pushed. The workflow publishes with:

```sh
luarocks upload lua/gmeow-gts-0.9.4-1.rockspec --api-key="$LUAROCKS_API_KEY"
```

Manual upload fallback after validation:

```sh
luarocks upload lua/gmeow-gts-0.9.4-1.rockspec --api-key="$LUAROCKS_API_KEY"
```

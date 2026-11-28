#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Production Clippy for the Rust workspace: library and binary targets, locked.
# The arguments match the production lint command of raf-census (its
# PRODUCTION_CLIPPY_ARGS, which `cargo raf-census lint-capture` runs) flag for flag, so a
# clean run of this script and the receipt capture judge the same build. Do not add, drop or
# reorder a flag here; add workspace lints to rust/Cargo.toml instead.
set -euo pipefail

cd "$(dirname "$0")/../rust"
CARGO_ENCODED_RUSTFLAGS='' exec cargo clippy --workspace --lib --bins --locked --target host-tuple -- \
  -D warnings -D unused -D unreachable-pub -D clippy::allow_attributes \
  -D clippy::used_underscore_binding -D clippy::needless_pass_by_value \
  -D clippy::trivially_copy_pass_by_ref -D clippy::redundant_clone

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Production Clippy for the Rust workspace: library and binary targets, locked.
# The arguments match the census production lint command flag for flag, so a
# lint receipt from this script is accepted by the census. Do not add, drop or
# reorder a flag here; add workspace lints to rust/Cargo.toml instead.
set -euo pipefail

cd "$(dirname "$0")/../rust"
CARGO_ENCODED_RUSTFLAGS='' exec cargo clippy --workspace --lib --bins --locked -- \
  -D warnings -D unused -D unreachable-pub -D clippy::allow_attributes \
  -D clippy::used_underscore_binding -D clippy::needless_pass_by_value \
  -D clippy::trivially_copy_pass_by_ref -D clippy::redundant_clone

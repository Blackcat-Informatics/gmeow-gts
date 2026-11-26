#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Census receipt for the Rust package: capture a fresh production Clippy receipt
# with the `cargo-raf-census` command at .stage/census/production.ttl, outside the
# Rust workspace root as the census requires. tests/census.rs reads that path (or
# RAF_CENSUS_LINT_EVIDENCE when set), so run this before any `cargo test` that
# includes the census test; a missing or stale receipt fails that test.
#
# The command and the `raf-census` dev-dependency must be the same release. Install
# the command with:
#   cargo install --locked --version "=<version>" cargo-raf-census
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
wanted="$(sed -n 's/^raf-census = "=\(.*\)"$/\1/p' "$root/rust/Cargo.toml")"
if [[ -z "$wanted" ]]; then
  echo "rust/Cargo.toml has no exact raf-census dev-dependency" >&2
  exit 1
fi
installed="$(cargo raf-census --version | awk '{print $NF}')"
if [[ "$installed" != "$wanted" ]]; then
  echo "cargo-raf-census $installed is installed but raf-census =$wanted is locked; run: cargo install --locked --version =$wanted cargo-raf-census" >&2
  exit 1
fi

receipt="$root/.stage/census/production.ttl"
mkdir -p "$root/.stage/census"
rm -f -- "$receipt"

cargo raf-census --workspace "$root/rust" lint-capture --trust-lints --out "$receipt"

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Enforce lockstep versioning: the Rust crate, Python package, and npm package
# must all declare the same version. (The Go module is versioned by git tag, so
# it has no manifest version to compare.) A release is a single `<v>` cut across
# all engines; this guards against a half-bumped release.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

rust_v=$(grep -m1 '^version = ' "$ROOT/rust/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')
py_v=$(grep -m1 '^version = ' "$ROOT/python/pyproject.toml" | sed -E 's/.*"([^"]+)".*/\1/')
npm_v=$(grep -m1 '"version"' "$ROOT/ts/package.json" | sed -E 's/.*"version": "([^"]+)".*/\1/')

printf 'rust   %s\npython %s\nnpm    %s\n' "$rust_v" "$py_v" "$npm_v"

if [ "$rust_v" = "$py_v" ] && [ "$py_v" = "$npm_v" ]; then
  echo "OK: all engine versions agree ($rust_v)"
else
  echo "ERROR: engine versions disagree — bump rust/Cargo.toml, python/pyproject.toml, and ts/package.json together." >&2
  exit 1
fi

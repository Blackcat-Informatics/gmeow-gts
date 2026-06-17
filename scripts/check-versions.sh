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
citation_v=$(grep -m1 '^version:' "$ROOT/CITATION.cff" | sed -E 's/.*"([^"]+)".*/\1/')
py_floor=$(grep -m1 '^requires-python = ' "$ROOT/python/pyproject.toml" | sed -E 's/.*"([^"]+)".*/\1/')
node_floor=$(grep -m1 '"node"' "$ROOT/ts/package.json" | sed -E 's/.*"node": "([^"]+)".*/\1/')
go_floor=$(grep -m1 '^go ' "$ROOT/go/go.mod" | awk '{print $2}')

printf 'rust     %s\npython   %s\nnpm      %s\ncitation %s\n' "$rust_v" "$py_v" "$npm_v" "$citation_v"

errors=0

check_contains() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if ! grep -Fq "$needle" "$ROOT/$file"; then
    echo "ERROR: $label drifted; expected '$needle' in $file." >&2
    errors=1
  fi
}

if [ "$rust_v" = "$py_v" ] && [ "$py_v" = "$npm_v" ] && [ "$citation_v" = "$rust_v" ]; then
  echo "OK: engine and citation versions agree ($rust_v)"
else
  echo "ERROR: versions disagree — bump rust/Cargo.toml, python/pyproject.toml, ts/package.json, and CITATION.cff together." >&2
  errors=1
fi

check_contains "README.md" "gmeow-gts = \"$rust_v\"" "README Rust dependency snippet"
check_contains "README.md" "gmeow-gts = { version = \"$rust_v\"" "README Rust feature snippet"
check_contains "rust/README.md" "gmeow-gts = \"$rust_v\"" "Rust README dependency snippet"
check_contains "docs/GTS-ECOSYSTEM-INTEGRATIONS.md" "gmeow-gts = { version = \"$rust_v\"" "ecosystem Rust feature snippet"
check_contains "README.md" "Runtime support policy: Python $py_floor, Node.js $node_floor, and Go $go_floor" "README runtime support policy"

if grep -Eiq 'rdf-star|RDF-star' "$ROOT/CITATION.cff"; then
  echo "ERROR: CITATION.cff should use current RDF 1.2 wording, not rdf-star." >&2
  errors=1
fi

if [ "$errors" -ne 0 ]; then
  exit 1
fi

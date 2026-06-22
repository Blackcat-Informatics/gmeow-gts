#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Verify versioned package metadata and docs. Cross-engine releases normally bump
# Rust, Python, npm, and citation metadata together, but narrow Rust-first
# releases may bump only the Rust crate. The Go module is versioned by git tag,
# so it has no manifest version to compare here.
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
  echo "OK: ecosystem versions are not lockstep; checking per-ecosystem surfaces independently."
  if [ "$py_v" != "$npm_v" ]; then
    echo "ERROR: Python ($py_v) and npm ($npm_v) versions disagree." >&2
    errors=1
  fi
  if [ "$citation_v" != "$rust_v" ] && [ "$citation_v" != "$py_v" ]; then
    echo "ERROR: Citation version ($citation_v) must match either Rust ($rust_v) or Python/npm ($py_v)." >&2
    errors=1
  fi
fi

check_contains "README.md" "gmeow-gts = \"$rust_v\"" "README Rust dependency snippet"
check_contains "README.md" "gmeow-gts = { version = \"$rust_v\"" "README Rust feature snippet"
check_contains "rust/README.md" "gmeow-gts = \"$rust_v\"" "Rust README dependency snippet"
check_contains "docs/GTS-ECOSYSTEM-INTEGRATIONS.md" "gmeow-gts = { version = \"$rust_v\"" "ecosystem Rust feature snippet"
check_contains "README.md" "Runtime support policy: Python $py_floor, Node.js $node_floor, and Go $go_floor" "README runtime support policy"

for file in CITATION.cff rust/Cargo.toml python/pyproject.toml; do
  if grep -Eiq 'rdf-star|RDF-star' "$ROOT/$file"; then
    echo "ERROR: $file should use current RDF 1.2 wording, not rdf-star." >&2
    errors=1
  fi
done

if ! keyword_errors="$(ROOT="$ROOT" python3 - <<'PY'
from pathlib import Path
import os
import sys
import tomllib

root = Path(os.environ["ROOT"])
rust_keywords = tomllib.loads((root / "rust/Cargo.toml").read_text())["package"]["keywords"]
python_keywords = tomllib.loads((root / "python/pyproject.toml").read_text())["project"]["keywords"]

errors = []
if len(rust_keywords) > 5:
    errors.append("rust/Cargo.toml declares more than five keywords; crates.io allows at most five.")
if "rdf-12" not in rust_keywords:
    errors.append("rust/Cargo.toml keywords should include Cargo-safe RDF 1.2 wording: rdf-12.")
if "rdf-1.2" not in python_keywords:
    errors.append("python/pyproject.toml keywords should include RDF 1.2 wording: rdf-1.2.")

if errors:
    print("\n".join(f"ERROR: {error}" for error in errors))
    sys.exit(1)
PY
)"; then
  printf '%s\n' "$keyword_errors" >&2
  errors=1
fi

if [ "$errors" -ne 0 ]; then
  exit 1
fi

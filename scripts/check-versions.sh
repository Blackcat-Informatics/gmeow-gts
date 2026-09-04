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
capi_v=$(grep -m1 '^version = ' "$ROOT/rust/capi/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')
py_v=$(grep -m1 '^version = ' "$ROOT/python/pyproject.toml" | sed -E 's/.*"([^"]+)".*/\1/')
npm_v=$(grep -m1 '"version"' "$ROOT/ts/package.json" | sed -E 's/.*"version": "([^"]+)".*/\1/')
citation_v=$(grep -m1 '^version:' "$ROOT/CITATION.cff" | sed -E 's/.*"([^"]+)".*/\1/')
py_floor=$(grep -m1 '^requires-python = ' "$ROOT/python/pyproject.toml" | sed -E 's/.*"([^"]+)".*/\1/')
node_floor=$(grep -m1 '"node"' "$ROOT/ts/package.json" | sed -E 's/.*"node": "([^"]+)".*/\1/')
go_floor=$(grep -m1 '^go ' "$ROOT/go/go.mod" | awk '{print $2}')

printf 'rust     %s\ncapi     %s\npython   %s\nnpm      %s\ncitation %s\n' "$rust_v" "$capi_v" "$py_v" "$npm_v" "$citation_v"

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

if [ "$capi_v" != "$rust_v" ]; then
  echo "ERROR: C ABI crate version ($capi_v) must stay lockstep with Rust crate version ($rust_v)." >&2
  errors=1
fi

# Both Cargo LOCKFILES must already name the new version. The release workflows
# build with --locked, so a lockfile left behind by a version bump does not fail
# here or in CI -- it fails at PUBLISH time, after the tag is pushed, which
# docs/GTS-V1-RC1-CHECKLIST.md:425 makes a release incident rather than
# something you can quietly redo. That is exactly how capi-v1.0.0-rc.1 failed:
#
#   error: cannot update the lock file .../rust/capi/Cargo.lock
#   because --locked was passed to prevent this
#
# rust/capi is the easy one to miss, being a second standalone lockfile.
for lock_dir in rust rust/capi; do
  lock="$ROOT/$lock_dir/Cargo.lock"
  if [ "$lock_dir" = "rust" ]; then crate="gmeow-gts"; else crate="gmeow-gts-capi"; fi
  if ! grep -A1 "^name = \"$crate\"$" "$lock" | grep -Fq "version = \"$rust_v\""; then
    echo "ERROR: $lock_dir/Cargo.lock does not name $crate $rust_v; regenerate it (cargo update -p $crate)." >&2
    errors=1
  fi
done

# Wrapper lanes. These were unchecked until now, which is exactly why Kotlin,
# Lua, Ruby, R and Julia all silently drifted to 0.9.4 and stayed there through
# seven releases: nothing in CI ever compared them. They publish through their
# own manual runbooks rather than a tag workflow, so a guard here is the only
# thing that keeps them honest. Compared against the Rust crate, which is the
# release family's anchor (the C ABI they wrap is versioned lockstep with it).
check_lane() {
  local label="$1"
  local actual="$2"
  local file="$3"
  if [ -z "$actual" ]; then
    echo "ERROR: could not read $label version from $file." >&2
    errors=1
  elif [ "$actual" != "$rust_v" ]; then
    echo "ERROR: $label version ($actual) must match the Rust crate version ($rust_v); see $file." >&2
    errors=1
  fi
}

kotlin_v=$(grep -m1 '^version = ' "$ROOT/kotlin/build.gradle.kts" | sed -E 's/.*"([^"]+)".*/\1/')
ruby_v=$(grep -m1 'VERSION = ' "$ROOT/ruby/lib/gmeow/gts.rb" | sed -E 's/.*"([^"]+)".*/\1/')
r_v=$(grep -m1 '^Version:' "$ROOT/r/DESCRIPTION" | awk '{print $2}')
julia_v=$(grep -m1 '^version = ' "$ROOT/julia/Project.toml" | sed -E 's/.*"([^"]+)".*/\1/')
julia_src_v=$(grep -m1 'const VERSION = v"' "$ROOT/julia/src/GmeowGTS.jl" | sed -E 's/.*v"([^"]+)".*/\1/')

printf 'kotlin   %s\nruby     %s\nr        %s\njulia    %s\n' \
  "$kotlin_v" "$ruby_v" "$r_v" "$julia_v"

check_lane "Kotlin" "$kotlin_v" "kotlin/build.gradle.kts"
# RubyGems rejects a semver pre-release outright: `gem build` on version
# 1.0.0-rc.1 raises Gem::InvalidSpecificationException. A gem version is dotted
# segments, and any segment containing a letter makes it a prerelease that sorts
# BEFORE the final release -- so the dash simply becomes a dot.
ruby_expected="$(printf '%s' "$rust_v" | tr '-' '.')"
if [ "$ruby_v" != "$ruby_expected" ]; then
  echo "ERROR: Ruby version ($ruby_v) must be $ruby_expected for family version $rust_v; see ruby/lib/gmeow/gts.rb." >&2
  errors=1
fi
# R is the one lane that cannot carry a semver pre-release. R's DESCRIPTION
# grammar admits only integers separated by "." or "-", so `1.0.0-rc.1` is
# rejected outright with "Malformed package version" and the package will not
# install. For a pre-release the R lane therefore uses R's own development-version
# convention (`<last-release>.9000+`, which sorts BEFORE the coming release) and
# is checked for validity rather than for equality. It returns to lockstep at the
# final release, where the family version is plain semver and R accepts it.
case "$rust_v" in
  *-*)
    if ! printf '%s' "$r_v" | grep -Eq '^[0-9]+(\.[0-9]+){2,3}$'; then
      echo "ERROR: R version ($r_v) is not a valid R package version (integers separated by '.'); see r/DESCRIPTION." >&2
      errors=1
    fi
    ;;
  *)
    check_lane "R" "$r_v" "r/DESCRIPTION"
    ;;
esac
check_lane "Julia" "$julia_v" "julia/Project.toml"
check_lane "Julia source" "$julia_src_v" "julia/src/GmeowGTS.jl"

# LuaRocks encodes the version in the rockspec FILENAME as well as in its
# `version` and `source.tag` fields, and release-luarocks.yaml greps all three
# with an exact match, so a bump that misses one fails only at publish time.
# LuaRocks parses a rockspec version as `[%w.]+-[%d]+`, so the part before the
# revision may contain only alphanumerics and dots — a semver pre-release like
# 1.0.0-rc.1 is rejected ("Type mismatch on field version"). Collapse the
# pre-release punctuation to LuaRocks' spelling, which lands on the same string
# PyPI normalizes to: 1.0.0-rc.1 -> 1.0.0rc1. A final release is unchanged.
lua_v="$(printf '%s' "$rust_v" | tr -d '-' | sed -E 's/([a-z])\.([0-9]+)$/\1\2/')"
lua_rockspec="$ROOT/lua/gmeow-gts-${lua_v}-1.rockspec"
if [ ! -f "$lua_rockspec" ]; then
  echo "ERROR: expected LuaRocks rockspec lua/gmeow-gts-${lua_v}-1.rockspec (rename it on bump)." >&2
  errors=1
else
  check_contains "lua/gmeow-gts-${lua_v}-1.rockspec" "version = \"${lua_v}-1\"" "rockspec version"
  check_contains "lua/gmeow-gts-${lua_v}-1.rockspec" "tag = \"lua-v${lua_v}\"" "rockspec source tag"
fi

check_contains "README.md" "gmeow-gts = \"$rust_v\"" "README Rust dependency snippet"
check_contains "README.md" "gmeow-gts = { version = \"$rust_v\"" "README Rust feature snippet"
check_contains "rust/README.md" "gmeow-gts = \"$rust_v\"" "Rust README dependency snippet"
check_contains "docs/GTS-ECOSYSTEM-INTEGRATIONS.md" "gmeow-gts = { version = \"$rust_v\"" "ecosystem Rust feature snippet"
check_contains "rust/capi/Cargo.toml" "gmeow-gts = { version = \"$rust_v\"" "C ABI Rust dependency"
check_contains "rust/capi/gts.pc.in" "Version: $rust_v" "C ABI pkg-config template version"
check_contains "packaging/vcpkg/ports/gmeow-gts/vcpkg.json" "\"version\": \"$rust_v\"" "vcpkg overlay package version"
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

if ! python3 "$ROOT/scripts/check_doc_roster.py"; then
  errors=1
fi
if ! python3 "$ROOT/scripts/check_doc_roster.py" --self-test; then
  errors=1
fi

if [ "$errors" -ne 0 ]; then
  exit 1
fi

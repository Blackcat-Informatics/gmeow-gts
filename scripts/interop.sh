#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Live cross-engine interoperability check.
#
# The frozen corpus in vectors/ proves each engine matches a *Python-generated*
# snapshot. This script proves the stronger property: every engine reads every
# *other* engine's fresh output identically. Each engine packs the same fixture
# directory; then every engine folds and unpacks every engine's package, and we
# assert the folds are byte-identical and the unpacked trees match the original.
#
# Usage: scripts/interop.sh        (builds each CLI as needed)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
ENGINES=(rust python go ts)
NODE_BIN="${NODE_BIN:-node}"

log() { printf '\033[1m== %s\033[0m\n' "$*"; }

# --- build / locate each engine's `gts` CLI -------------------------------- #
log "Building engine CLIs"
( cd "$ROOT/rust" && cargo build --release --quiet )
RUST_BIN="$ROOT/rust/target/release/gts"

GO_BIN="$WORK/gts-go"
( cd "$ROOT/go" && CGO_ENABLED=0 go build -o "$GO_BIN" ./cmd/gts )

( cd "$ROOT/ts" && npm ci --silent && npm run build --silent )
TS_BIN="$ROOT/ts/dist/bin/gts.js"

( cd "$ROOT/python" && uv sync --quiet )

# Dispatch a verb to a named engine.
gts() {
  local engine="$1"; shift
  case "$engine" in
    rust)   "$RUST_BIN" "$@" ;;
    go)     "$GO_BIN" "$@" ;;
    ts)     "$NODE_BIN" "$TS_BIN" "$@" ;;
    python) ( cd "$ROOT/python" && uv run --quiet gts "$@" ) ;;
  esac
}

# --- fixture: a deterministic directory tree (fixed mtime/mode) ------------ #
FIX="$WORK/fixture"
mkdir -p "$FIX/sub"
printf 'hello gts\n'   > "$FIX/a.txt"
printf 'second file\n' > "$FIX/sub/b.txt"
printf 'shared\n'      > "$FIX/c.txt"
printf 'shared\n'      > "$FIX/sub/d.txt"   # dedup target (same content as c.txt)
find "$FIX" -type f -exec chmod 644 {} +
find "$FIX" -type f -exec touch -d '@1700000000' {} +

# --- each engine packs the same fixture ------------------------------------ #
log "Each engine packs the fixture"
for e in "${ENGINES[@]}"; do
  gts "$e" pack "$FIX" -o "$WORK/packed_$e.gts"
  printf '  %-7s -> packed_%s.gts (%s bytes)\n' "$e" "$e" "$(wc -c < "$WORK/packed_$e.gts")"
done

fail=0

# --- byte-identity gate: identical input MUST pack to identical bytes ------- #
# Hard gate (#5): GTS is content-addressed and cat-append composable, so the same
# fixture must serialize identically regardless of which engine wrote it.
log "Byte-identity: all engines must pack the fixture to identical bytes"
if [ "$(sha256sum "$WORK"/packed_*.gts | awk '{print $1}' | sort -u | wc -l)" -eq 1 ]; then
  printf '  all four packages are byte-identical\n'
else
  echo "  MISMATCH: engines produced byte-divergent packages:" >&2
  sha256sum "$WORK"/packed_*.gts | sed 's/^/    /' >&2
  fail=1
fi

# --- cross-fold: EVERY package must fold to the SAME graph in every engine -- #
# One global reference: this catches both reader disagreement and writer drift.
log "Cross-fold: every package folds to the same graph in every engine"
ref=""
for writer in "${ENGINES[@]}"; do
  for reader in "${ENGINES[@]}"; do
    out="$(gts "$reader" fold "$WORK/packed_$writer.gts" | LC_ALL=C sort)"
    if [ -z "$ref" ]; then
      ref="$out"
    elif [ "$out" != "$ref" ]; then
      echo "  MISMATCH: $reader folding $writer's package differs from the reference" >&2
      fail=1
    fi
  done
  printf '  package from %-7s folds identically everywhere\n' "$writer"
done

# --- cross-unpack: every engine unpacks rust's package to the original ------ #
log "Cross-unpack: every engine restores the original tree"
for reader in "${ENGINES[@]}"; do
  dst="$WORK/out_$reader"
  gts "$reader" unpack "$WORK/packed_rust.gts" -C "$dst"
  if diff -r "$FIX" "$dst" >/dev/null; then
    printf '  %-7s unpack matches the original tree\n' "$reader"
  else
    echo "  MISMATCH: $reader unpack of rust's package differs from the original" >&2
    fail=1
  fi
done

# --- cross-diff: every engine compares every package consistently ----------- #
log "Cross-diff: every engine accepts matching trees for every package"
for writer in "${ENGINES[@]}"; do
  for reader in "${ENGINES[@]}"; do
    err="$WORK/diff_${reader}_${writer}.err"
    set +e
    out="$(gts "$reader" diff "$WORK/packed_$writer.gts" "$FIX" 2>"$err")"
    code=$?
    set -e
    if [ "$code" -ne 0 ] || [ -n "$out" ]; then
      echo "  MISMATCH: $reader diff of $writer's matching package failed" >&2
      printf '    exit=%s stdout=%q stderr=%q\n' "$code" "$out" "$(cat "$err")" >&2
      fail=1
    fi
  done
  printf '  package from %-7s diffs cleanly everywhere\n' "$writer"
done

MUT="$WORK/mutated"
cp -R "$FIX" "$MUT"
printf 'changed content\n' > "$MUT/a.txt"
rm "$MUT/sub/b.txt"
printf 'new content\n' > "$MUT/new.txt"
expected_diff=$'added: new.txt\nmodified: a.txt\nremoved: sub/b.txt'

log "Cross-diff: every engine reports the same changed-tree lines"
for writer in "${ENGINES[@]}"; do
  for reader in "${ENGINES[@]}"; do
    err="$WORK/diff_changed_${reader}_${writer}.err"
    set +e
    out="$(gts "$reader" diff "$WORK/packed_$writer.gts" "$MUT" 2>"$err")"
    code=$?
    set -e
    if [ "$code" -ne 1 ] || [ "$out" != "$expected_diff" ]; then
      echo "  MISMATCH: $reader changed-tree diff of $writer's package diverged" >&2
      printf '    exit=%s stdout=%q stderr=%q\n' "$code" "$out" "$(cat "$err")" >&2
      fail=1
    fi
  done
  printf '  package from %-7s reports changed trees identically everywhere\n' "$writer"
done

# --- cross-fold a frozen corpus vector ------------------------------------- #
log "Cross-fold a frozen corpus vector (01-minimal.gts)"
ref=""
for reader in "${ENGINES[@]}"; do
  out="$(gts "$reader" fold "$ROOT/vectors/01-minimal.gts" | LC_ALL=C sort)"
  if [ -z "$ref" ]; then ref="$out"; elif [ "$out" != "$ref" ]; then
    echo "  MISMATCH: $reader folds 01-minimal differently" >&2; fail=1
  fi
done
[ "$fail" -eq 0 ] && printf '  all engines fold 01-minimal identically\n'

if [ "$fail" -ne 0 ]; then
  log "INTEROP FAILED"; exit 1
fi
log "INTEROP OK — all four engines are mutually interoperable"

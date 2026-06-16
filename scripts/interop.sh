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
    ts)     node "$TS_BIN" "$@" ;;
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

# Informational: are the packed bytes identical across engines? (strong
# determinism signal, but not the hard gate — encodings may legitimately differ.)
if [ "$(sha256sum "$WORK"/packed_*.gts | awk '{print $1}' | sort -u | wc -l)" -eq 1 ]; then
  log "All four engines produced BYTE-IDENTICAL packages"
else
  log "Packages differ at the byte level (folds are still gated below)"
fi

fail=0

# --- cross-fold: every engine folds every package; folds must match -------- #
log "Cross-fold: every engine folds every package (sorted N-Quads must match)"
for writer in "${ENGINES[@]}"; do
  ref=""
  for reader in "${ENGINES[@]}"; do
    out="$(gts "$reader" fold "$WORK/packed_$writer.gts" | LC_ALL=C sort)"
    if [ -z "$ref" ]; then
      ref="$out"
    elif [ "$out" != "$ref" ]; then
      echo "  MISMATCH: $reader folding $writer's package differs from the others" >&2
      fail=1
    fi
  done
  printf '  package from %-7s folds identically in all engines\n' "$writer"
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

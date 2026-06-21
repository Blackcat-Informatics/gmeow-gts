#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
PACKAGE="${ROOT}/swift"
SMOKE_TARGET="GmeowGTSSmoke"
VECTOR="vectors/01-minimal.gts"
SWIFT_IMAGE="${SWIFT_IMAGE:-swift@sha256:4e50a9e711e8682a8c42bacfeed204568adfd6985a63b3789a165f28d296a28a}"
SWIFT_SCRATCH="${SWIFT_SCRATCH:-${TMPDIR:-/tmp}/gmeow-gts-swift-build}"

cd "${ROOT}"

diff -u rust/capi/include/gts.h swift/Sources/CGts/include/gts.h
cargo build --manifest-path "${CAPI}/Cargo.toml"

case "$(uname -s)" in
  Darwin)
    LIB_NAME="libgts.dylib"
    ;;
  MINGW* | MSYS* | CYGWIN*)
    LIB_NAME="gts.dll"
    ;;
  *)
    LIB_NAME="libgts.so"
    ;;
esac
LIB_PATH="${CAPI_TARGET}/${LIB_NAME}"

run_smoke() {
  swift run \
    --package-path "${PACKAGE}" \
    --scratch-path "${SWIFT_SCRATCH}" \
    -Xlinker "-L${CAPI_TARGET}" \
    -Xlinker "-rpath" \
    -Xlinker "${CAPI_TARGET}" \
    "${SMOKE_TARGET}" \
    "${VECTOR}"
}

if command -v swift >/dev/null 2>&1; then
  export LIBRARY_PATH="${CAPI_TARGET}${LIBRARY_PATH:+:${LIBRARY_PATH}}"
  export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
  run_smoke
else
  if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Swift is unavailable locally and the Docker fallback requires a Linux libgts build." >&2
    exit 1
  fi
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp/swift-home \
    -e LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${SWIFT_IMAGE}" \
    swift run \
      --package-path swift \
      --scratch-path /tmp/gmeow-gts-swift-build \
      -Xlinker -L/workspace/rust/capi/target/debug \
      -Xlinker -rpath \
      -Xlinker /workspace/rust/capi/target/debug \
      "${SMOKE_TARGET}" \
      "${VECTOR}"
fi

echo "Swift C ABI wrapper smoke test passed"

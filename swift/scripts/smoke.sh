#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
PACKAGE="${GTS_SWIFT_PACKAGE_PATH:-${ROOT}}"
SMOKE_TARGET="GmeowGTSSmoke"
VECTOR="vectors/01-minimal.gts"
SWIFT_IMAGE="${SWIFT_IMAGE:-swift@sha256:4e50a9e711e8682a8c42bacfeed204568adfd6985a63b3789a165f28d296a28a}"
SWIFT_SCRATCH="${SWIFT_SCRATCH:-${TMPDIR:-/tmp}/gmeow-gts-swift-build}"

cd "${ROOT}"

container_package_path() {
  case "${PACKAGE}" in
    "${ROOT}")
      printf '/workspace'
      ;;
    "${ROOT}/"*)
      printf '/workspace/%s' "${PACKAGE#"${ROOT}/"}"
      ;;
    .)
      printf '/workspace'
      ;;
    ./*)
      printf '/workspace/%s' "${PACKAGE#./}"
      ;;
    /*)
      echo "GTS_SWIFT_PACKAGE_PATH must point inside ${ROOT} when using Docker fallback." >&2
      exit 1
      ;;
    *)
      printf '/workspace/%s' "${PACKAGE}"
      ;;
  esac
}

diff -u rust/capi/include/gts.h swift/Sources/CGts/include/gts.h
cargo build --manifest-path "${CAPI}/Cargo.toml"

run_smoke() {
  swift package dump-package --package-path "${PACKAGE}" >/dev/null
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
  PACKAGE_CONTAINER="$(container_package_path)"
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp/swift-home \
    -e LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -e GTS_SWIFT_PACKAGE_PATH="${PACKAGE_CONTAINER}" \
    -e GTS_SWIFT_SMOKE_TARGET="${SMOKE_TARGET}" \
    -e GTS_SWIFT_VECTOR=/workspace/vectors/01-minimal.gts \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${SWIFT_IMAGE}" \
    bash -lc 'set -euo pipefail
    swift package dump-package --package-path "${GTS_SWIFT_PACKAGE_PATH}" >/dev/null
    swift run \
      --package-path "${GTS_SWIFT_PACKAGE_PATH}" \
      --scratch-path /tmp/gmeow-gts-swift-build \
      -Xlinker -L/workspace/rust/capi/target/debug \
      -Xlinker -rpath \
      -Xlinker /workspace/rust/capi/target/debug \
      "${GTS_SWIFT_SMOKE_TARGET}" \
      "${GTS_SWIFT_VECTOR}"'
fi

echo "Swift C ABI wrapper smoke test passed"

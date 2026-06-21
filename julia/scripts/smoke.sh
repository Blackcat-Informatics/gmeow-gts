#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
VECTOR="vectors/01-minimal.gts"
JULIA_IMAGE="${JULIA_IMAGE:-gmeow-gts-julia-smoke:1.12.2}"
JULIA_PKG="${JULIA_PKG:-${TMPDIR:-/tmp}/gmeowgts-julia-package}"
export JULIA_NUM_THREADS="${JULIA_NUM_THREADS:-4}"

cd "${ROOT}"

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
  rm -rf "${JULIA_PKG}"
  cp -R julia "${JULIA_PKG}"
  GTS_JULIA_VECTOR="${ROOT}/${VECTOR}" julia --project="${JULIA_PKG}" -e 'using Pkg; Pkg.test()'
}

if [[ "${GTS_JULIA_FORCE_DOCKER:-0}" != "1" ]] && command -v julia >/dev/null 2>&1; then
  export GTS_LIBGTS="${LIB_PATH}"
  export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
  run_smoke
else
  if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Julia is unavailable locally and the Docker fallback requires a Linux libgts build." >&2
    exit 1
  fi
  docker build -t "${JULIA_IMAGE}" julia
  docker run --rm \
    -e GTS_LIBGTS=/workspace/rust/capi/target/debug/libgts.so \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -e JULIA_NUM_THREADS="${JULIA_NUM_THREADS}" \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${JULIA_IMAGE}" \
    sh -c 'rm -rf /tmp/gmeowgts-julia-package && cp -R julia /tmp/gmeowgts-julia-package && GTS_JULIA_VECTOR=/workspace/vectors/01-minimal.gts julia --project=/tmp/gmeowgts-julia-package -e "using Pkg; Pkg.test()"'
fi

echo "Julia C ABI wrapper smoke test passed"

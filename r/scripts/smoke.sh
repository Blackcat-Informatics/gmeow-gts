#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
R_IMAGE="${R_IMAGE:-gmeow-gts-r-smoke:4.5.1}"
R_LIB="${R_LIB:-${TMPDIR:-/tmp}/gmeowgts-r-lib}"
R_PKG="${R_PKG:-${TMPDIR:-/tmp}/gmeowgts-r-package}"
R_CHECK="${R_CHECK:-${TMPDIR:-/tmp}/gmeowgts-r-check}"
# shellcheck source=/dev/null
source "${ROOT}/scripts/wrapper_smoke_matrix.sh"

cd "${ROOT}"

diff -u rust/capi/include/gts.h r/src/gts.h
cargo build --manifest-path "${CAPI}/Cargo.toml"

run_smoke() {
  rm -rf "${R_LIB}" "${R_PKG}" "${R_CHECK}"
  mkdir -p "${R_LIB}" "${R_CHECK}"
  cp -R r "${R_PKG}"
  GTS_LIB_DIR="${CAPI_TARGET}" R CMD INSTALL --library="${R_LIB}" "${R_PKG}"
  R_LIBS_USER="${R_LIB}" Rscript r/tests/smoke.R \
    "${GTS_WRAPPER_CLEAN_VECTOR}" \
    "${GTS_WRAPPER_DAMAGED_VECTOR}" \
    "${GTS_WRAPPER_EMPTY_VECTOR}"
  (
    cd "${R_CHECK}"
    GTS_LIB_DIR="${CAPI_TARGET}" R CMD build "${R_PKG}"
    GTS_LIB_DIR="${CAPI_TARGET}" R CMD check --no-manual --library="${R_LIB}" gmeowgts_*.tar.gz
  )
}

if [[ "${GTS_R_FORCE_DOCKER:-0}" != "1" ]] && command -v R >/dev/null 2>&1 && command -v Rscript >/dev/null 2>&1; then
  export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
  run_smoke
else
  if [[ "$(uname -s)" != "Linux" ]]; then
    echo "R is unavailable locally and the Docker fallback requires a Linux libgts build." >&2
    exit 1
  fi
  docker build -t "${R_IMAGE}" r
  docker run --rm \
    -e GTS_LIB_DIR=/workspace/rust/capi/target/debug \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -e GTS_WRAPPER_BAD_NQUADS="${GTS_WRAPPER_BAD_NQUADS}" \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${R_IMAGE}" \
    sh -c 'rm -rf /tmp/gmeowgts-r-lib /tmp/gmeowgts-r-package /tmp/gmeowgts-r-check && mkdir -p /tmp/gmeowgts-r-lib /tmp/gmeowgts-r-check && cp -R r /tmp/gmeowgts-r-package && R CMD INSTALL --library=/tmp/gmeowgts-r-lib /tmp/gmeowgts-r-package && R_LIBS_USER=/tmp/gmeowgts-r-lib Rscript r/tests/smoke.R /workspace/vectors/01-minimal.gts /workspace/vectors/04-damaged-frame.gts /workspace/vectors/28-empty-file.gts && cd /tmp/gmeowgts-r-check && R CMD build /tmp/gmeowgts-r-package && R CMD check --no-manual --library=/tmp/gmeowgts-r-lib gmeowgts_*.tar.gz'
fi

echo "R C ABI wrapper smoke test passed"

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CAPI="${ROOT}/rust/capi"
TARGET="${CAPI}/target/debug"
# shellcheck source=/dev/null
source "${ROOT}/scripts/wrapper_smoke_matrix.sh"

cargo build --manifest-path "${CAPI}/Cargo.toml"

cc -std=c11 -Wall -Wextra -Werror \
  -D_GNU_SOURCE \
  -I"${CAPI}/include" \
  "${CAPI}/examples/smoke.c" \
  -L"${TARGET}" -lgts \
  -Wl,-rpath,"${TARGET}" \
  -o "${TARGET}/gts-capi-smoke"

LD_LIBRARY_PATH="${TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
  "${TARGET}/gts-capi-smoke" \
  "${GTS_WRAPPER_CLEAN_VECTOR}" \
  "${GTS_WRAPPER_DAMAGED_VECTOR}" \
  "${GTS_WRAPPER_EMPTY_VECTOR}"

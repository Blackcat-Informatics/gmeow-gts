#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CPP="${ROOT}/cpp"
CAPI_TARGET="${CAPI}/target/debug"
CXX_BIN="${CXX:-c++}"

cargo build --manifest-path "${CAPI}/Cargo.toml"

"${CXX_BIN}" -std=c++17 -Wall -Wextra -Werror \
  -I"${CPP}/include" \
  -I"${CAPI}/include" \
  "${CPP}/examples/smoke.cpp" \
  -L"${CAPI_TARGET}" -lgts \
  -Wl,-rpath,"${CAPI_TARGET}" \
  -o "${CAPI_TARGET}/gts-cpp-smoke"

LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
  "${CAPI_TARGET}/gts-cpp-smoke" "${ROOT}/vectors/01-minimal.gts"

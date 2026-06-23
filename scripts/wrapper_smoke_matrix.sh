#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

# Source this file from C ABI wrapper smoke scripts. It defines the shared
# observable fixture matrix that all wrappers assert against.

if [ -z "${ROOT:-}" ]; then
  ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fi

export GTS_WRAPPER_SMOKE_MATRIX_ID="${GTS_WRAPPER_SMOKE_MATRIX_ID:-wrapper-smoke-v1}"

export GTS_WRAPPER_CLEAN_FIXTURE="${GTS_WRAPPER_CLEAN_FIXTURE:-clean-read}"
export GTS_WRAPPER_CLEAN_VECTOR="${GTS_WRAPPER_CLEAN_VECTOR:-${ROOT}/vectors/01-minimal.gts}"

export GTS_WRAPPER_DAMAGED_FIXTURE="${GTS_WRAPPER_DAMAGED_FIXTURE:-damaged-diagnostic-read}"
export GTS_WRAPPER_DAMAGED_VECTOR="${GTS_WRAPPER_DAMAGED_VECTOR:-${ROOT}/vectors/04-damaged-frame.gts}"

export GTS_WRAPPER_EMPTY_FIXTURE="${GTS_WRAPPER_EMPTY_FIXTURE:-empty-malformed-refusal}"
export GTS_WRAPPER_EMPTY_VECTOR="${GTS_WRAPPER_EMPTY_VECTOR:-${ROOT}/vectors/28-empty-file.gts}"

export GTS_WRAPPER_PARSE_FIXTURE="${GTS_WRAPPER_PARSE_FIXTURE:-malformed-nquads-refusal}"
export GTS_WRAPPER_BAD_NQUADS="${GTS_WRAPPER_BAD_NQUADS:-<https://example/s> <https://example/p> .
}"

export GTS_WRAPPER_EXPECT_ABI_VERSION="${GTS_WRAPPER_EXPECT_ABI_VERSION:-1}"
export GTS_WRAPPER_EXPECT_BUILD_SCHEMA="${GTS_WRAPPER_EXPECT_BUILD_SCHEMA:-gts-capi-build-v1}"
export GTS_WRAPPER_EXPECT_CAPABILITIES_SCHEMA="${GTS_WRAPPER_EXPECT_CAPABILITIES_SCHEMA:-gts-capi-capabilities-v1}"
export GTS_WRAPPER_EXPECT_READ_SCHEMA="${GTS_WRAPPER_EXPECT_READ_SCHEMA:-gts-capi-read-v1}"
export GTS_WRAPPER_EXPECT_VERIFY_SCHEMA="${GTS_WRAPPER_EXPECT_VERIFY_SCHEMA:-gts-capi-verify-v1}"
export GTS_WRAPPER_EXPECT_CLEAN_NQUADS_NEEDLE="${GTS_WRAPPER_EXPECT_CLEAN_NQUADS_NEEDLE:-\"Cat\"@en}"
export GTS_WRAPPER_EXPECT_DAMAGED_DIAGNOSTIC="${GTS_WRAPPER_EXPECT_DAMAGED_DIAGNOSTIC:-DamagedFrame}"
export GTS_WRAPPER_EXPECT_EMPTY_DIAGNOSTIC="${GTS_WRAPPER_EXPECT_EMPTY_DIAGNOSTIC:-EmptyFile}"
export GTS_WRAPPER_EXPECT_FILE_TEXT="${GTS_WRAPPER_EXPECT_FILE_TEXT:-hello
}"

wrapper_smoke_args() {
  printf '%s\n' \
    "${GTS_WRAPPER_CLEAN_VECTOR}" \
    "${GTS_WRAPPER_DAMAGED_VECTOR}" \
    "${GTS_WRAPPER_EMPTY_VECTOR}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  env | grep '^GTS_WRAPPER_' | sort
fi

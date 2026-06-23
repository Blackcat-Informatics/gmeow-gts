#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
SMOKE_PROJECT="dotnet/Gmeow.Gts.Smoke/Gmeow.Gts.Smoke.csproj"
DOTNET_SDK_IMAGE="${DOTNET_SDK_IMAGE:-mcr.microsoft.com/dotnet/sdk:8.0@sha256:d80fdd84f7e18eea12f8e45c52914f1353395009c95c41197178ea19944e6d48}"
# shellcheck source=/dev/null
source "${ROOT}/scripts/wrapper_smoke_matrix.sh"

cd "${ROOT}"

cargo build --manifest-path "${CAPI}/Cargo.toml"

run_smoke() {
  dotnet run --project "${SMOKE_PROJECT}" -- \
    "${GTS_WRAPPER_CLEAN_VECTOR}" \
    "${GTS_WRAPPER_DAMAGED_VECTOR}" \
    "${GTS_WRAPPER_EMPTY_VECTOR}"
}

if command -v dotnet >/dev/null 2>&1; then
  export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
  run_smoke
else
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e DOTNET_CLI_HOME=/tmp/dotnet-cli \
    -e NUGET_PACKAGES=/tmp/nuget-packages \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -e GTS_WRAPPER_BAD_NQUADS="${GTS_WRAPPER_BAD_NQUADS}" \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${DOTNET_SDK_IMAGE}" \
    dotnet run --project "${SMOKE_PROJECT}" -- \
    /workspace/vectors/01-minimal.gts \
    /workspace/vectors/04-damaged-frame.gts \
    /workspace/vectors/28-empty-file.gts
fi

echo ".NET C ABI wrapper smoke test passed"

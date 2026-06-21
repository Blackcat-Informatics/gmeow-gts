#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
SMOKE="php/tests/smoke.php"
VECTOR="vectors/01-minimal.gts"
COMPOSER_IMAGE="${COMPOSER_IMAGE:-composer@sha256:7725eb4545c438629ae8bde3ef0bb9a5038ef566126ad878442a69007242d267}"
PHP_FFI_IMAGE="${PHP_FFI_IMAGE:-gmeow-gts-php-ffi-smoke:8.4}"

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

if command -v composer >/dev/null 2>&1; then
  composer validate --strict php/composer.json
else
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e COMPOSER_HOME=/tmp/composer \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    --entrypoint composer \
    "${COMPOSER_IMAGE}" \
    validate --strict php/composer.json
fi

has_php_ffi() {
  command -v php >/dev/null 2>&1 && php -m | grep -Eq '^FFI$'
}

if has_php_ffi; then
  export GTS_LIBGTS="${LIB_PATH}"
  export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
  php -d ffi.enable=1 "${SMOKE}" "${VECTOR}"
else
  if [[ "$(uname -s)" != "Linux" ]]; then
    echo "PHP FFI is unavailable locally and the Docker fallback requires a Linux libgts build." >&2
    exit 1
  fi
  docker build -t "${PHP_FFI_IMAGE}" php
  docker run --rm \
    -e GTS_LIBGTS=/workspace/rust/capi/target/debug/libgts.so \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${PHP_FFI_IMAGE}" \
    php -d ffi.enable=1 php/tests/smoke.php vectors/01-minimal.gts
fi

echo "PHP C ABI wrapper smoke test passed"

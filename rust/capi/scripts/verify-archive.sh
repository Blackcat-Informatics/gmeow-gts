#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 dist/capi/gmeow-gts-capi_<version>_<os>_<arch>.tar.gz" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
archive="$1"
tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

tar -C "${tmp}" -xzf "${archive}"
prefix="$(find "${tmp}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [ -z "${prefix}" ]; then
  echo "archive did not contain a top-level directory" >&2
  exit 1
fi

required=(
  "README.md"
  "include/gts.h"
  "include/gts/gts.hpp"
  "lib/pkgconfig/gts.pc"
  "lib/cmake/Gts/GtsConfig.cmake"
  "licenses/LICENSE-MIT"
  "licenses/LICENSE-APACHE"
  "share/gts/archive.json"
  "share/gts/VERSION"
  "share/gts/ABI_VERSION"
)
for rel in "${required[@]}"; do
  if [ ! -e "${prefix}/${rel}" ]; then
    echo "missing archive member: ${rel}" >&2
    exit 1
  fi
done

if ! find "${prefix}/lib" "${prefix}/bin" -maxdepth 1 -type f \( \
  -name 'libgts.so' -o \
  -name 'libgts.dylib' -o \
  -name 'gts.dll' \
\) | grep -q .; then
  echo "archive does not contain a dynamic libgts library" >&2
  exit 1
fi

if [ -n "${PKG_CONFIG_PATH-}" ]; then
  export PKG_CONFIG_PATH="${prefix}/lib/pkgconfig:${PKG_CONFIG_PATH}"
else
  export PKG_CONFIG_PATH="${prefix}/lib/pkgconfig"
fi
pkg-config --cflags --libs gts >/dev/null

case "$(uname -s)" in
  Linux*) library_env=LD_LIBRARY_PATH ;;
  Darwin*) library_env=DYLD_LIBRARY_PATH ;;
  MINGW*|MSYS*|CYGWIN*) library_env=PATH ;;
  *)
    echo "Unsupported verifier platform: $(uname -s)" >&2
    exit 1
    ;;
esac

if [ "${library_env}" = PATH ]; then
  export PATH="${prefix}/bin:${PATH}"
else
  current_library_path="${!library_env-}"
  if [ -n "${current_library_path}" ]; then
    export "${library_env}=${prefix}/lib:${current_library_path}"
  else
    export "${library_env}=${prefix}/lib"
  fi
fi

cat > "${tmp}/cmake-smoke.c" <<'EOF'
#include "gts.h"

int main(void) {
  return gts_abi_version() == GTS_ABI_VERSION ? 0 : 1;
}
EOF
cat > "${tmp}/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.16)
project(gts_capi_archive_smoke C)
find_package(Gts REQUIRED)
add_executable(cmake-smoke cmake-smoke.c)
target_link_libraries(cmake-smoke PRIVATE Gts::gts)
EOF
cmake -S "${tmp}" -B "${tmp}/build" -DCMAKE_PREFIX_PATH="${prefix}" >/dev/null
cmake --build "${tmp}/build" >/dev/null
"${tmp}/build/cmake-smoke"

cc -std=c11 -Wall -Wextra -Werror \
  -D_GNU_SOURCE \
  $(pkg-config --cflags gts) \
  "${ROOT}/rust/capi/examples/smoke.c" \
  $(pkg-config --libs gts) \
  -Wl,-rpath,"${prefix}/lib" \
  -o "${tmp}/gts-capi-smoke"
"${tmp}/gts-capi-smoke" "${ROOT}/vectors/01-minimal.gts"

echo "archive verification OK: ${archive}"

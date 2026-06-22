#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CAPI="${ROOT}/rust/capi"
TARGET="${CAPI}/target/release"
DIST="${ROOT}/dist/capi"

metadata_value() {
  cargo metadata --manifest-path "${CAPI}/Cargo.toml" --no-deps --format-version 1 \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['packages'][0]['$1'])"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1"
  else
    echo "sha256sum or shasum is required" >&2
    exit 1
  fi
}

VERSION="${GTS_CAPI_VERSION:-$(metadata_value version)}"
ABI_VERSION="$(
  sed -n 's/^#define GTS_ABI_VERSION \([0-9][0-9]*\).*/\1/p' "${CAPI}/include/gts.h" \
    | head -n 1
)"
if [ -z "${ABI_VERSION}" ]; then
  echo "Could not determine GTS_ABI_VERSION from rust/capi/include/gts.h" >&2
  exit 1
fi

case "${GTS_CAPI_OS:-$(uname -s)}" in
  Linux*) os=linux ;;
  Darwin*) os=macos ;;
  MINGW*|MSYS*|CYGWIN*|Windows_NT) os=windows ;;
  *) os="$(uname -s | tr '[:upper:]' '[:lower:]')" ;;
esac

case "${GTS_CAPI_ARCH:-$(uname -m)}" in
  x86_64|amd64) arch=x86_64 ;;
  aarch64|arm64) arch=arm64 ;;
  *) arch="$(uname -m | tr '[:upper:]' '[:lower:]')" ;;
esac

cargo build --manifest-path "${CAPI}/Cargo.toml" --release --locked

base="gmeow-gts-capi_${VERSION}_${os}_${arch}"
stage="${DIST}/${base}"
archive="${DIST}/${base}.tar.gz"
rm -rf "${stage}" "${archive}" "${archive}.sha256"
mkdir -p \
  "${stage}/bin" \
  "${stage}/docs" \
  "${stage}/include/gts" \
  "${stage}/lib/cmake/Gts" \
  "${stage}/lib/pkgconfig" \
  "${stage}/licenses" \
  "${stage}/share/gts"

cp "${CAPI}/include/gts.h" "${stage}/include/gts.h"
cp "${ROOT}/cpp/include/gts/gts.hpp" "${stage}/include/gts/gts.hpp"
cp "${CAPI}/cmake/GtsConfig.cmake" "${stage}/lib/cmake/Gts/GtsConfig.cmake"
cp "${CAPI}/README.md" "${stage}/README.md"
cp "${ROOT}/LICENSE-MIT" "${ROOT}/LICENSE-APACHE" "${ROOT}/LICENSING.md" "${stage}/licenses/"
cp -R "${ROOT}/LICENSES" "${stage}/licenses/LICENSES"

cat > "${stage}/lib/pkgconfig/gts.pc" <<EOF
prefix=\${pcfiledir}/../..
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: gts
Description: Graph Transport Substrate Rust C ABI
Version: ${VERSION}
Libs: -L\${libdir} -lgts
Cflags: -I\${includedir}
EOF

cat > "${stage}/share/gts/archive.json" <<EOF
{
  "schema": "gts-capi-archive-v1",
  "package": "gmeow-gts-capi",
  "version": "${VERSION}",
  "abi_version": ${ABI_VERSION},
  "os": "${os}",
  "arch": "${arch}"
}
EOF
printf '%s\n' "${VERSION}" > "${stage}/share/gts/VERSION"
printf '%s\n' "${ABI_VERSION}" > "${stage}/share/gts/ABI_VERSION"

copy_if_present() {
  local source="$1"
  local dest_dir="$2"
  if [ -f "${source}" ]; then
    cp "${source}" "${dest_dir}/"
  fi
}

copy_if_present "${TARGET}/libgts.so" "${stage}/lib"
copy_if_present "${TARGET}/libgts.dylib" "${stage}/lib"
copy_if_present "${TARGET}/libgts.a" "${stage}/lib"
copy_if_present "${TARGET}/gts.dll" "${stage}/bin"
copy_if_present "${TARGET}/gts.dll.lib" "${stage}/lib"
copy_if_present "${TARGET}/gts.lib" "${stage}/lib"
copy_if_present "${TARGET}/gts.pdb" "${stage}/bin"

if ! find "${stage}/lib" "${stage}/bin" -maxdepth 1 -type f \( \
  -name 'libgts.so' -o \
  -name 'libgts.dylib' -o \
  -name 'gts.dll' \
\) | grep -q .; then
  echo "No dynamic C ABI library was produced in ${TARGET}" >&2
  exit 1
fi

tar -C "${DIST}" -czf "${archive}" "${base}"
(
  cd "${DIST}"
  sha256_file "${base}.tar.gz" > "${base}.tar.gz.sha256"
)
printf '%s\n' "${archive}"

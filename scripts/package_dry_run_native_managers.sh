#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${GTS_NATIVE_PACKAGE_DRY_RUN_OUT:-${ROOT}/dist/native-package-dry-runs}"
TMP="$(mktemp -d "${ROOT}/.native-package-dry-run.XXXXXXXXXX")"
trap 'rm -rf "${TMP}"' EXIT
cd "${ROOT}"

log() {
  printf '\n==> %s\n' "$*"
}

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "$1 is required for native package-manager dry-runs" >&2
    exit 1
  fi
}

metadata_value() {
  cargo metadata --manifest-path rust/capi/Cargo.toml --no-deps --format-version 1 \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['packages'][0]['$1'])"
}

library_path_entries() {
  local prefix="$1"
  case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) printf '%s' "${prefix}/bin" ;;
    *) printf '%s' "${prefix}/lib:${prefix}/bin" ;;
  esac
}

run_with_loader_path() {
  local prefix="$1"
  shift
  case "$(uname -s)" in
    Darwin*)
      DYLD_LIBRARY_PATH="$(library_path_entries "${prefix}")${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" "$@"
      ;;
    MINGW* | MSYS* | CYGWIN*)
      PATH="$(library_path_entries "${prefix}")${PATH:+:${PATH}}" "$@"
      ;;
    *)
      LD_LIBRARY_PATH="$(library_path_entries "${prefix}")${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" "$@"
      ;;
  esac
}

require_tool cargo
require_tool cmake
require_tool conan
require_tool ninja
require_tool python3
require_tool vcpkg

rm -rf "${OUT}"
mkdir -p "${OUT}/conan" "${OUT}/vcpkg"

VERSION="$(metadata_value version)"
CONAN_HOME="${GTS_CONAN_HOME:-${TMP}/conan-home}"
export CONAN_HOME

log "Conan package create"
conan profile detect --force
conan create . --version "${VERSION}" --build=missing --settings=build_type=Release

log "Conan CMake consumer"
conan_build="${TMP}/conan-consumer-build"
mkdir -p "${conan_build}"
conan install \
  --requires "gmeow-gts/${VERSION}" \
  --output-folder "${conan_build}" \
  --build=missing \
  --settings=build_type=Release \
  --generator CMakeDeps \
  --generator CMakeToolchain
cmake -S packaging/native-consumer -B "${conan_build}" \
  -GNinja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE="${conan_build}/conan_toolchain.cmake"
cmake --build "${conan_build}"
(
  set +u
  if [ -f "${conan_build}/conanrun.sh" ]; then
    # shellcheck source=/dev/null
    source "${conan_build}/conanrun.sh"
  fi
  "${conan_build}/gts-native-consumer" vectors/01-minimal.gts
)
printf '%s\n' "conan create . --version ${VERSION}" > "${OUT}/conan/create.txt"
printf '%s\n' "conan install packaging/native-consumer --requires gmeow-gts/${VERSION}" > "${OUT}/conan/consumer.txt"

log "vcpkg overlay install"
triplet="${GTS_VCPKG_TRIPLET:-x64-linux-dynamic}"
vcpkg_root="${VCPKG_ROOT:-}"
if [ -z "${vcpkg_root}" ]; then
  vcpkg_bin="$(command -v vcpkg)"
  vcpkg_dir="$(cd "$(dirname "${vcpkg_bin}")" && pwd)"
  if [ -f "${vcpkg_dir}/scripts/buildsystems/vcpkg.cmake" ]; then
    vcpkg_root="${vcpkg_dir}"
  elif [ "$(basename "${vcpkg_dir}")" = "bin" ] \
    && [ -f "${vcpkg_dir}/../scripts/buildsystems/vcpkg.cmake" ]; then
    vcpkg_root="$(cd "${vcpkg_dir}/.." && pwd)"
  else
    vcpkg_root="${vcpkg_dir}"
  fi
fi
if [ ! -f "${vcpkg_root}/scripts/buildsystems/vcpkg.cmake" ]; then
  echo "could not locate vcpkg.cmake under VCPKG_ROOT=${vcpkg_root}" >&2
  exit 1
fi
vcpkg_installed="${TMP}/vcpkg-installed"
GMEOW_GTS_SOURCE_PATH="${ROOT}" \
  vcpkg install gmeow-gts \
    --overlay-ports="${ROOT}/packaging/vcpkg/ports" \
    --triplet="${triplet}" \
    --x-install-root="${vcpkg_installed}" \
    --x-buildtrees-root="${TMP}/vcpkg-buildtrees" \
    --x-packages-root="${TMP}/vcpkg-packages" \
    --downloads-root="${TMP}/vcpkg-downloads"

log "vcpkg CMake consumer"
vcpkg_build="${TMP}/vcpkg-consumer-build"
cmake -S packaging/native-consumer -B "${vcpkg_build}" \
  -GNinja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_TOOLCHAIN_FILE="${vcpkg_root}/scripts/buildsystems/vcpkg.cmake" \
  -DVCPKG_INSTALLED_DIR="${vcpkg_installed}" \
  -DVCPKG_TARGET_TRIPLET="${triplet}"
cmake --build "${vcpkg_build}"
run_with_loader_path "${vcpkg_installed}/${triplet}" \
  "${vcpkg_build}/gts-native-consumer" vectors/01-minimal.gts
printf '%s\n' "vcpkg install gmeow-gts --overlay-ports=packaging/vcpkg/ports --triplet=${triplet}" \
  > "${OUT}/vcpkg/install.txt"
printf '%s\n' "cmake -S packaging/native-consumer -B <build> -DCMAKE_TOOLCHAIN_FILE=<vcpkg.cmake>" \
  > "${OUT}/vcpkg/consumer.txt"

log "Native package-manager dry-runs completed"
find "${OUT}" -maxdepth 2 -type f | sort

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CAPI="${ROOT}/rust/capi"
OUT="${GTS_PACKAGE_DRY_RUN_OUT:-${ROOT}/dist/package-dry-runs}"
TMP="$(mktemp -d "${ROOT}/.package-dry-run.XXXXXXXXXX")"
CACHE="${TMP}/cache"
ARCHIVE_TMP="${TMP}/archive"
trap 'rm -rf "${TMP}"' EXIT
cd "${ROOT}"

case "${OUT}" in
  "${ROOT}/dist"/*)
    OUT_REL="${OUT#"${ROOT}/"}"
    ;;
  *)
    echo "GTS_PACKAGE_DRY_RUN_OUT must be inside ${ROOT}/dist for Docker-backed dry-runs" >&2
    exit 1
    ;;
esac

DOTNET_SDK_IMAGE="${DOTNET_SDK_IMAGE:-mcr.microsoft.com/dotnet/sdk:8.0@sha256:d80fdd84f7e18eea12f8e45c52914f1353395009c95c41197178ea19944e6d48}"
COMPOSER_IMAGE="${COMPOSER_IMAGE:-composer@sha256:7725eb4545c438629ae8bde3ef0bb9a5038ef566126ad878442a69007242d267}"
PHP_FFI_IMAGE="${PHP_FFI_IMAGE:-gmeow-gts-php-ffi-smoke:8.4}"
LUAJIT_IMAGE="${LUAJIT_IMAGE:-gmeow-gts-luajit-package-dry-run:2.1}"
SWIFT_IMAGE="${SWIFT_IMAGE:-swift@sha256:4e50a9e711e8682a8c42bacfeed204568adfd6985a63b3789a165f28d296a28a}"
RUBY_IMAGE="${RUBY_IMAGE:-gmeow-gts-ruby-package-dry-run:3.4}"
R_IMAGE="${R_IMAGE:-gmeow-gts-r-package-dry-run:4.5.1}"
JULIA_IMAGE="${JULIA_IMAGE:-gmeow-gts-julia-package-dry-run:1.12.2}"

log() {
  printf '\n==> %s\n' "$*"
}

target_directory() {
  cargo metadata --manifest-path "${CAPI}/Cargo.toml" --no-deps --format-version 1 \
    | python3 -c "import json,sys; print(json.load(sys.stdin)['target_directory'])"
}

container_path() {
  case "$1" in
    "${ROOT}"/*)
      printf '/workspace/%s' "${1#"${ROOT}/"}"
      ;;
    "${ROOT}")
      printf '/workspace'
      ;;
    *)
      printf '%s' "$1"
      ;;
  esac
}

require_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required for package dry-run fallbacks on this host" >&2
    exit 1
  fi
}

docker_run() {
  local image="$1"
  shift
  require_docker
  docker run --rm \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp/gts-package-home \
    -e DOTNET_CLI_HOME="$(container_path "${CACHE}/dotnet-cli")" \
    -e NUGET_HTTP_CACHE_PATH="$(container_path "${CACHE}/nuget-http-cache")" \
    -e NUGET_PACKAGES="$(container_path "${CACHE}/nuget-packages")" \
    -e NUGET_SCRATCH="$(container_path "${CACHE}/nuget-scratch")" \
    -e XDG_DATA_HOME="$(container_path "${CACHE}/xdg-data")" \
    -e GEM_HOME=/tmp/gts-gems \
    -e GEM_PATH=/tmp/gts-gems:/usr/local/bundle \
    -e GTS_WORKSPACE=/workspace \
    -e GTS_PACKAGE_DRY_RUN_OUT="$(container_path "${OUT}")" \
    -e GTS_LIBGTS="${LIB_PATH_CONTAINER}" \
    -e GTS_CAPI_TARGET="${CAPI_TARGET_CONTAINER}" \
    -e GTS_LIB_DIR="${CAPI_TARGET_CONTAINER}" \
    -e LD_LIBRARY_PATH="${CAPI_TARGET_CONTAINER}" \
    -e DYLD_LIBRARY_PATH="${CAPI_TARGET_CONTAINER}" \
    -e LIBRARY_PATH="${CAPI_TARGET_CONTAINER}" \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${image}" \
    "$@"
}

docker_build() {
  local image="$1"
  local context="$2"
  require_docker
  docker build -t "${image}" "${context}"
}

run_dotnet() {
  if [ "${GTS_PACKAGE_DRY_RUN_USE_LOCAL_DOTNET:-0}" = "1" ]; then
    if ! command -v dotnet >/dev/null 2>&1; then
      echo "GTS_PACKAGE_DRY_RUN_USE_LOCAL_DOTNET=1 requires dotnet on PATH" >&2
      exit 1
    fi
    DOTNET_CLI_HOME="${CACHE}/dotnet-cli" \
      NUGET_HTTP_CACHE_PATH="${CACHE}/nuget-http-cache" \
      NUGET_PACKAGES="${CACHE}/nuget-packages" \
      NUGET_SCRATCH="${CACHE}/nuget-scratch" \
      XDG_DATA_HOME="${CACHE}/xdg-data" \
      LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" \
      dotnet "$@"
  else
    docker_run "${DOTNET_SDK_IMAGE}" dotnet "$@"
  fi
}

run_composer() {
  if command -v composer >/dev/null 2>&1; then
    COMPOSER_HOME="${TMP}/composer" composer "$@"
  else
    docker_run "${COMPOSER_IMAGE}" composer "$@"
  fi
}

has_php_ffi() {
  command -v php >/dev/null 2>&1 && php -m 2>/dev/null | grep -Eq '^FFI$'
}

run_php() {
  if has_php_ffi; then
    php "$@"
  else
    docker_build "${PHP_FFI_IMAGE}" php
    docker_run "${PHP_FFI_IMAGE}" php "$@"
  fi
}

run_swift_shell() {
  local command="$1"
  if command -v swift >/dev/null 2>&1; then
    GTS_WORKSPACE="${ROOT}" \
      GTS_PACKAGE_DRY_RUN_OUT="${OUT}" \
      GTS_CAPI_TARGET="${CAPI_TARGET}" \
    LIBRARY_PATH="${CAPI_TARGET}${LIBRARY_PATH:+:${LIBRARY_PATH}}" \
      LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" \
      bash -lc "${command}"
  else
    docker_run "${SWIFT_IMAGE}" bash -lc "${command}"
  fi
}

run_lua_shell() {
  local command="$1"
  if command -v luarocks >/dev/null 2>&1 && command -v luajit >/dev/null 2>&1; then
    GTS_WORKSPACE="${ROOT}" \
      GTS_PACKAGE_DRY_RUN_OUT="${OUT}" \
      GTS_LIBGTS="${LIB_PATH}" \
      GTS_CAPI_TARGET="${CAPI_TARGET}" \
      LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" \
      bash -lc "${command}"
  else
    docker_build "${LUAJIT_IMAGE}" lua
    docker_run "${LUAJIT_IMAGE}" bash -lc "${command}"
  fi
}

run_ruby_shell() {
  local command="$1"
  if command -v ruby >/dev/null 2>&1 && ruby -rffi -e 'exit 0' >/dev/null 2>&1; then
    local gem_home="${TMP}/ruby-gems"
    local gem_path
    mkdir -p "${gem_home}"
    gem_path="$(GEM_HOME="${gem_home}" ruby -e 'puts Gem.path.join(":")')"
    GEM_HOME="${gem_home}" \
      GEM_PATH="${gem_path}" \
      GTS_WORKSPACE="${ROOT}" \
      GTS_PACKAGE_DRY_RUN_OUT="${OUT}" \
      GTS_LIBGTS="${LIB_PATH}" \
      GTS_CAPI_TARGET="${CAPI_TARGET}" \
      LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" \
      bash -lc "${command}"
  else
    docker_build "${RUBY_IMAGE}" ruby
    docker_run "${RUBY_IMAGE}" bash -lc "${command}"
  fi
}

run_r_shell() {
  local command="$1"
  if command -v R >/dev/null 2>&1 && command -v Rscript >/dev/null 2>&1; then
    GTS_WORKSPACE="${ROOT}" \
      GTS_PACKAGE_DRY_RUN_OUT="${OUT}" \
      GTS_CAPI_TARGET="${CAPI_TARGET}" \
      GTS_LIB_DIR="${CAPI_TARGET}" \
      LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" \
      bash -lc "${command}"
  else
    docker_build "${R_IMAGE}" r
    docker_run "${R_IMAGE}" bash -lc "${command}"
  fi
}

run_julia_shell() {
  local command="$1"
  if command -v julia >/dev/null 2>&1; then
    GTS_WORKSPACE="${ROOT}" \
      GTS_PACKAGE_DRY_RUN_OUT="${OUT}" \
      GTS_LIBGTS="${LIB_PATH}" \
      GTS_CAPI_TARGET="${CAPI_TARGET}" \
      LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}" \
      DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}" \
      julia -e "${command}"
  else
    docker_build "${JULIA_IMAGE}" julia
    docker_run "${JULIA_IMAGE}" julia -e "${command}"
  fi
}

case "$(uname -s)" in
  Darwin*) LIB_NAME="libgts.dylib" ;;
  MINGW* | MSYS* | CYGWIN*) LIB_NAME="gts.dll" ;;
  *) LIB_NAME="libgts.so" ;;
esac

rm -rf "${OUT}"
mkdir -p \
  "${ARCHIVE_TMP}" \
  "${CACHE}" \
  "${OUT}/capi" \
  "${OUT}/cpp" \
  "${OUT}/dotnet" \
  "${OUT}/lua" \
  "${OUT}/native" \
  "${OUT}/php" \
  "${OUT}/r" \
  "${OUT}/ruby" \
  "${OUT}/rust" \
  "${OUT}/swift"

log "Build shared C ABI library"
cargo build --manifest-path "${CAPI}/Cargo.toml"
CAPI_TARGET="$(target_directory)/debug"
LIB_PATH="${CAPI_TARGET}/${LIB_NAME}"
CAPI_TARGET_CONTAINER="$(container_path "${CAPI_TARGET}")"
LIB_PATH_CONTAINER="$(container_path "${LIB_PATH}")"
if [ ! -f "${LIB_PATH}" ]; then
  echo "expected C ABI library was not built: ${LIB_PATH}" >&2
  exit 1
fi

export GTS_LIBGTS="${LIB_PATH}"
export GTS_LIB_DIR="${CAPI_TARGET}"
export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
export LIBRARY_PATH="${CAPI_TARGET}${LIBRARY_PATH:+:${LIBRARY_PATH}}"

log "Rust C ABI cargo package list"
cargo package --manifest-path rust/capi/Cargo.toml --locked --list \
  > "${OUT}/rust/cargo-package-list.txt"
if grep -Eq '^publish = false$' rust/capi/Cargo.toml; then
  printf '%s\n' "cargo publish --dry-run skipped: rust/capi/Cargo.toml has publish = false" \
    > "${OUT}/rust/publish-dry-run.txt"
else
  cargo publish --manifest-path rust/capi/Cargo.toml --locked --dry-run
fi

log "C ABI release archive and installed C++ consumer"
archive="$(bash rust/capi/scripts/package.sh)"
bash rust/capi/scripts/verify-archive.sh "${archive}"
cp "${archive}" "${archive}.sha256" "${OUT}/capi/"
tar -C "${ARCHIVE_TMP}" -xzf "${archive}"
rm -rf "${archive%.tar.gz}" "${archive}" "${archive}.sha256"
archive_prefix="$(find "${ARCHIVE_TMP}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [ -z "${archive_prefix}" ]; then
  echo "archive did not contain a top-level directory" >&2
  exit 1
fi
pkg_cflags=()
pkg_libs=()
PKG_CONFIG_PATH="${archive_prefix}/lib/pkgconfig${PKG_CONFIG_PATH:+:${PKG_CONFIG_PATH}}"
export PKG_CONFIG_PATH
read -r -a pkg_cflags <<< "$(pkg-config --cflags gts)"
read -r -a pkg_libs <<< "$(pkg-config --libs gts)"
"${CXX:-c++}" -std=c++17 -Wall -Wextra -Werror \
  "${pkg_cflags[@]}" \
  cpp/examples/smoke.cpp \
  "${pkg_libs[@]}" \
  "-Wl,-rpath,${archive_prefix}/lib" \
  -o "${OUT}/cpp/gts-cpp-archive-smoke"
"${OUT}/cpp/gts-cpp-archive-smoke" vectors/01-minimal.gts

log "Conan and vcpkg native package-manager dry-runs"
GTS_NATIVE_PACKAGE_DRY_RUN_OUT="${OUT}/native" \
  bash scripts/package_dry_run_native_managers.sh

log ".NET pack and local NuGet consumer"
run_dotnet pack dotnet/Gmeow.Gts/Gmeow.Gts.csproj -c Release -o "${OUT_REL}/dotnet"
rm -rf "${OUT}/dotnet-consumer"
run_dotnet new console --force -n GtsPackageConsumer -o "${OUT_REL}/dotnet-consumer"
cat > "${OUT}/dotnet-consumer/Program.cs" <<'EOF'
using Gmeow.Gts;

if (Gts.AbiVersion == 0)
{
    throw new InvalidOperationException("ABI version was zero.");
}
if (string.IsNullOrEmpty(Gts.Version))
{
    throw new InvalidOperationException("GTS version was empty.");
}
Console.WriteLine(Gts.Version);
EOF
run_dotnet add "${OUT_REL}/dotnet-consumer/GtsPackageConsumer.csproj" package Gmeow.Gts \
  --source "${OUT_REL}/dotnet"
run_dotnet run --no-restore --project "${OUT_REL}/dotnet-consumer/GtsPackageConsumer.csproj"

log "PHP Composer validation"
run_composer validate --strict php/composer.json
printf '%s\n' "composer validate --strict php/composer.json" > "${OUT}/php/composer-validate.txt"

log "PHP Packagist package-root validation"
php_package_root="${OUT}/php/packagist-root"
php_consumer="${OUT}/php/consumer"
bash scripts/package_php_packagist_root.sh "${php_package_root}" > "${OUT}/php/packagist-root-files.txt"
run_composer validate --strict "${OUT_REL}/php/packagist-root/composer.json"
rm -rf "${php_consumer}"
mkdir -p "${php_consumer}"
cat > "${php_consumer}/composer.json" <<'EOF'
{
  "name": "gmeow-gts/php-packagist-dry-run",
  "description": "Local Composer consumer for the gmeow-gts PHP package dry-run.",
  "type": "project",
  "minimum-stability": "dev",
  "prefer-stable": true,
  "repositories": [
    {
      "type": "path",
      "url": "../packagist-root",
      "options": {
        "symlink": false
      }
    }
  ],
  "require": {
    "blackcatinformatics/gmeow-gts": "*"
  }
}
EOF
# The pinned Composer image used on hosts without Composer does not enable ext-ffi.
# The runtime check immediately below runs in the FFI-enabled PHP smoke image.
run_composer --working-dir "${OUT_REL}/php/consumer" install --no-interaction --no-progress --ignore-platform-req=ext-ffi
cat > "${php_consumer}/smoke.php" <<'EOF'
<?php
declare(strict_types=1);

use Gmeow\Gts\Gts;

require __DIR__ . '/vendor/autoload.php';

if ($argc !== 2) {
    fwrite(STDERR, "usage: php -d ffi.enable=1 smoke.php vectors/01-minimal.gts\n");
    exit(2);
}

$gts = Gts::load();
if ($gts->abiVersion() !== 1) {
    throw new RuntimeException(sprintf('Unexpected ABI version: %d', $gts->abiVersion()));
}
if ($gts->version() === '') {
    throw new RuntimeException('Empty library version.');
}

$input = file_get_contents($argv[1]);
if ($input === false) {
    throw new RuntimeException(sprintf('Unable to read vector: %s', $argv[1]));
}

$decoded = json_decode($gts->verifyJson($input), true, 512, JSON_THROW_ON_ERROR);
if (!is_array($decoded) || ($decoded['schema'] ?? null) !== 'gts-capi-verify-v1') {
    throw new RuntimeException('Verification smoke did not return the expected schema.');
}
EOF
run_php -d ffi.enable=1 "${OUT_REL}/php/consumer/smoke.php" vectors/01-minimal.gts
printf '%s\n' \
  "bash scripts/package_php_packagist_root.sh ${php_package_root}" \
  "composer validate --strict ${OUT_REL}/php/packagist-root/composer.json" \
  "composer --working-dir ${OUT_REL}/php/consumer install --no-interaction --no-progress --ignore-platform-req=ext-ffi" \
  "php -d ffi.enable=1 ${OUT_REL}/php/consumer/smoke.php vectors/01-minimal.gts" \
  > "${OUT}/php/packagist-consumer.txt"

log "LuaRocks lint, make, and pack"
# shellcheck disable=SC2016 # expanded inside the local/container shell.
run_lua_shell 'set -euo pipefail
mkdir -p "${GTS_PACKAGE_DRY_RUN_OUT}/lua"
cd "${GTS_WORKSPACE}/lua"
luarocks lint gmeow-gts-dev-1.rockspec
rm -rf /tmp/gts-luarocks
luarocks make gmeow-gts-dev-1.rockspec --tree /tmp/gts-luarocks
rm -f gmeow-gts-dev-1.all.rock
luarocks --tree /tmp/gts-luarocks pack gmeow-gts dev-1
cp ./*.rock "${GTS_PACKAGE_DRY_RUN_OUT}/lua/"
rm -f gmeow-gts-dev-1.all.rock'

log "Swift package dump and smoke executable"
# shellcheck disable=SC2016 # expanded inside the local/container shell.
run_swift_shell 'set -euo pipefail
mkdir -p "${GTS_PACKAGE_DRY_RUN_OUT}/swift"
swift package dump-package --package-path "${GTS_WORKSPACE}/swift" > "${GTS_PACKAGE_DRY_RUN_OUT}/swift/package.json"
swift run \
  --package-path "${GTS_WORKSPACE}/swift" \
  --scratch-path /tmp/gts-swift-package-dry-run \
  -Xlinker "-L${GTS_CAPI_TARGET}" \
  -Xlinker -rpath \
  -Xlinker "${GTS_CAPI_TARGET}" \
  GmeowGTSSmoke \
  "${GTS_WORKSPACE}/vectors/01-minimal.gts"'

log "Ruby gem build, install, and installed-gem load"
# shellcheck disable=SC2016 # expanded inside the local/container shell.
run_ruby_shell 'set -euo pipefail
mkdir -p "${GTS_PACKAGE_DRY_RUN_OUT}/ruby"
cd "${GTS_WORKSPACE}/ruby"
gem build gmeow-gts.gemspec --output "${GTS_PACKAGE_DRY_RUN_OUT}/ruby/gmeow-gts.gem" >/dev/null
gem install --local "${GTS_PACKAGE_DRY_RUN_OUT}/ruby/gmeow-gts.gem" --no-document
ruby <<RUBY
require "gmeow/gts"

gts = Gmeow::Gts.load
raise "bad ABI" unless gts.abi_version == Gmeow::Gts::ABI_VERSION
raise "empty version" if gts.version.empty?
RUBY'

log "R CMD build and check"
# shellcheck disable=SC2016 # expanded inside the local/container shell.
run_r_shell 'set -euo pipefail
rm -rf /tmp/gmeowgts-r-lib /tmp/gmeowgts-r-package "${GTS_PACKAGE_DRY_RUN_OUT}/r"
mkdir -p /tmp/gmeowgts-r-lib "${GTS_PACKAGE_DRY_RUN_OUT}/r"
cp -R "${GTS_WORKSPACE}/r" /tmp/gmeowgts-r-package
GTS_LIB_DIR="${GTS_CAPI_TARGET}" R CMD INSTALL --library=/tmp/gmeowgts-r-lib /tmp/gmeowgts-r-package
cd "${GTS_PACKAGE_DRY_RUN_OUT}/r"
GTS_LIB_DIR="${GTS_CAPI_TARGET}" R CMD build /tmp/gmeowgts-r-package
GTS_LIB_DIR="${GTS_CAPI_TARGET}" R CMD check --no-manual --library=/tmp/gmeowgts-r-lib gmeowgts_*.tar.gz'

log "Julia package instantiate and test"
run_julia_shell 'using Pkg; pkg = "/tmp/gmeowgts-julia-package"; rm(pkg; recursive=true, force=true); cp(joinpath(ENV["GTS_WORKSPACE"], "julia"), pkg); ENV["GTS_LIBGTS"] = get(ENV, "GTS_LIBGTS", ""); ENV["LD_LIBRARY_PATH"] = ENV["GTS_CAPI_TARGET"]; Pkg.activate(pkg); Pkg.instantiate(); Pkg.test()'

log "Package dry-runs completed"
find "${OUT}" -maxdepth 2 -type f | sort

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CAPI="${ROOT}/rust/capi"
CAPI_TARGET="${CAPI}/target/debug"
VECTOR="vectors/01-minimal.gts"
RUBY_IMAGE="${RUBY_IMAGE:-gmeow-gts-ruby-ffi-smoke:3.4}"

cd "${ROOT}"

TMP="$(mktemp -d "${TMPDIR:-/tmp}/gmeow-gts-ruby-smoke.XXXXXXXXXX")"
trap 'rm -rf "${TMP}"' EXIT

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

has_ruby_ffi() {
  command -v ruby >/dev/null 2>&1 && ruby -rffi -e 'exit 0' >/dev/null 2>&1
}

run_smoke() {
  local gem_file="${TMP}/gmeow-gts-ruby-smoke.gem"
  local gem_home="${TMP}/gems"
  local base_gem_path
  local gem_path
  local sep

  mkdir -p "${gem_home}"
  sep="$(ruby -e 'puts File::PATH_SEPARATOR')"
  base_gem_path="$(ruby -e 'puts Gem.path.join(File::PATH_SEPARATOR)')"
  gem_path="${gem_home}${sep}${base_gem_path}"

  (cd ruby && gem build gmeow-gts.gemspec --output "${gem_file}" >/dev/null)
  ruby -I ruby/lib ruby/tests/smoke.rb "${VECTOR}"
  GEM_HOME="${gem_home}" \
    GEM_PATH="${gem_path}" \
    gem install --local "${gem_file}" --no-document >/dev/null
  GEM_HOME="${gem_home}" \
    GEM_PATH="${gem_path}" \
    GTS_RUBY_SMOKE_INSTALLED=1 \
    ruby ruby/tests/smoke.rb "${VECTOR}"
}

if [[ "${GTS_RUBY_FORCE_DOCKER:-0}" != "1" ]] && has_ruby_ffi; then
  export GTS_LIBGTS="${LIB_PATH}"
  export LD_LIBRARY_PATH="${CAPI_TARGET}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_LIBRARY_PATH="${CAPI_TARGET}${DYLD_LIBRARY_PATH:+:${DYLD_LIBRARY_PATH}}"
  run_smoke
else
  if [[ "$(uname -s)" != "Linux" ]]; then
    echo "Ruby with ffi is unavailable locally and the Docker fallback requires a Linux libgts build." >&2
    exit 1
  fi
  docker build -t "${RUBY_IMAGE}" ruby
  docker run --rm \
    -e GTS_LIBGTS=/workspace/rust/capi/target/debug/libgts.so \
    -e LD_LIBRARY_PATH=/workspace/rust/capi/target/debug \
    -v "${ROOT}:/workspace" \
    -w /workspace \
    "${RUBY_IMAGE}" \
    bash -lc 'set -euo pipefail
      tmp="$(mktemp -d /tmp/gmeow-gts-ruby-smoke.XXXXXXXXXX)"
      trap "rm -rf \"${tmp}\"" EXIT
      gem_file="${tmp}/gmeow-gts-ruby-smoke.gem"
      gem_home="${tmp}/gems"
      mkdir -p "${gem_home}"
      sep="$(ruby -e "puts File::PATH_SEPARATOR")"
      base_gem_path="$(ruby -e "puts Gem.path.join(File::PATH_SEPARATOR)")"
      gem_path="${gem_home}${sep}${base_gem_path}"
      cd ruby
      gem build gmeow-gts.gemspec --output "${gem_file}" >/dev/null
      cd /workspace
      ruby -I ruby/lib ruby/tests/smoke.rb vectors/01-minimal.gts
      GEM_HOME="${gem_home}" GEM_PATH="${gem_path}" gem install --local "${gem_file}" --no-document >/dev/null
      GEM_HOME="${gem_home}" GEM_PATH="${gem_path}" GTS_RUBY_SMOKE_INSTALLED=1 ruby ruby/tests/smoke.rb vectors/01-minimal.gts'
fi

echo "Ruby C ABI wrapper smoke test passed"

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="${1:-${ROOT}/dist/php-packagist-root}"

rm -rf "${OUT}"
mkdir -p "${OUT}/src" "${OUT}/tests"

cp "${ROOT}/php/composer.json" "${OUT}/composer.json"
cp "${ROOT}/php/README.md" "${OUT}/README.md"
cp "${ROOT}/LICENSE-MIT" "${OUT}/LICENSE-MIT"
cp "${ROOT}/LICENSE-APACHE" "${OUT}/LICENSE-APACHE"
cp -R "${ROOT}/php/src/." "${OUT}/src/"
cp -R "${ROOT}/php/tests/." "${OUT}/tests/"

unexpected="$(
  cd "${OUT}"
  find . -mindepth 1 -maxdepth 1 | sed 's#^\./##' \
    | grep -Ev '^(composer\.json|README\.md|LICENSE-MIT|LICENSE-APACHE|src|tests)$' \
    || true
)"
if [ -n "${unexpected}" ]; then
  printf 'Unexpected top-level package files:\n%s\n' "${unexpected}" >&2
  exit 1
fi

(
  cd "${OUT}"
  find . -type f | sed 's#^\./##' | sort
)

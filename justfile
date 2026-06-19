# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Developer shortcuts across the four engines. Run `just` to list recipes.
# Requires the per-engine toolchains: cargo, go, node/npm, and uv.

set shell := ["bash", "-cu"]

# List available recipes.
default:
    @just --list

# --- tests ----------------------------------------------------------------- #

# Run every engine's test suite.
test: test-rust test-go test-ts test-py

test-rust:
    cd rust && cargo test

test-go:
    cd go && CGO_ENABLED=0 go test ./...

test-ts:
    cd ts && npm ci && npm test

test-py:
    cd python && uv sync --extra rdf && uv run pytest

# --- lint / format --------------------------------------------------------- #

# Run every engine's lint + the repo-wide pre-commit hooks.
lint:
    cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings
    cd go && go vet ./... && golangci-lint run ./...
    cd ts && npm run lint
    cd python && uv run ruff check . && uv run mypy
    pre-commit run --all-files

# Auto-format every engine.
fmt:
    cd rust && cargo fmt
    cd go && gofmt -w .
    cd ts && npm run format
    cd python && uv run ruff format . && uv run ruff check --fix .

# --- conformance & interop ------------------------------------------------- #

# Regenerate the frozen conformance corpus from the Python reference.
gen-vectors:
    cd python && uv run python scripts/gen_vectors.py

# Fail if the committed corpus is not reproducible from the reference.
check-vectors: gen-vectors
    git diff --exit-code vectors

# Fail if vector manifest metadata or validator self-tests drift.
check-vector-manifest:
    python scripts/check_vector_manifest.py --self-test

# Fail if the CLI parity contract drifts from implementation or README docs.
check-cli-parity:
    python scripts/check_cli_parity.py

# Fail if deferred advanced verbs become public without being tiered.
check-advanced-contract:
    python scripts/check_advanced_contract.py

# Fail if ecosystem integration docs drift from current APIs and deferrals.
check-ecosystem-contract:
    python scripts/check_ecosystem_contract.py

# Fail if security/trust policy docs drift from current APIs and deferrals.
check-security-contract:
    python scripts/check_security_contract.py

# Fail if deferred multi-recipient COSE/ECDH policy and descriptors drift.
check-crypto-deferrals:
    python scripts/check_crypto_deferrals.py

# Live cross-engine interoperability check (each engine reads every other's output).
interop:
    bash scripts/interop.sh

# Run the release benchmark suite and write JSON/Markdown reports under dist/benchmarks/.
bench-release engines="rust,python,go,ts" iterations="3":
    python scripts/bench_release_suite.py --engines "{{engines}}" --iterations "{{iterations}}"

# Verify the Rust, Python, and npm versions agree (lockstep release).
check-versions:
    bash scripts/check-versions.sh

# Verify a published release family from public registries and attestations.
verify-release version visual_hashing_version extra_args="":
    python scripts/verify_release.py --version "{{version}}" --visual-hashing-version "{{visual_hashing_version}}" {{extra_args}}

# --- fuzzing --------------------------------------------------------------- #

# Fuzz the Rust reader (needs nightly + cargo-fuzz). Pass a duration, e.g. `just fuzz-rust 300`.
fuzz-rust seconds="60":
    cd rust && cargo +nightly fuzz run read -- -max_total_time={{seconds}}

# Fuzz the Go reader. Pass a duration, e.g. `just fuzz-go 5m`.
fuzz-go duration="60s":
    cd go && CGO_ENABLED=0 go test -run='^$' -fuzz=FuzzRead -fuzztime={{duration}} ./reader

# --- supply chain ---------------------------------------------------------- #

# Scan every lockfile for known vulnerabilities (needs osv-scanner).
audit:
    osv-scanner scan source --recursive .

# --- wasm ------------------------------------------------------------------ #

# Build the Rust library for wasm32 (backs the "wasm-friendly" claim).
wasm:
    rustup target add wasm32-unknown-unknown
    cd rust && cargo build --lib --target wasm32-unknown-unknown

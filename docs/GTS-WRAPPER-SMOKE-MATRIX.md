# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

# GTS Wrapper Smoke Matrix

The C ABI wrapper family is smoke-tested as bindings over the Rust `libgts`
engine. These checks are not full-engine parity columns and do not claim
independent parser, writer, or CLI conformance.

The shared matrix is defined in `scripts/wrapper_smoke_matrix.sh` and is used by
the C ABI, C++, .NET, PHP, Lua, Swift, Ruby, R, and Julia smoke entrypoints.

| Fixture | Source | Required observable result |
|---|---|---|
| `clean-read` | `vectors/01-minimal.gts` | ABI/version/build metadata are present; `read_json` reports `schema=gts-capi-read-v1` and `clean=true`; `verify_json` reports `schema=gts-capi-verify-v1`; N-Quads output contains `"Cat"@en`; N-Quads round-trip returns non-empty GTS bytes. |
| `damaged-diagnostic-read` | `vectors/04-damaged-frame.gts` | `read_json` reports `clean=false` and diagnostic code `DamagedFrame`; clean-output conversions refuse the input with a structured diagnostic error. |
| `empty-malformed-refusal` | `vectors/28-empty-file.gts` | `read_json` reports `clean=false` and diagnostic code `EmptyFile`; clean-output conversions refuse the input with a structured diagnostic error. |
| `malformed-nquads-refusal` | `GTS_WRAPPER_BAD_NQUADS` | `from_nquads` refuses malformed N-Quads with structured parse status, code, and detail. |
| package dry-run linkage | `scripts/package_dry_run_wrappers.sh` | Credential-free package consumers inherit the same matrix through environment variables and run lightweight fixture checks after local package build/install steps. |

Failures should include both the wrapper name and fixture name so CI identifies
the diverging wrapper and observable fixture.

<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Unchecked Panic-Like Triage

This records the issue #318 triage for the largest `unchecked_panic_calls` clusters.
Counts are from the issue intake baseline and the scanner state after this pass.

| Cluster | Intake count | Current count | Classification | Resolution |
| --- | ---: | ---: | --- | --- |
| `rust/capi/src/lib.rs` | 38 | 1 | Test-only checker noise plus one local C-string invariant. | Rust `#[cfg(test)]` modules are excluded by the checker. The remaining `CString::new(...).expect(...)` follows NUL-byte escaping and is an acceptable invariant. |
| `rust/src/openpgp.rs` | 28 | 2 | Test-only checker noise plus local MPI length invariants. | Rust `#[cfg(test)]` module calls are excluded. The remaining conversions are guarded by exact length checks. |
| `smalltalk/src/Gts-Core/GtsOpenPGP.class.st` | 28 | 28 | Production-reachable crypto/parser failures. | Follow-up issue #344. |
| `smalltalk/src/Gts-Core/GtsNQuadsTokenizer.class.st` | 18 | 18 | Production-reachable parser failures. | Follow-up issue #344. |
| `smalltalk/src/Gts-Core/GtsMMR.class.st` | 15 | 15 | Production-reachable proof validation failures. | Follow-up issue #344. |
| `smalltalk/src/Gts-Core/GtsFiles.class.st` | 13 | 13 | Production-reachable archive/profile safety failures. | Follow-up issue #344. |
| `kotlin/src/main/kotlin/ca/blackcatinformatics/gts/FilesProfile.kt` | 12 | 0 | Production-reachable archive/profile safety failures. | Replaced unchecked `error(...)` calls with `FilesProfileException`. |
| `rust/src/reader.rs` | 12 | 12 | Production reader/fold invariant paths that need individual audit. | Follow-up issue #345. |
| `smalltalk/src/Gts-Core/GtsCose.class.st` | 11 | 11 | Production-reachable crypto parser failures. | Follow-up issue #344. |
| `rust/src/codec.rs` | 7 | 0 | Embedded test-only checker noise. | Rust `#[cfg(test)]` module calls are excluded by the checker. |
| `rust/src/bin/gts.rs` | 6 | 2 | CLI-local formatting invariants. | Remaining calls parse a static timestamp format and format UTC timestamps; not malformed-input failure paths. |

The quality baseline ratchets `unchecked_panic_calls` from 266 to 179 in this pass.

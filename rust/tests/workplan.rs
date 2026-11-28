// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workplan gate: the capability documents under `docs/capabilities` and the
//! changes in progress under `work/changes`, judged against the Rust sources.

use std::path::Path;

use raf_workplan::Gate;

/// Every capability document binds code, tests and examples that exist, carries its
/// seven sections, and no change in progress names a missing capability. The gate
/// refuses a repository with no capability document, so an empty `docs/capabilities`
/// can never pass as clean. Items that no capability binds yet are held in the shrinking
/// record `work/debt/default.md`; a bound item that leaves it, or a new unbound item,
/// fails the gate.
#[test]
fn capability_documents_and_changes_match_the_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the Rust package sits one level below the repository root");
    Gate::workspace(root).source_roots(&["rust"]).assert_clean();
}

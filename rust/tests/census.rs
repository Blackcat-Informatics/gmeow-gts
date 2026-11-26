// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The census gate: the repository's helper ledger and debt record judged against
//! its own sources, over the receipt of the production Clippy run.
//!
//! `scripts/rust-census.sh` captures a fresh receipt first; `RAF_CENSUS_LINT_EVIDENCE`
//! overrides its path. A missing or stale receipt is a failure, never a skip. The
//! receipt cases run on native Linux only, as the receipt capture does. The ledger,
//! debt record and detector self-test checks run on every platform. The repository
//! depends on no preferred provider, so no registry archive is sealed and the plain
//! disk check applies.

use std::error::Error;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use raf_census::{DebtChange, ProductionLintRun};
use raf_census::{DebtRecord, Gate, Ledger, DEBT_FILE, LEDGER_FILE};

type TestResult = Result<(), Box<dyn Error>>;

#[cfg(target_os = "linux")]
const REQUIRED_RULES: [&str; 3] = ["serde-json", "rdf-text-writer", "rdf-term-model"];

/// The package directory that owns the workspace root, the ledger and the debt record.
fn workspace_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn ledger_path() -> PathBuf {
    workspace_dir().join(LEDGER_FILE)
}

fn debt_path() -> PathBuf {
    workspace_dir().join(DEBT_FILE)
}

/// The gate over the receipt named by `RAF_CENSUS_LINT_EVIDENCE` or the default path.
#[cfg(target_os = "linux")]
fn checked_gate() -> Result<Gate, Box<dyn Error>> {
    let gate = Gate::workspace(workspace_dir()).require_rules(REQUIRED_RULES);
    let path = std::env::var_os("RAF_CENSUS_LINT_EVIDENCE")
        .unwrap_or_else(|| "../.stage/census/production.ttl".into());
    let run = ProductionLintRun::read_external(&gate.root()?, &workspace_dir().join(path))?;
    Ok(gate.with_production_lint_run(run))
}

/// The receipt records a passing run of the exact production command over the inputs
/// of this checkout: no stale, failed or absent receipt finding may appear.
#[test]
#[cfg(target_os = "linux")]
fn production_clippy_receipt_covers_this_snapshot_and_passed() -> TestResult {
    let report = checked_gate()?.check()?;
    let receipt_findings: Vec<&String> = report
        .findings()
        .iter()
        .filter(|finding| {
            finding.contains("production lint evidence")
                || finding.contains("production Clippy failed")
                || finding.contains("production lint policy")
        })
        .collect();
    assert!(
        receipt_findings.is_empty(),
        "the production Clippy receipt does not hold for this snapshot: {receipt_findings:?}"
    );
    Ok(())
}

/// The census of the real workspace is clean with the serde-json, rdf-text-writer and
/// rdf-term-model rules required: no duplicate beyond the recorded debt, no JSON
/// dependency, no hand-rolled RDF text writer or term model.
#[test]
#[cfg(target_os = "linux")]
fn workspace_census_is_clean_with_the_representation_rules_required() -> TestResult {
    let report = checked_gate()?.check()?;
    assert!(
        !report.census().units().is_empty(),
        "the census read no source item, so it proved nothing"
    );
    assert!(report.is_clean(), "{}", report.findings().join("\n"));
    Ok(())
}

/// The ledger is canonical (the reader refuses any other bytes) and states that the
/// repository has no helper job, no preferred provider and no reference selector,
/// because it depends on neither purrdf nor Katamari.
#[test]
fn ledger_is_canonical_and_declares_no_helper_or_provider() -> TestResult {
    let text = std::fs::read_to_string(ledger_path())?;
    let ledger = Ledger::from_turtle(&text)?;
    assert!(ledger.jobs().is_empty(), "the ledger declares a helper job");
    assert!(
        ledger.reference().is_empty(),
        "the ledger names a reference package, which needs a sealed registry"
    );
    assert_eq!(ledger.roots(), ["."], "the census scans the workspace root");
    Ok(())
}

/// The debt record is canonical, and on Linux the present sources add no duplicate
/// group, job copy or ratcheted site beyond it, so it can only shrink.
#[test]
fn debt_record_is_canonical_and_the_sources_add_nothing_to_it() -> TestResult {
    let text = std::fs::read_to_string(debt_path())?;
    let record = DebtRecord::from_turtle(&text)?;
    assert_eq!(record.to_turtle()?, text, "the record is not canonical");
    #[cfg(target_os = "linux")]
    {
        let report = checked_gate()?.check()?;
        let growth: Vec<_> = record
            .compare(report.debt())?
            .into_iter()
            .filter(DebtChange::is_growth)
            .collect();
        assert!(growth.is_empty(), "debt grew beyond its record: {growth:?}");
    }
    Ok(())
}

/// The census build still finds what it is built to find: every built-in detector
/// and the fixed engine cases pass their positive and negative fixtures, so a clean
/// verdict above is not a detector that stopped detecting.
#[test]
fn census_self_test_passes_every_detector_case() {
    let outcome = Gate::workspace(workspace_dir()).self_test();
    assert!(!outcome.cases().is_empty(), "the self-test ran no case");
    let failed: Vec<String> = outcome
        .cases()
        .iter()
        .filter_map(|case| case.failure().map(|why| format!("{}: {why}", case.id())))
        .collect();
    assert!(failed.is_empty(), "{}", failed.join("\n"));
    assert!(outcome.passed());
}

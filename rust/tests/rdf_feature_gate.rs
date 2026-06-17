// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(not(feature = "rdf"))]
#[test]
fn rdf_adapter_is_not_enabled_by_default() {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml is readable");
    assert!(manifest.contains("default = []"));
    assert!(manifest.contains("rdf = [\"dep:oxrdf\"]"));
    assert!(manifest.contains("optional = true"));
}

#[cfg(feature = "rdf")]
#[test]
fn rdf_feature_enables_adapter_module() {
    let options = gmeow_gts::rdf::ExportOptions::default();
    assert!(!options.allow_rdf12_lossy);
}

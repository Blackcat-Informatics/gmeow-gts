// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(not(feature = "rdf"))]
#[test]
fn rdf_adapter_is_not_enabled_by_default() {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = std::process::Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    let package = metadata["packages"]
        .as_array()
        .expect("metadata packages are an array")
        .iter()
        .find(|package| package["name"] == "gmeow-gts")
        .expect("gmeow-gts package is present");

    assert_eq!(package["features"]["default"], serde_json::json!([]));
    assert_eq!(package["features"]["rdf"], serde_json::json!(["dep:oxrdf"]));

    let oxrdf = package["dependencies"]
        .as_array()
        .expect("metadata dependencies are an array")
        .iter()
        .find(|dependency| dependency["name"] == "oxrdf")
        .expect("oxrdf dependency is present");
    assert_eq!(oxrdf["optional"], serde_json::json!(true));
    assert_eq!(oxrdf["uses_default_features"], serde_json::json!(false));
    assert_eq!(oxrdf["features"], serde_json::json!(["rdf-12"]));
}

#[cfg(feature = "rdf")]
#[test]
fn rdf_feature_enables_adapter_module() {
    let options = gmeow_gts::rdf::ExportOptions::default();
    assert!(!options.allow_rdf12_lossy);
}

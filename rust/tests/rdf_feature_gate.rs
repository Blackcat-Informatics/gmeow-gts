// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(not(any(
    feature = "rdf",
    feature = "rdf-codecs",
    feature = "duckdb",
    feature = "xsd",
    feature = "yaml-ld"
)))]
#[test]
fn optional_adapters_are_not_enabled_by_default() {
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
    assert_eq!(package["features"]["duckdb"], serde_json::json!([]));
    assert_eq!(package["features"]["rdf"], serde_json::json!(["xsd"]));
    assert_eq!(
        package["features"]["rdf-codecs"],
        serde_json::json!(["rdf", "dep:quick-xml"])
    );
    assert_eq!(
        package["features"]["native-store"],
        serde_json::json!(["rdf"])
    );
    assert!(
        package["features"].get("oxigraph-adapter").is_none(),
        "oxigraph-adapter feature must be removed"
    );
    assert!(
        package["features"].get("sophia-adapter").is_none(),
        "sophia-adapter feature must be removed"
    );
    assert_eq!(
        package["features"]["policy-config"],
        serde_json::json!(["dep:serde", "dep:serde_json"])
    );
    assert_eq!(
        package["features"]["policy-config-yaml"],
        serde_json::json!(["policy-config", "dep:serde_yaml"])
    );
    assert_eq!(
        package["features"]["yaml-ld"],
        serde_json::json!(["dep:serde", "dep:serde_json", "dep:serde_yaml"])
    );
    assert_eq!(
        package["features"]["xsd"],
        serde_json::json!(["dep:oxsdatatypes"])
    );

    let oxsdatatypes = package["dependencies"]
        .as_array()
        .expect("metadata dependencies are an array")
        .iter()
        .find(|dependency| dependency["name"] == "oxsdatatypes")
        .expect("oxsdatatypes dependency is present");
    assert_eq!(oxsdatatypes["optional"], serde_json::json!(true));

    let quick_xml = package["dependencies"]
        .as_array()
        .expect("metadata dependencies are an array")
        .iter()
        .find(|dependency| dependency["name"] == "quick-xml")
        .expect("quick-xml dependency is present");
    assert_eq!(quick_xml["optional"], serde_json::json!(true));

    for removed in ["oxttl", "oxrdf", "oxrdfxml", "oxigraph"] {
        assert!(
            !package["dependencies"]
                .as_array()
                .expect("metadata dependencies are an array")
                .iter()
                .any(|dependency| dependency["name"] == removed),
            "rdf-codecs must not depend on {removed}"
        );
    }

    for name in ["serde", "serde_json", "serde_yaml"] {
        let dependency = package["dependencies"]
            .as_array()
            .expect("metadata dependencies are an array")
            .iter()
            .find(|dependency| dependency["name"] == name)
            .unwrap_or_else(|| panic!("{name} dependency is present"));
        assert_eq!(dependency["optional"], serde_json::json!(true));
    }

    for removed in ["sophia_api", "sophia_inmem", "sophia_turtle", "uuid"] {
        assert!(
            !package["dependencies"]
                .as_array()
                .expect("metadata dependencies are an array")
                .iter()
                .any(|dependency| dependency["name"] == removed),
            "gmeow-gts must not depend on {removed}"
        );
    }

    assert!(
        !package["dependencies"]
            .as_array()
            .expect("metadata dependencies are an array")
            .iter()
            .any(|dependency| dependency["name"] == "duckdb"),
        "duckdb feature must stay a no-dependency runtime shell-out"
    );
}

#[cfg(feature = "rdf")]
#[test]
fn rdf_feature_enables_adapter_module() {
    let options = gmeow_gts::rdf::ExportOptions::default();
    assert!(!options.allow_rdf12_lossy);
}

#[cfg(feature = "rdf-codecs")]
#[test]
fn rdf_codecs_feature_enables_rdf_text_codec_modules() {
    let turtle = "@prefix ex: <https://ex/> .\nex:s ex:p ex:o .\n";
    let turtle_bytes = gmeow_gts::rdf_codecs::from_turtle(turtle).unwrap();
    assert!(!turtle_bytes.is_empty());

    let ntriples = "<https://ex/s> <https://ex/p> <https://ex/o> .\n";
    let ntriples_bytes = gmeow_gts::rdf_codecs::from_ntriples(ntriples).unwrap();
    assert!(!ntriples_bytes.is_empty());

    let rdf_xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="https://ex/">
  <rdf:Description rdf:about="https://ex/s">
    <ex:p rdf:resource="https://ex/o"/>
  </rdf:Description>
</rdf:RDF>"#;
    let rdf_xml_bytes = gmeow_gts::rdf_codecs::from_rdf_xml(rdf_xml).unwrap();
    assert!(!rdf_xml_bytes.is_empty());
}

#[cfg(feature = "native-store")]
#[test]
fn native_store_feature_enables_adapter_module() {
    let sidecar = gmeow_gts::native_store::GtsSidecar::default();
    assert!(sidecar.diagnostics.is_empty());
}

#[cfg(feature = "yaml-ld")]
#[test]
fn yaml_ld_feature_enables_codec_modules() {
    let value = gmeow_gts::yamlld::to_json_ld(&gmeow_gts::model::Graph::default());
    let text = serde_json::to_string(&value).unwrap();
    let bytes = gmeow_gts::from_yamlld::from_json_ld(&text).unwrap();
    assert!(!bytes.is_empty());
}

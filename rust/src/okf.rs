// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! GTS OKF profile exporter.
//!
//! This module projects only the OKF vocabulary to Markdown bundle files.
//! Unsupported RDF remains visible in `_unmapped.nq` instead of being silently
//! dropped.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping as YamlMapping, Value as YamlValue};

use crate::model::{Graph, TermKind, Triple3};
use crate::nquads::render_term;

pub const OKF_NS: &str = "https://blackcatinformatics.ca/projects/gts/okf#";
pub const DEFAULT_BASE_IRI: &str = "https://blackcatinformatics.ca/projects/gts/okf/doc/";
pub const OKF_PATH: &str = "https://blackcatinformatics.ca/projects/gts/okf#path";
pub const OKF_TYPE: &str = "https://blackcatinformatics.ca/projects/gts/okf#type";
pub const OKF_TITLE: &str = "https://blackcatinformatics.ca/projects/gts/okf#title";
pub const OKF_DESCRIPTION: &str = "https://blackcatinformatics.ca/projects/gts/okf#description";
pub const OKF_RESOURCE: &str = "https://blackcatinformatics.ca/projects/gts/okf#resource";
pub const OKF_TAG: &str = "https://blackcatinformatics.ca/projects/gts/okf#tag";
pub const OKF_TIMESTAMP: &str = "https://blackcatinformatics.ca/projects/gts/okf#timestamp";
pub const OKF_BODY: &str = "https://blackcatinformatics.ca/projects/gts/okf#body";
pub const OKF_LINKS: &str = "https://blackcatinformatics.ca/projects/gts/okf#links";
pub const OKF_LINK_TEXT: &str = "https://blackcatinformatics.ca/projects/gts/okf#linkText";
pub const OKF_LINK_OCCURRENCE: &str =
    "https://blackcatinformatics.ca/projects/gts/okf#linkOccurrence";
pub const OKF_JSON: &str = "https://blackcatinformatics.ca/projects/gts/okf#json";
pub const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
pub const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

/// Options for exporting an OKF bundle directory.
#[derive(Clone, Debug)]
pub struct OkfExportOptions {
    /// Base IRI recorded in the bundle manifest.
    pub base_iri: String,
    /// Permit `okf:body` inline string literals as a body source. Digest-backed
    /// body blobs are always supported.
    pub inline_body: bool,
}

impl Default for OkfExportOptions {
    fn default() -> Self {
        Self {
            base_iri: DEFAULT_BASE_IRI.to_string(),
            inline_body: false,
        }
    }
}

/// Summary returned by [`to_okf`].
#[derive(Clone, Debug)]
pub struct OkfExportReport {
    pub directory: PathBuf,
    pub documents: usize,
    pub unmapped_triples: usize,
}

/// Raised when a graph cannot be projected to an OKF bundle without guessing.
#[derive(Debug)]
pub struct OkfExportError {
    detail: String,
}

impl OkfExportError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for OkfExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for OkfExportError {}

impl From<io::Error> for OkfExportError {
    fn from(value: io::Error) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Default)]
struct ExportDocument {
    subject: usize,
    path: Option<String>,
    fields: BTreeMap<String, YamlValue>,
    tags: BTreeSet<String>,
    body: Option<Vec<u8>>,
}

/// Project an OKF-profile folded graph into a Markdown bundle directory.
pub fn to_okf(
    graph: &Graph,
    directory: &Path,
    options: &OkfExportOptions,
) -> Result<OkfExportReport, OkfExportError> {
    if directory.exists() {
        return Err(OkfExportError::new(format!(
            "destination {} already exists",
            directory.display()
        )));
    }
    let staged = staged_path(directory);
    let _ = fs::remove_dir_all(&staged);
    fs::create_dir_all(&staged)?;

    match write_okf(graph, &staged, options) {
        Ok(mut report) => {
            fs::rename(&staged, directory)?;
            report.directory = directory.to_path_buf();
            Ok(report)
        }
        Err(err) => {
            let _ = fs::remove_dir_all(&staged);
            Err(err)
        }
    }
}

fn write_okf(
    graph: &Graph,
    directory: &Path,
    options: &OkfExportOptions,
) -> Result<OkfExportReport, OkfExportError> {
    let mut docs: BTreeMap<usize, ExportDocument> = BTreeMap::new();
    let mut consumed_quads = BTreeSet::new();
    let mut link_statements = BTreeSet::new();

    for (idx, &(subject, predicate, object, graph_name)) in graph.quads.iter().enumerate() {
        if graph_name.is_some() {
            continue;
        }
        let Some(predicate_iri) = iri_value(graph, predicate) else {
            continue;
        };
        let doc = docs.entry(subject).or_insert_with(|| ExportDocument {
            subject,
            ..ExportDocument::default()
        });
        match predicate_iri {
            OKF_PATH => {
                if let Some(path) = literal_value(graph, object) {
                    safe_relative_path(path)?;
                    doc.path = Some(path.to_string());
                    consumed_quads.insert(idx);
                }
            }
            OKF_TYPE => {
                if let Some(value) = literal_value(graph, object) {
                    doc.fields
                        .insert("type".to_string(), YamlValue::String(value.to_string()));
                    consumed_quads.insert(idx);
                }
            }
            OKF_TITLE => consume_string_field(
                graph,
                object,
                &mut doc.fields,
                "title",
                idx,
                &mut consumed_quads,
            ),
            OKF_DESCRIPTION => consume_string_field(
                graph,
                object,
                &mut doc.fields,
                "description",
                idx,
                &mut consumed_quads,
            ),
            OKF_TIMESTAMP => consume_string_field(
                graph,
                object,
                &mut doc.fields,
                "timestamp",
                idx,
                &mut consumed_quads,
            ),
            OKF_RESOURCE => {
                if let Some(value) = iri_value(graph, object) {
                    doc.fields
                        .insert("resource".to_string(), YamlValue::String(value.to_string()));
                    consumed_quads.insert(idx);
                }
            }
            OKF_TAG => {
                if let Some(value) = literal_value(graph, object) {
                    doc.tags.insert(value.to_string());
                    consumed_quads.insert(idx);
                }
            }
            OKF_BODY => {
                doc.body = Some(body_bytes(graph, object, options)?);
                consumed_quads.insert(idx);
            }
            OKF_LINKS => {
                consumed_quads.insert(idx);
                link_statements.insert((subject, predicate, object));
            }
            _ if predicate_iri.starts_with(OKF_NS) => {
                let local = predicate_iri
                    .strip_prefix(OKF_NS)
                    .expect("predicate matched OKF namespace");
                if known_local(local) {
                    continue;
                }
                if let Some(value) = yaml_value(graph, object)? {
                    doc.fields.insert(local.to_string(), value);
                    consumed_quads.insert(idx);
                }
            }
            _ => {}
        }
    }

    let mut unmapped = Vec::new();
    for (idx, &(subject, predicate, object, graph_name)) in graph.quads.iter().enumerate() {
        if !consumed_quads.contains(&idx) {
            unmapped.push(match graph_name {
                Some(name) => format!(
                    "{} {} {} {} .",
                    render_term(graph, subject),
                    render_term(graph, predicate),
                    render_term(graph, object),
                    render_term(graph, name)
                ),
                None => format!(
                    "{} {} {} .",
                    render_term(graph, subject),
                    render_term(graph, predicate),
                    render_term(graph, object)
                ),
            });
        }
    }

    let consumed_reifiers = consumed_link_reifiers(graph, &link_statements, &mut unmapped);
    for &(reifier, (subject, predicate, object)) in &graph.reifiers {
        if !consumed_reifiers.contains(&reifier) {
            unmapped.push(format!(
                "{} <{RDF_REIFIES}> <<( {} {} {} )>> .",
                render_term(graph, reifier),
                render_term(graph, subject),
                render_term(graph, predicate),
                render_term(graph, object)
            ));
        }
    }

    for &(reifier, predicate, value) in &graph.annotations {
        if !consumed_reifiers.contains(&reifier) {
            unmapped.push(format!(
                "{} {} {} .",
                render_term(graph, reifier),
                render_term(graph, predicate),
                render_term(graph, value)
            ));
        }
    }

    let mut documents = Vec::new();
    for (_, mut doc) in docs {
        let Some(path) = doc.path.clone() else {
            unmapped.push(format!(
                "# subject {} has OKF fields but no okf:path",
                render_term(graph, doc.subject)
            ));
            continue;
        };
        if !doc.fields.contains_key("type") {
            return Err(OkfExportError::new(format!(
                "OKF document {path} is missing okf:type"
            )));
        }
        let body = doc.body.take().unwrap_or_default();
        write_document(directory, &path, &doc.fields, &doc.tags, &body)?;
        documents.push(path);
    }
    documents.sort();

    if !unmapped.is_empty() {
        fs::write(
            directory.join("_unmapped.nq"),
            format!("{}\n", unmapped.join("\n")),
        )?;
    }
    write_manifest(directory, options, &documents, unmapped.len())?;
    Ok(OkfExportReport {
        directory: directory.to_path_buf(),
        documents: documents.len(),
        unmapped_triples: unmapped.len(),
    })
}

fn consume_string_field(
    graph: &Graph,
    object: usize,
    fields: &mut BTreeMap<String, YamlValue>,
    key: &str,
    index: usize,
    consumed: &mut BTreeSet<usize>,
) {
    if let Some(value) = literal_value(graph, object) {
        fields.insert(key.to_string(), YamlValue::String(value.to_string()));
        consumed.insert(index);
    }
}

fn body_bytes(
    graph: &Graph,
    object: usize,
    options: &OkfExportOptions,
) -> Result<Vec<u8>, OkfExportError> {
    let value = literal_value(graph, object)
        .ok_or_else(|| OkfExportError::new("okf:body must be a literal"))?;
    if value.starts_with("blake3:") {
        let entry = graph
            .blob_entry(value)
            .ok_or_else(|| OkfExportError::new(format!("missing OKF body blob {value}")))?;
        return entry
            .decoded_vec()
            .map_err(|err| OkfExportError::new(format!("cannot decode OKF body blob: {err:?}")));
    }
    if options.inline_body {
        Ok(value.as_bytes().to_vec())
    } else {
        Err(OkfExportError::new(
            "inline okf:body literal requires --inline-body",
        ))
    }
}

fn yaml_value(graph: &Graph, object: usize) -> Result<Option<YamlValue>, OkfExportError> {
    let Some(term) = graph.terms.get(object) else {
        return Ok(None);
    };
    if term.kind != TermKind::Literal {
        return Ok(None);
    }
    let value = term.value.as_deref().unwrap_or("");
    let datatype = term.datatype.and_then(|id| iri_value(graph, id));
    Ok(Some(match datatype {
        Some(XSD_BOOLEAN) => YamlValue::Bool(value == "true"),
        Some(XSD_INTEGER) => serde_yaml::from_str(value).unwrap_or(YamlValue::String(value.into())),
        Some(XSD_DECIMAL) => serde_yaml::from_str(value).unwrap_or(YamlValue::String(value.into())),
        Some(OKF_JSON) => {
            let json: JsonValue = serde_json::from_str(value).map_err(|e| {
                OkfExportError::new(format!("invalid okf:json literal {value:?}: {e}"))
            })?;
            serde_yaml::to_value(json)
                .map_err(|e| OkfExportError::new(format!("cannot convert JSON literal: {e}")))?
        }
        _ => YamlValue::String(value.to_string()),
    }))
}

fn consumed_link_reifiers(
    graph: &Graph,
    link_statements: &BTreeSet<Triple3>,
    unmapped: &mut Vec<String>,
) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    for &(reifier, statement) in &graph.reifiers {
        if !link_statements.contains(&statement) {
            continue;
        }
        let annotations: Vec<_> = graph
            .annotations
            .iter()
            .filter(|&&(candidate, _, _)| candidate == reifier)
            .collect();
        let ok = annotations.iter().all(|&&(_, predicate, _)| {
            iri_value(graph, predicate)
                .is_some_and(|iri| iri == OKF_LINK_TEXT || iri == OKF_LINK_OCCURRENCE)
        });
        if ok {
            out.insert(reifier);
        } else {
            unmapped.push(format!(
                "# link reifier {} has non-OKF annotations",
                render_term(graph, reifier)
            ));
        }
    }
    out
}

fn write_document(
    root: &Path,
    path: &str,
    fields: &BTreeMap<String, YamlValue>,
    tags: &BTreeSet<String>,
    body: &[u8],
) -> Result<(), OkfExportError> {
    safe_relative_path(path)?;
    let target = root.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut all_fields = fields.clone();
    if !tags.is_empty() {
        all_fields.insert(
            "tags".to_string(),
            YamlValue::Sequence(tags.iter().cloned().map(YamlValue::String).collect()),
        );
    }
    let mut mapping = YamlMapping::new();
    for (key, value) in all_fields {
        mapping.insert(YamlValue::String(key.clone()), value.clone());
    }
    let yaml = serde_yaml::to_string(&mapping)
        .map_err(|e| OkfExportError::new(format!("cannot serialize frontmatter: {e}")))?;
    let mut out = fs::File::create(target)?;
    out.write_all(b"---\n")?;
    out.write_all(yaml.as_bytes())?;
    out.write_all(b"---\n")?;
    out.write_all(body)?;
    Ok(())
}

#[derive(Serialize)]
struct Manifest<'a> {
    schema: &'a str,
    base_iri: &'a str,
    doc_count: usize,
    source_paths: &'a [String],
    unmapped_triples: usize,
}

fn write_manifest(
    root: &Path,
    options: &OkfExportOptions,
    paths: &[String],
    unmapped_triples: usize,
) -> Result<(), OkfExportError> {
    let control = root.join(".gts-okf");
    fs::create_dir_all(&control)?;
    let manifest = Manifest {
        schema: "gts-okf-v1",
        base_iri: &options.base_iri,
        doc_count: paths.len(),
        source_paths: paths,
        unmapped_triples,
    };
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|e| OkfExportError::new(format!("cannot encode OKF manifest: {e}")))?;
    fs::write(control.join("manifest.json"), format!("{text}\n"))?;
    Ok(())
}

fn staged_path(directory: &Path) -> PathBuf {
    let parent = directory
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = directory
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "okf".into());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(
        ".{name}.gts-okf.{}.{}.tmp",
        std::process::id(),
        nanos
    ))
}

fn safe_relative_path(path: &str) -> Result<(), OkfExportError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(OkfExportError::new(format!(
            "unsafe OKF relative path: {path}"
        )));
    }
    Ok(())
}

fn iri_value(graph: &Graph, term: usize) -> Option<&str> {
    graph
        .terms
        .get(term)
        .and_then(|term| match (term.kind, term.value.as_deref()) {
            (TermKind::Iri, Some(value)) => Some(value),
            _ => None,
        })
}

fn literal_value(graph: &Graph, term: usize) -> Option<&str> {
    graph
        .terms
        .get(term)
        .and_then(|term| match (term.kind, term.value.as_deref()) {
            (TermKind::Literal, Some(value)) => Some(value),
            _ => None,
        })
}

fn known_local(local: &str) -> bool {
    matches!(
        local,
        "path"
            | "type"
            | "title"
            | "description"
            | "resource"
            | "tag"
            | "timestamp"
            | "body"
            | "links"
            | "linkText"
            | "linkOccurrence"
            | "json"
    )
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! OKF bundle -> GTS importer.
//!
//! OKF is deliberately a directory of Markdown documents with YAML
//! frontmatter. This importer maps that human surface to a deterministic GTS
//! segment in the OKF profile.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use ciborium::value::Value as CborValue;
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping as YamlMapping, Value as YamlValue};

use crate::model::{AnnotationRow, Graph, Quad, ReifierRow, Term, TermKind};
use crate::okf::{
    DEFAULT_BASE_IRI, OKF_BODY, OKF_DESCRIPTION, OKF_JSON, OKF_LINKS, OKF_LINK_OCCURRENCE,
    OKF_LINK_TEXT, OKF_PATH, OKF_RESOURCE, OKF_TAG, OKF_TIMESTAMP, OKF_TITLE, OKF_TYPE,
    XSD_BOOLEAN, XSD_DATETIME, XSD_DECIMAL, XSD_INTEGER,
};
use crate::wire::digest_str;
use crate::writer::{Writer, WriterOptions};

/// Import options for OKF bundles.
#[derive(Clone, Debug)]
pub struct FromOkfOptions {
    /// Base IRI used when a document lacks `resource:`.
    pub base_iri: String,
    /// Store Markdown bodies directly as `okf:body` string literals instead of
    /// content-addressed blobs.
    pub inline_body: bool,
    /// Reject Markdown links whose target document is not in the bundle.
    pub strict_links: bool,
}

impl Default for FromOkfOptions {
    fn default() -> Self {
        Self {
            base_iri: DEFAULT_BASE_IRI.to_string(),
            inline_body: false,
            strict_links: false,
        }
    }
}

/// Raised when an OKF bundle cannot be imported without guessing.
#[derive(Debug)]
pub struct OkfParseError {
    detail: String,
}

impl OkfParseError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for OkfParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for OkfParseError {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TermKey {
    Atom {
        kind: TermKind,
        value: String,
        lang: Option<String>,
        datatype: Option<String>,
    },
}

#[derive(Default)]
struct GraphBuilder {
    ids: HashMap<TermKey, usize>,
    terms: Vec<Term>,
    quads: Vec<Quad>,
    reifiers: Vec<ReifierRow>,
    annotations: Vec<AnnotationRow>,
}

impl GraphBuilder {
    fn atom(
        &mut self,
        kind: TermKind,
        value: String,
        lang: Option<String>,
        datatype: Option<String>,
    ) -> usize {
        let key = TermKey::Atom {
            kind,
            value: value.clone(),
            lang: lang.clone(),
            datatype: datatype.clone(),
        };
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let datatype_id = if kind == TermKind::Literal {
            datatype
                .as_ref()
                .map(|iri| self.atom(TermKind::Iri, iri.clone(), None, None))
        } else {
            None
        };
        let id = self.terms.len();
        self.terms.push(Term {
            kind,
            value: Some(value),
            datatype: datatype_id,
            lang,
            direction: None,
            reifier: None,
        });
        self.ids.insert(key, id);
        id
    }

    fn iri(&mut self, value: &str) -> usize {
        self.atom(TermKind::Iri, value.to_string(), None, None)
    }

    fn bnode(&mut self, label: &str) -> usize {
        self.atom(TermKind::Bnode, label.to_string(), None, None)
    }

    fn literal(&mut self, value: &str, datatype: Option<&str>) -> usize {
        self.atom(
            TermKind::Literal,
            value.to_string(),
            None,
            datatype.map(str::to_string),
        )
    }

    fn quad_lit(&mut self, subject: usize, predicate: &str, value: &str, datatype: Option<&str>) {
        let p = self.iri(predicate);
        let o = self.literal(value, datatype);
        self.quads.push((subject, p, o, None));
    }

    fn quad_iri(&mut self, subject: usize, predicate: &str, value: &str) {
        let p = self.iri(predicate);
        let o = self.iri(value);
        self.quads.push((subject, p, o, None));
    }
}

struct Document {
    path: String,
    subject_iri: String,
    frontmatter: YamlMapping,
    body: Vec<u8>,
}

/// Import an OKF bundle directory into one canonical GTS segment.
pub fn from_okf(dir: &Path) -> Result<Vec<u8>, OkfParseError> {
    from_okf_with_options(dir, &FromOkfOptions::default())
}

/// Import an OKF bundle directory with explicit options.
pub fn from_okf_with_options(
    dir: &Path,
    options: &FromOkfOptions,
) -> Result<Vec<u8>, OkfParseError> {
    if !dir.is_dir() {
        return Err(OkfParseError::new(format!(
            "OKF bundle root is not a directory: {}",
            dir.display()
        )));
    }

    let paths = markdown_files(dir)?;
    let mut documents = Vec::with_capacity(paths.len());
    for path in paths {
        let rel = relative_okf_path(dir, &path)?;
        let bytes = fs::read(&path)
            .map_err(|e| OkfParseError::new(format!("cannot read {}: {e}", path.display())))?;
        if is_frontmatterless_index(&bytes, &rel)? {
            continue;
        }
        let (frontmatter, body) = parse_markdown(&bytes, &rel)?;
        let resource = string_field(&frontmatter, "resource")?;
        let subject_iri = resource
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| minted_doc_iri(&options.base_iri, &rel));
        documents.push(Document {
            path: rel,
            subject_iri,
            frontmatter,
            body,
        });
    }
    documents.sort_by(|a, b| a.path.cmp(&b.path));

    let subject_by_path: BTreeMap<String, String> = documents
        .iter()
        .map(|doc| (doc.path.clone(), doc.subject_iri.clone()))
        .collect();

    let mut builder = GraphBuilder::default();
    let mut blobs: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for document in &documents {
        import_document(document, &mut builder, options, &mut blobs)?;
    }
    import_links(&documents, &subject_by_path, &mut builder, options)?;

    builder.quads.sort();
    builder.reifiers.sort();
    builder.annotations.sort();

    let graph = Graph {
        terms: builder.terms,
        quads: builder.quads,
        reifiers: builder.reifiers,
        annotations: builder.annotations,
        ..Graph::default()
    };

    let mut writer = Writer::with_options(
        "okf",
        WriterOptions {
            meta: Some(manifest_meta(options, &documents)),
            ..WriterOptions::default()
        },
    )
    .map_err(|e| OkfParseError::new(format!("cannot author OKF GTS header: {e}")))?;
    if !graph.terms.is_empty() {
        writer.add_terms(&graph.terms);
    }
    if !graph.quads.is_empty() {
        writer.add_quads(&graph.quads);
    }
    if !graph.reifiers.is_empty() {
        writer.add_reifies(&graph.reifiers);
    }
    if !graph.annotations.is_empty() {
        writer.add_annot(&graph.annotations);
    }
    for (_digest, body) in blobs {
        writer.add_blob(&body, Some("text/markdown"), None);
    }
    Ok(writer.to_bytes())
}

fn import_document(
    document: &Document,
    builder: &mut GraphBuilder,
    options: &FromOkfOptions,
    blobs: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), OkfParseError> {
    safe_relative_path(&document.path)?;
    let subject = builder.iri(&document.subject_iri);
    builder.quad_lit(subject, OKF_PATH, &document.path, None);

    let type_value = required_string_field(&document.frontmatter, "type", &document.path)?;
    builder.quad_lit(subject, OKF_TYPE, &type_value, None);

    for (key, predicate) in [
        ("title", OKF_TITLE),
        ("description", OKF_DESCRIPTION),
        ("timestamp", OKF_TIMESTAMP),
    ] {
        if let Some(value) = string_field(&document.frontmatter, key)? {
            let datatype = if key == "timestamp" {
                Some(XSD_DATETIME)
            } else {
                None
            };
            builder.quad_lit(subject, predicate, &value, datatype);
        }
    }
    if let Some(resource) = string_field(&document.frontmatter, "resource")? {
        if !resource.is_empty() {
            builder.quad_iri(subject, OKF_RESOURCE, &resource);
        }
    }
    for tag in tags_field(&document.frontmatter)? {
        builder.quad_lit(subject, OKF_TAG, &tag, None);
    }
    for (key, value) in &document.frontmatter {
        let Some(key) = key.as_str() else {
            return Err(OkfParseError::new(format!(
                "{}: frontmatter keys must be strings",
                document.path
            )));
        };
        if matches!(
            key,
            "type" | "title" | "description" | "resource" | "tags" | "timestamp"
        ) {
            continue;
        }
        import_extension_value(subject, key, value, builder)?;
    }

    if options.inline_body {
        let body = std::str::from_utf8(&document.body).map_err(|e| {
            OkfParseError::new(format!("{}: body is not valid UTF-8: {e}", document.path))
        })?;
        builder.quad_lit(subject, OKF_BODY, body, None);
    } else {
        let digest = digest_str(&document.body);
        builder.quad_lit(subject, OKF_BODY, &digest, None);
        blobs.entry(digest).or_insert_with(|| document.body.clone());
    }
    Ok(())
}

fn import_extension_value(
    subject: usize,
    key: &str,
    value: &YamlValue,
    builder: &mut GraphBuilder,
) -> Result<(), OkfParseError> {
    let predicate = format!("{}{}", crate::okf::OKF_NS, key);
    match value {
        YamlValue::Bool(flag) => {
            builder.quad_lit(subject, &predicate, &flag.to_string(), Some(XSD_BOOLEAN));
        }
        YamlValue::Number(number) => {
            let text = number.to_string();
            let datatype = if text.contains('.') || text.contains('e') || text.contains('E') {
                XSD_DECIMAL
            } else {
                XSD_INTEGER
            };
            builder.quad_lit(subject, &predicate, &text, Some(datatype));
        }
        YamlValue::String(text) => builder.quad_lit(subject, &predicate, text, None),
        YamlValue::Null | YamlValue::Sequence(_) | YamlValue::Mapping(_) | YamlValue::Tagged(_) => {
            let json = yaml_to_json(value)?;
            let text = serde_json::to_string(&json)
                .map_err(|e| OkfParseError::new(format!("cannot encode JSON literal: {e}")))?;
            builder.quad_lit(subject, &predicate, &text, Some(OKF_JSON));
        }
    }
    Ok(())
}

fn import_links(
    documents: &[Document],
    subject_by_path: &BTreeMap<String, String>,
    builder: &mut GraphBuilder,
    options: &FromOkfOptions,
) -> Result<(), OkfParseError> {
    for (doc_index, document) in documents.iter().enumerate() {
        let body = std::str::from_utf8(&document.body).map_err(|e| {
            OkfParseError::new(format!("{}: body is not valid UTF-8: {e}", document.path))
        })?;
        let source = builder.iri(&document.subject_iri);
        for (occurrence, link) in extract_markdown_links(body).into_iter().enumerate() {
            let Some(target_path) = resolve_link_path(&document.path, &link.target) else {
                continue;
            };
            let target = match subject_by_path.get(&target_path) {
                Some(iri) => iri.clone(),
                None if options.strict_links => {
                    return Err(OkfParseError::new(format!(
                        "{}: dangling OKF link target {}",
                        document.path, link.target
                    )))
                }
                None => minted_doc_iri(&options.base_iri, &target_path),
            };
            let predicate = builder.iri(OKF_LINKS);
            let object = builder.iri(&target);
            let statement = (source, predicate, object);
            builder.quads.push((source, predicate, object, None));
            let reifier = builder.bnode(&format!("okf_link_{doc_index}_{occurrence}"));
            builder.reifiers.push((reifier, statement, None));
            let text_predicate = builder.iri(OKF_LINK_TEXT);
            let text = builder.literal(&link.text, None);
            builder
                .annotations
                .push((reifier, text_predicate, text, None));
            let occurrence_predicate = builder.iri(OKF_LINK_OCCURRENCE);
            let occurrence_literal =
                builder.literal(&(occurrence + 1).to_string(), Some(XSD_INTEGER));
            builder
                .annotations
                .push((reifier, occurrence_predicate, occurrence_literal, None));
        }
    }
    Ok(())
}

struct MarkdownLink {
    text: String,
    target: String,
}

fn extract_markdown_links(body: &str) -> Vec<MarkdownLink> {
    let mut links = Vec::new();
    let mut cursor = 0;
    while let Some(open_rel) = body[cursor..].find('[') {
        let open = cursor + open_rel;
        if open > 0 && body[..open].ends_with('!') {
            cursor = open + 1;
            continue;
        }
        let text_start = open + 1;
        let Some(close_rel) = body[text_start..].find(']') else {
            break;
        };
        let close = text_start + close_rel;
        let after_close = close + 1;
        let Some(after_open_paren) = body[after_close..].strip_prefix('(') else {
            cursor = after_close;
            continue;
        };
        let target_start = body.len() - after_open_paren.len();
        let Some(end_rel) = body[target_start..].find(')') else {
            break;
        };
        let target_end = target_start + end_rel;
        links.push(MarkdownLink {
            text: body[text_start..close].to_string(),
            target: body[target_start..target_end].to_string(),
        });
        cursor = target_end + 1;
        if cursor >= body.len() {
            break;
        }
    }
    links
}

fn resolve_link_path(source_path: &str, target: &str) -> Option<String> {
    let target = target.split('#').next().unwrap_or(target);
    if target.is_empty()
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with('/')
    {
        return None;
    }
    let mut base = PathBuf::from(source_path);
    base.pop();
    for part in target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                base.pop();
            }
            component => base.push(component),
        }
    }
    let normalized = pathbuf_to_posix(&base).ok()?;
    safe_relative_path(&normalized).ok()?;
    Some(normalized)
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, OkfParseError> {
    fn recurse(out: &mut Vec<PathBuf>, root: &Path, dir: &Path) -> Result<(), OkfParseError> {
        let mut entries = fs::read_dir(dir)
            .map_err(|e| OkfParseError::new(format!("cannot read {}: {e}", dir.display())))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| OkfParseError::new(format!("cannot read {}: {e}", dir.display())))?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type().map_err(|e| {
                OkfParseError::new(format!("cannot inspect {}: {e}", path.display()))
            })?;
            if file_type.is_symlink() {
                return Err(OkfParseError::new(format!(
                    "symlink not supported in OKF bundle: {}",
                    path.display()
                )));
            }
            if file_type.is_dir() {
                if path.strip_prefix(root).ok().is_some_and(|rel| {
                    rel.components()
                        .next()
                        .is_some_and(|c| c.as_os_str() == ".gts-okf")
                }) {
                    continue;
                }
                recurse(out, root, &path)?;
            } else if file_type.is_file()
                && path.extension().and_then(|ext| ext.to_str()) == Some("md")
            {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    recurse(&mut out, root, root)?;
    out.sort();
    Ok(out)
}

fn is_frontmatterless_index(bytes: &[u8], path: &str) -> Result<bool, OkfParseError> {
    if path.rsplit('/').next() != Some("index.md") {
        return Ok(false);
    }
    if has_yaml_frontmatter_bytes(bytes) {
        return Ok(false);
    }
    std::str::from_utf8(bytes)
        .map_err(|e| OkfParseError::new(format!("{path}: markdown is not UTF-8: {e}")))?;
    Ok(true)
}

fn has_yaml_frontmatter_bytes(bytes: &[u8]) -> bool {
    bytes.starts_with(b"---\n") || bytes.starts_with(b"---\r\n")
}

fn has_yaml_frontmatter(text: &str) -> bool {
    has_yaml_frontmatter_bytes(text.as_bytes())
}

fn relative_okf_path(root: &Path, path: &Path) -> Result<String, OkfParseError> {
    let rel = path.strip_prefix(root).map_err(|_| {
        OkfParseError::new(format!(
            "path {} is outside bundle root {}",
            path.display(),
            root.display()
        ))
    })?;
    let text = pathbuf_to_posix(rel)?;
    safe_relative_path(&text)?;
    Ok(text)
}

fn pathbuf_to_posix(path: &Path) -> Result<String, OkfParseError> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| OkfParseError::new("OKF paths must be UTF-8"))?
                    .to_string(),
            ),
            Component::CurDir => {}
            _ => {
                return Err(OkfParseError::new(format!(
                    "unsupported OKF path component in {}",
                    path.display()
                )))
            }
        }
    }
    Ok(parts.join("/"))
}

fn safe_relative_path(path: &str) -> Result<(), OkfParseError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(OkfParseError::new(format!(
            "unsafe OKF relative path: {path}"
        )));
    }
    Ok(())
}

fn parse_markdown(bytes: &[u8], path: &str) -> Result<(YamlMapping, Vec<u8>), OkfParseError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| OkfParseError::new(format!("{path}: markdown is not UTF-8: {e}")))?;
    if !has_yaml_frontmatter(text) {
        return Err(OkfParseError::new(format!(
            "{path}: OKF document is missing YAML frontmatter"
        )));
    }
    let first_line_len = if text.starts_with("---\r\n") { 5 } else { 4 };
    let mut offset = first_line_len;
    while offset <= text.len() {
        let Some(line_end_rel) = text[offset..].find('\n') else {
            break;
        };
        let line_end = offset + line_end_rel;
        let line = text[offset..line_end].trim_end_matches('\r');
        let after_line = line_end + 1;
        if line == "---" {
            let yaml_text = &text[first_line_len..offset];
            let value: YamlValue = serde_yaml::from_str(yaml_text).map_err(|e| {
                OkfParseError::new(format!("{path}: invalid YAML frontmatter: {e}"))
            })?;
            let YamlValue::Mapping(mapping) = value else {
                return Err(OkfParseError::new(format!(
                    "{path}: YAML frontmatter must be a mapping"
                )));
            };
            return Ok((mapping, bytes[after_line..].to_vec()));
        }
        offset = after_line;
    }
    Err(OkfParseError::new(format!(
        "{path}: YAML frontmatter is missing closing ---"
    )))
}

fn string_field(mapping: &YamlMapping, key: &str) -> Result<Option<String>, OkfParseError> {
    match mapping.get(YamlValue::String(key.to_string())) {
        None => Ok(None),
        Some(YamlValue::String(text)) => Ok(Some(text.clone())),
        Some(YamlValue::Number(number)) => Ok(Some(number.to_string())),
        Some(YamlValue::Bool(flag)) => Ok(Some(flag.to_string())),
        Some(_) => Err(OkfParseError::new(format!(
            "frontmatter field {key} must be a scalar"
        ))),
    }
}

fn required_string_field(
    mapping: &YamlMapping,
    key: &str,
    path: &str,
) -> Result<String, OkfParseError> {
    match string_field(mapping, key)? {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(OkfParseError::new(format!(
            "{path}: OKF document is missing required `{key}` frontmatter"
        ))),
    }
}

fn tags_field(mapping: &YamlMapping) -> Result<Vec<String>, OkfParseError> {
    match mapping.get(YamlValue::String("tags".to_string())) {
        None => Ok(Vec::new()),
        Some(YamlValue::Sequence(items)) => {
            let mut tags = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    YamlValue::String(text) => tags.push(text.clone()),
                    YamlValue::Number(number) => tags.push(number.to_string()),
                    YamlValue::Bool(flag) => tags.push(flag.to_string()),
                    _ => return Err(OkfParseError::new("frontmatter tags must be scalars")),
                }
            }
            tags.sort();
            tags.dedup();
            Ok(tags)
        }
        Some(_) => Err(OkfParseError::new(
            "frontmatter tags must be a YAML sequence",
        )),
    }
}

fn yaml_to_json(value: &YamlValue) -> Result<JsonValue, OkfParseError> {
    match value {
        YamlValue::Null => Ok(JsonValue::Null),
        YamlValue::Bool(flag) => Ok(JsonValue::Bool(*flag)),
        YamlValue::Number(number) => Ok(serde_json::from_str(&number.to_string())
            .unwrap_or_else(|_| JsonValue::String(number.to_string()))),
        YamlValue::String(text) => Ok(JsonValue::String(text.clone())),
        YamlValue::Sequence(items) => items.iter().map(yaml_to_json).collect(),
        YamlValue::Mapping(map) => {
            let mut out = serde_json::Map::new();
            let mut entries = Vec::new();
            for (key, value) in map {
                let Some(key) = key.as_str() else {
                    return Err(OkfParseError::new(
                        "nested OKF extension object keys must be strings",
                    ));
                };
                entries.push((key.to_string(), yaml_to_json(value)?));
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (key, value) in entries {
                out.insert(key, value);
            }
            Ok(JsonValue::Object(out))
        }
        YamlValue::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

fn minted_doc_iri(base_iri: &str, path: &str) -> String {
    format!("{base_iri}{}", percent_encode_path(path))
}

fn percent_encode_path(path: &str) -> String {
    let mut out = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn manifest_meta(options: &FromOkfOptions, documents: &[Document]) -> CborValue {
    CborValue::Map(vec![
        ("schema".into(), "gts-okf-v1".into()),
        ("base_iri".into(), options.base_iri.clone().into()),
        ("doc_count".into(), CborValue::from(documents.len() as u64)),
        (
            "source_paths".into(),
            CborValue::Array(
                documents
                    .iter()
                    .map(|doc| CborValue::Text(doc.path.clone()))
                    .collect(),
            ),
        ),
    ])
}

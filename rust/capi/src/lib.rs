// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stable C ABI wrapper for the Rust GTS core.
//!
//! The ABI deliberately returns bytes or JSON reports instead of exposing Rust
//! graph structs. Every exported operation catches panics, reports structured
//! errors through an opaque handle, and returns only caller-freeable buffers.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;

use gmeow_gts::files::{diff, pack, unpack_with_options, UnpackOptions};
use gmeow_gts::from_nquads::from_nquads;
use gmeow_gts::from_yamlld::from_json_ld;
use gmeow_gts::model::{BlobEntry, Diagnostic, Graph};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::rdf_codecs::{
    from_ntriples, from_rdf_xml, from_trig, from_turtle, to_ntriples, to_rdf_xml, to_trig,
    to_turtle,
};
use gmeow_gts::reader::read;
use gmeow_gts::verify::{verify_file, VerificationResult};
use gmeow_gts::wire::hex;
use gmeow_gts::yamlld::to_json_ld_string;
use serde_json::json;

const ABI_VERSION: u32 = 1;

const STATUS_OK: c_int = 0;
const STATUS_INVALID_ARGUMENT: c_int = 1;
const STATUS_IO: c_int = 2;
const STATUS_PARSE: c_int = 3;
const STATUS_DIAGNOSTIC: c_int = 4;
const STATUS_INTERNAL: c_int = 5;
const STATUS_PANIC: c_int = 6;

const UNPACK_INCLUDE_SUPPRESSED: u32 = 1 << 0;
const UNPACK_ALLOW_SYMLINKS: u32 = 1 << 1;
const UNPACK_ALLOW_SPECIAL: u32 = 1 << 2;
const UNPACK_SAME_OWNER: u32 = 1 << 3;
const UNPACK_PRESERVE_SETID: u32 = 1 << 4;

const VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[repr(C)]
pub struct GtsBuffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

pub struct GtsError {
    code: CString,
    message: CString,
}

#[derive(Debug)]
struct ApiError {
    status: c_int,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_INVALID_ARGUMENT,
            code: "invalid-argument",
            message: message.into(),
        }
    }

    fn io(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_IO,
            code: "io-error",
            message: message.into(),
        }
    }

    fn parse(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_PARSE,
            code: "parse-error",
            message: message.into(),
        }
    }

    fn diagnostic(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_DIAGNOSTIC,
            code: "diagnostic",
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_INTERNAL,
            code: "internal-error",
            message: message.into(),
        }
    }

    fn panic() -> Self {
        Self {
            status: STATUS_PANIC,
            code: "panic",
            message: "Rust panic was caught at the C ABI boundary".to_string(),
        }
    }
}

type FormatParser = fn(&str) -> Result<Vec<u8>, ApiError>;
type FormatSerializer = fn(&Graph) -> Result<String, ApiError>;

struct FormatCodec {
    id: &'static str,
    label: &'static str,
    media_types: &'static [&'static str],
    aliases: &'static [&'static str],
    extensions: &'static [&'static str],
    parse: FormatParser,
    serialize: FormatSerializer,
}

impl FormatCodec {
    fn matches(&self, normalized: &str) -> bool {
        self.id == normalized
            || self.aliases.contains(&normalized)
            || self.media_types.contains(&normalized)
            || self.extensions.contains(&normalized)
    }
}

const FORMAT_CODECS: &[FormatCodec] = &[
    FormatCodec {
        id: "nquads",
        label: "N-Quads",
        media_types: &["application/n-quads", "application/nquads"],
        aliases: &["n-quads", "nq"],
        extensions: &["nq"],
        parse: parse_nquads_format,
        serialize: serialize_nquads_format,
    },
    FormatCodec {
        id: "ntriples",
        label: "N-Triples",
        media_types: &["application/n-triples", "application/ntriples"],
        aliases: &["n-triples", "nt"],
        extensions: &["nt"],
        parse: parse_ntriples_format,
        serialize: serialize_ntriples_format,
    },
    FormatCodec {
        id: "turtle",
        label: "Turtle",
        media_types: &["text/turtle", "application/turtle"],
        aliases: &["ttl"],
        extensions: &["ttl"],
        parse: parse_turtle_format,
        serialize: serialize_turtle_format,
    },
    FormatCodec {
        id: "trig",
        label: "TriG",
        media_types: &["application/trig", "application/x-trig"],
        aliases: &[],
        extensions: &["trig"],
        parse: parse_trig_format,
        serialize: serialize_trig_format,
    },
    FormatCodec {
        id: "rdfxml",
        label: "RDF/XML",
        media_types: &["application/rdf+xml"],
        aliases: &["rdf-xml"],
        extensions: &["rdf", "owl", "xml"],
        parse: parse_rdf_xml_format,
        serialize: serialize_rdf_xml_format,
    },
    FormatCodec {
        id: "jsonld",
        label: "JSON-LD-star profile",
        media_types: &["application/ld+json"],
        aliases: &["json-ld", "json"],
        extensions: &["jsonld", "json"],
        parse: parse_json_ld_format,
        serialize: serialize_json_ld_format,
    },
];

fn parse_nquads_format(text: &str) -> Result<Vec<u8>, ApiError> {
    from_nquads(text).map_err(|err| ApiError::parse(err.to_string()))
}

fn serialize_nquads_format(graph: &Graph) -> Result<String, ApiError> {
    Ok(to_nquads(graph))
}

fn parse_ntriples_format(text: &str) -> Result<Vec<u8>, ApiError> {
    from_ntriples(text).map_err(|err| ApiError::parse(err.to_string()))
}

fn serialize_ntriples_format(graph: &Graph) -> Result<String, ApiError> {
    to_ntriples(graph).map_err(|err| ApiError::diagnostic(err.to_string()))
}

fn parse_turtle_format(text: &str) -> Result<Vec<u8>, ApiError> {
    from_turtle(text).map_err(|err| ApiError::parse(err.to_string()))
}

fn serialize_turtle_format(graph: &Graph) -> Result<String, ApiError> {
    to_turtle(graph).map_err(|err| ApiError::diagnostic(err.to_string()))
}

fn parse_trig_format(text: &str) -> Result<Vec<u8>, ApiError> {
    from_trig(text).map_err(|err| ApiError::parse(err.to_string()))
}

fn serialize_trig_format(graph: &Graph) -> Result<String, ApiError> {
    to_trig(graph).map_err(|err| ApiError::diagnostic(err.to_string()))
}

fn parse_rdf_xml_format(text: &str) -> Result<Vec<u8>, ApiError> {
    from_rdf_xml(text).map_err(|err| ApiError::parse(err.to_string()))
}

fn serialize_rdf_xml_format(graph: &Graph) -> Result<String, ApiError> {
    to_rdf_xml(graph).map_err(|err| ApiError::diagnostic(err.to_string()))
}

fn parse_json_ld_format(text: &str) -> Result<Vec<u8>, ApiError> {
    from_json_ld(text).map_err(|err| ApiError::parse(err.to_string()))
}

fn serialize_json_ld_format(graph: &Graph) -> Result<String, ApiError> {
    to_json_ld_string(graph)
        .map_err(|err| ApiError::internal(format!("JSON-LD serialization failed: {err}")))
}

fn normalize_format(value: &str) -> String {
    let without_parameters = value.split_once(';').map_or(value, |(base, _)| base);
    without_parameters
        .trim()
        .strip_prefix('.')
        .unwrap_or_else(|| without_parameters.trim())
        .to_ascii_lowercase()
}

fn resolve_format(format: &str) -> Result<&'static FormatCodec, ApiError> {
    let normalized = normalize_format(format);
    if normalized.is_empty() {
        return Err(ApiError::invalid_argument("format is empty"));
    }
    FORMAT_CODECS
        .iter()
        .find(|codec| codec.matches(&normalized))
        .ok_or_else(|| {
            ApiError::invalid_argument(format!("unsupported RDF format or media type: {format}"))
        })
}

fn format_entries_json() -> Vec<serde_json::Value> {
    FORMAT_CODECS
        .iter()
        .map(|codec| {
            json!({
                "id": codec.id,
                "label": codec.label,
                "media_types": codec.media_types,
                "aliases": codec.aliases,
                "extensions": codec.extensions,
                "can_parse": true,
                "can_serialize": true,
            })
        })
        .collect()
}

fn format_registry_json() -> serde_json::Value {
    json!({
        "schema": "gts-capi-format-registry-v1",
        "formats": format_entries_json(),
    })
}

fn serialize_gts_as_format(data: &[u8], format: &str) -> Result<Vec<u8>, ApiError> {
    let codec = resolve_format(format)?;
    let graph = clean_graph(data)?;
    (codec.serialize)(&graph).map(String::into_bytes)
}

fn parse_format_to_gts(format: &str, text: &str) -> Result<Vec<u8>, ApiError> {
    let codec = resolve_format(format)?;
    (codec.parse)(text)
}

fn ffi_entry<F>(error_out: *mut *mut GtsError, f: F) -> c_int
where
    F: FnOnce() -> Result<(), ApiError>,
{
    clear_error_slot(error_out);
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => STATUS_OK,
        Ok(Err(err)) => {
            write_error(error_out, &err);
            err.status
        }
        Err(_) => {
            let err = ApiError::panic();
            write_error(error_out, &err);
            err.status
        }
    }
}

fn clear_error_slot(error_out: *mut *mut GtsError) {
    if !error_out.is_null() {
        unsafe {
            *error_out = ptr::null_mut();
        }
    }
}

fn write_error(error_out: *mut *mut GtsError, err: &ApiError) {
    if error_out.is_null() {
        return;
    }
    let error = GtsError {
        code: cstring_lossy(err.code),
        message: cstring_lossy(&err.message),
    };
    unsafe {
        *error_out = Box::into_raw(Box::new(error));
    }
}

fn cstring_lossy(value: &str) -> CString {
    let sanitized = value.replace('\0', "\\0");
    CString::new(sanitized).expect("NUL bytes were escaped")
}

fn require_out(out: *mut GtsBuffer) -> Result<(), ApiError> {
    if out.is_null() {
        Err(ApiError::invalid_argument("output buffer pointer is null"))
    } else {
        Ok(())
    }
}

fn write_buffer(out: *mut GtsBuffer, mut data: Vec<u8>) -> Result<(), ApiError> {
    require_out(out)?;
    if data.is_empty() {
        unsafe {
            *out = GtsBuffer {
                data: ptr::null_mut(),
                len: 0,
                capacity: 0,
            };
        }
        return Ok(());
    }
    data.shrink_to_fit();
    let buffer = GtsBuffer {
        data: data.as_mut_ptr(),
        len: data.len(),
        capacity: data.capacity(),
    };
    std::mem::forget(data);
    unsafe {
        *out = buffer;
    }
    Ok(())
}

fn input_slice<'a>(data: *const u8, len: usize) -> Result<&'a [u8], ApiError> {
    if data.is_null() {
        if len == 0 {
            Ok(&[])
        } else {
            Err(ApiError::invalid_argument("input data pointer is null"))
        }
    } else {
        Ok(unsafe { slice::from_raw_parts(data, len) })
    }
}

fn input_str<'a>(data: *const c_char, len: usize) -> Result<&'a str, ApiError> {
    let bytes = input_slice(data.cast::<u8>(), len)?;
    std::str::from_utf8(bytes)
        .map_err(|err| ApiError::invalid_argument(format!("input is not UTF-8: {err}")))
}

fn input_cstr<'a>(value: *const c_char, name: &str) -> Result<&'a str, ApiError> {
    if value.is_null() {
        return Err(ApiError::invalid_argument(format!(
            "{name} pointer is null"
        )));
    }
    let raw = unsafe { CStr::from_ptr(value) };
    raw.to_str()
        .map_err(|err| ApiError::invalid_argument(format!("{name} is not UTF-8: {err}")))
}

fn c_path<'a>(path: *const c_char, name: &str) -> Result<&'a Path, ApiError> {
    if path.is_null() {
        return Err(ApiError::invalid_argument(format!(
            "{name} path pointer is null"
        )));
    }
    let raw = unsafe { CStr::from_ptr(path) };
    let text = raw
        .to_str()
        .map_err(|err| ApiError::invalid_argument(format!("{name} path is not UTF-8: {err}")))?;
    Ok(Path::new(text))
}

fn c_paths(paths: *const *const c_char, count: usize) -> Result<Vec<PathBuf>, ApiError> {
    if count == 0 {
        return Err(ApiError::invalid_argument("path list is empty"));
    }
    if paths.is_null() {
        return Err(ApiError::invalid_argument("path list pointer is null"));
    }
    let raw = unsafe { slice::from_raw_parts(paths, count) };
    raw.iter()
        .enumerate()
        .map(|(idx, path)| c_path(*path, &format!("source[{idx}]")).map(Path::to_path_buf))
        .collect()
}

fn clean_graph(data: &[u8]) -> Result<Graph, ApiError> {
    let graph = read(data, true, None);
    if graph.segment_heads.is_empty() {
        return Err(ApiError::diagnostic(
            "input did not contain a valid GTS segment",
        ));
    }
    if !graph.diagnostics.is_empty() {
        let codes = graph
            .diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ApiError::diagnostic(format!(
            "input folded with diagnostics: {codes}"
        )));
    }
    Ok(graph)
}

fn diagnostic_json(diagnostic: &Diagnostic) -> serde_json::Value {
    json!({
        "code": diagnostic.code,
        "detail": diagnostic.detail,
        "frame_index": diagnostic.frame_index,
    })
}

fn graph_json(graph: &Graph) -> serde_json::Value {
    json!({
        "schema": "gts-capi-read-v1",
        "clean": graph.diagnostics.is_empty() && !graph.segment_heads.is_empty(),
        "counts": {
            "terms": graph.terms.len(),
            "quads": graph.quads.len(),
            "reifiers": graph.reifiers.len(),
            "annotations": graph.annotations.len(),
            "blobs": graph.blobs.len(),
            "opaque": graph.opaque.len(),
            "signatures": graph.signatures.len(),
            "segments": graph.segment_heads.len(),
            "diagnostics": graph.diagnostics.len(),
        },
        "segment_heads": graph.segment_heads.iter().map(|head| hex(head)).collect::<Vec<_>>(),
        "profiles": graph.segment_profiles,
        "streamable": graph.segment_streamable.iter().map(|item| json!({
            "claimed": item.claimed,
            "covered": item.covered,
            "tail": item.tail,
            "head": item.head.as_ref().map(|head| hex(head)),
        })).collect::<Vec<_>>(),
        "diagnostics": graph.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
        "signatures": graph.signatures.iter().map(|sig| json!({
            "frame_id": hex(&sig.frame_id),
            "kid": sig.kid,
            "status": sig.status,
        })).collect::<Vec<_>>(),
        "blobs": graph.blobs.iter().map(|(digest, entry)| json!({
            "digest": digest,
            "size": blob_size(entry),
        })).collect::<Vec<_>>(),
    })
}

fn blob_size(entry: &BlobEntry) -> serde_json::Value {
    match entry.decoded_len() {
        Ok(size) => json!(size),
        Err(err) => json!({"error": err.to_string()}),
    }
}

fn verify_json(result: &VerificationResult) -> serde_json::Value {
    json!({
        "schema": "gts-capi-verify-v1",
        "ok": result.ok,
        "kid": result.kid,
        "fingerprint": result.fingerprint,
        "emojihash": result.emojihash,
        "emojihash_labels": result.emojihash_labels,
        "randomart": result.randomart,
        "frames": result.frames,
        "signed": result.signed,
        "valid": result.valid,
        "trusted": result.trusted,
        "invalid": result.invalid,
        "unverified": result.unverified,
        "errors": result.errors,
        "diagnostics": result.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
        "profile_findings": result.profile_findings.iter().map(|finding| json!({
            "code": finding.code,
            "severity": finding.severity.as_str(),
            "detail": finding.detail,
            "profile": finding.profile,
            "segment_index": finding.segment_index,
        })).collect::<Vec<_>>(),
    })
}

fn json_bytes(value: serde_json::Value) -> Result<Vec<u8>, ApiError> {
    serde_json::to_vec(&value)
        .map_err(|err| ApiError::internal(format!("JSON serialization failed: {err}")))
}

fn unpack_options(flags: u32) -> UnpackOptions {
    UnpackOptions {
        include_suppressed: flags & UNPACK_INCLUDE_SUPPRESSED != 0,
        allow_symlinks: flags & UNPACK_ALLOW_SYMLINKS != 0,
        allow_special: flags & UNPACK_ALLOW_SPECIAL != 0,
        same_owner: flags & UNPACK_SAME_OWNER != 0,
        preserve_setid: flags & UNPACK_PRESERVE_SETID != 0,
    }
}

fn build_metadata_json() -> serde_json::Value {
    json!({
        "schema": "gts-capi-build-v1",
        "abi_version": ABI_VERSION,
        "version": env!("CARGO_PKG_VERSION"),
        "package": env!("CARGO_PKG_NAME"),
        "core_package": "gmeow-gts",
        "library": "libgts",
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "target": {
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
            "os": std::env::consts::OS,
            "pointer_width": std::mem::size_of::<usize>() * 8,
        }
    })
}

#[no_mangle]
pub extern "C" fn gts_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn gts_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

#[no_mangle]
/// Release a buffer returned by this library.
///
/// # Safety
///
/// `buffer` must be null or a valid pointer to a `GtsBuffer` initialized by
/// this library. After return the buffer is reset to empty.
pub unsafe extern "C" fn gts_buffer_free(buffer: *mut GtsBuffer) {
    if buffer.is_null() {
        return;
    }
    unsafe {
        let buf = &mut *buffer;
        if !buf.data.is_null() && buf.capacity > 0 {
            drop(Vec::from_raw_parts(buf.data, buf.len, buf.capacity));
        }
        buf.data = ptr::null_mut();
        buf.len = 0;
        buf.capacity = 0;
    }
}

#[no_mangle]
/// Release an error handle returned by this library.
///
/// # Safety
///
/// `error` must be null or a pointer returned through a `GtsError **` output
/// parameter by this library.
pub unsafe extern "C" fn gts_error_free(error: *mut GtsError) {
    if !error.is_null() {
        unsafe {
            drop(Box::from_raw(error));
        }
    }
}

#[no_mangle]
/// Return the stable error code for an error handle.
///
/// # Safety
///
/// `error` must be null or a live error handle returned by this library. The
/// returned pointer is owned by the error handle and becomes invalid after
/// `gts_error_free`.
pub unsafe extern "C" fn gts_error_code(error: *const GtsError) -> *const c_char {
    if error.is_null() {
        ptr::null()
    } else {
        unsafe { (*error).code.as_ptr() }
    }
}

#[no_mangle]
/// Return the human-readable message for an error handle.
///
/// # Safety
///
/// `error` must be null or a live error handle returned by this library. The
/// returned pointer is owned by the error handle and becomes invalid after
/// `gts_error_free`.
pub unsafe extern "C" fn gts_error_message(error: *const GtsError) -> *const c_char {
    if error.is_null() {
        ptr::null()
    } else {
        unsafe { (*error).message.as_ptr() }
    }
}

#[no_mangle]
/// Return build metadata as UTF-8 JSON.
///
/// # Safety
///
/// `out` must be a valid writable buffer pointer. `error` may be null or a
/// valid writable error-handle slot.
pub unsafe extern "C" fn gts_build_metadata_json(
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        write_buffer(out, json_bytes(build_metadata_json())?)
    })
}

#[no_mangle]
/// Return ABI capabilities as UTF-8 JSON.
///
/// # Safety
///
/// `out` must be a valid writable buffer pointer. `error` may be null or a
/// valid writable error-handle slot.
pub unsafe extern "C" fn gts_capabilities_json(
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        write_buffer(
            out,
            json_bytes(json!({
                "schema": "gts-capi-capabilities-v1",
                "abi_version": ABI_VERSION,
                "version": env!("CARGO_PKG_VERSION"),
                "library": "libgts",
                "core": "gmeow-gts",
                "threading": "operations are reentrant; buffers and errors are caller-owned",
                "operations": [
                    "build_metadata_json",
                    "formats_json",
                    "read_json",
                    "verify_json",
                    "to_format",
                    "from_format",
                    "to_nquads",
                    "from_nquads",
                    "files_pack",
                    "files_unpack",
                    "files_diff_json"
                ],
                "features": {
                    "format_registry": true,
                    "files_profile": true,
                    "rdf_formats": true,
                    "json_ld_star_profile": true,
                    "nquads": true,
                    "verification": true,
                    "opaque_graph_handles": false
                },
                "formats": format_entries_json()
            }))?,
        )
    })
}

#[no_mangle]
/// Return the registered RDF text codecs as UTF-8 JSON.
///
/// # Safety
///
/// `out` must be a valid writable buffer pointer. `error` may be null or a
/// valid writable error-handle slot.
pub unsafe extern "C" fn gts_formats_json(out: *mut GtsBuffer, error: *mut *mut GtsError) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        write_buffer(out, json_bytes(format_registry_json())?)
    })
}

#[no_mangle]
/// Fold GTS bytes and return a stable JSON report.
///
/// # Safety
///
/// `data` must point to `len` readable bytes unless `len` is zero. `out` must
/// be writable. `error` may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_read_json(
    data: *const u8,
    len: usize,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let graph = read(input_slice(data, len)?, true, None);
        write_buffer(out, json_bytes(graph_json(&graph))?)
    })
}

#[no_mangle]
/// Verify GTS bytes and return a stable JSON report.
///
/// # Safety
///
/// `data` must point to `len` readable bytes unless `len` is zero. `out` must
/// be writable. `error` may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_verify_json(
    data: *const u8,
    len: usize,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let result = verify_file(input_slice(data, len)?);
        write_buffer(out, json_bytes(verify_json(&result))?)
    })
}

#[no_mangle]
/// Convert clean GTS bytes to N-Quads text.
///
/// # Safety
///
/// `data` must point to `len` readable bytes unless `len` is zero. `out` must
/// be writable. `error` may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_to_nquads(
    data: *const u8,
    len: usize,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let output = serialize_gts_as_format(input_slice(data, len)?, "nquads")?;
        write_buffer(out, output)
    })
}

#[no_mangle]
/// Convert N-Quads text to GTS bytes.
///
/// # Safety
///
/// `text` must point to `len` readable UTF-8 bytes unless `len` is zero. `out`
/// must be writable. `error` may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_from_nquads(
    text: *const c_char,
    len: usize,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let nq = input_str(text, len)?;
        let data = parse_format_to_gts("nquads", nq)?;
        write_buffer(out, data)
    })
}

#[no_mangle]
/// Convert clean GTS bytes to a registered RDF text format.
///
/// `format` may be a registry id, alias, extension, or media type. Media type
/// parameters such as `charset=utf-8` are ignored during dispatch.
///
/// # Safety
///
/// `data` must point to `len` readable bytes unless `len` is zero. `format`
/// must be a NUL-terminated UTF-8 C string. `out` must be writable. `error`
/// may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_to_format(
    data: *const u8,
    len: usize,
    format: *const c_char,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let format = input_cstr(format, "format")?;
        let output = serialize_gts_as_format(input_slice(data, len)?, format)?;
        write_buffer(out, output)
    })
}

#[no_mangle]
/// Convert registered RDF text format input to GTS bytes.
///
/// `format` may be a registry id, alias, extension, or media type. Media type
/// parameters such as `charset=utf-8` are ignored during dispatch.
///
/// # Safety
///
/// `format` must be a NUL-terminated UTF-8 C string. `text` must point to
/// `len` readable UTF-8 bytes unless `len` is zero. `out` must be writable.
/// `error` may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_from_format(
    format: *const c_char,
    text: *const c_char,
    len: usize,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let format = input_cstr(format, "format")?;
        let text = input_str(text, len)?;
        let data = parse_format_to_gts(format, text)?;
        write_buffer(out, data)
    })
}

#[no_mangle]
/// Pack paths into files-profile GTS bytes.
///
/// # Safety
///
/// `paths` must point to `path_count` NUL-terminated UTF-8 C string pointers.
/// `out` must be writable. `error` may be null or a writable error-handle slot.
pub unsafe extern "C" fn gts_files_pack(
    paths: *const *const c_char,
    path_count: usize,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let owned = c_paths(paths, path_count)?;
        let refs = owned.iter().map(PathBuf::as_path).collect::<Vec<_>>();
        let data = pack(&refs).map_err(ApiError::io)?;
        write_buffer(out, data)
    })
}

#[no_mangle]
/// Unpack a clean files-profile GTS archive into a destination directory.
///
/// # Safety
///
/// `data` must point to `len` readable bytes unless `len` is zero. `dest` must
/// be a NUL-terminated UTF-8 path. `out` must be writable. `error` may be null
/// or a writable error-handle slot.
pub unsafe extern "C" fn gts_files_unpack(
    data: *const u8,
    len: usize,
    dest: *const c_char,
    flags: u32,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let destination = c_path(dest, "destination")?;
        let graph = clean_graph(input_slice(data, len)?)?;
        unpack_with_options(&graph, destination, &unpack_options(flags)).map_err(ApiError::io)?;
        write_buffer(
            out,
            json_bytes(json!({
                "schema": "gts-capi-files-unpack-v1",
                "ok": true,
                "destination": destination.display().to_string(),
            }))?,
        )
    })
}

#[no_mangle]
/// Compare a clean files-profile GTS archive to a directory and return JSON.
///
/// # Safety
///
/// `data` must point to `len` readable bytes unless `len` is zero. `directory`
/// must be a NUL-terminated UTF-8 path. `out` must be writable. `error` may be
/// null or a writable error-handle slot.
pub unsafe extern "C" fn gts_files_diff_json(
    data: *const u8,
    len: usize,
    directory: *const c_char,
    out: *mut GtsBuffer,
    error: *mut *mut GtsError,
) -> c_int {
    ffi_entry(error, || {
        require_out(out)?;
        let directory = c_path(directory, "directory")?;
        let graph = clean_graph(input_slice(data, len)?)?;
        let changes = diff(&graph, directory).map_err(ApiError::io)?;
        write_buffer(
            out,
            json_bytes(json!({
                "schema": "gts-capi-files-diff-v1",
                "clean": changes.is_empty(),
                "changes": changes,
            }))?,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn vectors() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors")
    }

    fn sample_default_graph_gts() -> Vec<u8> {
        from_nquads("<https://example.test/s> <https://example.test/p> \"Cat\"@en .\n").unwrap()
    }

    fn call_buffer(
        f: impl FnOnce(*mut GtsBuffer, *mut *mut GtsError) -> c_int,
    ) -> Result<Vec<u8>, String> {
        let mut buffer = GtsBuffer {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        };
        let mut error: *mut GtsError = ptr::null_mut();
        let status = f(&mut buffer, &mut error);
        if status != STATUS_OK {
            let message = unsafe {
                CStr::from_ptr(gts_error_message(error))
                    .to_string_lossy()
                    .into_owned()
            };
            unsafe {
                gts_error_free(error);
            }
            return Err(message);
        }
        let out = unsafe { slice::from_raw_parts(buffer.data, buffer.len).to_vec() };
        unsafe {
            gts_buffer_free(&mut buffer);
        }
        Ok(out)
    }

    #[test]
    fn capabilities_and_build_metadata_are_reported() {
        let capabilities =
            call_buffer(|out, err| unsafe { gts_capabilities_json(out, err) }).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&capabilities).unwrap();
        assert_eq!(value["schema"], "gts-capi-capabilities-v1");
        assert!(value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "build_metadata_json"));
        assert!(value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "to_format"));
        assert_eq!(value["features"]["format_registry"], true);

        let metadata =
            call_buffer(|out, err| unsafe { gts_build_metadata_json(out, err) }).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&metadata).unwrap();
        assert_eq!(value["schema"], "gts-capi-build-v1");
        assert_eq!(value["abi_version"], ABI_VERSION);
        assert_eq!(value["library"], "libgts");
        assert!(!value["target"]["arch"].as_str().unwrap().is_empty());
    }

    #[test]
    fn format_registry_reports_supported_codecs() {
        let formats = call_buffer(|out, err| unsafe { gts_formats_json(out, err) }).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&formats).unwrap();
        assert_eq!(value["schema"], "gts-capi-format-registry-v1");
        let ids = value["formats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        for expected in ["nquads", "ntriples", "turtle", "trig", "rdfxml", "jsonld"] {
            assert!(ids.contains(&expected), "missing format {expected}");
        }
    }

    #[test]
    fn read_verify_and_nquads_round_trip() {
        let data = fs::read(vectors().join("01-minimal.gts")).unwrap();
        let read_json =
            call_buffer(|out, err| unsafe { gts_read_json(data.as_ptr(), data.len(), out, err) })
                .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&read_json).unwrap();
        assert_eq!(value["schema"], "gts-capi-read-v1");
        assert_eq!(value["clean"], true);

        let verify_json =
            call_buffer(|out, err| unsafe { gts_verify_json(data.as_ptr(), data.len(), out, err) })
                .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&verify_json).unwrap();
        assert_eq!(value["schema"], "gts-capi-verify-v1");

        let nq =
            call_buffer(|out, err| unsafe { gts_to_nquads(data.as_ptr(), data.len(), out, err) })
                .unwrap();
        assert!(String::from_utf8(nq.clone())
            .unwrap()
            .contains("\"Cat\"@en"));

        let round = call_buffer(|out, err| unsafe {
            gts_from_nquads(nq.as_ptr().cast::<c_char>(), nq.len(), out, err)
        })
        .unwrap();
        assert!(!round.is_empty());
    }

    #[test]
    fn registered_formats_round_trip_through_c_abi() {
        let data = sample_default_graph_gts();
        for format in ["nquads", "ntriples", "turtle", "trig", "rdfxml", "jsonld"] {
            let format_c = CString::new(format).unwrap();
            let text = call_buffer(|out, err| unsafe {
                gts_to_format(data.as_ptr(), data.len(), format_c.as_ptr(), out, err)
            })
            .unwrap_or_else(|err| panic!("{format} serialize failed: {err}"));
            assert!(!text.is_empty(), "{format} serialization was empty");

            let round = call_buffer(|out, err| unsafe {
                gts_from_format(
                    format_c.as_ptr(),
                    text.as_ptr().cast::<c_char>(),
                    text.len(),
                    out,
                    err,
                )
            })
            .unwrap_or_else(|err| panic!("{format} parse failed: {err}"));
            assert!(!round.is_empty(), "{format} parse output was empty");

            let read_json = call_buffer(|out, err| unsafe {
                gts_read_json(round.as_ptr(), round.len(), out, err)
            })
            .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&read_json).unwrap();
            assert_eq!(value["clean"], true, "{format} round trip was not clean");
        }
    }

    #[test]
    fn media_type_and_extension_dispatch_share_the_registry() {
        let data = sample_default_graph_gts();
        let media_type = CString::new("text/turtle; charset=utf-8").unwrap();
        let turtle = call_buffer(|out, err| unsafe {
            gts_to_format(data.as_ptr(), data.len(), media_type.as_ptr(), out, err)
        })
        .unwrap();
        assert!(String::from_utf8(turtle.clone()).unwrap().contains("Cat"));

        let extension = CString::new(".ttl").unwrap();
        let round = call_buffer(|out, err| unsafe {
            gts_from_format(
                extension.as_ptr(),
                turtle.as_ptr().cast::<c_char>(),
                turtle.len(),
                out,
                err,
            )
        })
        .unwrap();
        assert!(!round.is_empty());
    }

    #[test]
    fn unsupported_format_reports_structured_error() {
        let data = sample_default_graph_gts();
        let format = CString::new("application/x-not-rdf").unwrap();
        let mut buffer = GtsBuffer {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        };
        let mut error: *mut GtsError = ptr::null_mut();
        let status = unsafe {
            gts_to_format(
                data.as_ptr(),
                data.len(),
                format.as_ptr(),
                &mut buffer,
                &mut error,
            )
        };
        assert_eq!(status, STATUS_INVALID_ARGUMENT);
        assert!(!error.is_null());
        let message = unsafe { CStr::from_ptr(gts_error_message(error)) }
            .to_str()
            .unwrap();
        assert!(message.contains("unsupported RDF format"));
        unsafe {
            gts_error_free(error);
            gts_buffer_free(&mut buffer);
        }
    }

    #[test]
    fn invalid_nquads_reports_structured_error() {
        let text = b"<https://example/s> <https://example/p> .\n";
        let mut buffer = GtsBuffer {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        };
        let mut error: *mut GtsError = ptr::null_mut();
        let status = unsafe {
            gts_from_nquads(
                text.as_ptr().cast::<c_char>(),
                text.len(),
                &mut buffer,
                &mut error,
            )
        };
        assert_eq!(status, STATUS_PARSE);
        assert!(!error.is_null());
        let code = unsafe { CStr::from_ptr(gts_error_code(error)) }
            .to_str()
            .unwrap();
        assert_eq!(code, "parse-error");
        unsafe {
            gts_error_free(error);
        }
    }

    #[test]
    fn unpack_rejects_null_output_before_side_effects() {
        let data = fs::read(vectors().join("01-minimal.gts")).unwrap();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dest =
            std::env::temp_dir().join(format!("gts-capi-null-out-{}-{nonce}", std::process::id()));
        let dest_c = CString::new(dest.to_str().unwrap()).unwrap();
        let mut error: *mut GtsError = ptr::null_mut();
        let status = unsafe {
            gts_files_unpack(
                data.as_ptr(),
                data.len(),
                dest_c.as_ptr(),
                0,
                ptr::null_mut(),
                &mut error,
            )
        };
        assert_eq!(status, STATUS_INVALID_ARGUMENT);
        assert!(!error.is_null());
        assert!(!dest.exists());
        unsafe {
            gts_error_free(error);
        }
    }
}

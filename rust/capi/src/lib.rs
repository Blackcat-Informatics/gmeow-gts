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
use gmeow_gts::model::{BlobEntry, Diagnostic, Graph};
use gmeow_gts::nquads::to_nquads;
use gmeow_gts::reader::read;
use gmeow_gts::verify::{verify_file, VerificationResult};
use gmeow_gts::wire::hex;
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
                    "read_json",
                    "verify_json",
                    "to_nquads",
                    "from_nquads",
                    "files_pack",
                    "files_unpack",
                    "files_diff_json"
                ],
                "features": {
                    "files_profile": true,
                    "nquads": true,
                    "verification": true,
                    "opaque_graph_handles": false
                }
            }))?,
        )
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
        let graph = clean_graph(input_slice(data, len)?)?;
        write_buffer(out, to_nquads(&graph).into_bytes())
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
        let nq = input_str(text, len)?;
        let data = from_nquads(nq).map_err(|err| ApiError::parse(err.to_string()))?;
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
        let graph = clean_graph(input_slice(data, len)?)?;
        let destination = c_path(dest, "destination")?;
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
        let graph = clean_graph(input_slice(data, len)?)?;
        let directory = c_path(directory, "directory")?;
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

    fn vectors() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors")
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
}

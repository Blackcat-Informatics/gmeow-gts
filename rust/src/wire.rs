// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Wire primitives: deterministic CBOR, BLAKE3 content-ids, and the id/prev
//! rule — mirror of `src/gmeow_tools/gts/wire.py`.
//!
//! A frame's `"id"` is BLAKE3-256 of the deterministic CBOR (RFC 8949 §4.2)
//! of its content — every key except `"id"` and `"sig"` (§6, §9.1). The
//! Header is hashed the same way, excluding only `"id"` (§5). `"prev"` names
//! the previous item's `"id"`; the first frame's `"prev"` is the Header's.

use ciborium::value::Value;

/// CBOR self-describe tag (RFC 8949 §3.4.6); MAY prefix the Header item (§3).
pub const SELF_DESCRIBE_TAG: u64 = 55799;

/// The magic string of the GTS wire format.
pub const MAGIC: &str = "GTS1";
/// The version number of the GTS wire format.
pub const VERSION: u8 = 1;

/// Encode a CBOR value as-is (definite lengths, shortest-form integers;
/// map entries in their current order).
///
/// # Panics
///
/// Panics if the CBOR serializer reports an error. Writing to a `Vec<u8>`
/// does not fail, so the serializer error is a violated invariant.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "writing a CBOR value to a Vec<u8> cannot fail; the expect records that invariant"
)]
pub fn encode(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::ser::into_writer(v, &mut out).expect("CBOR encoding to a Vec cannot fail");
    out
}

/// Recursively order map keys per RFC 8949 §4.2 (bytewise on encoded keys).
///
/// The spec mandates 8949 deterministic encoding, NOT a CBOR library's legacy
/// "canonical" (RFC 7049 length-first) mode — the two orderings diverge on
/// keys like `"x"` vs `"id"` (§4). Tags recurse into their value.
pub fn deterministic(v: &Value) -> Value {
    match v {
        Value::Tag(tag, inner) => Value::Tag(*tag, Box::new(deterministic(inner))),
        Value::Array(items) => Value::Array(items.iter().map(deterministic).collect()),
        Value::Map(entries) => {
            let mut keyed: Vec<(Vec<u8>, Value, Value)> = entries
                .iter()
                .map(|(k, val)| (encode(k), k.clone(), deterministic(val)))
                .collect();
            keyed.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Map(keyed.into_iter().map(|(_, k, val)| (k, val)).collect())
        }
        other => other.clone(),
    }
}

/// Encode an object as deterministic CBOR (RFC 8949 §4.2).
#[must_use]
pub fn canonical(v: &Value) -> Vec<u8> {
    encode(&deterministic(v))
}

/// The 32-byte BLAKE3-256 digest of `data`.
#[must_use]
pub fn blake3_256(data: &[u8]) -> Vec<u8> {
    blake3::hash(data).as_bytes().to_vec()
}

/// Lowercase hex of a byte string.
#[must_use]
pub fn hex(data: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(data.len() * 2);
    for &byte in data {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// A `blake3:<hex>` content digest for inline blob addressing (§12).
#[must_use]
pub fn digest_str(data: &[u8]) -> String {
    format!("blake3:{}", hex(&blake3_256(data)))
}

/// Get a map entry by text key (first match, like Python `dict.get`).
#[must_use]
pub fn map_get<'a>(entries: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v)
}

fn hash_excluding(entries: &[(Value, Value)], excluded: &[&str]) -> Vec<u8> {
    let content: Vec<(Value, Value)> = entries
        .iter()
        .filter(|(k, _)| !matches!(k, Value::Text(t) if excluded.contains(&t.as_str())))
        .cloned()
        .collect();
    blake3_256(&canonical(&Value::Map(content)))
}

/// Compute a frame's `"id"` over its content (excluding `"id"`/`"sig"`).
#[must_use]
pub fn content_id(frame: &[(Value, Value)]) -> Vec<u8> {
    hash_excluding(frame, &["id", "sig"])
}

/// Compute the Header's genesis `"id"` (excluding only `"id"`) — §5.
#[must_use]
pub fn header_id(header: &[(Value, Value)]) -> Vec<u8> {
    hash_excluding(header, &["id"])
}

struct Counting<'a> {
    data: &'a [u8],
    pos: usize,
}

impl std::io::Read for Counting<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len().min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Decode a CBOR Sequence into `(byte_offset, item)` pairs plus a torn marker.
///
/// Detects a torn append (a partial trailing item) by position: at an item
/// boundary the offset is either end-of-data (clean end) or the start of a
/// complete item; a decode failure there is a torn append (§3). Survivors are
/// returned regardless, so a reader can fold the intact prefix. The second
/// element is `None` for a clean end, or the byte offset of the incomplete
/// trailing item.
#[must_use]
pub fn iter_items(data: &[u8]) -> (Vec<(usize, Value)>, Option<usize>) {
    let mut reader = Counting { data, pos: 0 };
    let mut out = Vec::new();
    let mut torn = None;
    loop {
        let start = reader.pos;
        if start == data.len() {
            break;
        }
        if let Ok(item) = ciborium::de::from_reader::<Value, _>(&mut reader) {
            out.push((start, item));
        } else {
            // partial (EOF) or corrupt trailing item
            torn = Some(start);
            break;
        }
    }
    (out, torn)
}

/// Return the Header map, unwrapping the optional self-describe tag (§3).
///
/// # Errors
///
/// Returns an error message when the item carries a CBOR tag other than the
/// self-describe tag, or when the (untagged) item is not a CBOR map.
pub fn unwrap_header(item: &Value) -> Result<&Vec<(Value, Value)>, String> {
    let inner = match item {
        Value::Tag(tag, inner) => {
            if *tag != SELF_DESCRIBE_TAG {
                return Err(format!("unexpected CBOR tag {tag} on the header item"));
            }
            inner.as_ref()
        }
        other => other,
    };
    match inner {
        Value::Map(entries) => Ok(entries),
        _ => Err("header item is not a CBOR map".to_string()),
    }
}

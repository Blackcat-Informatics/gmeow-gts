// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal OpenPGP reader for Ed25519 armored public keys (`extract-key`, §9.2).
//!
//! This mirrors the Python `gts.openpgp` reference: it parses only the
//! unencrypted armored public-key certificates GPG emits for Ed25519 (OpenPGP
//! algorithm 22) keys, extracting the raw 32-byte key and computing the v4
//! fingerprint so GTS tooling can show the embedded transport key without
//! shelling out to `gpg`. Everything else (other algorithms, encrypted secret
//! keys, v5/v6 packets) is rejected with a clear error.

use sha1::{Digest, Sha1};

/// OpenPGP public-key algorithm id for EdDSA (RFC 9580 §9.1).
const ED25519_ALGO: u8 = 22;
/// The curve OID GPG writes for the Ed25519 signing curve (`1.3.6.1.4.1.11591.15.1`).
const ED25519_OID: [u8; 9] = [0x2b, 0x06, 0x01, 0x04, 0x01, 0xda, 0x47, 0x0f, 0x01];

/// An error parsing an armored OpenPGP key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPgpError(pub String);

impl std::fmt::Display for OpenPgpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OpenPgpError {}

type Result<T> = std::result::Result<T, OpenPgpError>;

fn err<T>(msg: &str) -> Result<T> {
    Err(OpenPgpError(msg.to_string()))
}

/// The parsed transport key: the raw Ed25519 public key plus its v4 fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportKey {
    /// The 32-byte raw Ed25519 public key (the `0x40` MPI marker stripped).
    pub raw_public: [u8; 32],
    /// Uppercase 40-hex-character OpenPGP v4 fingerprint.
    pub fingerprint: String,
}

/// Decode the packet bytes from an ASCII-armored OpenPGP block.
fn strip_armor(text: &str) -> Result<Vec<u8>> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.starts_with("-----BEGIN PGP"));
    let Some(start) = start else {
        return err("missing armor BEGIN line");
    };
    let end = lines
        .iter()
        .enumerate()
        .position(|(i, l)| i > start && l.starts_with("-----END PGP"));
    let Some(end) = end else {
        return err("missing armor END line");
    };

    let mut idx = start + 1;
    // Skip optional armor headers (Comment, Version, …) up to the blank line.
    while idx < end && !lines[idx].trim().is_empty() {
        if lines[idx].contains(':') {
            idx += 1;
        } else {
            break;
        }
    }

    let mut body = String::new();
    while idx < end {
        let line = lines[idx];
        if line.starts_with('=') {
            break; // CRC-24 checksum line — end of the base64 body.
        }
        if !line.is_empty() {
            body.push_str(line);
        }
        idx += 1;
    }
    if body.is_empty() {
        return err("empty armor body");
    }
    b64_decode(&body)
}

/// Decode a base64 string (standard alphabet, no line breaks) without pulling
/// in an external crate — the armor body is small.
fn b64_decode(s: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let Some(v) = val(c) else {
            return err("invalid base64 armor body");
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

/// Read an OpenPGP multi-precision integer; returns `(big_endian_bytes, next_offset)`.
fn read_mpi(data: &[u8], offset: usize) -> Result<(Vec<u8>, usize)> {
    if offset + 2 > data.len() {
        return err("truncated MPI length");
    }
    let bits = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    let length = bits.div_ceil(8);
    let end = offset + 2 + length;
    if end > data.len() {
        return err("truncated MPI payload");
    }
    Ok((data[offset + 2..end].to_vec(), end))
}

/// Parse one OpenPGP packet; returns `(tag, body, next_offset)`.
/// Supports both old- and new-format headers.
fn next_packet(data: &[u8], mut offset: usize) -> Result<(u8, Vec<u8>, usize)> {
    if offset >= data.len() {
        return err("truncated packet header");
    }
    let header = data[offset];
    if header & 0x80 == 0 {
        return err("invalid packet tag octet");
    }

    let tag;
    let length;
    if header & 0x40 != 0 {
        // New-format packet.
        tag = header & 0x3f;
        offset += 1;
        if offset >= data.len() {
            return err("truncated new-format length octet");
        }
        let lo = data[offset];
        if lo < 192 {
            length = lo as usize;
            offset += 1;
        } else if lo < 224 {
            if offset + 1 >= data.len() {
                return err("truncated new-format 2-octet length");
            }
            length = (((lo as usize) - 192) << 8) + data[offset + 1] as usize + 192;
            offset += 2;
        } else if lo == 255 {
            if offset + 4 >= data.len() {
                return err("truncated new-format 4-octet length");
            }
            length = u32::from_be_bytes([
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
            ]) as usize;
            offset += 5;
        } else {
            return err("partial body lengths are not supported");
        }
    } else {
        // Old-format packet.
        tag = (header >> 2) & 0x0f;
        let length_type = header & 0x03;
        offset += 1;
        match length_type {
            0 => {
                if offset >= data.len() {
                    return err("truncated old-format length octet");
                }
                length = data[offset] as usize;
                offset += 1;
            }
            1 => {
                if offset + 1 >= data.len() {
                    return err("truncated old-format 2-octet length");
                }
                length = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                offset += 2;
            }
            2 => {
                if offset + 3 >= data.len() {
                    return err("truncated old-format 4-octet length");
                }
                length = u32::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]) as usize;
                offset += 4;
            }
            _ => return err("indeterminate-length packets are not supported"),
        }
    }

    let end = offset + length;
    if end > data.len() {
        return err("packet body exceeds input");
    }
    Ok((tag, data[offset..end].to_vec(), end))
}

/// Iterate every `(tag, body)` packet in the de-armored data.
fn iter_packets(data: &[u8]) -> Result<Vec<(u8, Vec<u8>)>> {
    let mut packets = Vec::new();
    let mut offset = 0;
    while offset < data.len() {
        let (tag, body, next) = next_packet(data, offset)?;
        packets.push((tag, body));
        offset = next;
    }
    Ok(packets)
}

/// Parse the OID and raw key from a v4 public-key packet body; returns
/// `(raw_public_key, end_offset_of_public_material)`.
fn parse_ed25519_public_material(body: &[u8]) -> Result<([u8; 32], usize)> {
    if body.len() < 6 || body[0] != 4 {
        return err("only OpenPGP v4 public keys are supported");
    }
    if body[5] != ED25519_ALGO {
        return Err(OpenPgpError(format!(
            "unsupported public-key algorithm {}",
            body[5]
        )));
    }
    let mut offset = 6;
    if offset >= body.len() {
        return err("truncated public-key packet");
    }
    let oid_len = body[offset] as usize;
    offset += 1;
    if offset + oid_len > body.len() {
        return err("truncated OID");
    }
    let oid = &body[offset..offset + oid_len];
    offset += oid_len;
    if oid != ED25519_OID {
        return Err(OpenPgpError(format!(
            "unsupported curve OID {}",
            crate::wire::hex(oid)
        )));
    }

    let (mpi, end) = read_mpi(body, offset)?;
    // GPG encodes the Ed25519 public key as a 33-byte MPI (`0x40 || 32-byte key`);
    // a bare 32-byte MPI is also valid when the high bit is clear.
    let raw: [u8; 32] = match mpi.len() {
        33 => mpi[1..].try_into().expect("33-1 == 32"),
        32 => mpi[..].try_into().expect("len checked"),
        n => {
            return Err(OpenPgpError(format!(
                "unexpected Ed25519 public MPI length {n}"
            )))
        }
    };
    Ok((raw, end))
}

/// Compute the OpenPGP v4 fingerprint of a public-key packet body.
///
/// `SHA-1(0x99 || u16-be(len(body)) || body)`, uppercased. SHA-1 is mandated by
/// RFC 4880 for v4 fingerprints; it is not used here as a security primitive.
fn fingerprint(pub_key_body: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update([0x99]);
    hasher.update((pub_key_body.len() as u16).to_be_bytes());
    hasher.update(pub_key_body);
    let digest = hasher.finalize();
    crate::wire::hex(&digest).to_uppercase()
}

/// Parse an armored OpenPGP certificate into its raw Ed25519 key + v4 fingerprint.
///
/// Accepts either a public-key certificate (tag 6) or an unencrypted secret-key
/// block (tag 5); the fingerprint always covers only the public material.
pub fn parse_transport_key(armored: &str) -> Result<TransportKey> {
    let data = strip_armor(armored)?;
    for (tag, body) in iter_packets(&data)? {
        let (raw, pub_body): ([u8; 32], Vec<u8>) = match tag {
            6 => {
                let (raw, _) = parse_ed25519_public_material(&body)?;
                (raw, body.clone())
            }
            5 => {
                let (raw, end) = parse_ed25519_public_material(&body)?;
                (raw, body[..end].to_vec())
            }
            _ => continue,
        };
        return Ok(TransportKey {
            raw_public: raw,
            fingerprint: fingerprint(&pub_body),
        });
    }
    err("no public-key packet found")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vectors_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../vectors/openpgp")
    }

    #[test]
    fn parses_frozen_vector() {
        let raw = std::fs::read_to_string(vectors_dir().join("test-key.json")).unwrap();
        // Tiny hand-rolled extraction to avoid a serde_json dependency clash:
        // the test crate already has serde_json as a dev-dependency.
        let case: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let armored = case["armored"].as_str().unwrap();
        let key = parse_transport_key(armored).unwrap();
        assert_eq!(
            crate::wire::hex(&key.raw_public),
            case["raw_pub"].as_str().unwrap()
        );
        assert_eq!(key.fingerprint, case["fingerprint"].as_str().unwrap());
        assert_eq!(
            crate::emojihash::emojihash(&key.raw_public, 11),
            case["emojihash"].as_str().unwrap()
        );
    }

    #[test]
    fn rejects_non_pgp() {
        assert!(parse_transport_key("not a key").is_err());
    }
}

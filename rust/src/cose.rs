// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//! COSE_Sign1 (detached payload, EdDSA/Ed25519) over a frame id — GTS-SPEC §9.2.
//!
//! Byte-compatible with the Python reference: the detached payload is the frame
//! `id`, the protected header is `{1: -8}` (EdDSA), and the unprotected header
//! carries the `kid` (label 4). Ed25519 is deterministic (RFC 8032), so the same
//! key + id always yields the same signature — gated by `vectors/cose/*.json`.
//! Verification dispatches on the `alg` declared in the protected header: EdDSA
//! is checked as Ed25519, any other algorithm is left `unverified` (§9.2).

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use ciborium::value::{Integer, Value};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::model;
use crate::wire;

const ALG: i64 = 1;
const KID: i64 = 4;
const IV: i64 = 5;
const ALG_EDDSA: i64 = -8;
const ALG_A256GCM: i64 = 3;
const TAG_SIGN1: u64 = 18;
const TAG_ENCRYPT0: u64 = 16;

/// The verification outcome for a detached COSE_Sign1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigStatus {
    /// Cryptographically valid under the resolved key.
    Valid,
    /// Present but malformed or failed verification.
    Invalid,
    /// Well-formed, but not checked: no key was resolved, or the declared
    /// algorithm is not supported by this engine.
    Unverified,
}

fn protected_header() -> Vec<u8> {
    wire::encode(&Value::Map(vec![(
        Value::Integer(Integer::from(ALG)),
        Value::Integer(Integer::from(ALG_EDDSA)),
    )]))
}

/// The COSE `Sig_structure` to be signed/verified (RFC 9052 §4.4).
fn sig_structure(protected: &[u8], frame_id: &[u8]) -> Vec<u8> {
    wire::encode(&Value::Array(vec![
        Value::Text("Signature1".to_string()),
        Value::Bytes(protected.to_vec()),
        Value::Bytes(Vec::new()),
        Value::Bytes(frame_id.to_vec()),
    ]))
}

/// Produce a detached COSE_Sign1 over `frame_id` with the given Ed25519 key.
pub fn sign_id(frame_id: &[u8], signing_key: &SigningKey, kid: &str) -> Vec<u8> {
    let protected = protected_header();
    let signature: Signature = signing_key.sign(&sig_structure(&protected, frame_id));
    let cose = Value::Tag(
        TAG_SIGN1,
        Box::new(Value::Array(vec![
            Value::Bytes(protected),
            Value::Map(vec![(
                Value::Integer(Integer::from(KID)),
                Value::Bytes(kid.as_bytes().to_vec()),
            )]),
            Value::Null,
            Value::Bytes(signature.to_bytes().to_vec()),
        ])),
    );
    wire::encode(&cose)
}

/// The signing algorithm a COSE_Sign1 declares in its protected header.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Algorithm {
    /// EdDSA (COSE `-8`), verified as Ed25519.
    EdDsa,
    /// Any other declared algorithm: its integer or text identifier. This
    /// engine cannot check it, so the signature stays `unverified` (§9.2).
    Other(String),
}

/// A structurally well-formed COSE_Sign1.
struct Sign1 {
    kid: String,
    algorithm: Algorithm,
    protected: Vec<u8>,
    signature: Vec<u8>,
}

/// The deepest nesting a protected header may have; the header map is level 1.
const MAX_HEADER_DEPTH: usize = 8;

/// Read one item head at `pos`: `(major type, additional info, argument, next
/// position)`. `None` for an indefinite length, a reserved additional info, a
/// truncated head, or an argument that is not in shortest form.
fn scan_head(buf: &[u8], pos: usize) -> Option<(u8, u8, u64, usize)> {
    let initial = *buf.get(pos)?;
    let (major, info) = (initial >> 5, initial & 0x1f);
    let width = match info {
        0..=23 => 0,
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => return None,
    };
    let bytes = buf.get(pos + 1..pos + 1 + width)?;
    let arg = if width == 0 {
        u64::from(info)
    } else {
        bytes.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b))
    };
    let shortest = match width {
        1 => arg >= 24,
        2 => arg >= 1 << 8,
        4 => arg >= 1 << 16,
        8 => arg >= 1 << 32,
        _ => true,
    };
    shortest.then_some((major, info, arg, pos + 1 + width))
}

/// Check one item of a protected header against the grammar and return the
/// position after it. `depth` is the level of this item if it is a container.
fn scan_header_item(buf: &[u8], pos: usize, depth: usize, is_key: bool) -> Option<usize> {
    let (major, info, arg, next) = scan_head(buf, pos)?;
    if is_key && !matches!(major, 0 | 1 | 3) {
        return None;
    }
    match major {
        // Integers lie in -2^63 ..= 2^63-1.
        0 | 1 => i64::try_from(arg).ok().map(|_| next),
        2 | 3 => {
            let end = next.checked_add(usize::try_from(arg).ok()?)?;
            let body = buf.get(next..end)?;
            if major == 3 && std::str::from_utf8(body).is_err() {
                return None;
            }
            Some(end)
        }
        4 | 5 if depth > MAX_HEADER_DEPTH => None,
        4 => {
            let mut at = next;
            for _ in 0..arg {
                at = scan_header_item(buf, at, depth + 1, false)?;
            }
            Some(at)
        }
        5 => {
            let mut at = next;
            let mut previous: Option<&[u8]> = None;
            for _ in 0..arg {
                let key_end = scan_header_item(buf, at, depth + 1, true)?;
                let key = &buf[at..key_end];
                // Strictly ascending bytewise: sorted, and no duplicate key.
                if previous.is_some_and(|prev| prev >= key) {
                    return None;
                }
                previous = Some(key);
                at = scan_header_item(buf, key_end, depth + 1, false)?;
            }
            Some(at)
        }
        // null, true and false only.
        7 if info < 24 && (20..=22).contains(&arg) => Some(next),
        _ => None,
    }
}

/// The `alg` (label 1) of a protected header (RFC 9052 §3.1).
///
/// The protected header is a byte string whose content is deterministically
/// encoded CBOR (definite lengths, shortest-form integers and lengths, map keys
/// in bytewise order of their encodings, no duplicate keys, no trailing bytes)
/// consisting of exactly one map. Every map key, at any depth, is an integer or a
/// text string. Every value, at any depth, is an integer, a byte string, a text
/// string, an array, a map, null, true or false. Every integer, key or value,
/// lies in the range -2^63 to 2^63-1 inclusive. Text strings are valid UTF-8.
/// Nesting is at most 8 levels deep, counting the header map itself as level 1.
/// The map contains the integer key 1, whose value is an integer or a text
/// string. A header that does not meet all of this is malformed and the signature
/// is `invalid`.
///
/// Enforced by scanning the raw bytes (`scan_header_item`), so no decoder
/// behaviour (float, tag, bignum, boolean-key merging, depth limit) is involved.
fn declared_algorithm(protected: &[u8]) -> Option<Algorithm> {
    if protected.first()? >> 5 != 5 || scan_header_item(protected, 0, 1, false)? != protected.len()
    {
        return None;
    }
    // Valid by construction: find the entry whose key is the integer 1.
    let (_, _, count, mut at) = scan_head(protected, 0)?;
    for _ in 0..count {
        let key_end = scan_header_item(protected, at, 2, true)?;
        let value_end = scan_header_item(protected, key_end, 2, false)?;
        if protected[at..key_end] == [0x01] {
            let (major, _, arg, next) = scan_head(protected, key_end)?;
            return match major {
                0 => Some(Algorithm::Other(arg.to_string())),
                1 => {
                    let value = -1 - i128::from(arg);
                    Some(if value == i128::from(ALG_EDDSA) {
                        Algorithm::EdDsa
                    } else {
                        Algorithm::Other(value.to_string())
                    })
                }
                3 => Some(Algorithm::Other(
                    std::str::from_utf8(protected.get(next..value_end)?)
                        .ok()?
                        .to_string(),
                )),
                _ => None,
            };
        }
        at = value_end;
    }
    None
}

/// Parse a COSE_Sign1, or `None` if malformed: not a four-element array, a
/// protected header that is not a bstr holding a map declaring an `alg`, a
/// non-bstr signature, or no `kid` in the unprotected header.
fn parse(sig: &[u8]) -> Option<Sign1> {
    let value: Value = ciborium::de::from_reader(sig).ok()?;
    let body = match value {
        Value::Tag(_, inner) => *inner,
        other => other,
    };
    let array = body.as_array()?;
    if array.len() != 4 {
        return None;
    }
    let protected = array[0].as_bytes()?.clone();
    let algorithm = declared_algorithm(&protected)?;
    let unprotected = array[1].as_map()?;
    let signature = array[3].as_bytes()?.clone();
    let kid_target = Integer::from(KID);
    let kid = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == kid_target => {
            String::from_utf8(b.clone()).ok()
        }
        _ => None,
    })?;
    Some(Sign1 {
        kid,
        algorithm,
        protected,
        signature,
    })
}

/// The `kid` of a COSE_Sign1 (for key lookup), or `None` if malformed.
pub fn signature_kid(sig: &[u8]) -> Option<String> {
    parse(sig).map(|parts| parts.kid)
}

/// The declared algorithm identifier of a well-formed COSE_Sign1 whose
/// algorithm this engine cannot verify (anything but EdDSA), for diagnostics.
pub fn unsupported_algorithm(sig: &[u8]) -> Option<String> {
    match parse(sig)?.algorithm {
        Algorithm::EdDsa => None,
        Algorithm::Other(id) => Some(id),
    }
}

/// Verify a parsed COSE_Sign1 over `frame_id`, dispatching on its declared algorithm.
fn verify_parsed(parts: &Sign1, frame_id: &[u8], public: &VerifyingKey) -> SigStatus {
    if parts.algorithm != Algorithm::EdDsa {
        return SigStatus::Unverified;
    }
    let Ok(signature) = <[u8; 64]>::try_from(parts.signature.as_slice()) else {
        return SigStatus::Invalid;
    };
    let signature = Signature::from_bytes(&signature);
    match public.verify(&sig_structure(&parts.protected, frame_id), &signature) {
        Ok(()) => SigStatus::Valid,
        Err(_) => SigStatus::Invalid,
    }
}

/// Verify a detached COSE_Sign1 over `frame_id` against `public`.
///
/// Dispatches on the algorithm declared in the protected header (§9.2): EdDSA
/// is verified as Ed25519; any other algorithm is [`SigStatus::Unverified`].
/// A malformed COSE_Sign1 is [`SigStatus::Invalid`].
pub fn verify_sig(sig: &[u8], frame_id: &[u8], public: &VerifyingKey) -> SigStatus {
    match parse(sig) {
        Some(parts) => verify_parsed(&parts, frame_id, public),
        None => SigStatus::Invalid,
    }
}

/// Verify the COSE signatures recorded in a folded graph against keys resolved
/// by `kid`. Updates each signature's `kid` and `status` in place: `"valid"` /
/// `"invalid"` when an EdDSA signature's key resolves, `"unverified"` when no
/// key resolves or the declared algorithm is not supported, and `"invalid"`
/// for a malformed COSE_Sign1 (§9.2).
pub fn verify_signatures(
    signatures: &mut [model::Signature],
    resolve: impl Fn(&str) -> Option<VerifyingKey>,
) {
    for sig in signatures.iter_mut() {
        let Some(cose) = sig.cose.as_deref() else {
            continue;
        };
        let parsed = parse(cose);
        let status = match &parsed {
            None => SigStatus::Invalid,
            Some(parts) => match resolve(&parts.kid) {
                Some(key) => verify_parsed(parts, &sig.frame_id, &key),
                None => SigStatus::Unverified,
            },
        };
        sig.kid = parsed.map(|parts| parts.kid);
        sig.status = match status {
            SigStatus::Valid => "valid",
            SigStatus::Invalid => "invalid",
            SigStatus::Unverified => "unverified",
        }
        .to_string();
    }
}

// -- COSE_Encrypt0 (AES-256-GCM, keyed by kid) — GTS-SPEC §9.3 -----------------

/// Why a `decrypt0` could not return plaintext.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encrypt0Error {
    /// The COSE_Encrypt0 structure could not be parsed.
    Malformed,
    /// No content key was resolved for the recipient `kid`.
    MissingKey,
    /// AES-GCM authentication failed (wrong key or tampered ciphertext).
    AuthFailed,
}

impl std::fmt::Display for Encrypt0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Malformed => "malformed COSE_Encrypt0",
            Self::MissingKey => "no content key for recipient",
            Self::AuthFailed => "authentication failed (AES-GCM tag mismatch)",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for Encrypt0Error {}

/// The protected header for A256GCM: `{1: 3}` (canonical CBOR).
fn encrypt0_protected() -> Vec<u8> {
    wire::encode(&Value::Map(vec![(
        Value::Integer(Integer::from(ALG)),
        Value::Integer(Integer::from(ALG_A256GCM)),
    )]))
}

/// The COSE `Enc_structure` bound as AAD (RFC 9052 §5.3): no external AAD.
fn enc_structure(protected: &[u8]) -> Vec<u8> {
    wire::encode(&Value::Array(vec![
        Value::Text("Encrypt0".to_string()),
        Value::Bytes(protected.to_vec()),
        Value::Bytes(Vec::new()),
    ]))
}

/// Seal `plaintext` as a COSE_Encrypt0 with an explicit 12-byte `iv` (§9.3).
///
/// Splitting the IV out keeps the transform deterministic so it can be frozen
/// in `vectors/encrypt0/*.json`; [`encrypt0`] is the production entry point and
/// always draws a fresh random IV.
pub fn encrypt0_with_iv(plaintext: &[u8], kid: &str, key: &[u8; 32], iv: &[u8; 12]) -> Vec<u8> {
    let protected = encrypt0_protected();
    let aad = enc_structure(&protected);
    let cipher = Aes256Gcm::new(key.into());
    let ciphertext = cipher
        .encrypt(
            &Nonce::from(*iv),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .expect("AES-256-GCM encryption cannot fail for a valid key/nonce");
    // Unprotected header keys in canonical order: kid (4) before iv (5).
    let cose = Value::Tag(
        TAG_ENCRYPT0,
        Box::new(Value::Array(vec![
            Value::Bytes(protected),
            Value::Map(vec![
                (
                    Value::Integer(Integer::from(KID)),
                    Value::Bytes(kid.as_bytes().to_vec()),
                ),
                (Value::Integer(Integer::from(IV)), Value::Bytes(iv.to_vec())),
            ]),
            Value::Bytes(ciphertext),
        ])),
    );
    wire::canonical(&cose)
}

/// Seal `plaintext` as a COSE_Encrypt0 to the recipient `kid` (§9.3).
///
/// Draws a fresh random 12-byte IV from the OS CSPRNG; only available off wasm
/// (the deterministic [`encrypt0_with_iv`] / [`decrypt0`] paths cover wasm).
#[cfg(not(target_arch = "wasm32"))]
pub fn encrypt0(plaintext: &[u8], kid: &str, key: &[u8; 32]) -> Vec<u8> {
    let mut iv = [0u8; 12];
    getrandom::fill(&mut iv).expect("OS CSPRNG is available");
    encrypt0_with_iv(plaintext, kid, key, &iv)
}

/// The cleartext fields of a parsed COSE_Encrypt0.
struct Encrypt0Parts {
    kid: String,
    protected: Vec<u8>,
    iv: Vec<u8>,
    ciphertext: Vec<u8>,
}

/// Parse a COSE_Encrypt0 into its cleartext fields, or `None` if malformed.
fn parse_encrypt0(blob: &[u8]) -> Option<Encrypt0Parts> {
    let value: Value = ciborium::de::from_reader(blob).ok()?;
    let body = match value {
        Value::Tag(_, inner) => *inner,
        other => other,
    };
    let array = body.as_array()?;
    if array.len() != 3 {
        return None;
    }
    let protected = array[0].as_bytes()?.clone();
    let unprotected = array[1].as_map()?;
    let ciphertext = array[2].as_bytes()?.clone();
    let kid_target = Integer::from(KID);
    let iv_target = Integer::from(IV);
    let kid = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == kid_target => {
            String::from_utf8(b.clone()).ok()
        }
        _ => None,
    })?;
    let iv = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == iv_target => Some(b.clone()),
        _ => None,
    })?;
    Some(Encrypt0Parts {
        kid,
        protected,
        iv,
        ciphertext,
    })
}

/// The recipient `kid` of a COSE_Encrypt0 (for key lookup), or `None`.
pub fn recipient_kid(blob: &[u8]) -> Option<String> {
    parse_encrypt0(blob).map(|p| p.kid)
}

/// Open a COSE_Encrypt0 using a content key resolved by `kid` (§9.3).
pub fn decrypt0(
    blob: &[u8],
    resolve: impl Fn(&str) -> Option<[u8; 32]>,
) -> Result<Vec<u8>, Encrypt0Error> {
    let parts = parse_encrypt0(blob).ok_or(Encrypt0Error::Malformed)?;
    let key = resolve(&parts.kid).ok_or(Encrypt0Error::MissingKey)?;
    let iv: [u8; 12] = parts
        .iv
        .as_slice()
        .try_into()
        .map_err(|_| Encrypt0Error::Malformed)?;
    let aad = enc_structure(&parts.protected);
    let cipher = Aes256Gcm::new((&key).into());
    cipher
        .decrypt(
            &Nonce::from(iv),
            Payload {
                msg: &parts.ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| Encrypt0Error::AuthFailed)
}

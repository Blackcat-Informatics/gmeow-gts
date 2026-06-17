// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//! COSE_Sign1 (detached payload, EdDSA/Ed25519) over a frame id — GTS-SPEC §9.2.
//!
//! Byte-compatible with the Python reference: the detached payload is the frame
//! `id`, the protected header is `{1: -8}` (EdDSA), and the unprotected header
//! carries the `kid` (label 4). Ed25519 is deterministic (RFC 8032), so the same
//! key + id always yields the same signature — gated by `vectors/cose/*.json`.

use ciborium::value::{Integer, Value};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::model;
use crate::wire;

const ALG: i64 = 1;
const KID: i64 = 4;
const ALG_EDDSA: i64 = -8;
const TAG_SIGN1: u64 = 18;

/// The verification outcome for a detached COSE_Sign1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigStatus {
    /// Cryptographically valid under the resolved key.
    Valid,
    /// Present but malformed or failed verification.
    Invalid,
    /// Well-formed, but no key was resolved to check it.
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

/// Parse a COSE_Sign1 into `(kid, protected, signature)`, or `None` if malformed.
pub fn parse(sig: &[u8]) -> Option<(String, Vec<u8>, [u8; 64])> {
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
    let unprotected = array[1].as_map()?;
    let signature: [u8; 64] = array[3].as_bytes()?.as_slice().try_into().ok()?;
    let kid_target = Integer::from(KID);
    let kid = unprotected.iter().find_map(|(k, v)| match (k, v) {
        (Value::Integer(i), Value::Bytes(b)) if *i == kid_target => {
            String::from_utf8(b.clone()).ok()
        }
        _ => None,
    })?;
    Some((kid, protected, signature))
}

/// The `kid` of a COSE_Sign1 (for key lookup), or `None` if malformed.
pub fn signature_kid(sig: &[u8]) -> Option<String> {
    parse(sig).map(|(kid, _, _)| kid)
}

/// Verify a detached COSE_Sign1 over `frame_id` against `public`.
pub fn verify_sig(sig: &[u8], frame_id: &[u8], public: &VerifyingKey) -> SigStatus {
    let Some((_kid, protected, signature)) = parse(sig) else {
        return SigStatus::Invalid;
    };
    let signature = Signature::from_bytes(&signature);
    match public.verify(&sig_structure(&protected, frame_id), &signature) {
        Ok(()) => SigStatus::Valid,
        Err(_) => SigStatus::Invalid,
    }
}

/// Verify the COSE signatures recorded in a folded graph against keys resolved
/// by `kid`. Updates each signature's `kid` and `status` in place: `"valid"` /
/// `"invalid"` when a key resolves, `"unverified"` when none does (§9.2).
pub fn verify_signatures(
    signatures: &mut [model::Signature],
    resolve: impl Fn(&str) -> Option<VerifyingKey>,
) {
    for sig in signatures.iter_mut() {
        let Some(cose) = sig.cose.clone() else {
            continue;
        };
        let kid = signature_kid(&cose);
        sig.kid.clone_from(&kid);
        sig.status = match kid.as_deref().and_then(&resolve) {
            Some(key) => match verify_sig(&cose, &sig.frame_id, &key) {
                SigStatus::Valid => "valid",
                _ => "invalid",
            },
            None => "unverified",
        }
        .to_string();
    }
}

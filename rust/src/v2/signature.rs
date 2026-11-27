// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Verification of the signatures of a frame (P6 sections 2 to 4 and 6).

use ciborium::value::Value;
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey};
use ml_dsa::{EncodedVerifyingKey, MlDsa65, Signature as MlDsaSignature};
use sha2::{Digest, Sha512};

use super::frame::Signature;
use crate::wire;

const ED25519: u64 = 1;
const COMPOSITE: u64 = 2;
const ED25519_KEY_LENGTH: usize = 32;
const ED25519_VALUE_LENGTH: usize = 64;
const MLDSA_KEY_LENGTH: usize = 1952;
const MLDSA_VALUE_LENGTH: usize = 3309;
const COMPOSITE_KEY_LENGTH: usize = MLDSA_KEY_LENGTH + ED25519_KEY_LENGTH;
const COMPOSITE_VALUE_LENGTH: usize = MLDSA_VALUE_LENGTH + ED25519_VALUE_LENGTH;
const COMPOSITE_PREFIX: &[u8; 32] = b"CompositeAlgorithmSignatures2025";
const COMPOSITE_LABEL: &[u8; 30] = b"COMPSIG-MLDSA65-Ed25519-SHA512";

/// The group order `L` of Ed25519 as 32 little-endian bytes.
const GROUP_ORDER: [u8; 32] = [
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
];

/// The eight encodings of the points of small order (P6 section 4.1).
const SMALL_ORDER: [[u8; 32]; 8] = [
    [
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ],
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x7f,
    ],
    [0; 32],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0x80,
    ],
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x05,
    ],
    [
        0x26, 0xe8, 0x95, 0x8f, 0xc2, 0xb2, 0x27, 0xb0, 0x45, 0xc3, 0xf4, 0x89, 0xf2, 0xef, 0x98,
        0xf0, 0xd5, 0xdf, 0xac, 0x05, 0xd3, 0xc6, 0x33, 0x39, 0xb1, 0x38, 0x02, 0x88, 0x6d, 0x53,
        0xfc, 0x85,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0x7a,
    ],
    [
        0xc7, 0x17, 0x6a, 0x70, 0x3d, 0x4d, 0xd8, 0x4f, 0xba, 0x3c, 0x0b, 0x76, 0x0d, 0x10, 0x67,
        0x0f, 0x2a, 0x20, 0x53, 0xfa, 0x2c, 0x39, 0xcc, 0xc6, 0x4e, 0xc7, 0xfd, 0x77, 0x92, 0xac,
        0x03, 0xfa,
    ],
];

/// A public key the verifier can check a signature with (P6 section 3.3).
///
/// `key_id` is the identifier under which the caller files the key. A held key
/// verifies a signature only when it is consistent with it: its `algorithm`
/// equals the signature's and the hash of `[algorithm, public_key]` equals
/// `key_id`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldKey {
    /// The identifier the caller files the key under.
    pub key_id: [u8; 32],
    /// The registry code of the signature algorithm of the key.
    pub algorithm: u64,
    /// The public key bytes in the form of the algorithm.
    pub public_key: Vec<u8>,
}

impl HeldKey {
    /// The key identifier of `public_key` under `algorithm`:
    /// `H_ctx(key_id, E([algorithm, public_key]))` (P6 section 3.2).
    #[must_use]
    pub fn identifier(algorithm: u64, public_key: &[u8]) -> [u8; 32] {
        let preimage = Value::Array(vec![
            Value::from(algorithm),
            Value::Bytes(public_key.to_vec()),
        ]);
        blake3::derive_key("gts2 key id", &wire::encode(&preimage))
    }

    /// `E(held_key)`, the encoding that orders the set of held keys.
    fn encoding(&self) -> Vec<u8> {
        wire::encode(&Value::Array(vec![
            Value::Bytes(self.key_id.to_vec()),
            Value::from(self.algorithm),
            Value::Bytes(self.public_key.clone()),
        ]))
    }

    fn is_consistent_with(&self, signature: &Signature<'_>) -> bool {
        self.algorithm == signature.algorithm
            && Self::identifier(self.algorithm, &self.public_key) == *signature.key_id
    }
}

/// The held keys of a verifier, as the set of P6 section 3.3.
///
/// The set holds each key once, in ascending bytewise order of `E(held_key)`,
/// whatever order and repetition the caller gives (P6-13). Two keys with one
/// `key_id` and different bytes are two keys.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HeldKeys {
    keys: Vec<HeldKey>,
}

impl HeldKeys {
    /// Form the set of `keys`: sorted by `E(held_key)` and without repeats.
    #[must_use]
    pub fn new(keys: impl IntoIterator<Item = HeldKey>) -> Self {
        let mut keyed: Vec<(Vec<u8>, HeldKey)> =
            keys.into_iter().map(|key| (key.encoding(), key)).collect();
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        keyed.dedup_by(|left, right| left.0 == right.0);
        Self {
            keys: keyed.into_iter().map(|(_, key)| key).collect(),
        }
    }

    /// The keys in ascending bytewise order of `E(held_key)`.
    #[must_use]
    pub fn as_slice(&self) -> &[HeldKey] {
        &self.keys
    }
}

/// The status of one signature (P6 section 6.1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureStatus {
    /// The value verifies under a held key.
    Valid,
    /// The value does not verify, or the key is inconsistent with the record.
    Invalid,
    /// The verifier lacks what it needs to check the signature.
    Unverified,
}

/// The reason of a signature that is not valid (P2, "The `reason` enumeration").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureReason {
    /// The algorithm is not a code of the reader's registry version.
    UnknownSignatureAlgorithm,
    /// No held key has the `key_id` of the signature.
    KeyNotHeld,
    /// Held keys have the `key_id`, and none is consistent with the signature.
    KeyMismatch,
    /// The public key or the value is not the length of its algorithm.
    ValueLength,
    /// The Ed25519 scalar `S` is not below the group order.
    NonCanonicalScalar,
    /// The Ed25519 public key is one of the eight encodings of small order.
    SmallOrderKey,
    /// The Ed25519 `R` is one of the eight encodings of small order.
    SmallOrderNonce,
    /// The Ed25519 equation does not hold, or the key does not decode.
    EquationFailed,
    /// The Ed25519 half of a composite failed.
    CompositeClassicalFailed,
    /// The ML-DSA half of a composite failed.
    CompositePostQuantumFailed,
}

/// The outcome of one signature of one frame (P6 section 6.1).
///
/// Coverage and authentication (P6 section 7) are not part of this record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureResult {
    /// The status of the signature.
    pub status: SignatureStatus,
    /// The reason; absent exactly when the status is [`SignatureStatus::Valid`].
    pub reason: Option<SignatureReason>,
    /// The algorithm code of the record as read.
    pub algorithm: u64,
    /// The `key_id` of the record as read.
    pub key_id: [u8; 32],
}

impl SignatureResult {
    fn of(signature: &Signature<'_>, reason: Option<(SignatureStatus, SignatureReason)>) -> Self {
        let (status, reason) = match reason {
            Some((status, reason)) => (status, Some(reason)),
            None => (SignatureStatus::Valid, None),
        };
        Self {
            status,
            reason,
            algorithm: signature.algorithm,
            key_id: *signature.key_id,
        }
    }
}

/// Run `Verify(signature, frame)` of P6 section 6.2 for one signature.
///
/// `h0` is the identifier of the opener of the segment that holds the frame
/// and `frame_identifier` the effective identifier of the frame. The outcome
/// is that of the first step that gives one.
pub(super) fn verify_signature(
    signature: &Signature<'_>,
    h0: &[u8; 32],
    frame_identifier: &[u8; 32],
    keys: &HeldKeys,
) -> SignatureResult {
    use SignatureReason::{KeyMismatch, KeyNotHeld, UnknownSignatureAlgorithm, ValueLength};
    use SignatureStatus::{Invalid, Unverified};
    let failure = |status, reason| SignatureResult::of(signature, Some((status, reason)));
    let (key_length, value_length) = match signature.algorithm {
        ED25519 => (ED25519_KEY_LENGTH, ED25519_VALUE_LENGTH),
        COMPOSITE => (COMPOSITE_KEY_LENGTH, COMPOSITE_VALUE_LENGTH),
        _ => return failure(Unverified, UnknownSignatureAlgorithm),
    };
    let mut filed = keys
        .as_slice()
        .iter()
        .filter(|key| key.key_id == *signature.key_id)
        .peekable();
    if filed.peek().is_none() {
        return failure(Unverified, KeyNotHeld);
    }
    let Some(key) = filed.find(|key| key.is_consistent_with(signature)) else {
        return failure(Invalid, KeyMismatch);
    };
    if key.public_key.len() != key_length || signature.value.len() != value_length {
        return failure(Invalid, ValueLength);
    }
    let message = subject_message(signature, h0, frame_identifier);
    let reason = if signature.algorithm == ED25519 {
        match (key.public_key.first_chunk(), signature.value.first_chunk()) {
            (Some(key), Some(value)) => ed25519_strict(key, &message, value),
            _ => Some(ValueLength),
        }
    } else {
        composite_strict(&key.public_key, &message, signature.value)
    };
    SignatureResult::of(signature, reason.map(|reason| (Invalid, reason)))
}

/// `H_ctx(signature_subject, E(subject))` of P6 section 2.
fn subject_message(
    signature: &Signature<'_>,
    h0: &[u8; 32],
    frame_identifier: &[u8; 32],
) -> [u8; 32] {
    let subject = Value::Array(vec![
        Value::from(signature.algorithm),
        Value::Bytes(signature.key_id.to_vec()),
        Value::Bytes(h0.to_vec()),
        Value::Bytes(frame_identifier.to_vec()),
    ]);
    blake3::derive_key("gts2 signature subject", &wire::encode(&subject))
}

/// Whether the 32-byte little-endian integer `scalar` is below the group order.
fn below_group_order(scalar: &[u8]) -> bool {
    for (byte, order) in scalar.iter().zip(&GROUP_ORDER).rev() {
        if byte != order {
            return byte < order;
        }
    }
    false
}

/// `Ed25519-Strict` of P6 section 4.1: the first failing check of the policy.
///
/// The caller has checked the lengths. The first three checks are byte
/// comparisons. The equation is the cofactorless check of the library's
/// strict verification, entered only for a key that decodes by the
/// canonical decoding of the policy.
fn ed25519_strict(
    key: &[u8; ED25519_KEY_LENGTH],
    message: &[u8],
    value: &[u8; ED25519_VALUE_LENGTH],
) -> Option<SignatureReason> {
    let (nonce, scalar) = value.split_at(32);
    if !below_group_order(scalar) {
        return Some(SignatureReason::NonCanonicalScalar);
    }
    if SMALL_ORDER.iter().any(|point| point == key) {
        return Some(SignatureReason::SmallOrderKey);
    }
    if SMALL_ORDER.iter().any(|point| point == nonce) {
        return Some(SignatureReason::SmallOrderNonce);
    }
    // A key decodes only when its bytes are the encoding of its point: this
    // refuses y at least p and a set sign bit on x = 0, which the library
    // would accept.
    let decoded = VerifyingKey::from_bytes(key)
        .ok()
        .filter(|decoded| decoded.to_edwards().compress().to_bytes() == *key);
    let holds = decoded.is_some_and(|decoded| {
        decoded
            .verify_strict(message, &Ed25519Signature::from_bytes(value))
            .is_ok()
    });
    (!holds).then_some(SignatureReason::EquationFailed)
}

/// `Composite-Strict` of P6 section 4.2: the Ed25519 half over `M'`, then the
/// ML-DSA-65 half over `M'` with `Label` as its context.
///
/// The caller has checked the lengths.
fn composite_strict(
    public_key: &[u8],
    message: &[u8; 32],
    value: &[u8],
) -> Option<SignatureReason> {
    let (Some((mldsa_key, ed25519_key)), Some((mldsa_value, ed25519_value))) = (
        public_key.split_last_chunk::<ED25519_KEY_LENGTH>(),
        value.split_last_chunk::<ED25519_VALUE_LENGTH>(),
    ) else {
        return Some(SignatureReason::CompositeClassicalFailed);
    };
    let mut representative = Vec::with_capacity(127);
    representative.extend_from_slice(COMPOSITE_PREFIX);
    representative.extend_from_slice(COMPOSITE_LABEL);
    representative.push(0);
    representative.extend_from_slice(&Sha512::digest(message));
    if ed25519_strict(ed25519_key, &representative, ed25519_value).is_some() {
        return Some(SignatureReason::CompositeClassicalFailed);
    }
    let holds = || -> Option<bool> {
        let key = EncodedVerifyingKey::<MlDsa65>::try_from(mldsa_key).ok()?;
        let signature = MlDsaSignature::<MlDsa65>::try_from(mldsa_value).ok()?;
        Some(
            ml_dsa::VerifyingKey::<MlDsa65>::decode(&key).verify_with_context(
                &representative,
                COMPOSITE_LABEL,
                &signature,
            ),
        )
    };
    (!holds().unwrap_or(false)).then_some(SignatureReason::CompositePostQuantumFailed)
}

#[cfg(test)]
mod composite_halves {
    use ed25519_dalek::{Signer, SigningKey as Ed25519SigningKey};
    use ml_dsa::{ExpandedSigningKey, MlDsa65, Seed};

    use super::{
        subject_message, verify_signature, HeldKey, HeldKeys, Signature, SignatureReason,
        SignatureStatus, COMPOSITE, COMPOSITE_LABEL, COMPOSITE_PREFIX,
    };
    use sha2::{Digest, Sha512};

    /// A composite signer of the tests: its ML-DSA-65 and Ed25519 keys and the composite public key.
    struct Signer2 {
        ml: ExpandedSigningKey<MlDsa65>,
        ed: Ed25519SigningKey,
        public_key: Vec<u8>,
    }

    fn signer(seed: u8) -> Signer2 {
        let ml = ExpandedSigningKey::<MlDsa65>::from_seed(&Seed::from([seed; 32]));
        let ed = Ed25519SigningKey::from_bytes(&[seed.wrapping_add(0x20); 32]);
        let mut public_key = ml.verifying_key().encode().as_slice().to_vec();
        public_key.extend_from_slice(&ed.verifying_key().to_bytes());
        Signer2 { ml, ed, public_key }
    }

    /// The ML-DSA and Ed25519 halves of a composite of the subject (P6 4.2).
    fn halves(signer: &Signer2, h0: &[u8; 32], frame: &[u8; 32]) -> (Vec<u8>, Vec<u8>) {
        let key_id = HeldKey::identifier(COMPOSITE, &signer.public_key);
        let subject = Signature {
            algorithm: COMPOSITE,
            key_id: &key_id,
            value: &[],
        };
        let mut representative = Vec::with_capacity(127);
        representative.extend_from_slice(COMPOSITE_PREFIX);
        representative.extend_from_slice(COMPOSITE_LABEL);
        representative.push(0);
        representative.extend_from_slice(&Sha512::digest(subject_message(&subject, h0, frame)));
        let ml = signer
            .ml
            .sign_deterministic(&representative, COMPOSITE_LABEL)
            .expect("ML-DSA signs the representative")
            .encode();
        let ed = signer.ed.sign(&representative).to_bytes().to_vec();
        (ml.as_slice().to_vec(), ed)
    }

    fn held(signer: &Signer2) -> HeldKey {
        HeldKey {
            key_id: HeldKey::identifier(COMPOSITE, &signer.public_key),
            algorithm: COMPOSITE,
            public_key: signer.public_key.clone(),
        }
    }

    /// P6-26: when the halves of two valid composite signatures over one subject are swapped,
    /// the classical half refuses the composite first, and the composite is never valid.
    #[test]
    fn swapped_halves_of_two_valid_composites_are_refused_by_the_classical_half() {
        let (h0, frame) = ([1_u8; 32], [2_u8; 32]);
        let first = signer(1);
        let second = signer(2);
        let keys = HeldKeys::new([held(&first), held(&second)]);
        let first_id = HeldKey::identifier(COMPOSITE, &first.public_key);
        let (first_ml, first_ed) = halves(&first, &h0, &frame);
        let (_, second_ed) = halves(&second, &h0, &frame);

        let valid = [first_ml.clone(), first_ed].concat();
        let result = verify_signature(
            &Signature {
                algorithm: COMPOSITE,
                key_id: &first_id,
                value: &valid,
            },
            &h0,
            &frame,
            &keys,
        );
        assert_eq!(result.status, SignatureStatus::Valid);

        let swapped = [first_ml, second_ed].concat();
        let result = verify_signature(
            &Signature {
                algorithm: COMPOSITE,
                key_id: &first_id,
                value: &swapped,
            },
            &h0,
            &frame,
            &keys,
        );
        assert_eq!(result.status, SignatureStatus::Invalid);
        assert_eq!(
            result.reason,
            Some(SignatureReason::CompositeClassicalFailed)
        );
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Verification of v2 frame signatures (P6 sections 2 to 4 and 6).

import * as ed from "@noble/ed25519";
import { ml_dsa65 } from "@noble/post-quantum/ml-dsa.js";
import { sha512 } from "@noble/hashes/sha2.js";
import { contextHash } from "./v2_integrity.js";

export const ED25519_ALGORITHM = 1n;
export const COMPOSITE_ALGORITHM = 2n;

const ED25519_KEY_LENGTH = 32;
const ED25519_VALUE_LENGTH = 64;
const MLDSA_KEY_LENGTH = 1952;
const MLDSA_VALUE_LENGTH = 3309;
const COMPOSITE_KEY_LENGTH = MLDSA_KEY_LENGTH + ED25519_KEY_LENGTH;
const COMPOSITE_VALUE_LENGTH = MLDSA_VALUE_LENGTH + ED25519_VALUE_LENGTH;

// Synchronous Ed25519 verification needs a synchronous SHA-512 (as in cose.ts).
ed.hashes.sha512 = sha512;

const encoder = new TextEncoder();
const COMPOSITE_PREFIX = encoder.encode("CompositeAlgorithmSignatures2025");
const COMPOSITE_LABEL = encoder.encode("COMPSIG-MLDSA65-Ed25519-SHA512");

/** The group order `L` of Ed25519 as 32 little-endian bytes. */
const GROUP_ORDER = Uint8Array.from([
    0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2,
    0xde, 0xf9, 0xde, 0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
]);

/** The eight encodings of the points of small order (P6 section 4.1). */
const SMALL_ORDER_HEX = [
    "0100000000000000000000000000000000000000000000000000000000000000",
    "ecffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff7f",
    "0000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000080",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
    "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc85",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
    "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac03fa",
];

/** A reason a signature is not valid (P2, "The `reason` enumeration"). */
export type SignatureReason =
    | "unknown_signature_algorithm"
    | "key_not_held"
    | "key_mismatch"
    | "value_length"
    | "non_canonical_scalar"
    | "small_order_key"
    | "small_order_nonce"
    | "equation_failed"
    | "composite_classical_failed"
    | "composite_post_quantum_failed";

/** The status of one signature (P6 section 6.1). */
export type SignatureStatus = "valid" | "invalid" | "unverified";

/** The outcome of one signature of one frame (P6 section 6.1). */
export interface SignatureResult {
    status: SignatureStatus;
    /** Absent exactly when the status is `valid`. */
    reason?: SignatureReason;
    algorithm: bigint;
    keyId: Uint8Array;
}

/** A public key a verifier can check a signature with (P6 section 3.3). */
export interface HeldKey {
    keyId: Uint8Array;
    algorithm: bigint;
    publicKey: Uint8Array;
}

/** One signature record of a frame as read: algorithm, key identifier and value. */
export interface SignatureRecord {
    algorithm: bigint;
    keyId: Uint8Array;
    value: Uint8Array;
}

/** `H_ctx(key_id, E([algorithm, public_key]))` (P6 section 3.2). */
export function heldKeyIdentifier(
    algorithm: bigint,
    publicKey: Uint8Array,
): Uint8Array {
    return contextHash("gts2 key id", cborArray([algorithm, publicKey]));
}

/** `H_ctx(signature_subject, E(subject))` of P6 section 2. */
export function signatureSubject(
    record: SignatureRecord,
    h0: Uint8Array,
    frameIdentifier: Uint8Array,
): Uint8Array {
    return contextHash(
        "gts2 signature subject",
        cborArray([record.algorithm, record.keyId, h0, frameIdentifier]),
    );
}

/** The head of a CBOR item of `major` type with the argument `value` (RFC 8949 section 3). */
function cborHead(major: number, value: number | bigint): Uint8Array {
    const argument = BigInt(value);
    if (argument < 24n) return Uint8Array.of((major << 5) | Number(argument));
    if (argument < 0x100n)
        return Uint8Array.of((major << 5) | 24, Number(argument));
    if (argument < 0x10000n) {
        return Uint8Array.of(
            (major << 5) | 25,
            Number(argument >> 8n),
            Number(argument & 0xffn),
        );
    }
    throw new RangeError(
        "CBOR argument outside the range this encoder supports",
    );
}

/**
 * The deterministic CBOR encoding of an array of unsigned integers and byte
 * strings, the only shapes the signature preimages use (P6 section 2 and 3.2).
 */
function cborArray(items: readonly (bigint | Uint8Array)[]): Uint8Array {
    const parts: Uint8Array[] = [cborHead(4, items.length)];
    for (const item of items) {
        if (typeof item === "bigint") {
            parts.push(cborHead(0, item));
        } else {
            parts.push(cborHead(2, item.length), item);
        }
    }
    const length = parts.reduce((sum, part) => sum + part.length, 0);
    const out = new Uint8Array(length);
    let offset = 0;
    for (const part of parts) {
        out.set(part, offset);
        offset += part.length;
    }
    return out;
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
    if (left.length !== right.length) return false;
    for (let index = 0; index < left.length; index++) {
        if (left[index] !== right[index]) return false;
    }
    return true;
}

/** Whether the 32-byte little-endian integer `scalar` is below the group order. */
function belowGroupOrder(scalar: Uint8Array): boolean {
    for (let index = scalar.length - 1; index >= 0; index--) {
        if (scalar[index] !== GROUP_ORDER[index]) {
            return scalar[index] < GROUP_ORDER[index];
        }
    }
    return false;
}

function isSmallOrder(point: Uint8Array): boolean {
    return SMALL_ORDER_HEX.some((hex) => equalBytes(point, hexBytes(hex)));
}

function hexBytes(hex: string): Uint8Array {
    const out = new Uint8Array(hex.length / 2);
    for (let index = 0; index < out.length; index++) {
        out[index] = Number.parseInt(hex.slice(2 * index, 2 * index + 2), 16);
    }
    return out;
}

/**
 * `Ed25519-Strict` of P6 section 4.1: the first failing check of the policy.
 *
 * Lengths are checked by the caller. Returns `undefined` when the signature holds.
 */
export function ed25519Strict(
    publicKey: Uint8Array,
    message: Uint8Array,
    value: Uint8Array,
): SignatureReason | undefined {
    const nonce = value.subarray(0, 32);
    const scalar = value.subarray(32, 64);
    if (!belowGroupOrder(scalar)) return "non_canonical_scalar";
    if (isSmallOrder(publicKey)) return "small_order_key";
    if (isSmallOrder(nonce)) return "small_order_nonce";
    let holds = false;
    try {
        holds = ed.verify(value, message, publicKey, { zip215: false });
    } catch {
        holds = false;
    }
    return holds ? undefined : "equation_failed";
}

/**
 * `Composite-Strict` of P6 section 4.2: the Ed25519 half over `M'`, then the
 * ML-DSA-65 half over `M'` with `Label` as its context.
 *
 * Lengths are checked by the caller. Returns `undefined` when the signature holds.
 */
export function compositeStrict(
    publicKey: Uint8Array,
    message: Uint8Array,
    value: Uint8Array,
): SignatureReason | undefined {
    const mldsaKey = publicKey.subarray(0, MLDSA_KEY_LENGTH);
    const ed25519Key = publicKey.subarray(MLDSA_KEY_LENGTH);
    const mldsaValue = value.subarray(0, MLDSA_VALUE_LENGTH);
    const ed25519Value = value.subarray(MLDSA_VALUE_LENGTH);
    const representative = new Uint8Array([
        ...COMPOSITE_PREFIX,
        ...COMPOSITE_LABEL,
        0,
        ...sha512(message),
    ]);
    if (ed25519Strict(ed25519Key, representative, ed25519Value) !== undefined) {
        return "composite_classical_failed";
    }
    let holds = false;
    try {
        holds = ml_dsa65.verify(mldsaValue, representative, mldsaKey, {
            context: COMPOSITE_LABEL,
        });
    } catch {
        holds = false;
    }
    return holds ? undefined : "composite_post_quantum_failed";
}

/**
 * Run `Verify(signature, frame)` of P6 section 6.2 for one signature.
 *
 * `keys` are the held keys of the verifier. The outcome is that of the first
 * step that gives one.
 */
export function verifySignature(
    record: SignatureRecord,
    h0: Uint8Array,
    frameIdentifier: Uint8Array,
    keys: readonly HeldKey[],
): SignatureResult {
    const result = (
        status: SignatureStatus,
        reason?: SignatureReason,
    ): SignatureResult => ({
        status,
        ...(reason === undefined ? {} : { reason }),
        algorithm: record.algorithm,
        keyId: record.keyId,
    });
    let keyLength: number;
    let valueLength: number;
    if (record.algorithm === ED25519_ALGORITHM) {
        keyLength = ED25519_KEY_LENGTH;
        valueLength = ED25519_VALUE_LENGTH;
    } else if (record.algorithm === COMPOSITE_ALGORITHM) {
        keyLength = COMPOSITE_KEY_LENGTH;
        valueLength = COMPOSITE_VALUE_LENGTH;
    } else {
        return result("unverified", "unknown_signature_algorithm");
    }
    const filed = keys.filter((key) => equalBytes(key.keyId, record.keyId));
    if (filed.length === 0) return result("unverified", "key_not_held");
    const key = filed.find(
        (candidate) =>
            candidate.algorithm === record.algorithm &&
            equalBytes(
                heldKeyIdentifier(candidate.algorithm, candidate.publicKey),
                record.keyId,
            ),
    );
    if (key === undefined) return result("invalid", "key_mismatch");
    if (
        key.publicKey.length !== keyLength ||
        record.value.length !== valueLength
    ) {
        return result("invalid", "value_length");
    }
    const message = signatureSubject(record, h0, frameIdentifier);
    const reason =
        record.algorithm === ED25519_ALGORITHM
            ? ed25519Strict(key.publicKey, message, record.value)
            : compositeStrict(key.publicKey, message, record.value);
    return reason === undefined ? result("valid") : result("invalid", reason);
}

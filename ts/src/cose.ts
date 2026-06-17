// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// COSE_Sign1 (detached payload, EdDSA/Ed25519) over a frame id — GTS-SPEC §9.2.
// Byte-compatible with the Python reference; gated by vectors/cose/*.json.
// Ed25519 is deterministic (RFC 8032), so the same key + id yields the same
// signature.

import { Tagged } from "cbor";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512.js";
import { gcm } from "@noble/ciphers/aes.js";

import { encode, decodeFirst } from "./wire.js";
import type { Signature } from "./model.js";

// Enable @noble's synchronous sign/verify (it needs a sync SHA-512).
ed.etc.sha512Sync = (...m: Uint8Array[]) => sha512(ed.etc.concatBytes(...m));

const ALG = 1;
const KID = 4;
const IV = 5;
const ALG_EDDSA = -8;
const ALG_A256GCM = 3;
const TAG_SIGN1 = 18;
const TAG_ENCRYPT0 = 16;

export type SigStatus = "valid" | "invalid" | "unverified";

function protectedHeader(): Uint8Array {
    return encode(new Map<number, number>([[ALG, ALG_EDDSA]]));
}

function sigStructure(prot: Uint8Array, frameId: Uint8Array): Uint8Array {
    return encode(["Signature1", prot, new Uint8Array(0), frameId]);
}

/** Produce a detached COSE_Sign1 over `frameId` with the given Ed25519 key. */
export function signId(
    frameId: Uint8Array,
    privateKey: Uint8Array,
    kid: string,
): Uint8Array {
    const prot = protectedHeader();
    const signature = ed.sign(sigStructure(prot, frameId), privateKey);
    const cose = new Tagged(TAG_SIGN1, [
        prot,
        new Map<number, Uint8Array>([[KID, new TextEncoder().encode(kid)]]),
        null,
        signature,
    ]);
    return encode(cose);
}

function asBytes(v: unknown): Uint8Array | null {
    if (v instanceof Uint8Array) return v;
    if (Buffer.isBuffer(v)) return new Uint8Array(v);
    return null;
}

/** Parse a COSE_Sign1 into `{ kid, protected, signature }`, or null if malformed. */
export function parse(
    sig: Uint8Array,
): { kid: string; protected: Uint8Array; signature: Uint8Array } | null {
    try {
        let body = decodeFirst(sig);
        if (body instanceof Tagged) body = body.value;
        if (!Array.isArray(body) || body.length !== 4) return null;
        const prot = asBytes(body[0]);
        const signature = asBytes(body[3]);
        const unprotected = body[1];
        if (!prot || !signature || !(unprotected instanceof Map)) return null;
        const kidVal = asBytes(unprotected.get(KID));
        if (!kidVal) return null;
        return { kid: new TextDecoder().decode(kidVal), protected: prot, signature };
    } catch {
        return null;
    }
}

/** The kid of a COSE_Sign1 (for key lookup), or null if malformed. */
export function signatureKid(sig: Uint8Array): string | null {
    return parse(sig)?.kid ?? null;
}

/** Verify a detached COSE_Sign1 over `frameId` against `publicKey`. */
export function verifySig(
    sig: Uint8Array,
    frameId: Uint8Array,
    publicKey: Uint8Array,
): SigStatus {
    const parsed = parse(sig);
    if (!parsed || parsed.signature.length !== 64) return "invalid";
    try {
        return ed.verify(
            parsed.signature,
            sigStructure(parsed.protected, frameId),
            publicKey,
        )
            ? "valid"
            : "invalid";
    } catch {
        return "invalid";
    }
}

/**
 * Verify the COSE signatures recorded in a folded graph against keys resolved
 * by kid. Mutates each signature's `kid` and `status`: "valid"/"invalid" when a
 * key resolves, "unverified" when none does (§9.2).
 */
export function verifySignatures(
    signatures: Signature[],
    resolve: (kid: string) => Uint8Array | null,
): void {
    for (const sig of signatures) {
        if (!sig.cose) continue;
        const kid = signatureKid(sig.cose);
        if (kid === null) {
            sig.status = "invalid";
            continue;
        }
        sig.kid = kid;
        const pub = resolve(kid);
        if (!pub) {
            sig.status = "unverified";
            continue;
        }
        sig.status =
            verifySig(sig.cose, sig.frameId, pub) === "valid" ? "valid" : "invalid";
    }
}

// -- COSE_Encrypt0 (AES-256-GCM, keyed by kid) — GTS-SPEC §9.3 ----------------

/** Why a {@link decrypt0} could not return plaintext. */
export type Decrypt0Reason = "malformed" | "missing-key" | "auth-failed";

/** Thrown by {@link decrypt0} when a COSE_Encrypt0 cannot be opened. */
export class Encrypt0Error extends Error {
    constructor(
        readonly reason: Decrypt0Reason,
        message: string,
    ) {
        super(message);
        this.name = "Encrypt0Error";
    }
}

function encrypt0Protected(): Uint8Array {
    return encode(new Map<number, number>([[ALG, ALG_A256GCM]]));
}

/** The COSE `Enc_structure` bound as AAD (RFC 9052 §5.3): no external AAD. */
function encStructure(prot: Uint8Array): Uint8Array {
    return encode(["Encrypt0", prot, new Uint8Array(0)]);
}

/**
 * Seal `plaintext` as a COSE_Encrypt0 with an explicit 12-byte `iv` (§9.3).
 *
 * The split-out IV keeps the transform deterministic so it can be frozen in
 * `vectors/encrypt0`; {@link encrypt0} is the production path with a random IV.
 */
export function encrypt0WithIv(
    plaintext: Uint8Array,
    kid: string,
    key: Uint8Array,
    iv: Uint8Array,
): Uint8Array {
    const prot = encrypt0Protected();
    const ciphertext = gcm(key, iv, encStructure(prot)).encrypt(plaintext);
    // Unprotected header keys in canonical order: kid (4) before iv (5).
    const cose = new Tagged(TAG_ENCRYPT0, [
        prot,
        new Map<number, Uint8Array>([
            [KID, new TextEncoder().encode(kid)],
            [IV, iv],
        ]),
        ciphertext,
    ]);
    return encode(cose);
}

/**
 * Seal `plaintext` as a COSE_Encrypt0 to the recipient `kid` (§9.3), drawing a
 * fresh random 12-byte IV from the platform CSPRNG.
 */
export function encrypt0(
    plaintext: Uint8Array,
    kid: string,
    key: Uint8Array,
): Uint8Array {
    const iv = new Uint8Array(12);
    globalThis.crypto.getRandomValues(iv);
    return encrypt0WithIv(plaintext, kid, key, iv);
}

function parseEncrypt0(
    blob: Uint8Array,
): { kid: string; protected: Uint8Array; iv: Uint8Array; ciphertext: Uint8Array } | null {
    try {
        let body = decodeFirst(blob);
        if (body instanceof Tagged) body = body.value;
        if (!Array.isArray(body) || body.length !== 3) return null;
        const prot = asBytes(body[0]);
        const ciphertext = asBytes(body[2]);
        const unprotected = body[1];
        if (!prot || !ciphertext || !(unprotected instanceof Map)) return null;
        const kidVal = asBytes(unprotected.get(KID));
        const iv = asBytes(unprotected.get(IV));
        if (!kidVal || !iv) return null;
        return {
            kid: new TextDecoder().decode(kidVal),
            protected: prot,
            iv,
            ciphertext,
        };
    } catch {
        return null;
    }
}

/** The recipient `kid` of a COSE_Encrypt0 (for key lookup), or null. */
export function recipientKid(blob: Uint8Array): string | null {
    return parseEncrypt0(blob)?.kid ?? null;
}

/** Open a COSE_Encrypt0 using a content key resolved by `kid` (§9.3). */
export function decrypt0(
    blob: Uint8Array,
    resolve: (kid: string) => Uint8Array | null,
): Uint8Array {
    const parsed = parseEncrypt0(blob);
    if (!parsed) throw new Encrypt0Error("malformed", "malformed COSE_Encrypt0");
    const key = resolve(parsed.kid);
    if (!key || key.length !== 32) {
        throw new Encrypt0Error("missing-key", `no content key for ${parsed.kid}`);
    }
    if (parsed.iv.length !== 12) {
        throw new Encrypt0Error("malformed", "bad COSE_Encrypt0 IV length");
    }
    try {
        return gcm(key, parsed.iv, encStructure(parsed.protected)).decrypt(
            parsed.ciphertext,
        );
    } catch {
        throw new Encrypt0Error(
            "auth-failed",
            "authentication failed (AES-GCM tag mismatch)",
        );
    }
}

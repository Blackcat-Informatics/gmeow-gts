// SPDX-FileCopyrightText: 2026 Blackcat Informatics(R) Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { Tagged } from "cbor";

import {
    asBytes,
    copyBytes,
    decodeFirst,
    encodeCanonical,
    toBufferSource,
} from "./browser_wire.js";

const KID = 4;
const IV = 5;
const TAG_SIGN1 = 18;
const TAG_ENCRYPT0 = 16;

type MaybePromise<T> = T | Promise<T>;
type KeyLike = CryptoKey | Uint8Array | ArrayBuffer;

/** WebCrypto-backed key lookup for browser-side signature and envelope handling. */
export interface BrowserKeyProvider {
    /** Return an Ed25519 verification key for a signer kid, or null/undefined if unknown. */
    verificationKey?: (kid: string) => MaybePromise<KeyLike | null | undefined>;
    /** Return a 32-byte AES-GCM content key for a recipient kid, or null/undefined if absent. */
    contentKey?: (kid: string) => MaybePromise<KeyLike | null | undefined>;
}

/** Signature status reported by browser WebCrypto verification. */
export type BrowserSigStatus = "valid" | "invalid" | "unverified";
/** Failure class for browser COSE_Encrypt0 handling. */
export type BrowserDecrypt0Reason =
    | "malformed"
    | "missing-key"
    | "auth-failed"
    | "unsupported";

/** Error raised when browser COSE parsing, key lookup, or decryption fails. */
export class BrowserCoseError extends Error {
    constructor(
        readonly reason: BrowserDecrypt0Reason,
        message: string,
    ) {
        super(message);
        this.name = "BrowserCoseError";
    }
}

/** Return the kid from a COSE_Sign1, or null if malformed. */
export function signatureKid(cose: Uint8Array): string | null {
    return parseSign1(cose)?.kid ?? null;
}

/** Verify a detached COSE_Sign1 over a frame id using WebCrypto. */
export async function verifySign1WithWebCrypto(
    cose: Uint8Array,
    frameId: Uint8Array,
    keys: BrowserKeyProvider,
): Promise<{ kid: string; status: BrowserSigStatus }> {
    const parsed = parseSign1(cose);
    if (!parsed || parsed.signature.length !== 64) {
        return { kid: "", status: "invalid" };
    }
    const key = await keys.verificationKey?.(parsed.kid);
    if (!key) return { kid: parsed.kid, status: "unverified" };
    try {
        const cryptoKey = await ed25519VerificationKey(key);
        const ok = await subtleCrypto().verify(
            { name: "Ed25519" },
            cryptoKey,
            toBufferSource(parsed.signature),
            toBufferSource(sigStructure(parsed.protected, frameId)),
        );
        return { kid: parsed.kid, status: ok ? "valid" : "invalid" };
    } catch {
        return { kid: parsed.kid, status: "invalid" };
    }
}

/** Return the kid from a COSE_Encrypt0, or null if malformed. */
export function recipientKid(cose: Uint8Array): string | null {
    return parseEncrypt0(cose)?.kid ?? null;
}

/** Decrypt a COSE_Encrypt0 envelope using a WebCrypto AES-GCM content key. */
export async function decrypt0WithWebCrypto(
    cose: Uint8Array,
    keys: BrowserKeyProvider,
): Promise<Uint8Array> {
    const parsed = parseEncrypt0(cose);
    if (!parsed) {
        throw new BrowserCoseError("malformed", "malformed COSE_Encrypt0");
    }
    if (parsed.iv.length !== 12) {
        throw new BrowserCoseError("malformed", "bad COSE_Encrypt0 IV length");
    }
    const key = await keys.contentKey?.(parsed.kid);
    if (!key) {
        throw new BrowserCoseError(
            "missing-key",
            `no content key for ${parsed.kid}`,
        );
    }
    try {
        const cryptoKey = await aesGcmKey(key);
        const plaintext = await subtleCrypto().decrypt(
            {
                name: "AES-GCM",
                iv: toBufferSource(parsed.iv),
                additionalData: toBufferSource(encStructure(parsed.protected)),
                tagLength: 128,
            },
            cryptoKey,
            toBufferSource(parsed.ciphertext),
        );
        return new Uint8Array(plaintext);
    } catch (e) {
        if (e instanceof BrowserCoseError) throw e;
        throw new BrowserCoseError(
            "auth-failed",
            "authentication failed (AES-GCM tag mismatch)",
        );
    }
}

interface ParsedSign1 {
    kid: string;
    protected: Uint8Array;
    signature: Uint8Array;
}

function parseSign1(cose: Uint8Array): ParsedSign1 | null {
    try {
        let body = decodeFirst(cose);
        if (body instanceof Tagged) {
            if (body.tag !== TAG_SIGN1) return null;
            body = body.value;
        }
        if (!Array.isArray(body) || body.length !== 4) return null;
        const prot = asBytes(body[0]);
        const signature = asBytes(body[3]);
        const unprotected = body[1];
        if (!prot || !signature || !(unprotected instanceof Map)) return null;
        const kidVal = asBytes(unprotected.get(KID));
        if (!kidVal) return null;
        return {
            kid: new TextDecoder().decode(kidVal),
            protected: prot,
            signature,
        };
    } catch {
        return null;
    }
}

interface ParsedEncrypt0 {
    kid: string;
    protected: Uint8Array;
    iv: Uint8Array;
    ciphertext: Uint8Array;
}

function parseEncrypt0(cose: Uint8Array): ParsedEncrypt0 | null {
    try {
        let body = decodeFirst(cose);
        if (body instanceof Tagged) {
            if (body.tag !== TAG_ENCRYPT0) return null;
            body = body.value;
        }
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

function sigStructure(prot: Uint8Array, frameId: Uint8Array): Uint8Array {
    return encodeCanonical(["Signature1", prot, new Uint8Array(0), frameId]);
}

function encStructure(prot: Uint8Array): Uint8Array {
    return encodeCanonical(["Encrypt0", prot, new Uint8Array(0)]);
}

function subtleCrypto(): SubtleCrypto {
    const subtle = globalThis.crypto?.subtle;
    if (!subtle) {
        throw new BrowserCoseError(
            "unsupported",
            "WebCrypto SubtleCrypto is not available",
        );
    }
    return subtle;
}

async function ed25519VerificationKey(key: KeyLike): Promise<CryptoKey> {
    if (isCryptoKey(key)) return key;
    return subtleCrypto().importKey(
        "raw",
        toBufferSource(keyBytes(key)),
        { name: "Ed25519" },
        false,
        ["verify"],
    );
}

async function aesGcmKey(key: KeyLike): Promise<CryptoKey> {
    if (isCryptoKey(key)) return key;
    const raw = keyBytes(key);
    if (raw.length !== 32) {
        throw new BrowserCoseError(
            "missing-key",
            "AES-GCM content key must be 32 bytes",
        );
    }
    return subtleCrypto().importKey(
        "raw",
        toBufferSource(raw),
        { name: "AES-GCM" },
        false,
        ["decrypt"],
    );
}

function keyBytes(key: Uint8Array | ArrayBuffer): Uint8Array {
    if (key instanceof Uint8Array) return copyBytes(key);
    return new Uint8Array(key);
}

function isCryptoKey(value: unknown): value is CryptoKey {
    return typeof CryptoKey !== "undefined" && value instanceof CryptoKey;
}

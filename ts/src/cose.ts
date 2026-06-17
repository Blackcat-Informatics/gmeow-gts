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

import { encode, decodeFirst } from "./wire.js";
import type { Signature } from "./model.js";

// Enable @noble's synchronous sign/verify (it needs a sync SHA-512).
ed.etc.sha512Sync = (...m: Uint8Array[]) => sha512(ed.etc.concatBytes(...m));

const ALG = 1;
const KID = 4;
const ALG_EDDSA = -8;
const TAG_SIGN1 = 18;

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

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Stored and decoded checks of a frame's payload under a file-wide capacity (P1 sections
//! 5.3 and 5.4). The classifier reads a parsed frame; the budget is shared by all frames of
//! one read, and the scan of resync candidates charges against the same total.

import { type Frame, type Storage } from "./v2_frame.js";

/** The fields of a frame that its payload check reads: a decoded frame has them all. */
export type PayloadFields = Pick<
    Frame,
    | "storage"
    | "storedPayload"
    | "decodedLength"
    | "payloadDigest"
    | "encodedSignaturesSize"
> & { content: { length: number } };
import {
    contextHash,
    contextHashParts,
    decodeZstd,
    zstdHeader,
} from "./v2_integrity.js";
import { mustEncode } from "./wire.js";

/** The capacity of one read (P1 section 5.4). */
export interface ReadCapacity {
    storedBytesPerFrame: bigint;
    decodedBytesPerFrame: bigint;
    decodedBytesPerRead: bigint;
    itemsPerRead: bigint;
}

/** The capacity that a reader applies when its caller gives none. */
export const DEFAULT_CAPACITY: ReadCapacity = {
    storedBytesPerFrame: 1n << 26n,
    decodedBytesPerFrame: 1n << 26n,
    decodedBytesPerRead: 1n << 36n,
    itemsPerRead: 1n << 26n,
};

/** Why a payload that is present was not checked. */
export type PayloadRefusal =
    | { kind: "stored_size"; count: bigint }
    | { kind: "decoded_size"; count: bigint }
    | { kind: "total_decoded"; count: bigint }
    | { kind: "unknown_codec" };

/** Why a payload that was checked does not hold. */
export type PayloadDamage =
    | "codec_constraint"
    | "decode_failed"
    | "decoded_length"
    | "payload_digest"
    | "malformed_fields";

/** The outcome of a payload check. */
export type PayloadState =
    | { kind: "verified"; decoded: Uint8Array }
    | { kind: "damaged"; damage: PayloadDamage }
    | { kind: "unchecked"; refusal: PayloadRefusal };

/** The payload check of one frame: the stored-check outcome, and the payload outcome. */
export interface PayloadCheck {
    storedMatches: boolean | undefined;
    state: PayloadState;
}

/**
 * Whether the stored check of a compressed frame matches its stored chunks (P1 section
 * 5.3). `undefined` for plain storage, which has no stored check.
 */
export function storedMatches(
    storage: Storage,
    chunks: readonly Uint8Array[],
): boolean | undefined {
    if (storage.kind !== "compressed") return undefined;
    const digest = contextHashParts("gts2 stored", [
        mustEncode([storage.codec]),
        ...chunks,
    ]);
    return equal(digest.subarray(0, 16), storage.storedCheck);
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

function startsWith(data: Uint8Array, prefix: Uint8Array): boolean {
    return (
        data.length >= prefix.length &&
        equal(data.subarray(0, prefix.length), prefix)
    );
}

/** The decoded bytes of a frame's payload, or the reason it does not hold (P1 section 5.4). */
function checkPayload(frame: PayloadFields): PayloadState {
    const stored = frame.storedPayload;
    const length = frame.decodedLength;
    let decoded: Uint8Array;
    const storage = frame.storage;
    if (storage.kind === "plain") {
        if (BigInt(stored.length) !== length) {
            return { kind: "damaged", damage: "decoded_length" };
        }
        decoded = stored;
    } else if (storage.kind === "compressed" && storage.codec === 1n) {
        const header = zstdHeader(Number(length));
        if (length === 0n || !startsWith(stored, header)) {
            return { kind: "damaged", damage: "codec_constraint" };
        }
        const output = decodeZstd(stored, header.length, Number(length));
        if (output === undefined) {
            return { kind: "damaged", damage: "decode_failed" };
        }
        decoded = output;
    } else {
        return { kind: "unchecked", refusal: { kind: "unknown_codec" } };
    }
    if (!equal(contextHash("gts2 payload", decoded), frame.payloadDigest)) {
        return { kind: "damaged", damage: "payload_digest" };
    }
    return { kind: "verified", decoded };
}

/** Why the charge of a parity candidate was refused (P5 section 7.2, step 6). */
export type ChargeRefusal =
    { kind: "frame" } | { kind: "total"; count: bigint | undefined };

/**
 * The file-wide budget of decoded bytes for one read. A charge that the total refuses
 * is refused from then on: every later charge and check in the read is unchecked.
 */
export class PayloadBudget {
    #total = 0n;
    #refused = false;
    #prepaid: bigint | undefined;

    constructor(private readonly capacity: ReadCapacity = DEFAULT_CAPACITY) {}

    /** Whether a charge has refused the total of the read. */
    get totalRefused(): boolean {
        return this.#refused;
    }

    /**
     * Charges `length` decoded bytes to the total. Returns the total that the charge would
     * have made when it is refused, or `undefined` when it is admitted.
     */
    charge(length: bigint): bigint | undefined {
        const next = this.#total + length;
        if (!this.#refused && next <= this.capacity.decodedBytesPerRead) {
            this.#total = next;
            return undefined;
        }
        this.#refused = true;
        return next;
    }

    /**
     * Charges one parity candidate of `stored` and `decoded` bytes (P5 section 7.2, step 6). A
     * candidate above the size of one frame is refused without a charge; a candidate that the
     * total refuses reports the total it would make, or none when the total was refused before.
     */
    chargeCandidate(
        stored: bigint,
        decoded: bigint,
    ): ChargeRefusal | undefined {
        if (this.#refused) return { kind: "total", count: undefined };
        if (
            stored > this.capacity.storedBytesPerFrame ||
            decoded > this.capacity.decodedBytesPerFrame
        ) {
            return { kind: "frame" };
        }
        const count = this.charge(decoded);
        return count === undefined ? undefined : { kind: "total", count };
    }

    /** Records that the decoded length of the next frame was already charged. */
    prepay(decoded: bigint): void {
        this.#prepaid = decoded;
    }

    private admit(frame: PayloadFields): PayloadRefusal | undefined {
        if (!this.#refused) {
            const sizes = [
                frame.storedPayload.length,
                frame.content.length,
                frame.encodedSignaturesSize,
            ];
            for (const size of sizes) {
                if (BigInt(size) > this.capacity.storedBytesPerFrame) {
                    return { kind: "stored_size", count: BigInt(size) };
                }
            }
            if (frame.decodedLength > this.capacity.decodedBytesPerFrame) {
                return { kind: "decoded_size", count: frame.decodedLength };
            }
        }
        const prepaid = this.#prepaid;
        this.#prepaid = undefined;
        const owed = prepaid === frame.decodedLength ? 0n : frame.decodedLength;
        const count = this.charge(owed);
        return count === undefined
            ? undefined
            : { kind: "total_decoded", count };
    }

    /** The payload check of `frame`, charged against this budget. */
    check(
        frame: PayloadFields,
        storedMatches: boolean | undefined,
    ): PayloadCheck {
        const refusal = this.admit(frame);
        const state =
            refusal === undefined
                ? checkPayload(frame)
                : { kind: "unchecked" as const, refusal };
        return { storedMatches, state };
    }
}

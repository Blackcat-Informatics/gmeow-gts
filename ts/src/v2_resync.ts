// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Walked resync openers and charged candidates (P1 sections 2.4a and 6.7).

import {
    BLAKE3_256,
    compareBytes as compare,
    Decoder,
    HeaderError,
} from "./v2_decoder.js";
import { contextHash, type PayloadBudget } from "./v2_integrity.js";

/** The bytes a resync opener may span, and the distance a scan tests past a pattern. */
export const RESYNC_MAX = 4096;

const PATTERN = Uint8Array.of(
    0x8b,
    0x05,
    0x02,
    0x50,
    0xfc,
    0xf3,
    0xeb,
    0x32,
    0xa2,
    0xf0,
    0x26,
    0x7d,
    0xd1,
    0x6a,
    0x53,
    0xf0,
    0x9b,
    0xff,
    0x87,
    0x92,
);

/** The fields of a header that a resync opener repeats. */
export interface ResyncHeader {
    registryVersion: bigint;
    hashAlgorithm: bigint;
    profile: bigint | undefined;
    lineage: { previousHead: Uint8Array; ordinal: bigint } | undefined;
}

/** A resync opener and its original advisory bytes. */
export interface DecodedResync {
    header: ResyncHeader;
    /** The offset that the resync item claims for its segment. */
    claimedOffset: bigint;
    /** The bytes that precede the claimed offset, if any. */
    past: Uint8Array | undefined;
    /** The marks of the resync item. */
    marks: Uint8Array;
    /** The identifier stored in the resync item. */
    storedIdentifier: Uint8Array;
    /** The identifier computed from the item, when its hash is known. */
    identifier: Uint8Array | undefined;
    /** The offset of the first byte after the resync item. */
    nextOffset: number;
}

function startsWith(data: Uint8Array, offset: number, prefix: Uint8Array) {
    if (offset + prefix.length > data.length) return false;
    return prefix.every((byte, index) => data[offset + index] === byte);
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

function decodePrefix(
    cursor: Decoder,
    candidate: boolean,
): { header: ResyncHeader; offset: bigint } {
    cursor.record(11);
    const kindError = cursor.error("unregistered_value");
    if (cursor.uint() !== 5n) throw kindError;
    const generationError = cursor.error("out_of_range");
    if (cursor.uint() !== 2n) throw generationError;
    const markerError = cursor.error("out_of_range");
    const marker = PATTERN.subarray(4);
    if (
        cursor.head(2) !== BigInt(marker.length) ||
        !equal(cursor.take(marker.length), marker)
    ) {
        throw markerError;
    }
    const { registryVersion, hashAlgorithm, profile } =
        cursor.openerFields(candidate);
    const lineage = cursor.option((d) =>
        d.lineage(hashAlgorithm === BLAKE3_256),
    );
    const offset = cursor.uint();
    return {
        header: { registryVersion, hashAlgorithm, profile, lineage },
        offset,
    };
}

function readMarks(cursor: Decoder): Uint8Array {
    const marksAt = cursor.offset;
    const count = cursor.head(4);
    let last = cursor.bytes.subarray(0, 0);
    for (let index = 0n; index < count; index++) {
        const entryAt = cursor.offset;
        const elements = cursor.head(4);
        if (elements === 0n) throw new HeaderError("wrong_type", entryAt);
        // The first element must be a uint by shape before count and order.
        if (cursor.peekByte() >> 5 !== 0) {
            throw new HeaderError("wrong_type", entryAt);
        }
        if (index >= 64n) throw new HeaderError("out_of_range", entryAt);
        // A complete item cannot be a proper prefix of another complete item.
        const available = cursor.bytes.subarray(entryAt);
        const prefix = available.subarray(
            0,
            Math.min(available.length, last.length),
        );
        if (
            last.length !== 0 &&
            compare(prefix, last) <= 0 &&
            (prefix.length === last.length || !startsWith(last, 0, prefix))
        ) {
            throw new HeaderError("unordered_or_repeated", entryAt);
        }
        for (let element = 0n; element < elements; element++) {
            cursor.closedItem();
        }
        last = cursor.bytes.subarray(entryAt, cursor.offset);
    }
    return cursor.bytes.subarray(marksAt, cursor.offset);
}

function decodeTail(
    cursor: Decoder,
    offset: number,
    header: ResyncHeader,
    claimedOffset: bigint,
): DecodedResync {
    const known = header.hashAlgorithm === BLAKE3_256;
    const digestSize = known ? 32 : undefined;
    const past = cursor.option((d) => d.byteString(digestSize));
    const marks = readMarks(cursor);
    const body = cursor.bytes.subarray(offset + 1, cursor.offset);
    const storedIdentifier = cursor.byteString(digestSize);
    const limit = cursor.fieldStartLimit;
    if (limit !== undefined && cursor.offset > limit) {
        throw new HeaderError("out_of_range", limit);
    }
    const identifier = known
        ? contextHash("gts2 header", Uint8Array.of(0x8a), body)
        : undefined;
    return {
        header,
        claimedOffset,
        past,
        marks,
        storedIdentifier,
        identifier,
        nextOffset: cursor.offset,
    };
}

/**
 * Decodes the resync item at `offset` within `data[..end]`, whose fields must start
 * within `RESYNC_MAX` bytes of it.
 *
 * @throws HeaderError at the first malformed field.
 */
export function decodeResync(
    data: Uint8Array,
    offset: number,
    end: number,
): DecodedResync {
    const cursor = new Decoder(data.subarray(0, end), offset);
    cursor.fieldStartLimit = offset + RESYNC_MAX;
    const { header, offset: claimed } = decodePrefix(cursor, false);
    return decodeTail(cursor, offset, header, claimed);
}

/** What a scan for a resync candidate finds. */
export type Candidate =
    | { kind: "accepted"; offset: number; value: DecodedResync }
    | { kind: "refused"; offset: number; count: bigint };

/** Scans the bytes between item boundaries for resync candidates (P1 section 6.7). */
export class ResyncScanner {
    private testedUntil = 0;

    constructor(private readonly data: Uint8Array) {}

    /** The next candidate in `[start, end)`, charging each tested one to `budget`. */
    scan(
        start: number,
        end: number,
        budget: PayloadBudget,
    ): Candidate | undefined {
        if (budget.totalRefused) return undefined;
        let next = Math.max(start, this.testedUntil);
        while (next < end) {
            const found = this.data.subarray(next, end).indexOf(PATTERN[0]);
            if (found < 0) break;
            const offset = next + found;
            next = offset + 1;
            if (!startsWith(this.data, offset, PATTERN)) continue;
            this.testedUntil = next;
            const limit = Math.min(offset + RESYNC_MAX, this.data.length);
            const cursor = new Decoder(this.data.subarray(0, limit), offset);
            let prefix: ReturnType<typeof decodePrefix>;
            try {
                prefix = decodePrefix(cursor, true);
            } catch (error) {
                if (error instanceof HeaderError) continue;
                throw error;
            }
            if (prefix.offset !== BigInt(offset)) continue;
            const count = budget.charge(BigInt(RESYNC_MAX));
            if (count !== undefined) return { kind: "refused", offset, count };
            try {
                const value = decodeTail(
                    cursor,
                    offset,
                    prefix.header,
                    prefix.offset,
                );
                if (
                    value.identifier !== undefined &&
                    equal(value.identifier, value.storedIdentifier)
                ) {
                    return { kind: "accepted", offset, value };
                }
            } catch (error) {
                if (!(error instanceof HeaderError)) throw error;
            }
        }
        this.testedUntil = Math.max(this.testedUntil, end);
        return undefined;
    }
}

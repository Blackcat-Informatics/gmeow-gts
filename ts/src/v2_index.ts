// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The claims of commit and index payloads that the wire reader checks (P5 sections 4.1
//! and 4.3). These functions read only bytes; the segment reader supplies the frames.

import { Decoder, HeaderError } from "./v2_decoder.js";

/** The registry code of the parity frame type, the last code of the frame type registry. */
const PARITY = 16n;

/** The record of a commit claim: the ordinary count and the range root. */
export function decodeClaim(content: Uint8Array): {
    count: bigint;
    root: Uint8Array;
} {
    const cursor = new Decoder(content, 0);
    cursor.record(2);
    const count = cursor.uint();
    const root = cursor.byteString(32);
    return { count, root };
}

/** The registry rule that an index payload applies to its codes (P5 4.3). */
export interface IndexRegistry {
    /** Whether the segment's registry version is later than the reader's. */
    newer: boolean;
    /** Whether a code of a later registry stops the fold, so later codes are refused. */
    stopped: boolean;
}

/** An enumeration code that a later registry introduces and this reader does not know. */
export class UnknownEnumeration extends Error {
    constructor(readonly offset: number) {
        super(`unknown enumeration at offset ${offset}`);
    }
}

/** A run of ordinary frames of one type, as the index records them. */
export interface TypeRun {
    frameType: bigint | undefined;
    length: bigint;
}

/**
 * Reads a registry code of an index payload. A code of `0`, or one beyond `last` in a
 * segment that is not later, is refused; a later code the registry stops at is unknown.
 */
function readCode(
    cursor: Decoder,
    registry: IndexRegistry,
    last: bigint,
): bigint {
    const error = cursor.error("unregistered_value");
    const code = cursor.uint();
    if (code === 0n || (code > last && !registry.newer)) throw error;
    if (code > last && !registry.stopped) {
        throw new UnknownEnumeration(error.offset);
    }
    return code;
}

/** A graph name: a tagged record of its fields (P5 4.3). */
function readGraphName(cursor: Decoder, registry: IndexRegistry): void {
    const error = cursor.error("wrong_length");
    const fields = cursor.head(4);
    if (fields === 0n) throw error;
    const tag = readCode(cursor, registry, 2n);
    if ((tag === 1n || tag === 2n) && tag !== fields) throw error;
    if (tag === 2n) {
        cursor.uint();
    } else if (tag !== 1n) {
        for (let index = 1n; index < fields; index++) cursor.closedItem();
    }
}

/**
 * Checks the grammar of an index payload and reads its type runs and root.
 *
 * Returns whether the payload's runs equal `runs`, and the range root it claims.
 * A failure is a `malformed` error at its offset, or an `unknown` enumeration that a
 * later registry version introduces.
 */
export function readIndexPayload(
    payload: Uint8Array,
    registry: IndexRegistry,
    runs: readonly TypeRun[],
):
    | { ok: true; runsMatch: boolean; root: Uint8Array }
    | { ok: false; kind: "malformed"; error: HeaderError }
    | { ok: false; kind: "unknown"; offset: number } {
    try {
        const cursor = new Decoder(payload, 0);
        cursor.record(6);
        const count = cursor.head(4);
        let runsMatch = count === BigInt(runs.length);
        for (let index = 0n; index < count; index++) {
            cursor.record(2);
            const frameType = readCode(cursor, registry, PARITY);
            const run = cursor.uint();
            const expected = runs[Number(index)];
            runsMatch =
                runsMatch &&
                expected !== undefined &&
                expected.frameType === frameType &&
                expected.length === run;
        }
        const graphs = cursor.head(4);
        for (let graph = 0n; graph < graphs; graph++) {
            cursor.record(2);
            readGraphName(cursor, registry);
            const entries = cursor.head(4);
            for (let entry = 0n; entry < entries; entry++) {
                cursor.record(4);
                readCode(cursor, registry, PARITY);
                for (let field = 0; field < 3; field++) cursor.uint();
            }
        }
        cursor.record(4);
        for (let field = 0; field < 3; field++) cursor.uint();
        const pairs = cursor.head(4);
        for (let pair = 0n; pair < pairs; pair++) {
            cursor.record(2);
            cursor.uint();
            cursor.uint();
        }
        const root = cursor.byteString(32);
        cursor.option((d) => d.byteString(32));
        const records = cursor.head(4);
        for (let record = 0n; record < records; record++) {
            cursor.record(3);
            cursor.byteString(32);
            cursor.uint();
            cursor.uint();
        }
        if (cursor.offset !== payload.length) {
            throw cursor.error("wrong_length");
        }
        return { ok: true, runsMatch, root };
    } catch (failure) {
        if (failure instanceof HeaderError) {
            return { ok: false, kind: "malformed", error: failure };
        }
        if (failure instanceof UnknownEnumeration) {
            return { ok: false, kind: "unknown", offset: failure.offset };
        }
        throw failure;
    }
}

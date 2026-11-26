// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The item walk of a v2 input: the boundaries of its items and the kind of each
//! item (P1 sections 6.3 and 7). The recovery of a lost item is not part of this module.

import {
    FramingError,
    head,
    itemEnd,
    type FramingReason,
} from "./v2_framing.js";

/** The reasons an item of a segment is malformed (P2 section 4.7, `malformed_value`). */
export type MalformedReason =
    "wrong_type" | "not_deterministic" | "unregistered_value" | "wrong_length";

/** The kind of an item of a segment (`ItemKind` of the reader). */
export type ItemKind =
    | { kind: "header" }
    | { kind: "resync" }
    | { kind: "frame" }
    | { kind: "locator" }
    | { kind: "offsets" }
    | { kind: "malformed"; reason: MalformedReason }
    | { kind: "unknown"; code: bigint }
    | { kind: "unchecked" }
    | { kind: "damaged_region" };

/** An item boundary: its offset, the offset after its last byte, and the failure that ends it, if any. */
export interface Boundary {
    offset: number;
    end: number;
    failure: FramingReason | undefined;
}

/**
 * Walks the items of an input in order, holding the boundaries that a caller
 * peeked at. The walk stops at `limit` items (the item limit of the reader).
 */
export class ItemWalk {
    private offset = 0;
    private count = 0;
    private readonly held: Boundary[] = [];

    constructor(
        private readonly data: Uint8Array,
        private readonly limit: number,
    ) {}

    /** The boundary of the next item that ends by `end`, or `undefined` when there is none. */
    private delimitTo(end: number): Boundary | undefined {
        const offset = this.offset;
        if (offset >= end || this.count === this.limit) return undefined;
        let stop = this.data.length;
        let failure: FramingReason | undefined;
        try {
            stop = itemEnd(this.data.subarray(0, end), offset);
        } catch (error) {
            if (!(error instanceof FramingError)) throw error;
            if (error.reason === "torn_tail" && end < this.data.length) {
                return undefined;
            }
            failure = error.reason;
        }
        if (failure !== "torn_tail") this.count += 1;
        this.offset = stop;
        return { offset, end: stop, failure };
    }

    /** The next boundary, consuming it. */
    take(): Boundary | undefined {
        return this.held.shift() ?? this.delimitTo(this.data.length);
    }

    /** The next boundary, not consumed. */
    peek(): Boundary | undefined {
        if (this.held.length === 0) {
            const next = this.delimitTo(this.data.length);
            if (next !== undefined) this.held.push(next);
        }
        return this.held[0];
    }
}

/**
 * The kind of the item at `offset` (`classify` of the reader): a definite array of
 * two items whose first is a code. `newer` admits the codes of a later registry
 * version as unknown items.
 */
export function classify(
    data: Uint8Array,
    offset: number,
    newer: boolean,
): ItemKind {
    const array = readHead(data, offset);
    if (array === undefined || array.major !== 4 || array.argument === 0n) {
        return { kind: "malformed", reason: "wrong_type" };
    }
    const kind = readHead(data, array.end);
    if (kind === undefined || kind.major !== 0 || kind.argument === undefined) {
        return { kind: "malformed", reason: "wrong_type" };
    }
    const code = kind.argument;
    if (code === 1n) return { kind: "header" };
    if (code === 5n) return { kind: "resync" };
    if (!array.shortest || !kind.shortest) {
        return { kind: "malformed", reason: "not_deterministic" };
    }
    if (code === 0n) return { kind: "malformed", reason: "unregistered_value" };
    if (code === 2n) {
        return array.argument === 11n
            ? { kind: "frame" }
            : { kind: "malformed", reason: "wrong_length" };
    }
    if (code === 3n) return { kind: "locator" };
    if (code === 4n) return { kind: "offsets" };
    if (newer) return { kind: "unknown", code };
    return { kind: "malformed", reason: "unregistered_value" };
}

/** The head at `offset`, or `undefined` when it cannot be read (`read_head` of the reader). */
function readHead(
    data: Uint8Array,
    offset: number,
):
    | {
          major: number;
          argument: bigint | undefined;
          end: number;
          shortest: boolean;
      }
    | undefined {
    try {
        const h = head(data, offset);
        return {
            major: h.major,
            argument: h.indefinite ? undefined : h.value,
            end: h.end,
            shortest: h.shortest,
        };
    } catch (error) {
        if (error instanceof FramingError) return undefined;
        throw error;
    }
}

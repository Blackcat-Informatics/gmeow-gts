// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The item walk of a v2 input: the boundaries of its items and the kind of each
//! item (P1 sections 6.3 and 7). The walk also moves past a lost item to the resync item that recovers it.

import {
    FramingError,
    head,
    itemEnd,
    type FramingReason,
} from "./v2_framing.js";
import { RESYNC_MAX, type Candidate, type ResyncScanner } from "./v2_resync.js";
import {
    nextPrefix,
    testParityCandidate,
    type ParityCandidate,
    type ParityContext,
} from "./v2_parity.js";
import type { PayloadBudget } from "./v2_payload.js";

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

    /**
     * Scans for the next candidate that resumes the read after a lost item: a resync item from
     * `start` to the end of `lost`, and, when `parity` is given, the parity candidates of the
     * segment in the same ascending order (P5 section 7.2, step 6). An accepted resync item or a
     * passing parity candidate moves the walk to the candidate's offset and drops the boundaries
     * held past it; the items it skips are not counted, except the torn item a parity region
     * replaces.
     */
    recover(
        scanner: ResyncScanner,
        start: number,
        lost: Boundary,
        budget: PayloadBudget,
        parity?: ParityContext,
    ): Candidate | ParityCandidate | undefined {
        if (lost.failure === undefined && this.peek() !== undefined) {
            const end = Math.min(lost.end + RESYNC_MAX, this.data.length);
            for (
                let next = this.delimitTo(end);
                next !== undefined;
                next = this.delimitTo(end)
            ) {
                this.held.push(next);
            }
        }
        if (this.count === this.limit) return undefined;
        const outcome =
            parity === undefined
                ? scanner.scan(start, lost.end, budget)
                : this.scanBoth(scanner, start, lost, budget, parity);
        if (outcome?.kind === "accepted") {
            this.count += 1;
            this.held.length = 0;
            this.offset = outcome.value.nextOffset;
        } else if (outcome?.kind === "parity") {
            this.count += lost.failure === "torn_tail" ? 1 : 0;
            this.held.length = 0;
            this.offset = outcome.offset;
        }
        return outcome;
    }

    /**
     * Tests the resync candidates and the parity candidates in one pass in ascending offset.
     * Resync candidates are tested over their own run from `start`; parity candidates from the
     * first byte that cannot be delimited. The first candidate that passes is returned, or the
     * first refusal by the total decoded length.
     */
    private scanBoth(
        scanner: ResyncScanner,
        start: number,
        lost: Boundary,
        budget: PayloadBudget,
        parity: ParityContext,
    ): Candidate | ParityCandidate | undefined {
        let resyncFrom = start;
        let parityFrom = lost.offset;
        for (;;) {
            const found = nextPrefix(this.data, parityFrom);
            const bound = found ?? lost.end;
            const candidate = scanner.scan(resyncFrom, bound, budget);
            if (candidate !== undefined) return candidate;
            resyncFrom = Math.max(resyncFrom, bound);
            if (found === undefined) return undefined;
            const verdict = testParityCandidate(
                this.data,
                found,
                parity,
                budget,
            );
            if (verdict.kind === "passed") {
                return {
                    kind: "parity",
                    offset: found,
                    anchor: verdict.anchor,
                };
            }
            if (verdict.kind === "refused") {
                return {
                    kind: "parity_refused",
                    offset: found,
                    length: verdict.length,
                    count: verdict.count,
                };
            }
            parityFrom = found + 1;
        }
    }

    /** The offset of the first item that the item limit left unread, if it did. */
    get limitOffset(): number | undefined {
        return this.count === this.limit && this.offset < this.data.length
            ? this.offset
            : undefined;
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

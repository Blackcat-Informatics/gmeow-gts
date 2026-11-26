// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Closing items and their placement (P1 sections 2.5 and 2.6, P5 sections 1 and 2): the
//! grammar of an offsets item and of a locator item, and the checks that tie them to the
//! items and index frame they describe. These functions take only items and frame positions.

import { Decoder, HeaderError } from "./v2_decoder.js";

/** The registry code of the index frame type (P5 section 4). */
const INDEX = 15n;

/** The kinds of item that the closing checks read. Other items are `other`. */
export interface ClosingItem {
    kind: "offsets" | "locator" | "other";
    offset: number;
    end: number;
}

/** The position and registry type of a frame of the segment, for the index claim. */
export interface ClosingFrame {
    position: number;
    frameType: bigint | undefined;
}

/** A closing item that fails its claim or its placement. */
export type ClosingFinding =
    | { kind: "malformed"; position: number; error: HeaderError }
    | { kind: "index_offsets"; position: number }
    | { kind: "index_locator"; position: number };

/**
 * Reads an offsets item, calling `visit` with each ordinal and the length it records, and
 * returns the count of its entries.
 */
function offsets(
    data: Uint8Array,
    item: ClosingItem,
    visit: (ordinal: bigint, length: bigint) => void,
): bigint {
    const cursor = new Decoder(data.subarray(0, item.end), item.offset);
    cursor.record(2);
    cursor.uint();
    const count = cursor.head(4);
    for (let ordinal = 0n; ordinal < count; ordinal++) {
        visit(ordinal, cursor.uint());
    }
    return count;
}

/** Reads a locator item and returns the distance it records back to its target. */
function locator(data: Uint8Array, item: ClosingItem): bigint {
    const cursor = new Decoder(data.subarray(0, item.end), item.offset);
    cursor.record(2);
    cursor.uint();
    if (data[cursor.offset] !== 0x1b) {
        // The locator is the exception to the shortest-head rule: a full 64-bit head.
        const error = cursor.error("not_deterministic");
        cursor.uint();
        throw error;
    }
    cursor.take(1);
    const bytes = cursor.take(8);
    return new DataView(bytes.buffer, bytes.byteOffset, 8).getBigUint64(0);
}

/**
 * Checks the grammar of a closing item of the kind the offsets and locator items take
 * (the `check_closing_grammar` of the reference reader).
 *
 * @throws HeaderError at the first malformed field of the item.
 */
export function checkClosingGrammar(data: Uint8Array, item: ClosingItem): void {
    if (item.kind === "offsets") {
        offsets(data, item, () => {});
    } else {
        locator(data, item);
    }
}

/**
 * Checks the placement and claims of the closing items of a segment (the `check_closing`
 * of the reference reader). An offsets item must follow the ordinary items it covers and
 * come last, or be followed only by the locator; a locator must come last. A claim that
 * does not hold is a finding at its item's position.
 */
export function checkClosing(
    data: Uint8Array,
    items: readonly ClosingItem[],
    frames: readonly ClosingFrame[],
): ClosingFinding[] {
    const findings: ClosingFinding[] = [];
    let offsetsSeen = false;
    let locatorSeen = false;
    // The offset of the index frame that the closing locator points at, if any.
    let lastIndexOffset: bigint | undefined;
    const lastItem = items[items.length - 1];
    if (lastItem?.kind === "locator") {
        let lastIndex: ClosingFrame | undefined;
        for (const frame of frames) {
            if (frame.frameType === INDEX) lastIndex = frame;
        }
        const indexItem = lastIndex && items[lastIndex.position];
        if (indexItem !== undefined) {
            lastIndexOffset = BigInt(indexItem.offset);
        }
    }
    for (let position = 0; position < items.length; position++) {
        const item = items[position];
        const rest = items.slice(position + 1);
        let misplaced: boolean;
        if (item.kind === "offsets") {
            const placeHolds =
                rest.length === 0 ||
                (rest.length === 1 && rest[0].kind === "locator");
            misplaced = offsetsSeen || locatorSeen || !placeHolds;
            offsetsSeen = true;
        } else if (item.kind === "locator") {
            misplaced = locatorSeen || rest.length !== 0;
            locatorSeen = true;
        } else {
            continue;
        }
        if (misplaced) {
            findings.push({
                kind: "malformed",
                position,
                error: new HeaderError("unordered_or_repeated", item.offset),
            });
            continue;
        }
        if (item.kind === "offsets") {
            let holds = true;
            try {
                const count = offsets(data, item, (ordinal, length) => {
                    const covered =
                        ordinal < BigInt(position)
                            ? items[Number(ordinal)]
                            : undefined;
                    const same =
                        covered !== undefined &&
                        BigInt(covered.end - covered.offset) === length;
                    holds = holds && same;
                });
                if (!(holds && count === BigInt(position))) {
                    findings.push({ kind: "index_offsets", position });
                }
            } catch (error) {
                if (!(error instanceof HeaderError)) throw error;
                findings.push({ kind: "malformed", position, error });
            }
        } else {
            try {
                const distance = locator(data, item);
                const matches =
                    distance !== 0n &&
                    lastIndexOffset !== undefined &&
                    BigInt(item.offset) - distance === lastIndexOffset;
                if (!matches)
                    findings.push({ kind: "index_locator", position });
            } catch (error) {
                if (!(error instanceof HeaderError)) throw error;
                findings.push({ kind: "malformed", position, error });
            }
        }
    }
    return findings;
}

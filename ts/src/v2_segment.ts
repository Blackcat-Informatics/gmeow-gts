// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The segment reader (P1 sections 2 to 4, P5 section 1): the segments of a whole input, each
//! opened by a header or a resync item, with the links, payload checks and closing items of its
//! frames. This slice reads an input whose items are all well formed and does not recover a
//! lost item: a lost item stops the read at its offset with the reason, and the commit, index
//! and parity checks come in later slices.

import { HeaderError } from "./v2_decoder.js";
import {
    checkClosing,
    checkClosingGrammar,
    type ClosingFinding,
    type ClosingItem,
} from "./v2_closing.js";
import { decodeFrame, FrameError, type Frame } from "./v2_frame.js";
import { decodeHeader, REGISTRY_VERSION, type Header } from "./v2_header.js";
import { frameIdentifier } from "./v2_integrity.js";
import { linkOf, type FrameLink } from "./v2_link.js";
import {
    DEFAULT_CAPACITY,
    PayloadBudget,
    type PayloadCheck,
    type ReadCapacity,
    storedMatches,
} from "./v2_payload.js";
import { decodeResync, type DecodedResync } from "./v2_resync.js";
import { classify, ItemWalk, type ItemKind } from "./v2_walk.js";

/** The opener of a segment: a header item, or a resync item that begins a segment. */
export type SegmentOpener =
    | { kind: "header"; header: Header; identifier: Uint8Array | undefined }
    | { kind: "resync"; value: DecodedResync };

/** A frame of a segment as read from the wire. */
export interface WireFrame {
    /** The position of the frame's item in its segment. */
    position: number;
    frame: Frame;
    link: FrameLink;
    payload: PayloadCheck;
}

/** An item of a segment and the bytes it covers. */
export interface WireItem {
    kind: ItemKind["kind"] | "frame";
    offset: number;
    end: number;
}

/** A segment of the input: its opener, its items and frames, and its closing findings. */
export interface WireSegment {
    opener: SegmentOpener;
    items: WireItem[];
    frames: WireFrame[];
    /** The identifier that the next frame is linked to. */
    head: Uint8Array | undefined;
    closing: ClosingFinding[];
}

/** Why the read stopped before the end of the input. */
export type ReadStop =
    | { kind: "framing"; reason: string; offset: number }
    | { kind: "lost_item"; offset: number }
    | { kind: "unreadable_frame"; offset: number }
    | { kind: "unreadable_opener"; offset: number };

/** The segments of an input, in order, and the reason reading stopped, if it did. */
export interface WireRead {
    segments: WireSegment[];
    stop: ReadStop | undefined;
}

function headerOf(opener: SegmentOpener): {
    registryVersion: bigint;
    identifier: Uint8Array | undefined;
} {
    if (opener.kind === "header") {
        return {
            registryVersion: opener.header.registryVersion,
            identifier: opener.identifier,
        };
    }
    return {
        registryVersion: opener.value.header.registryVersion,
        identifier: opener.value.identifier,
    };
}

function openerOf(
    data: Uint8Array,
    offset: number,
    end: number,
    kind: "header" | "resync",
): SegmentOpener | undefined {
    try {
        if (kind === "header") {
            const decoded = decodeHeader(data.subarray(0, end), offset);
            return {
                kind: "header",
                header: decoded.header,
                identifier: decoded.identifier,
            };
        }
        return { kind: "resync", value: decodeResync(data, offset, end) };
    } catch (error) {
        if (error instanceof HeaderError) return undefined;
        throw error;
    }
}

/**
 * Whether the next item is a frame whose stored identifier equals the identifier computed
 * from `candidate` as its predecessor (P1 section 3.4): the evidence that heals a link.
 */
function followingConfirms(
    data: Uint8Array,
    walk: ItemWalk,
    registryVersion: bigint,
    candidate: Uint8Array,
): boolean | undefined {
    const next = walk.peek();
    if (next === undefined) return undefined;
    if (next.failure !== undefined) return false;
    if (
        classify(data, next.offset, registryVersion > REGISTRY_VERSION).kind !==
        "frame"
    )
        return false;
    try {
        const following = decodeFrame(data, next.offset, registryVersion);
        if (
            following.identityFields === undefined ||
            following.identifier === undefined
        )
            return false;
        const computed = frameIdentifier(candidate, following.identityFields);
        return (
            computed.length === following.identifier.length &&
            computed.every(
                (byte, index) => byte === following.identifier?.[index],
            )
        );
    } catch (error) {
        if (error instanceof FrameError) return false;
        throw error;
    }
}

function emptySegment(opener: SegmentOpener): WireSegment {
    const { identifier } = headerOf(opener);
    return { opener, items: [], frames: [], head: identifier, closing: [] };
}

function finish(segment: WireSegment, data: Uint8Array): void {
    const items: ClosingItem[] = segment.items.map((item) => ({
        kind:
            item.kind === "offsets" || item.kind === "locator"
                ? item.kind
                : "other",
        offset: item.offset,
        end: item.end,
    }));
    const frames = segment.frames.map((frame) => ({
        position: frame.position,
        frameType: frame.frame.frameType,
    }));
    segment.closing = checkClosing(data, items, frames);
}

/**
 * Reads the segments of `data` in order. Each frame is linked to the identifier before it,
 * its payload is checked against `capacity`, and the closing items are checked when a segment
 * ends. The read stops at the first item it cannot yet read, with that item's offset.
 */
export function readWire(
    data: Uint8Array,
    capacity: ReadCapacity = DEFAULT_CAPACITY,
): WireRead {
    const result: WireRead = { segments: [], stop: undefined };
    const walk = new ItemWalk(data, Number(capacity.itemsPerRead));
    const budget = new PayloadBudget(capacity);
    const first = walk.take();
    if (first === undefined) return result;
    if (first.failure !== undefined) {
        result.stop = {
            kind: "framing",
            reason: first.failure,
            offset: first.offset,
        };
        return result;
    }
    const firstKind = classify(data, first.offset, false).kind;
    if (firstKind !== "header" && firstKind !== "resync") {
        result.stop = { kind: "lost_item", offset: first.offset };
        return result;
    }
    const opener = openerOf(data, first.offset, first.end, firstKind);
    if (opener === undefined) {
        result.stop = { kind: "unreadable_opener", offset: first.offset };
        return result;
    }
    let segment = emptySegment(opener);
    segment.items.push({
        kind: firstKind,
        offset: first.offset,
        end: first.end,
    });
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        if (next.failure !== undefined) {
            result.stop = {
                kind: "framing",
                reason: next.failure,
                offset: next.offset,
            };
            break;
        }
        const { registryVersion } = headerOf(segment.opener);
        const kind = classify(
            data,
            next.offset,
            registryVersion > REGISTRY_VERSION,
        ).kind;
        if (kind === "header" || kind === "resync") {
            const begun = openerOf(data, next.offset, next.end, kind);
            if (begun === undefined) {
                result.stop = {
                    kind: "unreadable_opener",
                    offset: next.offset,
                };
                break;
            }
            finish(segment, data);
            result.segments.push(segment);
            segment = emptySegment(begun);
            segment.items.push({ kind, offset: next.offset, end: next.end });
            continue;
        }
        if (kind === "frame") {
            let frame: Frame;
            try {
                frame = decodeFrame(data, next.offset, registryVersion);
            } catch (error) {
                if (!(error instanceof FrameError)) throw error;
                result.stop = { kind: "unreadable_frame", offset: next.offset };
                break;
            }
            const predecessor =
                segment.head ??
                headerOf(segment.opener).identifier ??
                new Uint8Array(32);
            const link = linkOf(
                {
                    transparent: frame.transparent,
                    identity: frame.identityFields,
                    stored: frame.identifier,
                },
                predecessor,
                (candidate) =>
                    followingConfirms(data, walk, registryVersion, candidate),
            );
            const payload = budget.check(
                frame,
                storedMatches(frame.storage, [frame.storedPayload]),
            );
            if (link.effectiveIdentifier !== undefined)
                segment.head = link.effectiveIdentifier;
            segment.frames.push({
                position: segment.items.length,
                frame,
                link,
                payload,
            });
            segment.items.push({
                kind: "frame",
                offset: next.offset,
                end: next.end,
            });
            continue;
        }
        if (kind === "locator" || kind === "offsets") {
            try {
                checkClosingGrammar(data, {
                    kind,
                    offset: next.offset,
                    end: next.end,
                });
            } catch (error) {
                if (!(error instanceof HeaderError)) throw error;
            }
            segment.items.push({ kind, offset: next.offset, end: next.end });
            continue;
        }
        result.stop = { kind: "lost_item", offset: next.offset };
        break;
    }
    finish(segment, data);
    result.segments.push(segment);
    return result;
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The segment reader (P1 sections 2 to 4, P5 section 1): the segments of a whole input, each
//! opened by a header or a resync item, with the links, payload checks and closing items of its
//! frames. A lost item (a framing failure, a malformed item, a frame whose link is broken, a
//! closing item that breaks its grammar, an opener that cannot be read) is recovered by the
//! next resync item that the scanner accepts (P1 section 6.7). The commit, index and parity
//! checks, and the fields of a frame that cannot be decoded, come in later slices: such a
//! frame stops the read.

import { FramingError, itemEnd, type FramingReason } from "./v2_framing.js";
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
import {
    decodeResync,
    RESYNC_MAX,
    ResyncScanner,
    type Candidate,
    type DecodedResync,
} from "./v2_resync.js";
import { classify, ItemWalk, type Boundary, type ItemKind } from "./v2_walk.js";

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
    /** The offset after the last byte that belongs to the segment. */
    endOffset: number;
    /** The framing damage that ends the segment before the resync item that recovered it. */
    framingFailure: FramingDamage | undefined;
    /** The candidate that the decoded-byte budget refused while recovering a lost item. */
    resyncRefusal: ResyncRefusal | undefined;
}

/** Framing damage over a run of bytes. */
export interface FramingDamage {
    reason: FramingReason;
    offset: number;
    length: number;
}

/** A resync candidate that the total decoded length refused (P1 section 5.4). */
export interface ResyncRefusal {
    /** The position in the segment of the item that was being recovered. */
    position: number;
    offset: number;
    length: number;
    count: bigint;
}

/** Why the read stopped before the end of the input. */
export type ReadStop =
    | { kind: "framing"; reason: string; offset: number }
    | { kind: "lost_item"; offset: number }
    | { kind: "item_limit"; offset: number }
    | { kind: "unreadable_frame"; offset: number }
    | { kind: "unreadable_opener"; offset: number };

/** The segments of an input, in order, and the reason reading stopped, if it did. */
export interface WireRead {
    segments: WireSegment[];
    stop: ReadStop | undefined;
    /** The framing damage before the first segment's resync opener, if one recovered the read. */
    prefixFailure: FramingDamage | undefined;
    /** The candidate refused before any segment began. */
    resyncRefusal: ResyncRefusal | undefined;
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
    return {
        opener,
        items: [],
        frames: [],
        head: identifier,
        closing: [],
        endOffset: 0,
        framingFailure: undefined,
        resyncRefusal: undefined,
    };
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

function damageBefore(
    data: Uint8Array,
    start: number,
    end: number,
): FramingDamage | undefined {
    if (start >= end) return undefined;
    try {
        itemEnd(data.subarray(0, end), start);
    } catch (error) {
        if (!(error instanceof FramingError)) throw error;
        return { reason: error.reason, offset: start, length: end - start };
    }
    return undefined;
}

function recoveredSegment(
    offset: number,
    candidate: Extract<Candidate, { kind: "accepted" }>,
): WireSegment {
    const segment = emptySegment({ kind: "resync", value: candidate.value });
    segment.items.push({
        kind: "resync",
        offset,
        end: candidate.value.nextOffset,
    });
    segment.endOffset = candidate.value.nextOffset;
    return segment;
}

/** Whether the last frame of `segment` is the last item and its link was healed. */
function healedLast(segment: WireSegment): WireFrame | undefined {
    const last = segment.frames.at(-1);
    return last !== undefined &&
        last.position + 1 === segment.items.length &&
        last.link.state === "healed"
        ? last
        : undefined;
}

/**
 * Ends `segment` at the accepted resync item at `offset`. A healed link of the last frame was
 * confirmed by the item the resync item replaces, so it is broken again and keeps its stored
 * identifier. Damage before the resync item is reported on the segment.
 */
function endAtResync(
    segment: WireSegment,
    data: Uint8Array,
    lost: Boundary,
    offset: number,
): void {
    const healed = healedLast(segment);
    if (healed !== undefined) {
        const stored = healed.frame.identifier;
        healed.link = {
            ...healed.link,
            state: "broken",
            effectiveIdentifier: stored,
        };
        segment.head = stored;
    }
    segment.endOffset = offset;
    segment.framingFailure = damageBefore(data, lost.offset, offset);
    const failure = segment.framingFailure;
    if (failure === undefined) return;
    if (failure.reason === "torn_tail") {
        segment.endOffset = failure.offset;
    } else {
        segment.items.push({
            kind: "damaged_region",
            offset: failure.offset,
            end: offset,
        });
    }
}

/**
 * Recovers from a lost item by scanning for the next resync item. An accepted candidate ends
 * `segment`; a refused one is recorded on it and the read goes on without recovery.
 */
function recoverLostItem(
    segment: WireSegment,
    walk: ItemWalk,
    scanner: ResyncScanner,
    budget: PayloadBudget,
    lost: Boundary,
    data: Uint8Array,
): Extract<Candidate, { kind: "accepted" }> | undefined {
    const last = segment.items.at(-1);
    const start =
        healedLast(segment) !== undefined || last === undefined
            ? lost.offset
            : last.offset + 1;
    const candidate = walk.recover(scanner, start, lost, budget);
    if (candidate?.kind === "accepted") {
        endAtResync(segment, data, lost, candidate.offset);
        return candidate;
    }
    if (candidate?.kind === "refused") {
        segment.resyncRefusal = {
            position:
                segment.items.length - (candidate.offset < lost.offset ? 1 : 0),
            offset: candidate.offset,
            length: RESYNC_MAX,
            count: candidate.count,
        };
    }
    return undefined;
}

/**
 * Reads the segments of `data` in order. Each frame is linked to the identifier before it,
 * its payload is checked against `capacity`, and the closing items are checked when a segment
 * ends. A lost item is recovered at the next accepted resync item, which begins a segment;
 * otherwise the read stops at the first item it cannot yet read, with that item's offset.
 */
export function readWire(
    data: Uint8Array,
    capacity: ReadCapacity = DEFAULT_CAPACITY,
): WireRead {
    const result: WireRead = {
        segments: [],
        stop: undefined,
        prefixFailure: undefined,
        resyncRefusal: undefined,
    };
    const walk = new ItemWalk(data, Number(capacity.itemsPerRead));
    const scanner = new ResyncScanner(data);
    const budget = new PayloadBudget(capacity);
    const first = walk.take();
    if (first === undefined) return result;
    let segment: WireSegment;
    const firstKind =
        first.failure === undefined
            ? classify(data, first.offset, false).kind
            : undefined;
    const opener =
        firstKind === "header" || firstKind === "resync"
            ? openerOf(data, first.offset, first.end, firstKind)
            : undefined;
    if (
        opener !== undefined &&
        (firstKind === "header" || firstKind === "resync")
    ) {
        segment = emptySegment(opener);
        segment.items.push({
            kind: firstKind,
            offset: first.offset,
            end: first.end,
        });
        segment.endOffset = first.end;
    } else {
        const candidate = walk.recover(scanner, 0, first, budget);
        if (candidate?.kind !== "accepted") {
            if (candidate?.kind === "refused") {
                result.resyncRefusal = {
                    position: 0,
                    offset: candidate.offset,
                    length: RESYNC_MAX,
                    count: candidate.count,
                };
            }
            result.stop =
                first.failure !== undefined
                    ? {
                          kind: "framing",
                          reason: first.failure,
                          offset: first.offset,
                      }
                    : firstKind === "header" || firstKind === "resync"
                      ? { kind: "unreadable_opener", offset: first.offset }
                      : { kind: "lost_item", offset: first.offset };
            return result;
        }
        result.prefixFailure = damageBefore(data, 0, candidate.offset);
        segment = recoveredSegment(candidate.offset, candidate);
    }
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        const { registryVersion } = headerOf(segment.opener);
        const blind = segment.head === undefined;
        const kind: ItemKind =
            next.failure === undefined
                ? classify(
                      data,
                      next.offset,
                      blind || registryVersion > REGISTRY_VERSION,
                  )
                : { kind: "unchecked" };
        let lost = next.failure !== undefined || kind.kind === "malformed";
        let begun: SegmentOpener | undefined;
        let frame: WireFrame | undefined;
        let unreadableFrame = false;
        if (next.failure === undefined) {
            if (kind.kind === "header" || kind.kind === "resync") {
                begun = openerOf(data, next.offset, next.end, kind.kind);
                lost = begun === undefined;
            } else if (kind.kind === "frame" && !blind) {
                try {
                    const decoded = decodeFrame(
                        data,
                        next.offset,
                        registryVersion,
                    );
                    const link = linkOf(
                        {
                            transparent: decoded.transparent,
                            identity: decoded.identityFields,
                            stored: decoded.identifier,
                        },
                        segment.head ?? new Uint8Array(32),
                        (candidate) =>
                            followingConfirms(
                                data,
                                walk,
                                registryVersion,
                                candidate,
                            ),
                    );
                    frame = {
                        position: segment.items.length,
                        frame: decoded,
                        link,
                        payload: budget.check(
                            decoded,
                            storedMatches(decoded.storage, [
                                decoded.storedPayload,
                            ]),
                        ),
                    };
                    lost = link.state === "broken";
                } catch (error) {
                    if (!(error instanceof FrameError)) throw error;
                    unreadableFrame = true;
                    lost = true;
                }
            } else if (
                (kind.kind === "locator" || kind.kind === "offsets") &&
                !blind
            ) {
                try {
                    checkClosingGrammar(data, {
                        kind: kind.kind,
                        offset: next.offset,
                        end: next.end,
                    });
                } catch (error) {
                    if (!(error instanceof HeaderError)) throw error;
                    lost = true;
                }
            }
        }
        if (lost) {
            const recovered = recoverLostItem(
                segment,
                walk,
                scanner,
                budget,
                next,
                data,
            );
            if (recovered !== undefined) {
                finish(segment, data);
                result.segments.push(segment);
                segment = recoveredSegment(recovered.offset, recovered);
                continue;
            }
        }
        if (next.failure !== undefined) {
            result.stop = {
                kind: "framing",
                reason: next.failure,
                offset: next.offset,
            };
            break;
        }
        if (unreadableFrame) {
            result.stop = { kind: "unreadable_frame", offset: next.offset };
            break;
        }
        if (kind.kind === "header" || kind.kind === "resync") {
            if (begun === undefined) {
                result.stop = {
                    kind: "unreadable_opener",
                    offset: next.offset,
                };
                break;
            }
            segment.endOffset = next.offset;
            finish(segment, data);
            result.segments.push(segment);
            segment = emptySegment(begun);
            segment.items.push({
                kind: kind.kind,
                offset: next.offset,
                end: next.end,
            });
            segment.endOffset = next.end;
            continue;
        }
        if (frame !== undefined) {
            if (frame.link.effectiveIdentifier !== undefined)
                segment.head = frame.link.effectiveIdentifier;
            segment.frames.push(frame);
        }
        segment.items.push({
            kind: blind ? "unchecked" : kind.kind,
            offset: next.offset,
            end: next.end,
        });
        segment.endOffset = next.end;
    }
    result.stop ??=
        walk.limitOffset === undefined
            ? undefined
            : { kind: "item_limit", offset: walk.limitOffset };
    finish(segment, data);
    result.segments.push(segment);
    return result;
}

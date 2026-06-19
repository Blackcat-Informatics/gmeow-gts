// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { Tagged } from "cbor";
import * as wire from "./wire.js";
import { ReadFileSegments } from "./reader.js";
import type { Diagnostic, StreamableInfo } from "./model.js";

/** Byte and chain metadata for one non-header frame. */
export interface FrameInventory {
    /** Absolute CBOR item index in the file. */
    itemIndex: number;
    /** Zero-based frame index within the segment. */
    frameIndex: number;
    /** Half-open byte offsets for the encoded CBOR item. */
    start: number;
    end: number;
    /** Stored frame id when present, otherwise the computed id. */
    id: Uint8Array;
    /** Frame "t" value, or a diagnostic placeholder. */
    frameType: string;
    /** True only when self-hash and prev-chain checks both pass. */
    valid: boolean;
}

/** Inventory for one GTS segment and its frames. */
export interface SegmentInventory {
    /** Zero-based segment index. */
    index: number;
    /** Half-open CBOR item range for the segment. */
    itemStart: number;
    itemEnd: number;
    /** Half-open byte range for the segment. */
    start: number;
    end: number;
    /** Folded segment profile, or the header fallback. */
    profile: string;
    /** Final id/prev head for the segment. */
    head?: Uint8Array;
    /** Number of non-header frames in the segment. */
    frameCount: number;
    /** Streamable-layout state for the segment. */
    layout: StreamableInfo;
    /** Reader diagnostics scoped to this segment. */
    diagnostics: Diagnostic[];
    /** Per-frame byte ranges and chain validity. */
    frames: FrameInventory[];
}

/** Cleanly bounded replication view of a GTS file. */
export interface Inventory {
    /** Segments in file order. */
    segments: SegmentInventory[];
    /** File-level parse diagnostic that prevents range answers. */
    fatal?: Diagnostic;
    /** Byte offset of an incomplete trailing CBOR item, or -1. */
    torn: number;
    /** First byte after the cleanly decoded prefix. */
    cleanEnd: number;
    /** Number of complete CBOR items decoded from the file. */
    itemCount: number;
}

/** Half-open byte range suitable for replication. */
export interface ByteRange {
    start: number;
    end: number;
}

/** Result of comparing a peer head against local segment/frame heads. */
export type MissingStatus = "complete" | "ranges" | "unknown" | "error";

/** Append-range answer for a peer's known head. */
export interface MissingResult {
    /** Complete, ranges, unknown, or error. */
    status: MissingStatus;
    /** Peer-supplied segment or frame head. */
    fromHead: Uint8Array;
    /** Clean append ranges after fromHead. */
    ranges: ByteRange[];
    /** True when the caller should fall back to inventory exchange. */
    scanRequired: boolean;
    /** Human-readable reason for unknown/error results. */
    detail?: string;
}

function bytesEqual(
    a: Uint8Array | undefined,
    b: Uint8Array | undefined,
): boolean {
    if (a === undefined || b === undefined) return false;
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
        if (a[i] !== b[i]) return false;
    }
    return true;
}

/** True when replication should not trust byte-range deltas from this inventory. */
export function hasProblems(inv: Inventory): boolean {
    if (inv.fatal !== undefined || inv.torn >= 0) return true;
    return inv.segments.some((segment) => segment.diagnostics.length > 0);
}

function problemDetail(inv: Inventory): string | undefined {
    if (inv.fatal !== undefined) {
        return `${inv.fatal.code}: ${inv.fatal.detail}`;
    }
    if (inv.torn >= 0) return `torn at offset ${inv.torn}`;
    for (const segment of inv.segments) {
        if (segment.diagnostics.length > 0) {
            const diagnostic = segment.diagnostics[0];
            return `${diagnostic.code}: ${diagnostic.detail}`;
        }
    }
    return undefined;
}

function isHeaderItem(item: unknown): boolean {
    let inner = item;
    if (item instanceof Tagged) inner = item.value;
    if (!(inner instanceof Map)) return false;
    return (
        wire.mapGet(inner, "gts") !== undefined &&
        wire.mapGet(inner, "t") === undefined
    );
}

function itemEnd(
    items: wire.CborItem[],
    torn: number,
    dataLen: number,
    index: number,
): number {
    if (index + 1 < items.length) return items[index + 1].offset;
    return torn >= 0 ? torn : dataLen;
}

function headerProfile(item: unknown): string {
    try {
        const header = wire.unwrapHeader(item);
        return wire.asText(wire.mapGet(header, "prof")) ?? "generic";
    } catch {
        return "generic";
    }
}

function headerStoredId(item: unknown): Uint8Array | undefined {
    try {
        return wire.asBytes(wire.mapGet(wire.unwrapHeader(item), "id"));
    } catch {
        return undefined;
    }
}

function headerComputedId(item: unknown): Uint8Array | undefined {
    try {
        return wire.headerId(wire.unwrapHeader(item));
    } catch {
        return undefined;
    }
}

function collectFrames(
    items: wire.CborItem[],
    torn: number,
    dataLen: number,
    start: number,
    end: number,
): FrameInventory[] {
    const frames: FrameInventory[] = [];
    let expectedPrev =
        headerStoredId(items[start].item) ??
        headerComputedId(items[start].item) ??
        new Uint8Array();
    for (let itemIndex = start + 1; itemIndex < end; itemIndex++) {
        const itemStart = items[itemIndex].offset;
        const itemStop = itemEnd(items, torn, dataLen, itemIndex);
        const frameIndex = itemIndex - start - 1;
        const item = items[itemIndex].item;
        if (!(item instanceof Map)) {
            frames.push({
                itemIndex,
                frameIndex,
                start: itemStart,
                end: itemStop,
                id: new Uint8Array(),
                frameType: "<non-map>",
                valid: false,
            });
            continue;
        }
        const computed = wire.contentId(item);
        const storedId = wire.asBytes(wire.mapGet(item, "id"));
        const frameId = storedId ?? computed;
        const prev = wire.asBytes(wire.mapGet(item, "prev"));
        frames.push({
            itemIndex,
            frameIndex,
            start: itemStart,
            end: itemStop,
            id: frameId,
            frameType: wire.asText(wire.mapGet(item, "t")) ?? "<unknown>",
            valid:
                storedId !== undefined &&
                bytesEqual(storedId, computed) &&
                bytesEqual(prev, expectedPrev),
        });
        expectedPrev = frameId;
    }
    return frames;
}

/** Parse data into segment and frame byte-range inventory. */
export function inventory(data: Uint8Array): Inventory {
    const { items, torn } = wire.iterItems(data);
    const cleanEnd = torn >= 0 ? torn : data.length;
    const fs = ReadFileSegments(data);
    if (items.length === 0 || fs.fatal !== undefined) {
        return {
            segments: [],
            fatal: fs.fatal,
            torn,
            cleanEnd,
            itemCount: items.length,
        };
    }
    const bounds: number[] = [];
    for (let i = 0; i < items.length; i++) {
        if (isHeaderItem(items[i].item)) bounds.push(i);
    }
    if (bounds.length === 0 || bounds[0] !== 0) {
        return {
            segments: [],
            fatal: fs.fatal,
            torn,
            cleanEnd,
            itemCount: items.length,
        };
    }
    const segments: SegmentInventory[] = [];
    for (let index = 0; index < bounds.length; index++) {
        const startItem = bounds[index];
        const endItem =
            index + 1 < bounds.length ? bounds[index + 1] : items.length;
        if (index >= fs.segments.length) {
            break;
        }
        const graph = fs.segments[index];
        const start = items[startItem].offset;
        const end = endItem < items.length ? items[endItem].offset : cleanEnd;
        segments.push({
            index,
            itemStart: startItem,
            itemEnd: endItem,
            start,
            end,
            profile:
                graph.segmentProfiles[0] ??
                headerProfile(items[startItem].item),
            head: graph.segmentHeads[0],
            frameCount: Math.max(0, endItem - startItem - 1),
            layout: graph.segmentStreamable[0] ?? {
                claimed: false,
                covered: 0,
                tail: 0,
            },
            diagnostics: [...graph.diagnostics],
            frames: collectFrames(items, torn, data.length, startItem, endItem),
        });
    }
    return {
        segments,
        fatal: fs.fatal,
        torn,
        cleanEnd,
        itemCount: items.length,
    };
}

function aggregateDigest(inv: Inventory): Uint8Array {
    const heads = inv.segments
        .map((segment) => segment.head)
        .filter((head): head is Uint8Array => head !== undefined);
    return wire.blake3_256(wire.encode(["gts-segment-heads-v1", heads]));
}

function optionalHex(value: Uint8Array | undefined): string | null {
    return value !== undefined ? wire.hex(value) : null;
}

function diagnosticJson(diagnostic: Diagnostic): Record<string, unknown> {
    return {
        code: diagnostic.code,
        detail: diagnostic.detail,
        frame_index: diagnostic.frameIndex ?? null,
    };
}

function layoutJson(layout: StreamableInfo): Record<string, unknown> {
    return {
        claimed: layout.claimed,
        covered: layout.covered,
        tail: layout.tail,
        head: optionalHex(layout.head),
    };
}

function rangeJson(range: ByteRange): Record<string, number> {
    return {
        start: range.start,
        end: range.end,
        length: Math.max(0, range.end - range.start),
    };
}

function jsonLine(value: unknown): string {
    return `${JSON.stringify(value)}\n`;
}

/** Return the stable gts-replication-heads-v1 JSON document. */
export function headsJson(inv: Inventory): string {
    const segmentHeads = inv.segments
        .map((segment) => segment.head)
        .filter((head): head is Uint8Array => head !== undefined)
        .map((head) => wire.hex(head));
    const fileHead =
        inv.segments.length > 0
            ? inv.segments[inv.segments.length - 1].head
            : undefined;
    return jsonLine({
        schema: "gts-replication-heads-v1",
        clean: !hasProblems(inv),
        segment_heads: segmentHeads,
        aggregate: {
            schema: "gts-segment-heads-v1",
            count: segmentHeads.length,
            digest: wire.hex(aggregateDigest(inv)),
            file_head: optionalHex(fileHead),
        },
        torn_at: inv.torn >= 0 ? inv.torn : null,
        fatal: inv.fatal !== undefined ? diagnosticJson(inv.fatal) : null,
    });
}

/** Return the stable gts-replication-segments-v1 JSON document. */
export function segmentsJson(inv: Inventory): string {
    return jsonLine({
        schema: "gts-replication-segments-v1",
        clean: !hasProblems(inv),
        segments: inv.segments.map((segment) => ({
            index: segment.index,
            byte_range: rangeJson({ start: segment.start, end: segment.end }),
            item_range: { start: segment.itemStart, end: segment.itemEnd },
            profile: segment.profile,
            head: optionalHex(segment.head),
            frame_count: segment.frameCount,
            layout: layoutJson(segment.layout),
            diagnostics: segment.diagnostics.map(diagnosticJson),
        })),
        item_count: inv.itemCount,
        torn_at: inv.torn >= 0 ? inv.torn : null,
        fatal: inv.fatal !== undefined ? diagnosticJson(inv.fatal) : null,
    });
}

/**
 * Return append ranges needed after fromHead, or request a scan.
 *
 * Known segment heads return bytes after the segment. Known valid frame ids
 * return bytes after that frame. Unknown heads are not guessed.
 */
export function missing(inv: Inventory, fromHead: Uint8Array): MissingResult {
    if (hasProblems(inv)) {
        return {
            status: "error",
            fromHead,
            ranges: [],
            scanRequired: false,
            detail: problemDetail(inv),
        };
    }
    for (const segment of inv.segments) {
        if (bytesEqual(segment.head, fromHead)) {
            const ranges =
                segment.end < inv.cleanEnd
                    ? [{ start: segment.end, end: inv.cleanEnd }]
                    : [];
            return {
                status: ranges.length > 0 ? "ranges" : "complete",
                fromHead,
                ranges,
                scanRequired: false,
            };
        }
        for (const frame of segment.frames) {
            if (frame.valid && bytesEqual(frame.id, fromHead)) {
                const ranges =
                    frame.end < inv.cleanEnd
                        ? [{ start: frame.end, end: inv.cleanEnd }]
                        : [];
                return {
                    status: ranges.length > 0 ? "ranges" : "complete",
                    fromHead,
                    ranges,
                    scanRequired: false,
                };
            }
        }
    }
    return {
        status: "unknown",
        fromHead,
        ranges: [],
        scanRequired: true,
        detail: "unknown peer head; scan required",
    };
}

/** Return the stable gts-replication-missing-v1 JSON document. */
export function missingJson(result: MissingResult): string {
    return jsonLine({
        schema: "gts-replication-missing-v1",
        status: result.status,
        from_head: wire.hex(result.fromHead),
        ranges: result.ranges.map(rangeJson),
        scan_required: result.scanRequired,
        detail: result.detail ?? null,
    });
}

/** Return clean trailing bytes after a validated frame id. */
export function resumeAfter(data: Uint8Array, frameId: Uint8Array): Uint8Array {
    const inv = inventory(data);
    if (hasProblems(inv)) {
        throw new Error(problemDetail(inv) ?? "input is not clean");
    }
    for (const segment of inv.segments) {
        for (const frame of segment.frames) {
            if (frame.valid && bytesEqual(frame.id, frameId)) {
                return data.subarray(frame.end, inv.cleanEnd);
            }
        }
    }
    throw new Error(`frame ${wire.hex(frameId)} not found`);
}

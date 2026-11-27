// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The parity findings, shard checks and repairs of a segment (P5 section 6.6 stages 2 and 3,
//! section 6.7 and section 7.2 steps 3 to 5).

import { fieldOffsets } from "./v2_inspect.js";
import { FramingError } from "./v2_framing.js";
import { PARITY } from "./v2_frame.js";
import {
    paritySpan,
    readLists,
    readParityRecord,
    shapeOf,
    SPAN,
} from "./v2_parity.js";
import type { ParityLists } from "./v2_parity.js";
import { rejectedSpans } from "./v2_parity_runs.js";
import type { PayloadBudget } from "./v2_payload.js";
import type { WireFrame, WireItem, WireSegment } from "./v2_segment.js";
import {
    shardAt,
    shardDigest,
    solveShards,
    type SolvedShards,
} from "./v2_shards.js";

/** The code of a parity finding, in the registry of finding codes. */
export type ParityCode =
    | "exceeds_capacity"
    | "parity_damaged"
    | "parity_mismatch"
    | "parity_unexplained"
    | "parity_stale"
    | "repair_failed"
    | "repaired"
    | "unprotected_tail";

/** The reason of a parity finding, in the registry of reason codes. */
export type ParityReason =
    | "decoded_size"
    | "stored_size"
    | "total_decoded"
    | "data"
    | "parity"
    | "too_many_erasures"
    | "verify_failed";

/** A parity finding: its code, and the reason, position, byte range and count its code takes. */
export interface ParityFinding {
    code: ParityCode;
    reason?: ParityReason;
    /** The position of the item the finding concerns. */
    position?: number;
    /** The offset of the first byte the finding covers. */
    offset?: number;
    /** The number of bytes the finding covers. */
    length?: number;
    count?: bigint;
}

/** A byte patch that an accepted repair writes at an offset of the input. */
export interface Patch {
    offset: number;
    data: Uint8Array;
}

/** The repair of one used span, before its run recheck (P5 section 7.2, step 5). */
export interface Repair {
    /** The position of the parity frame of the span. */
    position: number;
    /** The number of shards solved and written. */
    count: bigint;
    /** Whether every solved shard matches its digest. */
    digestsHold: boolean;
    data: Patch[];
    parity: Patch[];
}

/** The findings and the accepted patches of the parity layer of one segment. */
export interface Verification {
    findings: ParityFinding[];
    patches: Patch[];
}

/** One item of the segment's stream and its place in it. */
interface Part {
    stream: number;
    offset: number;
    length: number;
}

/** The index of the first element of `values` for which `holds` is false; `holds` is monotone. */
function partitionPoint<T>(values: readonly T[], holds: (value: T) => boolean) {
    let low = 0;
    let high = values.length;
    while (low < high) {
        const middle = (low + high) >> 1;
        if (holds(values[middle])) low = middle + 1;
        else high = middle;
    }
    return low;
}

/**
 * The stream of a segment: its items without the transparent frames and the offsets and
 * locator items, each with its place in the stream.
 */
class Stream {
    readonly parts: Part[] = [];
    end = 0;
    /** The stream ranges of the items that do not pass (P5 6.6, passing items). */
    private readonly failed: [number, number][] = [];

    push(offset: number, length: number, passes: boolean): void {
        this.parts.push({ stream: this.end, offset, length });
        if (!passes) this.failed.push([this.end, this.end + length]);
        this.end += length;
    }

    lastStart(): number | undefined {
        return this.parts[this.parts.length - 1]?.stream;
    }

    /** Each part that overlaps the stream range `start..end`, with the overlapping range of it. */
    private overlapping(
        start: number,
        end: number,
    ): { part: Part; left: number; right: number }[] {
        const first = partitionPoint(
            this.parts,
            (part) => part.stream + part.length <= start,
        );
        const found: { part: Part; left: number; right: number }[] = [];
        for (const part of this.parts.slice(first)) {
            if (part.stream >= end) break;
            const left = Math.max(start, part.stream);
            const right = Math.min(end, part.stream + part.length);
            if (left < right) found.push({ part, left, right });
        }
        return found;
    }

    /** The stream bytes of the range `start..end`, read from the input `data`. */
    covered(data: Uint8Array, start: number, end: number): Uint8Array {
        const covered = new Uint8Array(end - start);
        let used = 0;
        for (const { part, left, right } of this.overlapping(start, end)) {
            const from = part.offset + (left - part.stream);
            const chunk = data.subarray(
                from,
                part.offset + (right - part.stream),
            );
            covered.set(chunk, used);
            used += chunk.length;
        }
        return covered.subarray(0, used);
    }

    /** Whether a failing item overlaps the stream range `start..end`. */
    overlapsFailed(start: number, end: number): boolean {
        const index = partitionPoint(
            this.failed,
            ([, failedEnd]) => failedEnd <= start,
        );
        const next = this.failed[index];
        return next !== undefined && next[0] < end;
    }

    /** The patches that write the shard bytes of the stream range `start..end` into the input. */
    patches(start: number, end: number, shard: Uint8Array): Patch[] {
        return this.overlapping(start, end).map(({ part, left, right }) => ({
            offset: part.offset + (left - part.stream),
            data: shard.slice(left - start, right - start),
        }));
    }
}

/** The span of one used parity frame, with the bytes of the stream that it covers. */
interface Span {
    first: number;
    end: number;
    size: number;
    position: number;
    item: WireItem;
    covered: Uint8Array;
    lists: ParityLists;
}

/** The failing data and parity shards of a span, and the data shards no failing item explains. */
interface ShardFailures {
    data: Set<number>;
    parity: Set<number>;
    unexplained: Set<number>;
    stale: boolean;
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

function sameSet(left: ReadonlySet<number>, right: ReadonlySet<number>) {
    return left.size === right.size && [...left].every((v) => right.has(v));
}

function ascending(values: Iterable<number>): number[] {
    return [...values].sort((left, right) => left - right);
}

/**
 * Whether an item passes its checks: a frame that is linked, and whose payload or stored check
 * holds (P5 6.6, passing items).
 */
function passes(frame: WireFrame): boolean {
    if (!("frame" in frame.envelope)) return false;
    if (frame.link.state !== "linked") return false;
    return frame.envelope.frame.storage.kind === "plain"
        ? frame.payload.state.kind === "verified"
        : frame.payload.storedMatches === true;
}

/** The `exceeds_capacity` finding of a parity frame whose payload a size limit refused. */
function exceedsCapacity(
    position: number,
    item: WireItem,
    frame: WireFrame,
): ParityFinding | undefined {
    const state = frame.payload.state;
    if (state.kind !== "unchecked") return undefined;
    const refusal = state.refusal;
    if (refusal.kind === "unknown_codec") return undefined;
    return {
        code: "exceeds_capacity",
        reason: refusal.kind,
        count: refusal.count,
        position,
        offset: item.offset,
        length: item.end - item.offset,
    };
}

class Judge {
    private readonly stream = new Stream();
    private readonly findings: ParityFinding[] = [];
    private readonly repairs: Repair[] = [];
    private protectedEnd = 0;
    private judged = false;

    constructor(
        private readonly data: Uint8Array,
        private readonly segment: WireSegment,
        private readonly openerPasses: boolean,
    ) {}

    /** Judge one item of the segment in item order. */
    item(position: number, item: WireItem, frame: WireFrame | undefined): void {
        if (frame !== undefined && frame.frameType === PARITY) {
            if (frame.judgement !== undefined) {
                this.judged = true;
                this.judgeParity(position, item, frame, frame.judgement);
            }
        }
        if (
            frame?.transparent === true ||
            item.kind === "offsets" ||
            item.kind === "locator"
        ) {
            return;
        }
        const itemPasses =
            position === 0
                ? this.openerPasses
                : frame !== undefined && passes(frame);
        this.stream.push(item.offset, item.end - item.offset, itemPasses);
    }

    private judgeParity(
        position: number,
        item: WireItem,
        frame: WireFrame,
        judgement: NonNullable<WireFrame["judgement"]>,
    ): void {
        switch (judgement) {
            case "damaged":
                this.findings.push({ code: "parity_damaged", position });
                break;
            case "exceeds_capacity": {
                const finding = exceedsCapacity(position, item, frame);
                if (finding !== undefined) this.findings.push(finding);
                break;
            }
            case "unchecked":
                break;
            case "damaged_used":
                this.findings.push({ code: "parity_damaged", position });
                this.useSpan(position, item, frame);
                break;
            case "verified":
                this.useSpan(position, item, frame);
                break;
        }
    }

    /** Check the placement of a used span and, unless it is stale, its shards and its repair. */
    private useSpan(position: number, item: WireItem, frame: WireFrame): void {
        if (!("frame" in frame.envelope)) return;
        const envelope = frame.envelope.frame;
        const record = readParityRecord(envelope.content);
        if (record === undefined) return;
        const length = paritySpan(record);
        if (
            length === undefined ||
            !Number.isSafeInteger(Number(record.first)) ||
            !Number.isSafeInteger(Number(record.end))
        ) {
            return;
        }
        const first = Number(record.first);
        const end = Number(record.end);
        const lastStart = this.stream.lastStart();
        const within =
            lastStart !== undefined &&
            lastStart < end &&
            end <= this.stream.end;
        if (!within || (BigInt(length) < SPAN && end !== this.stream.end)) {
            this.findings.push({ code: "parity_stale", position });
            return;
        }
        const shape = shapeOf(length);
        const lists = readLists(envelope.storedPayload, shape);
        if (lists === undefined) return;
        const span: Span = {
            first,
            end,
            size: shape.shard,
            position,
            item,
            covered: this.stream.covered(this.data, first, end),
            lists,
        };
        const failures = this.shardFailures(span);
        if (failures.stale) return;
        this.protectedEnd = Math.max(this.protectedEnd, end);
        if (
            !sameSet(failures.data, failures.unexplained) ||
            failures.parity.size > 0
        ) {
            const repair = this.repair(span, failures);
            if (repair !== undefined) {
                this.repairs.push(repair);
            } else {
                this.findings.push({
                    code: "repair_failed",
                    reason: "too_many_erasures",
                    position,
                });
            }
        }
    }

    /**
     * Compare the shard digests of a span with its table and record the failures (P5 6.6,
     * stage 3, and the explained and unexplained shards of P5 7.2).
     */
    private shardFailures(span: Span): ShardFailures {
        const data = new Set<number>();
        const unexplained = new Set<number>();
        span.lists.dataDigests.forEach((digest, index) => {
            if (
                equal(
                    shardDigest(shardAt(span.covered, index, span.size)),
                    digest,
                )
            ) {
                return;
            }
            data.add(index);
            const start = span.first + index * span.size;
            if (!this.stream.overlapsFailed(start, start + span.size)) {
                unexplained.add(index);
            }
        });
        const parity = new Set<number>();
        span.lists.parityDigests.forEach((digest, row) => {
            const shard = shardAt(span.lists.parity, row, span.size);
            if (!equal(shardDigest(shard), digest)) parity.add(row);
        });
        const position = span.position;
        if (data.size > 0) {
            this.findings.push({
                code: "parity_mismatch",
                reason: "data",
                count: BigInt(data.size),
                position,
            });
        }
        if (parity.size > 0) {
            this.findings.push({
                code: "parity_mismatch",
                reason: "parity",
                count: BigInt(parity.size),
                position,
            });
        }
        const stale = data.size > 0 && sameSet(data, unexplained);
        if (stale) {
            this.findings.push({ code: "parity_stale", position });
        } else if (unexplained.size > 0) {
            this.findings.push({
                code: "parity_unexplained",
                count: BigInt(unexplained.size),
                position,
            });
        }
        return { data, parity, unexplained, stale };
    }

    /**
     * Solve the erased shards of a span and build its repair (P5 7.2, step 5, substep 1).
     * Absent when a stripe holds more unknowns than its parity rows.
     */
    private repair(span: Span, failures: ShardFailures): Repair | undefined {
        const solved = solveShards(
            span.covered,
            span.lists.parity,
            span.size,
            failures.data,
            failures.parity,
        );
        if (solved === undefined) return undefined;
        let digestsHold = true;
        const data: Patch[] = [];
        for (const index of ascending(solved.data.keys())) {
            const shard = solved.data.get(index) as Uint8Array;
            const digest = span.lists.dataDigests[index];
            digestsHold &&=
                digest !== undefined && equal(shardDigest(shard), digest);
            if (failures.unexplained.has(index)) continue;
            const start = span.first + index * span.size;
            const end = Math.min(start + span.size, span.end);
            data.push(...this.stream.patches(start, end, shard));
        }
        const parity = this.parityPatches(span, solved);
        if (parity === undefined) return undefined;
        digestsHold &&= parity.holds;
        const written =
            Math.max(0, solved.data.size - failures.unexplained.size) +
            solved.parity.size;
        return {
            position: span.position,
            count: BigInt(written),
            digestsHold,
            data,
            parity: parity.patches,
        };
    }

    /** The patches of the solved parity shards, and whether each matches its digest. */
    private parityPatches(
        span: Span,
        solved: SolvedShards,
    ): { holds: boolean; patches: Patch[] } | undefined {
        let offsets: readonly number[];
        try {
            offsets = fieldOffsets(
                this.data.subarray(0, span.item.end),
                span.item.offset,
            );
        } catch (error) {
            if (error instanceof FramingError) return undefined;
            throw error;
        }
        const parityStart = offsets[8] - span.lists.parity.length;
        if (parityStart < 0) return undefined;
        let holds = true;
        const patches: Patch[] = [];
        for (const row of ascending(solved.parity.keys())) {
            const shard = solved.parity.get(row) as Uint8Array;
            const digest = span.lists.parityDigests[row];
            holds &&= digest !== undefined && equal(shardDigest(shard), digest);
            patches.push({
                offset: parityStart + row * span.size,
                data: shard,
            });
        }
        return { holds, patches };
    }

    finish(
        budget: PayloadBudget,
        registry: bigint,
        opener: Uint8Array,
    ): Verification {
        const patches: Patch[] = [];
        if (this.repairs.length > 0) {
            const rejected = rejectedSpans(
                this.data,
                this.segment,
                this.repairs,
                budget,
                registry,
                opener,
            );
            for (const repair of this.repairs) {
                if (rejected.has(repair.position)) {
                    this.findings.push({
                        code: "repair_failed",
                        reason: "verify_failed",
                        position: repair.position,
                    });
                } else {
                    this.findings.push({
                        code: "repaired",
                        count: repair.count,
                        position: repair.position,
                    });
                    patches.push(...repair.data, ...repair.parity);
                }
            }
        }
        this.unprotectedTail();
        return { findings: this.findings, patches };
    }

    /** The bytes after the largest protected end, when a frame of the segment was judged (P5 6.7). */
    private unprotectedTail(): void {
        const openerLength = this.stream.parts[0]?.length ?? 0;
        const unprotected = Math.max(this.protectedEnd, openerLength);
        const end = this.stream.end;
        if (!this.judged || unprotected >= end) return;
        const index = Math.max(
            0,
            partitionPoint(
                this.stream.parts,
                (part) => part.stream <= unprotected,
            ) - 1,
        );
        const part = this.stream.parts[index];
        if (part !== undefined) {
            this.findings.push({
                code: "unprotected_tail",
                offset: part.offset + unprotected - part.stream,
                length: end - unprotected,
            });
        }
    }
}

/**
 * Judge the parity frames of `segment`, check and solve the spans they protect, and return the
 * findings in input order with the patches of the accepted repairs (P5 sections 6.6, 6.7, 7.2).
 * `registry` and `opener` are the registry version and the identifier of the segment's opener.
 */
export function verifyParity(
    data: Uint8Array,
    segment: WireSegment,
    budget: PayloadBudget,
    registry: bigint,
    opener: Uint8Array,
): Verification {
    const byPosition: (WireFrame | undefined)[] = new Array(
        segment.items.length,
    ).fill(undefined);
    for (const frame of segment.frames) {
        if (frame.position < byPosition.length) {
            byPosition[frame.position] = frame;
        }
    }
    const firstOrdinary = segment.frames.find((frame) => !frame.transparent);
    const judge = new Judge(
        data,
        segment,
        firstOrdinary !== undefined &&
            (firstOrdinary.link.state === "linked" ||
                firstOrdinary.link.state === "healed"),
    );
    segment.items.forEach((item, position) => {
        judge.item(position, item, byPosition[position]);
    });
    return judge.finish(budget, registry, opener);
}

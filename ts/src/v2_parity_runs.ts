// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The run recheck of repaired spans (P5 section 7.2, step 5, substeps 2 to 4).
//!
//! The repaired bytes are written into a copy of the segment, which is delimited again in runs.
//! Each run is walked once and either checks or fails. A span that touches a failed run, or that
//! depends on a run that a removed span touches, is removed.

import { decodeFrame, FrameError, type Frame } from "./v2_frame.js";
import { FramingError, itemEnd } from "./v2_framing.js";
import { frameIdentifier } from "./v2_integrity.js";
import { storedMatches, type PayloadBudget } from "./v2_payload.js";
import type { Patch, Repair } from "./v2_parity_repair.js";
import type { WireSegment } from "./v2_segment.js";

/** The envelope bytes that begin every parity frame (P5 section 6.5). */
const PARITY_PREFIX = [0x8b, 0x02];

interface Chain {
    predecessor: Uint8Array;
    predecessorRun: number | undefined;
    failedSincePredecessor: boolean;
}

interface RunOutcome {
    examinedEnd: number;
    holds: boolean;
    lastOrdinary: Uint8Array | undefined;
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

/** The index of the first element of the ascending `values` that is not below `bound`. */
function partitionPoint(values: readonly number[], below: number): number {
    let low = 0;
    let high = values.length;
    while (low < high) {
        const middle = (low + high) >> 1;
        if (values[middle] < below) low = middle + 1;
        else high = middle;
    }
    return low;
}

/**
 * The payload recheck of a walked frame: a matching stored check, and a verified payload or, for
 * a compressed frame, an unchecked one (P5 section 7.2, step 5, substep 3).
 */
function recheck(budget: PayloadBudget, frame: Frame): boolean {
    const stored = storedMatches(frame.storage, [frame.storedPayload]);
    if (frame.storage.kind !== "plain" && stored !== true) return false;
    const { state } = budget.check(frame, stored);
    return (
        state.kind === "verified" ||
        (frame.storage.kind === "compressed" && state.kind === "unchecked")
    );
}

function apply(working: Uint8Array, patch: Patch, first: number): void {
    working.set(patch.data, patch.offset - first);
}

/** Apply the patches of every repair to the copy, and map each data patch to the item it lies in. */
function touchedItems(
    working: Uint8Array,
    repairs: readonly Repair[],
    starts: readonly number[],
    first: number,
): Map<number, Set<number>> {
    const touched = new Map<number, Set<number>>();
    for (const repair of repairs) {
        for (const patch of repair.data) {
            apply(working, patch, first);
            const item = Math.max(
                0,
                partitionPoint(starts, patch.offset - first + 1) - 1,
            );
            let spans = touched.get(item);
            if (spans === undefined) {
                spans = new Set();
                touched.set(item, spans);
            }
            spans.add(repair.position);
        }
        for (const patch of repair.parity) apply(working, patch, first);
    }
    return touched;
}

class Walk {
    constructor(
        private readonly source: Uint8Array,
        private readonly working: Uint8Array,
        private readonly boundaries: ReadonlySet<number>,
        private readonly openerEnd: number,
        private readonly registry: bigint,
        private readonly budget: PayloadBudget,
    ) {}

    /** Walk the items of the copy from `start` to the next input boundary (P5 7.2, step 5). */
    run(start: number, state: Chain): RunOutcome {
        let cursor = start;
        let predecessor = state.predecessor;
        let lastOrdinary: Uint8Array | undefined;
        for (;;) {
            let end: number;
            try {
                end = itemEnd(this.working, cursor);
            } catch (error) {
                if (!(error instanceof FramingError)) throw error;
                return {
                    examinedEnd: error.examinedEnd,
                    holds: false,
                    lastOrdinary: undefined,
                };
            }
            const checked = this.itemChecks(cursor, end, predecessor);
            if (checked === undefined) {
                return {
                    examinedEnd: end,
                    holds: false,
                    lastOrdinary: undefined,
                };
            }
            if (checked.ordinary) {
                predecessor = checked.predecessor;
                lastOrdinary = predecessor;
            }
            if (this.boundaries.has(end)) {
                return { examinedEnd: end, holds: true, lastOrdinary };
            }
            cursor = end;
        }
    }

    /**
     * Check one walked item: the opener by its bytes, a frame by its envelope, its identifier and
     * its payload (P5 7.2, step 5, substep 3). Absent when the item fails.
     */
    private itemChecks(
        cursor: number,
        end: number,
        predecessor: Uint8Array,
    ): { ordinary: boolean; predecessor: Uint8Array } | undefined {
        if (cursor === 0) {
            const passes =
                end === this.openerEnd &&
                equal(
                    this.working.subarray(0, end),
                    this.source.subarray(0, end),
                );
            return passes ? { ordinary: false, predecessor } : undefined;
        }
        if (
            this.working[cursor] !== PARITY_PREFIX[0] ||
            this.working[cursor + 1] !== PARITY_PREFIX[1]
        ) {
            return undefined;
        }
        let frame: Frame;
        try {
            frame = decodeFrame(this.working, cursor, this.registry);
        } catch (error) {
            if (error instanceof FrameError) return undefined;
            throw error;
        }
        if (
            !equal(
                frameIdentifier(predecessor, frame.identityFields),
                frame.identifier,
            ) ||
            !recheck(this.budget, frame)
        ) {
            return undefined;
        }
        if (frame.transparent) return { ordinary: false, predecessor };
        return { ordinary: true, predecessor: frame.identifier };
    }
}

/** Remove every span that depends on a run that a removed span touches, until none is left. */
function propagate(
    rejected: Set<number>,
    runsBySpan: ReadonlyMap<number, ReadonlySet<number>>,
    dependents: Map<number, Set<number>>,
): void {
    const pending = [...rejected];
    for (let index = 0; index < pending.length; index++) {
        for (const run of runsBySpan.get(pending[index]) ?? []) {
            const spans = dependents.get(run);
            if (spans === undefined) continue;
            dependents.delete(run);
            for (const dependent of spans) {
                if (!rejected.has(dependent)) {
                    rejected.add(dependent);
                    pending.push(dependent);
                }
            }
        }
    }
}

function addAll(target: Set<number> | undefined, values: Iterable<number>) {
    for (const value of values) target?.add(value);
}

/**
 * The positions of the spans that the run recheck rejects.
 *
 * The candidate copy applies the data and parity patches of every repair, then checks each run.
 * A repair whose digests do not hold is rejected at once. `registry` and `opener` are the
 * registry version and the identifier of the segment's opener.
 */
export function rejectedSpans(
    data: Uint8Array,
    segment: WireSegment,
    repairs: readonly Repair[],
    budget: PayloadBudget,
    registry: bigint,
    opener: Uint8Array,
): Set<number> {
    const rejected = new Set(
        repairs
            .filter((repair) => !repair.digestsHold)
            .map((repair) => repair.position),
    );
    const firstItem = segment.items[0];
    const lastItem = segment.items[segment.items.length - 1];
    if (firstItem === undefined || lastItem === undefined) return rejected;
    if (repairs.every((repair) => repair.data.length === 0)) return rejected;
    const first = firstItem.offset;
    const working = data.slice(first, lastItem.end);
    const starts = segment.items.map((item) => item.offset - first);
    const touched = touchedItems(working, repairs, starts, first);
    const boundaries = new Set([...starts, working.length]);
    const originalHeads = new Map<number, Uint8Array>();
    for (const frame of segment.frames) {
        if (frame.link.effectiveIdentifier !== undefined) {
            originalHeads.set(frame.position, frame.link.effectiveIdentifier);
        }
    }
    const walk = new Walk(
        data.subarray(first, lastItem.end),
        working,
        boundaries,
        firstItem.end - first,
        registry,
        budget,
    );
    const state: Chain = {
        predecessor: opener,
        predecessorRun: undefined,
        failedSincePredecessor: false,
    };
    const dependents = new Map<number, Set<number>>();
    const runsBySpan = new Map<number, Set<number>>();
    let runCount = 0;
    let coveredUntil = 0;
    for (const [index, cursor] of starts.entries()) {
        if (index < coveredUntil) continue;
        if (!touched.has(index)) {
            const identifier = originalHeads.get(index);
            if (identifier !== undefined) {
                state.predecessor = identifier;
                state.predecessorRun = undefined;
                state.failedSincePredecessor = false;
            }
            continue;
        }
        const run = walk.run(cursor, state);
        coveredUntil =
            index +
            1 +
            partitionPoint(starts.slice(index + 1), run.examinedEnd);
        const participants = new Set<number>();
        for (let position = index; position < coveredUntil; position++) {
            addAll(participants, touched.get(position) ?? []);
        }
        for (const span of participants) {
            let runs = runsBySpan.get(span);
            if (runs === undefined) {
                runs = new Set();
                runsBySpan.set(span, runs);
            }
            runs.add(runCount);
        }
        const own = dependents.get(runCount) ?? new Set<number>();
        dependents.set(runCount, own);
        addAll(own, participants);
        if (state.predecessorRun !== undefined) {
            const previous =
                dependents.get(state.predecessorRun) ?? new Set<number>();
            dependents.set(state.predecessorRun, previous);
            addAll(previous, participants);
        }
        if (!run.holds || state.failedSincePredecessor) {
            addAll(rejected, participants);
        }
        if (!run.holds) {
            state.failedSincePredecessor = true;
        } else if (run.lastOrdinary !== undefined) {
            state.predecessor = run.lastOrdinary;
            state.predecessorRun = runCount;
            state.failedSincePredecessor = false;
        }
        runCount += 1;
    }
    propagate(rejected, runsBySpan, dependents);
    return rejected;
}

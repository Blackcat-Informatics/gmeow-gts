// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The claims that commit and index frames make about the frames before them (P5 sections 4.1
//! and 4.3), checked against the prefix of a segment as the reader observed it. A commit claims
//! the count and the range root of the ordinary frames it follows; an index claims those, the
//! head they end at, and the runs of frame types.

import { HeaderError } from "./v2_decoder.js";
import { COMMIT, INDEX, type Frame } from "./v2_frame.js";
import {
    decodeClaim,
    readIndexPayload,
    type IndexRegistry,
    type TypeRun,
} from "./v2_index.js";
import type { FrameLink } from "./v2_link.js";
import type { PayloadState } from "./v2_payload.js";
import { V2Range } from "./v2_range.js";

/** What the prefix check reads of an observed frame. */
export interface PrefixFrame {
    /** The position of the frame's item in its segment. */
    position: number;
    frameType: bigint | undefined;
    /** The decoded frame, or the first malformed field. */
    envelope:
        { frame: Pick<Frame, "frameType" | "content"> } | { error: unknown };
    link: Pick<FrameLink, "state" | "effectiveIdentifier">;
    payload: { state: PayloadState };
    /** Whether the frame is of a type a later registry introduces and the fold stops at. */
    unknownTypeStopsFold: boolean;
}

/** The claims a commit makes about its observed prefix. */
export interface CommitCheck {
    /** The position of the commit frame that was checked. */
    position: number;
    /** Number of preceding ordinary frames with effective identifiers. */
    ordinaryPosition: bigint;
    /** Whether the commit's frame count matches the frames it covers. */
    countMatches: boolean;
    /** Whether the commit's root matches the frames it covers. */
    rootMatches: boolean;
    /** Whether every claim of the commit holds. */
    valid: boolean;
}

/** Ordinary frames after the last valid commit, excluding index frames. */
export interface UncommittedTail {
    /** The position where the uncommitted tail begins. */
    position: number;
    /** The number of frames in the uncommitted tail. */
    count: bigint;
}

/** The commit checks of a segment and the frames they leave uncommitted. */
export interface CommitReport {
    checks: CommitCheck[];
    /** The position of the last commit that holds, if any. */
    lastValidPosition: number | undefined;
    /** The number of frames covered by a commit that holds. */
    committedFrames: bigint;
    uncommittedTail: UncommittedTail | undefined;
}

/** The claims that an index frame makes about the frames it covers. */
export interface IndexWireClaims {
    /** Both the frame count and the claimed head match the prefix. */
    headMatches: boolean;
    /** Whether the root of the index matches the frames it covers. */
    rootMatches: boolean;
    /** Whether the type runs of the index match the frames they cover. */
    typeRunsMatch: boolean;
}

/** Wire-level index results. Fold-dependent claims require a content reader. */
export type IndexOutcome =
    | { kind: "checked"; claims: IndexWireClaims }
    /** The error offset is relative to the decoded payload. */
    | { kind: "malformed_payload"; error: HeaderError }
    /** A later registry value in the payload at the given decoded offset. */
    | { kind: "unknown_enumeration"; offset: number };

/** The outcome of checking one index frame. */
export interface IndexCheck {
    position: number;
    outcome: IndexOutcome;
}

/** The content of a commit or index frame that cannot be read. */
export interface CommitReadError {
    /** The position of the frame whose claim is malformed. */
    position: number;
    error: HeaderError;
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

function commitContent(
    frame: PrefixFrame,
    storedLimit: bigint,
): Uint8Array | undefined {
    if (!("frame" in frame.envelope)) return undefined;
    const decoded = frame.envelope.frame;
    return decoded.frameType === COMMIT &&
        BigInt(decoded.content.length) <= storedLimit
        ? decoded.content
        : undefined;
}

function indexBytes(
    frame: PrefixFrame,
): { content: Uint8Array; payload: Uint8Array } | undefined {
    if (!("frame" in frame.envelope)) return undefined;
    const decoded = frame.envelope.frame;
    if (decoded.frameType !== INDEX || frame.link.state === "broken") {
        return undefined;
    }
    const state = frame.payload.state;
    return state.kind === "verified"
        ? { content: decoded.content, payload: state.decoded }
        : undefined;
}

function lastWhere(
    frames: readonly PrefixFrame[],
    holds: (frame: PrefixFrame) => boolean,
): number | undefined {
    for (let index = frames.length - 1; index >= 0; index--) {
        if (holds(frames[index])) return index;
    }
    return undefined;
}

/**
 * Checks each commit and index frame of `frames` against the prefix before it. `opener` is
 * the identifier of the segment's opener, `storedLimit` the stored bytes one frame may hold,
 * and `registry` the registry version of the segment.
 *
 * @returns the commit report and the index checks, or the first claim that cannot be read.
 */
export function checkPrefixes(
    opener: Uint8Array,
    frames: readonly PrefixFrame[],
    storedLimit: bigint,
    registry: bigint,
):
    | { ok: true; report: CommitReport; indexes: IndexCheck[] }
    | { ok: false; error: CommitReadError } {
    const report: CommitReport = {
        checks: [],
        lastValidPosition: undefined,
        committedFrames: 0n,
        uncommittedTail: undefined,
    };
    const indexes: IndexCheck[] = [];
    const lastCommit = lastWhere(
        frames,
        (frame) => commitContent(frame, storedLimit) !== undefined,
    );
    const lastIndex = lastWhere(
        frames,
        (frame) => indexBytes(frame) !== undefined,
    );
    // The same range covers both kinds of claim. Frames after them need no leaf.
    const lastClaim =
        lastCommit === undefined || lastIndex === undefined
            ? (lastCommit ?? lastIndex)
            : Math.max(lastCommit, lastIndex);
    const indexRegistry: IndexRegistry = {
        newer: registry > 1n,
        stopped: false,
    };
    let head = opener;
    const runs: TypeRun[] = [];
    const range = new V2Range();
    range.push(opener);
    for (const [index, frame] of frames.entries()) {
        const position = frame.position;
        const content = commitContent(frame, storedLimit);
        const ordinary = range.leafCount - 1n;
        if (content !== undefined) {
            let claim;
            try {
                claim = decodeClaim(content);
            } catch (error) {
                if (!(error instanceof HeaderError)) throw error;
                return { ok: false, error: { position, error } };
            }
            const countMatches = claim.count === ordinary;
            const rootMatches = equal(claim.root, range.root());
            const valid =
                countMatches && rootMatches && frame.link.state !== "broken";
            report.checks.push({
                position,
                ordinaryPosition: ordinary,
                countMatches,
                rootMatches,
                valid,
            });
            if (valid) {
                report.lastValidPosition = position;
                report.committedFrames = ordinary + 1n;
                report.uncommittedTail = undefined;
            }
        }
        if (lastIndex !== undefined) {
            const bytes = indexBytes(frame);
            if (bytes !== undefined) {
                let claim;
                try {
                    claim = decodeClaim(bytes.content);
                } catch (error) {
                    if (!(error instanceof HeaderError)) throw error;
                    return { ok: false, error: { position, error } };
                }
                const read = readIndexPayload(
                    bytes.payload,
                    indexRegistry,
                    runs,
                );
                let outcome: IndexOutcome;
                if (read.ok) {
                    outcome = {
                        kind: "checked",
                        claims: {
                            headMatches:
                                claim.count === ordinary &&
                                // The second field of an index claim is the head it ends at.
                                equal(claim.root, head),
                            rootMatches: equal(read.root, range.root()),
                            typeRunsMatch: read.runsMatch,
                        },
                    };
                } else if (read.kind === "malformed") {
                    outcome = { kind: "malformed_payload", error: read.error };
                } else {
                    outcome = {
                        kind: "unknown_enumeration",
                        offset: read.offset,
                    };
                }
                indexes.push({ position, outcome });
            }
            if (index < lastIndex) {
                indexRegistry.stopped ||= frame.unknownTypeStopsFold;
                if (frame.link.effectiveIdentifier !== undefined) {
                    head = frame.link.effectiveIdentifier;
                    const last = runs.at(-1);
                    if (
                        last !== undefined &&
                        last.frameType === frame.frameType
                    ) {
                        last.length += 1n;
                    } else {
                        runs.push({ frameType: frame.frameType, length: 1n });
                    }
                }
            }
        }
        if (frame.link.effectiveIdentifier !== undefined) {
            if (lastClaim !== undefined && index < lastClaim) {
                range.push(frame.link.effectiveIdentifier);
            }
            if (
                frame.frameType !== INDEX &&
                report.lastValidPosition !== position
            ) {
                report.uncommittedTail ??= { position, count: 0n };
                report.uncommittedTail.count += 1n;
            }
        }
    }
    return { ok: true, report, indexes };
}

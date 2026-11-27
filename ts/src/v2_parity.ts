// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The stage 1 judgement of an in-order parity frame (P5 section 6.6): the content record of the
//! frame, the shape of the span it protects, the table digest of its parity payload, and the
//! earlier parity frames that condition 6 compares it with. Repair and the parity scan of a
//! damaged region are separate modules.

import { Decoder, HeaderError } from "./v2_decoder.js";
import { FramingError, head } from "./v2_framing.js";

type Head = ReturnType<typeof head>;
import { decodeParityCandidate, FrameError, type Frame } from "./v2_frame.js";
import {
    contextHash,
    contextHashParts,
    frameIdentifier,
} from "./v2_integrity.js";
import type { LinkState } from "./v2_link.js";
import type { PayloadBudget, PayloadState } from "./v2_payload.js";

/** The largest span that one parity frame protects, in bytes (P5 section 6.6). */
export const SPAN = 1n << 22n;
const FULL_SHARD = 4096;
const MIN_SHARD = 64;
const COLUMNS = 64;
const ROWS = 4;
/** The encoded length of one digest in a parity payload (a head and 16 bytes). */
const DIGEST = 17;
/** The bytes of one digest. */
const DIGEST_BYTES = 16;

/** The content record of a parity frame (P5 section 6.6). */
export interface ParityRecord {
    geometry: bigint;
    first: bigint;
    end: bigint;
    anchor: Uint8Array;
    tableDigest: Uint8Array;
}

/** The content record of a parity frame, or absent when it is not well formed. */
export function readParityRecord(
    content: Uint8Array,
): ParityRecord | undefined {
    try {
        const cursor = new Decoder(content, 0);
        cursor.record(5);
        return {
            geometry: cursor.uint(),
            first: cursor.uint(),
            end: cursor.uint(),
            anchor: cursor.byteString(32),
            tableDigest: cursor.byteString(32),
        };
    } catch (error) {
        if (error instanceof HeaderError) return undefined;
        throw error;
    }
}

/**
 * The length of the span that a record protects, when the record is within condition 1 of P5
 * section 6.6: a geometry of 1, a first offset that is a multiple of the span size, and a length
 * from 1 to the span size.
 */
function spanOf(record: ParityRecord): number | undefined {
    if (record.end < record.first) return undefined;
    const length = record.end - record.first;
    return record.geometry === 1n &&
        record.first % SPAN === 0n &&
        length >= 1n &&
        length <= SPAN
        ? Number(length)
        : undefined;
}

function nextPowerOfTwo(value: number): number {
    let power = 1;
    while (power < value) power *= 2;
    return power;
}

function shardLength(span: number): number {
    if (span >= COLUMNS * FULL_SHARD) return FULL_SHARD;
    return Math.max(MIN_SHARD, nextPowerOfTwo(Math.ceil(span / COLUMNS)));
}

/** The size of the head of a definite-length item whose argument is `count`. */
function headSize(count: number): number {
    if (count <= 23) return 1;
    if (count <= 0xff) return 2;
    if (count <= 0xffff) return 3;
    if (count <= 0xffffffff) return 5;
    return 9;
}

/** The shape of the parity payload of a span of `span` bytes (P5 section 6.4). */
export interface ParityShape {
    /** The shard length of the span. */
    shard: number;
    /** The data shard count. */
    shards: number;
    /** The parity shard count, `ROWS` for each block of `COLUMNS` data shards. */
    parityShards: number;
    /** The parity byte count. */
    parityBytes: number;
    /** The length of the deterministic encoding of the parity payload. */
    encodedLength: number;
}

/** The shape of the parity payload of a span of `span` bytes. */
export function shapeOf(span: number): ParityShape {
    const shard = shardLength(span);
    const shards = Math.ceil(span / shard);
    const parityShards = ROWS * Math.ceil(shards / COLUMNS);
    const parityBytes = parityShards * shard;
    return {
        shard,
        shards,
        parityShards,
        parityBytes,
        encodedLength:
            1 +
            headSize(shards) +
            DIGEST * shards +
            headSize(parityShards) +
            DIGEST * parityShards +
            headSize(parityBytes) +
            parityBytes,
    };
}

/**
 * Whether `payload` is one deterministic parity payload of `shape` whose table digest under
 * the segment's opener identifier `h0` is `tableDigest` (P5 section 6.6, conditions 4 and 5).
 */
function tableMatches(
    payload: Uint8Array,
    shape: ParityShape,
    h0: Uint8Array,
    tableDigest: Uint8Array,
): boolean {
    let end: number;
    try {
        const cursor = new Decoder(payload, 0);
        cursor.record(3);
        for (const count of [shape.shards, shape.parityShards]) {
            if (cursor.head(4) !== BigInt(count)) return false;
            for (let index = 0; index < count; index++) {
                cursor.byteString(DIGEST_BYTES);
            }
        }
        end = cursor.offset;
        cursor.byteString(shape.parityBytes);
        if (cursor.offset !== payload.length) return false;
    } catch (error) {
        if (error instanceof HeaderError) return false;
        throw error;
    }
    const digest = contextHashParts("gts2 parity table", [
        Uint8Array.of(0x83, 0x58, 0x20),
        h0,
        payload.subarray(1, end),
    ]);
    return equal(digest, tableDigest);
}

function equal(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

/**
 * What condition 6 of P5 section 6.6 keeps of the earlier parity frames of a segment: the
 * first offset of each, and whether any has a short span. A candidate is compared with this
 * record, so the test costs the same however many frames stand before it.
 */
export class EarlierParity {
    readonly #firsts = new Set<bigint>();
    #shortSeen = false;

    /**
     * Records the parity frame whose content record is `content`. A frame of a later registry
     * version whose envelope is as the type states (`clean`) and whose geometry is above 1 is a
     * frame of unknown type, which condition 6 does not see.
     */
    remember(content: Uint8Array, newer: boolean, clean: boolean): void {
        const record = readParityRecord(content);
        if (record === undefined) return;
        if (newer && clean && record.geometry > 1n) return;
        this.#firsts.add(record.first);
        if (record.end >= record.first) {
            const length = record.end - record.first;
            if (length >= 1n && length < SPAN) this.#shortSeen = true;
        }
    }

    /** Whether a candidate whose span starts at `first` passes condition 6. */
    admits(first: bigint): boolean {
        return !this.#shortSeen && !this.#firsts.has(first);
    }
}

/** The stage 1 outcome of an in-order parity frame (P5 section 6.6). */
export type ParityJudgement =
    /** Condition 1, 2, 3, 6 or 7 holds, or the payload failed and the table does not hold. */
    | "damaged"
    /** The payload failed, but the table holds: the frame is used for repair. */
    | "damaged_used"
    /** The frame is damaged by none of those conditions, and its payload exceeds the capacity. */
    | "exceeds_capacity"
    /** The frame is damaged by none of those conditions, and its codec is not implemented. */
    | "unchecked"
    /** The frame passes stage 1 and its payload is verified. */
    | "verified";

/** The inputs of the stage 1 judgement that come from the frame and its segment. */
export interface ParityInput {
    /** The envelope as decoded, or absent when it does not decode. */
    envelope: Frame | undefined;
    link: LinkState;
    /** The identifier that the frame is linked to. */
    predecessor: Uint8Array;
    payload: PayloadState;
    /** The identifier of the segment's opener. */
    h0: Uint8Array;
    earlier: EarlierParity;
}

/**
 * Judges an in-order frame of type parity by stage 1 of P5 section 6.6, in item order. The
 * result is absent for a frame of a later registry version with a clean envelope and a geometry
 * above 1, which the reader hands to its unknown-type handling.
 */
export function judgeParity(
    input: ParityInput,
    newer: boolean,
): ParityJudgement | undefined {
    const frame = input.envelope;
    if (frame === undefined) return "damaged";
    const clean =
        !frame.critical &&
        frame.transparent &&
        frame.storage.kind === "plain" &&
        frame.signatures.length === 0;
    const record = readParityRecord(frame.content);
    if (record === undefined) return "damaged";
    if (newer && clean && record.geometry > 1n) return undefined;
    const span = spanOf(record);
    if (span === undefined) return "damaged";
    if (
        !clean ||
        input.link !== "linked" ||
        !equal(record.anchor, input.predecessor) ||
        !input.earlier.admits(record.first)
    ) {
        return "damaged";
    }
    const shape = shapeOf(span);
    const tableHolds = () =>
        BigInt(frame.storedPayload.length) === BigInt(shape.encodedLength) &&
        tableMatches(frame.storedPayload, shape, input.h0, record.tableDigest);
    const { payload } = input;
    if (payload.kind === "damaged") {
        return tableHolds() ? "damaged_used" : "damaged";
    }
    if (payload.kind === "verified") {
        return frame.decodedLength === BigInt(shape.encodedLength) &&
            tableHolds()
            ? "verified"
            : "damaged";
    }
    return payload.refusal.kind === "unknown_codec"
        ? "unchecked"
        : "exceeds_capacity";
}

function readHead(data: Uint8Array, offset: number): Head | undefined {
    try {
        return head(data, offset);
    } catch (error) {
        if (error instanceof FramingError) return undefined;
        throw error;
    }
}

/** Whether the array at `offset` is `[1]` in any head form of its length and its element. */
function isPlainStorage(data: Uint8Array, offset: number): boolean {
    const array = readHead(data, offset);
    const tag = array === undefined ? undefined : readHead(data, array.end);
    if (array === undefined || tag === undefined) return false;
    const closed = array.indefinite
        ? data[tag.end] === 0xff
        : array.value === 1n;
    return (
        array.major === 4 &&
        closed &&
        tag.major === 0 &&
        !tag.indefinite &&
        tag.value === 1n
    );
}

/** Whether the array at `offset` has no element in any head form of its length. */
function isEmptyArray(data: Uint8Array, offset: number): boolean {
    const array = readHead(data, offset);
    if (array === undefined) return false;
    return (
        array.major === 4 &&
        (array.indefinite ? data[array.end] === 0xff : array.value === 0n)
    );
}

/**
 * Whether the raw envelope of a parity frame is as the type states (P5 section 6.5): `critical`
 * false, `transparent` true, plain storage and an empty `signatures` list. It reads these fields
 * from the item's bytes, so it holds whatever else of the envelope fails.
 */
export function statesParityEnvelope(
    data: Uint8Array,
    offsets: readonly number[],
): boolean {
    return (
        data[offsets[1]] === 0xf4 &&
        data[offsets[2]] === 0xf5 &&
        isPlainStorage(data, offsets[6]) &&
        isEmptyArray(data, offsets[9])
    );
}

/** The bytes that begin every parity frame: the array of eleven, the item kind, the frame type, `critical` false and `transparent` true. */
const PREFIX = Uint8Array.of(0x8b, 0x02, 0x10, 0xf4, 0xf5);

/** The first offset at or after `from` where the bytes of a parity envelope begin (P5 section 7.2). */
export function nextPrefix(data: Uint8Array, from: number): number | undefined {
    for (let at = from; at + PREFIX.length <= data.length; at++) {
        if (PREFIX.every((byte, index) => data[at + index] === byte)) return at;
    }
    return undefined;
}

/** The inputs of a candidate test that come from the segment being read. */
export interface ParityContext {
    /** The identifier of the segment's opener. */
    h0: Uint8Array;
    /** The registry version of the segment. */
    registry: bigint;
    earlier: EarlierParity;
}

/** The outcome of the test of one parity candidate (P5 section 7.2, step 6). */
export type ParityVerdict =
    /** The candidate passes every part of the test; reading resumes at it. */
    | { kind: "passed"; anchor: Uint8Array }
    /** The candidate fails a part of the test, or the total refused it without a finding. */
    | { kind: "failed" }
    /** The charge of the candidate is the first that the total decoded length refuses. */
    | { kind: "refused"; length: number; count: bigint };

/** A parity candidate that the scan of a lost item found, or refused. */
export type ParityCandidate =
    | { kind: "parity"; offset: number; anchor: Uint8Array }
    | { kind: "parity_refused"; offset: number; length: number; count: bigint };

function checkDecodedPayload(frame: Frame): boolean {
    return (
        BigInt(frame.storedPayload.length) === frame.decodedLength &&
        equal(
            contextHash("gts2 payload", frame.storedPayload),
            frame.payloadDigest,
        )
    );
}

/**
 * Tests the parity candidate at `offset` by the four parts of P5 section 7.2, step 6. A
 * candidate that passes is charged to `budget`, and that charge is not made again when the
 * item is read.
 */
export function testParityCandidate(
    data: Uint8Array,
    offset: number,
    segment: ParityContext,
    budget: PayloadBudget,
): ParityVerdict {
    let frame: Frame;
    try {
        frame = decodeParityCandidate(data, offset, segment.registry).frame;
    } catch (error) {
        if (error instanceof FrameError) return { kind: "failed" };
        throw error;
    }
    const record = readParityRecord(frame.content);
    if (record === undefined) return { kind: "failed" };
    const span = spanOf(record);
    if (span === undefined || !segment.earlier.admits(record.first)) {
        return { kind: "failed" };
    }
    const shape = shapeOf(span);
    const expected = BigInt(shape.encodedLength);
    if (
        BigInt(frame.storedPayload.length) !== expected ||
        frame.decodedLength !== expected
    ) {
        return { kind: "failed" };
    }
    if (
        !equal(
            frameIdentifier(record.anchor, frame.identityFields),
            frame.identifier,
        )
    ) {
        return { kind: "failed" };
    }
    const refusal = budget.chargeCandidate(expected, expected);
    if (refusal !== undefined) {
        if (refusal.kind === "frame" || refusal.count === undefined) {
            return { kind: "failed" };
        }
        return {
            kind: "refused",
            length: frame.nextOffset - offset,
            count: refusal.count,
        };
    }
    if (
        !checkDecodedPayload(frame) ||
        !tableMatches(
            frame.storedPayload,
            shape,
            segment.h0,
            record.tableDigest,
        )
    ) {
        return { kind: "failed" };
    }
    budget.prepay(expected);
    return { kind: "passed", anchor: record.anchor };
}

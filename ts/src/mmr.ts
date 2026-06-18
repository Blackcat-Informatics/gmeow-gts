// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { blake3_256 } from "./wire.js";

export const ProofSchema = "gts-mmr-proof-v1";

const HashAlgorithm = "blake3-256";
const PreimageVersion = "gts-mmr-v1";
const LeafDomain = "gts-mmr-leaf-v1";
const ParentDomain = "gts-mmr-parent-v1";
const RootDomain = "gts-mmr-root-v1";

export interface MmrPeak {
    height: number;
    hash: Uint8Array;
}

export interface ProofStep {
    parentHeight: number;
    side: "left" | "right";
    hash: Uint8Array;
}

export interface Proof {
    count: number;
    leafIndex: number;
    frameId: Uint8Array;
    root: Uint8Array;
    peakIndex: number;
    peaks: MmrPeak[];
    path: ProofStep[];
}

type MmrPreimage = string | number | Uint8Array | MmrPreimage[];

function cborHead(major: number, length: number): Uint8Array {
    if (!Number.isSafeInteger(length) || length < 0) {
        throw new Error("CBOR length must be a safe unsigned integer");
    }
    const prefix = major << 5;
    if (length < 24) return new Uint8Array([prefix | length]);
    if (length <= 0xff) return new Uint8Array([prefix | 24, length]);
    if (length <= 0xffff) {
        return new Uint8Array([prefix | 25, length >> 8, length & 0xff]);
    }
    if (length <= 0xffffffff) {
        return new Uint8Array([
            prefix | 26,
            (length >>> 24) & 0xff,
            (length >>> 16) & 0xff,
            (length >>> 8) & 0xff,
            length & 0xff,
        ]);
    }
    let n = BigInt(length);
    const out = new Uint8Array(9);
    out[0] = prefix | 27;
    for (let i = 8; i >= 1; i--) {
        out[i] = Number(n & 0xffn);
        n >>= 8n;
    }
    return out;
}

function concat(parts: Uint8Array[]): Uint8Array {
    const total = parts.reduce((sum, part) => sum + part.length, 0);
    const out = new Uint8Array(total);
    let offset = 0;
    for (const part of parts) {
        out.set(part, offset);
        offset += part.length;
    }
    return out;
}

function encodeMmr(value: MmrPreimage): Uint8Array {
    if (typeof value === "string") {
        const bytes = new TextEncoder().encode(value);
        return concat([cborHead(3, bytes.length), bytes]);
    }
    if (typeof value === "number") {
        if (!Number.isSafeInteger(value) || value < 0) {
            throw new Error("MMR preimage integer must be unsigned");
        }
        return cborHead(0, value);
    }
    if (value instanceof Uint8Array) {
        return concat([cborHead(2, value.length), value]);
    }
    const items = value.map((item) => encodeMmr(item));
    return concat([cborHead(4, value.length), ...items]);
}

function leafHash(index: number, frameId: Uint8Array): Uint8Array {
    return blake3_256(encodeMmr([LeafDomain, index, frameId]));
}

function parentHash(
    parentHeight: number,
    left: Uint8Array,
    right: Uint8Array,
): Uint8Array {
    return blake3_256(encodeMmr([ParentDomain, parentHeight, left, right]));
}

function rootHash(count: number, peaks: MmrPeak[]): Uint8Array {
    const peakValues = peaks.map((peak) => [peak.height, peak.hash]);
    return blake3_256(encodeMmr([RootDomain, count, peakValues]));
}

function expectedPeakHeights(count: number): number[] {
    const heights: number[] = [];
    let remaining = count;
    while (remaining > 0) {
        const height = Math.floor(Math.log2(remaining));
        heights.push(height);
        remaining -= 2 ** height;
    }
    return heights;
}

function peakIndexForLeaf(
    count: number,
    heights: number[],
    leafIndex: number,
): number {
    if (leafIndex >= count) {
        throw new Error(
            `leaf_index ${leafIndex} is outside covered count ${count}`,
        );
    }
    let start = 0;
    for (let index = 0; index < heights.length; index++) {
        const end = start + 2 ** heights[index];
        if (leafIndex >= start && leafIndex < end) return index;
        start = end;
    }
    throw new Error(
        `peak ranges do not cover leaf_index ${leafIndex} for count ${count}`,
    );
}

export function parseHex32(input: string): Uint8Array {
    const raw = input.trim().replace(/^blake3:/, "");
    if (!/^[0-9a-fA-F]{64}$/.test(raw)) {
        throw new Error("expected a 32-byte hex value");
    }
    const out = new Uint8Array(Buffer.from(raw, "hex"));
    if (out.length !== 32) throw new Error("expected a 32-byte hex value");
    return out;
}

function objectValue(value: unknown, context: string): Record<string, unknown> {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error(`${context} must be a JSON object`);
    }
    return value as Record<string, unknown>;
}

function arrayValue(value: unknown, context: string): unknown[] {
    if (!Array.isArray(value)) {
        throw new Error(`${context} must be a JSON array`);
    }
    return value;
}

function stringField(obj: Record<string, unknown>, key: string): string {
    const value = obj[key];
    if (typeof value !== "string") {
        throw new Error(`${JSON.stringify(key)} must be a string`);
    }
    return value;
}

function intField(obj: Record<string, unknown>, key: string): number {
    const value = obj[key];
    if (typeof value !== "number" || !Number.isInteger(value) || value < 0) {
        throw new Error(`${JSON.stringify(key)} must be an unsigned integer`);
    }
    return value;
}

export function proofFromJson(text: string): Proof {
    const obj = objectValue(JSON.parse(text), "proof");
    const schema = stringField(obj, "schema");
    if (schema !== ProofSchema) {
        throw new Error(`unsupported proof schema ${JSON.stringify(schema)}`);
    }
    const hash = stringField(obj, "hash");
    if (hash !== HashAlgorithm) {
        throw new Error(`unsupported hash algorithm ${JSON.stringify(hash)}`);
    }
    const preimage = stringField(obj, "preimage");
    if (preimage !== PreimageVersion) {
        throw new Error(
            `unsupported preimage version ${JSON.stringify(preimage)}`,
        );
    }

    const peaks = arrayValue(obj.peaks, "peaks").map((item): MmrPeak => {
        const peak = objectValue(item, "peak");
        return {
            height: intField(peak, "height"),
            hash: parseHex32(stringField(peak, "hash")),
        };
    });
    const path = arrayValue(obj.path, "path").map((item): ProofStep => {
        const step = objectValue(item, "path step");
        const side = stringField(step, "side");
        if (side !== "left" && side !== "right") {
            throw new Error(`unsupported proof side ${JSON.stringify(side)}`);
        }
        return {
            parentHeight: intField(step, "parent_height"),
            side,
            hash: parseHex32(stringField(step, "hash")),
        };
    });

    return {
        count: intField(obj, "count"),
        leafIndex: intField(obj, "leaf_index"),
        frameId: parseHex32(stringField(obj, "frame_id")),
        root: parseHex32(stringField(obj, "root")),
        peakIndex: intField(obj, "peak_index"),
        peaks,
        path,
    };
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
    if (left.length !== right.length) return false;
    for (let i = 0; i < left.length; i++) {
        if (left[i] !== right[i]) return false;
    }
    return true;
}

function numbersEqual(left: number[], right: number[]): boolean {
    if (left.length !== right.length) return false;
    for (let i = 0; i < left.length; i++) {
        if (left[i] !== right[i]) return false;
    }
    return true;
}

export function verifyProof(proof: Proof): void {
    if (proof.frameId.length !== 32)
        throw new Error("frame_id must be 32 bytes");
    if (proof.root.length !== 32) throw new Error("root must be 32 bytes");
    if (proof.leafIndex >= proof.count) {
        throw new Error(
            `leaf_index ${proof.leafIndex} is outside covered count ${proof.count}`,
        );
    }
    if (proof.peakIndex >= proof.peaks.length) {
        throw new Error(`peak_index ${proof.peakIndex} is out of range`);
    }
    const expectedHeights = expectedPeakHeights(proof.count);
    const actualHeights = proof.peaks.map((peak) => peak.height);
    if (!numbersEqual(actualHeights, expectedHeights)) {
        throw new Error(
            `peak heights ${JSON.stringify(actualHeights)} ` +
                `do not match count ${proof.count}`,
        );
    }
    const computedPeakIndex = peakIndexForLeaf(
        proof.count,
        actualHeights,
        proof.leafIndex,
    );
    if (computedPeakIndex !== proof.peakIndex) {
        throw new Error(
            `leaf_index ${proof.leafIndex} belongs to peak ` +
                `${computedPeakIndex}, not ${proof.peakIndex}`,
        );
    }
    for (const peak of proof.peaks) {
        if (peak.hash.length !== 32)
            throw new Error("peak hash must be 32 bytes");
    }

    let carried = leafHash(proof.leafIndex, proof.frameId);
    let height = 0;
    for (const step of proof.path) {
        if (step.hash.length !== 32)
            throw new Error("path hash must be 32 bytes");
        if (step.parentHeight !== height + 1) {
            throw new Error(
                `path parent height ${step.parentHeight} does not follow ` +
                    `height ${height}`,
            );
        }
        if (step.side === "left") {
            carried = parentHash(step.parentHeight, step.hash, carried);
        } else {
            carried = parentHash(step.parentHeight, carried, step.hash);
        }
        height = step.parentHeight;
    }

    const peak = proof.peaks[proof.peakIndex];
    if (height !== peak.height) {
        throw new Error(
            `path height ${height} does not reach peak height ${peak.height}`,
        );
    }
    if (!bytesEqual(carried, peak.hash)) {
        throw new Error("proof path does not reconstruct the selected peak");
    }
    if (!bytesEqual(rootHash(proof.count, proof.peaks), proof.root)) {
        throw new Error("proof peaks do not reconstruct the declared root");
    }
}

export function verifyProofJson(text: string): Proof {
    const proof = proofFromJson(text);
    verifyProof(proof);
    return proof;
}

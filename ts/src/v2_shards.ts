// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shard digests and the erasure solving of a geometry-1 span (P5 sections 6.1 to 6.4). The
//! field is GF(256) under the reducing polynomial 0x11D; the parity rows of each stripe are
//! Cauchy coefficients over distinct points, so a stripe with no more unknowns than held rows
//! always solves.

import { contextHash } from "./v2_integrity.js";

/** The parity rows of each stripe (P5 6.4). */
export const ROWS = 4;
/** The bytes of a shard digest: the first sixteen bytes of the derived hash. */
export const DIGEST_BYTES = 16;

interface Field {
    powers: Uint8Array;
    logs: Uint8Array;
}

/** The exponent and logarithm tables of GF(256) under the reducing polynomial 0x11D. */
const FIELD: Field = (() => {
    const powers = new Uint8Array(255);
    const logs = new Uint8Array(256);
    let value = 1;
    for (let exponent = 0; exponent < 255; exponent++) {
        powers[exponent] = value;
        logs[value] = exponent;
        const overflow = (value & 0x80) !== 0;
        value = (value << 1) & 0xff;
        if (overflow) value ^= 0x1d;
    }
    return { powers, logs };
})();

/** The product of two field elements. */
export function multiply(left: number, right: number): number {
    if (left === 0 || right === 0) return 0;
    const exponent = (FIELD.logs[left] + FIELD.logs[right]) % 255;
    return FIELD.powers[exponent];
}

/** The multiplicative inverse of a nonzero field element. */
function inverse(value: number): number {
    return FIELD.powers[(255 - FIELD.logs[value]) % 255];
}

/**
 * The Cauchy coefficient of `column` in parity `row`: the inverse of `(255 - row) XOR column`.
 * The denominator is never zero, because `255 - row` is at least 252 and `column` is below 64.
 */
export function coefficient(row: number, column: number): number {
    return inverse((255 - row) ^ column);
}

function scale(block: Uint8Array, factor: number): Uint8Array {
    return block.map((byte) => multiply(byte, factor));
}

function xorInto(target: Uint8Array, source: Uint8Array): void {
    for (let index = 0; index < target.length; index++) {
        target[index] ^= source[index];
    }
}

/** The digest of one shard, which the writer computes over the shard padded to its size. */
export function shardDigest(shard: Uint8Array): Uint8Array {
    return contextHash("gts2 shard", shard).subarray(0, DIGEST_BYTES);
}

/** The shard at `index` of `covered`, padded with zeros to `size` bytes. */
export function shardAt(
    covered: Uint8Array,
    index: number,
    size: number,
): Uint8Array {
    const shard = new Uint8Array(size);
    const start = Math.min(index * size, covered.length);
    const end = Math.min((index + 1) * size, covered.length);
    shard.set(covered.subarray(start, end));
    return shard;
}

/** The parity contribution of the known columns of a stripe to `row`. */
function combine(
    columns: readonly (Uint8Array | undefined)[],
    row: number,
    size: number,
): Uint8Array {
    const sum = new Uint8Array(size);
    columns.forEach((block, column) => {
        if (block !== undefined) {
            xorInto(sum, scale(block, coefficient(row, column)));
        }
    });
    return sum;
}

/**
 * Gauss-Jordan elimination over GF(256), leaving the solution in `rhs`. Returns `false` when
 * the coefficient matrix is singular.
 */
function eliminate(coefficients: Uint8Array[], rhs: Uint8Array[]): boolean {
    const count = coefficients.length;
    for (let column = 0; column < count; column++) {
        let pivot = -1;
        for (let row = column; row < count; row++) {
            if (coefficients[row][column] !== 0) {
                pivot = row;
                break;
            }
        }
        if (pivot < 0) return false;
        [coefficients[column], coefficients[pivot]] = [
            coefficients[pivot],
            coefficients[column],
        ];
        [rhs[column], rhs[pivot]] = [rhs[pivot], rhs[column]];
        const factor = inverse(coefficients[column][column]);
        coefficients[column] = scale(coefficients[column], factor);
        rhs[column] = scale(rhs[column], factor);
        for (let other = 0; other < count; other++) {
            const cell = coefficients[other][column];
            if (other === column || cell === 0) continue;
            const pivotRow = coefficients[column].slice();
            for (let index = 0; index < pivotRow.length; index++) {
                coefficients[other][index] ^= multiply(cell, pivotRow[index]);
            }
            xorInto(rhs[other], scale(rhs[column].slice(), cell));
        }
    }
    return true;
}

/**
 * Solves the erased data columns of one stripe from its held parity rows (P5 6.4). Returns
 * each erased column with its solved bytes, or `undefined` when the stripe holds more unknowns
 * than its held rows.
 */
function solveStripe(
    columns: readonly (Uint8Array | undefined)[],
    blocks: readonly (Uint8Array | undefined)[],
    size: number,
): [number, Uint8Array][] | undefined {
    const missing: number[] = [];
    columns.forEach((block, column) => {
        if (block === undefined) missing.push(column);
    });
    if (missing.length === 0) return [];
    const held: [number, Uint8Array][] = [];
    blocks.forEach((block, row) => {
        if (block !== undefined && held.length < missing.length) {
            held.push([row, block]);
        }
    });
    if (held.length < missing.length) return undefined;
    const coefficients = held.map(([row]) =>
        Uint8Array.from(missing.map((column) => coefficient(row, column))),
    );
    const rhs = held.map(([row, block]) => {
        const value = block.slice();
        xorInto(value, combine(columns, row, size));
        return value;
    });
    if (!eliminate(coefficients, rhs)) return undefined;
    return missing.map((column, index) => [column, rhs[index]]);
}

/** The shards that a span's parity writes: each solved data shard and each solved parity row. */
export interface SolvedShards {
    /** The solved data shards by their index. */
    data: Map<number, Uint8Array>;
    /** The solved parity shards by their row. */
    parity: Map<number, Uint8Array>;
}

/**
 * Solves every erased shard of a span of `covered` bytes with `size`-byte shards. `parity` holds
 * the span's parity bytes, `depth` stripes of four rows. The unknowns are the failing data shards
 * `badData` and the failing parity rows `badParity`. Returns `undefined` when a stripe holds more
 * than four unknowns (`too_many_erasures`).
 */
export function solveShards(
    covered: Uint8Array,
    parity: Uint8Array,
    size: number,
    badData: ReadonlySet<number>,
    badParity: ReadonlySet<number>,
): SolvedShards | undefined {
    const depth = Math.max(1, Math.floor(parity.length / (size * ROWS)));
    const counts = new Array<number>(depth).fill(0);
    for (const index of badData) counts[index % depth] += 1;
    for (const row of badParity) {
        const stripe = Math.floor(row / ROWS);
        if (stripe >= depth) return undefined;
        counts[stripe] += 1;
    }
    if (counts.some((count) => count > ROWS)) return undefined;
    const dataCount = Math.ceil(covered.length / size);
    const solved: SolvedShards = { data: new Map(), parity: new Map() };
    for (let stripe = 0; stripe < depth; stripe++) {
        if (counts[stripe] === 0) continue;
        const indices: number[] = [];
        for (let index = stripe; index < dataCount; index += depth) {
            indices.push(index);
        }
        const columns = indices.map((index) =>
            badData.has(index) ? undefined : shardAt(covered, index, size),
        );
        const rows = Array.from(
            { length: ROWS },
            (_, local) => stripe * ROWS + local,
        );
        const blocks = rows.map((row) =>
            badParity.has(row)
                ? undefined
                : parity.subarray(row * size, (row + 1) * size),
        );
        const found = solveStripe(columns, blocks, size);
        if (found === undefined) return undefined;
        for (const [column, shard] of found) {
            columns[column] = shard;
            solved.data.set(indices[column], shard);
        }
        rows.forEach((row, local) => {
            if (blocks[local] === undefined) {
                solved.parity.set(row, combine(columns, local, size));
            }
        });
    }
    return solved;
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Strict COSE protected-header reader shared by the Node and browser engines.
//
// The CBOR decoder maps integers and integral floats to the same JS number, so
// an `alg` of -8.0 or a label of 1.0 would pass a decoded-value check. This
// module therefore inspects the raw header bytes for the CBOR major type.

import { cborItemLength, decodeFirst } from "./browser_wire.js";

const ALG = 1;

/** Read a definite-length argument at `off`: `[value, next offset]`, or null. */
function readHead(
    data: Uint8Array,
    off: number,
): { major: number; arg: number; next: number } | null {
    if (off >= data.length) return null;
    const major = data[off] >> 5;
    const info = data[off] & 0x1f;
    const width = info <= 23 ? 0 : info === 24 ? 1 : info === 25 ? 2 : info === 26 ? 4 : -1;
    if (width < 0 || off + 1 + width > data.length) return null;
    let arg = info;
    if (width > 0) {
        arg = 0;
        for (let i = 0; i < width; i++) arg = arg * 256 + data[off + 1 + i];
    }
    return { major, arg, next: off + 1 + width };
}

/**
 * The `alg` (label 1) of a protected header: a bstr holding exactly one CBOR
 * map whose integer key 1 maps to an integer or text string (RFC 9052 §3.1).
 * Null if there are trailing bytes, the label or `alg` is a float or boolean,
 * or no `alg` is declared.
 */
export function declaredAlgorithm(
    prot: Uint8Array,
): number | bigint | string | null {
    try {
        const head = readHead(prot, 0);
        if (!head || head.major !== 5) return null;
        let off = head.next;
        let algStart = -1;
        for (let i = 0; i < head.arg; i++) {
            const keyStart = off;
            off += cborItemLength(prot, off);
            const valueStart = off;
            off += cborItemLength(prot, off);
            const key = readHead(prot, keyStart);
            if (algStart < 0 && key && key.major === 0 && key.arg === ALG) {
                algStart = valueStart;
            }
            if (off > prot.length) return null;
        }
        if (off !== prot.length || algStart < 0) return null;
        const alg = readHead(prot, algStart);
        if (!alg || (alg.major !== 0 && alg.major !== 1 && alg.major !== 3)) {
            return null;
        }
        const value: unknown = decodeFirst(
            prot.subarray(algStart, algStart + cborItemLength(prot, algStart)),
        );
        if (typeof value === "string" || typeof value === "bigint") return value;
        if (typeof value === "number" && Number.isInteger(value)) return value;
        return null;
    } catch {
        return null;
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Strict COSE protected-header reader shared by the Node and browser engines.
// It scans the raw bytes and imports nothing from the wire codec.

const MAX_DEPTH = 8;
const MAX_INT = (1n << 63n) - 1n;
const UTF8 = new TextDecoder("utf-8", { fatal: true });

interface Head {
    major: number;
    info: number;
    arg: bigint;
    next: number;
}

/**
 * Read one item head at `pos`. Null for an indefinite length, a reserved
 * additional info, a truncated head, or an argument that is not in shortest form.
 */
function scanHead(buf: Uint8Array, pos: number): Head | null {
    if (pos >= buf.length) return null;
    const major = buf[pos] >> 5;
    const info = buf[pos] & 0x1f;
    if (info < 24) return { major, info, arg: BigInt(info), next: pos + 1 };
    if (info > 27) return null;
    const width = 1 << (info - 24);
    if (buf.length - pos - 1 < width) return null;
    let arg = 0n;
    for (let i = 0; i < width; i++) arg = (arg << 8n) | BigInt(buf[pos + 1 + i]);
    const minimum = [24n, 1n << 8n, 1n << 16n, 1n << 32n][info - 24];
    if (arg < minimum) return null;
    return { major, info, arg, next: pos + 1 + width };
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
    return a.length === b.length && a.every((byte, i) => byte === b[i]);
}

function compareBytes(a: Uint8Array, b: Uint8Array): number {
    const n = Math.min(a.length, b.length);
    for (let i = 0; i < n; i++) if (a[i] !== b[i]) return a[i] - b[i];
    return a.length - b.length;
}

/**
 * Check one header item against the grammar; returns the position after it, or
 * null. `depth` is the level of this item if it is a container.
 */
function scanItem(
    buf: Uint8Array,
    pos: number,
    depth: number,
    isKey: boolean,
): number | null {
    const head = scanHead(buf, pos);
    if (head === null) return null;
    const { major, info, arg, next } = head;
    if (isKey && major !== 0 && major !== 1 && major !== 3) return null;
    switch (major) {
        case 0:
        case 1:
            // Integers lie in -2^63 .. 2^63-1.
            return arg <= MAX_INT ? next : null;
        case 2:
        case 3: {
            if (arg > BigInt(buf.length - next)) return null;
            const end = next + Number(arg);
            if (major === 3) {
                try {
                    UTF8.decode(buf.subarray(next, end));
                } catch {
                    return null;
                }
            }
            return end;
        }
        case 4:
        case 5: {
            if (depth > MAX_DEPTH) return null;
            let at = next;
            let previous: Uint8Array | null = null;
            for (let i = 0n; i < arg; i++) {
                if (major === 4) {
                    const after = scanItem(buf, at, depth + 1, false);
                    if (after === null) return null;
                    at = after;
                    continue;
                }
                const keyEnd = scanItem(buf, at, depth + 1, true);
                if (keyEnd === null) return null;
                const key = buf.subarray(at, keyEnd);
                // Strictly ascending bytewise: sorted, and no duplicate key.
                if (previous !== null && compareBytes(previous, key) >= 0) {
                    return null;
                }
                previous = key;
                const after = scanItem(buf, keyEnd, depth + 1, false);
                if (after === null) return null;
                at = after;
            }
            return at;
        }
        case 7:
            // null, true and false only.
            return info >= 20 && info <= 22 ? next : null;
        default:
            return null;
    }
}

/**
 * The `alg` (label 1) of a protected header (RFC 9052 §3.1).
 *
 * The protected header is a byte string whose content is deterministically
 * encoded CBOR (definite lengths, shortest-form integers and lengths, map keys in
 * bytewise order of their encodings, no duplicate keys, no trailing bytes)
 * consisting of exactly one map. Every map key, at any depth, is an integer or a
 * text string. Every value, at any depth, is an integer, a byte string, a text
 * string, an array, a map, null, true or false. Every integer, key or value, lies
 * in the range -2^63 to 2^63-1 inclusive. Text strings are valid UTF-8. Nesting
 * is at most 8 levels deep, counting the header map itself as level 1. The map
 * contains the integer key 1, whose value is an integer or a text string. A
 * header that does not meet all of this is malformed and the signature is
 * invalid.
 *
 * Enforced by scanning the raw bytes (`scanItem`), so no decoder behaviour is
 * involved. Returns null if the header is malformed or declares no algorithm.
 */
export function declaredAlgorithm(
    prot: Uint8Array,
): number | bigint | string | null {
    try {
        if (prot.length === 0 || prot[0] >> 5 !== 5) return null;
        if (scanItem(prot, 0, 1, false) !== prot.length) return null;
        const root = scanHead(prot, 0);
        if (root === null) return null;
        let at = root.next;
        for (let i = 0n; i < root.arg; i++) {
            const keyEnd = scanItem(prot, at, 2, true);
            if (keyEnd === null) return null;
            const valueEnd = scanItem(prot, keyEnd, 2, false);
            if (valueEnd === null) return null;
            if (sameBytes(prot.subarray(at, keyEnd), Uint8Array.of(1))) {
                const value = scanHead(prot, keyEnd);
                if (value === null) return null;
                let result: bigint;
                if (value.major === 0) result = value.arg;
                else if (value.major === 1) result = -1n - value.arg;
                else if (value.major === 3) {
                    return UTF8.decode(prot.subarray(value.next, valueEnd));
                } else return null;
                const asNumber = Number(result);
                return Number.isSafeInteger(asNumber) ? asNumber : result;
            }
            at = valueEnd;
        }
        return null;
    } catch {
        return null;
    }
}

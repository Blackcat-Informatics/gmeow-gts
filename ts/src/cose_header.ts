// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Strict COSE protected-header reader shared by the Node and browser engines.

import { decodeFirst, encodeCanonical } from "./browser_wire.js";

const ALG = 1;

/**
 * Whether `value` holds only integers, byte strings, text strings, arrays, maps,
 * null, true and false at any depth: no float, tag, bignum, `undefined` or other
 * simple value.
 */
function isPlain(value: unknown): boolean {
    if (value === null || typeof value === "boolean") return true;
    if (typeof value === "string" || typeof value === "bigint") return true;
    if (typeof value === "number") return Number.isInteger(value);
    if (value instanceof Uint8Array) return true;
    if (Array.isArray(value)) return value.every(isPlain);
    if (value instanceof Map) {
        return [...value].every(([k, v]) => isPlain(k) && isPlain(v));
    }
    return false;
}

/**
 * The `alg` (label 1) of a protected header (RFC 9052 §3.1).
 *
 * Header rule (identical in every engine): the protected header must be
 * deterministically encoded CBOR (RFC 8949 §4.2, GTS-SPEC §4) holding exactly one
 * map, containing only integers, byte strings, text strings, arrays, maps, null,
 * true and false (no float, tag or other simple value at any depth), and its
 * integer key 1 must map to an integer or text string.
 *
 * Here: the header is decoded and re-encoded with the engine's deterministic
 * encoder and the bytes must be identical, which rejects trailing bytes,
 * indefinite lengths, non-shortest integers and lengths at every width, unsorted
 * keys, and duplicate keys (a JS `Map` collapses them, so the re-encoding is
 * shorter); an integral float such as -8.0 decodes to a number that re-encodes as
 * an integer, so the bytes differ. `isPlain` rejects tags and other simple values.
 *
 * Returns null if the header is malformed or declares no algorithm.
 */
export function declaredAlgorithm(
    prot: Uint8Array,
): number | bigint | string | null {
    try {
        const header = decodeFirst(prot);
        if (!(header instanceof Map) || !isPlain(header)) return null;
        const again = encodeCanonical(header);
        if (
            again.length !== prot.length ||
            again.some((byte, i) => byte !== prot[i])
        ) {
            return null;
        }
        const alg: unknown = header.get(ALG);
        if (typeof alg === "string" || typeof alg === "bigint") return alg;
        if (typeof alg === "number" && Number.isInteger(alg)) return alg;
        return null;
    } catch {
        return null;
    }
}

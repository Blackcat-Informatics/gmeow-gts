// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
//
// Strict COSE protected-header reader shared by the Node and browser engines.

import { decodeFirst, encodeCanonical } from "./browser_wire.js";

const ALG = 1;

/**
 * The `alg` (label 1) of a protected header (RFC 9052 §3.1).
 *
 * The header's bytes are signed, so they must be deterministically encoded CBOR
 * (RFC 8949 §4.2, GTS-SPEC §4) holding exactly one map. The header is decoded
 * and re-encoded with the engine's deterministic encoder, and the bytes must be
 * identical: this rejects trailing bytes, indefinite lengths, non-shortest
 * integers and lengths at every width, unsorted keys, and duplicate keys (a JS
 * `Map` collapses them, so the re-encoding is shorter). It also rejects floats,
 * which the encoder cannot represent, and an integral float such as -8.0 or 1.0
 * decodes to a number that re-encodes as an integer, so the bytes differ. The
 * integer key 1 must then map to an integer or a text string.
 *
 * Returns null if the header is malformed or declares no algorithm.
 */
export function declaredAlgorithm(
    prot: Uint8Array,
): number | bigint | string | null {
    try {
        const header = decodeFirst(prot);
        if (!(header instanceof Map)) return null;
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

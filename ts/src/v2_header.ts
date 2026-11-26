// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The header opener of a segment (P1 section 2.4): the decode of a header item at an
//! established item boundary, with the first malformed field reported by reason and offset.

import { BLAKE3_256, Decoder } from "./v2_decoder.js";
import { contextHash } from "./v2_integrity.js";

/** The registry version this reader implements. */
export const REGISTRY_VERSION = 1n;

/** The last registry code of the canonical form (P2 section 4.7). */
const CANONICAL_FORM = 1n;

/** The largest value of a registry code in a version that this reader does not know. */
const U64_MAX = (1n << 64n) - 1n;

/** The fields of a header after its kind and generation. */
export interface Header {
    registryVersion: bigint;
    hashAlgorithm: bigint;
    profile: bigint | undefined;
    canonical: bigint | undefined;
    lineage: { previousHead: Uint8Array; ordinal: bigint } | undefined;
}

/** A decoded header, the offset after it, and its identifier when its hash is known. */
export interface DecodedHeader {
    header: Header;
    nextOffset: number;
    identifier: Uint8Array | undefined;
}

/**
 * Decodes the header item at `offset` of `data`, which must end at the item's end (the
 * caller passes the bytes up to the item). The first malformed field determines the error.
 *
 * @throws HeaderError at the first malformed field, with its reason and absolute offset.
 */
export function decodeHeader(data: Uint8Array, offset: number): DecodedHeader {
    const decoder = new Decoder(data, offset);
    decoder.record(7);
    decoder.code(1);
    const generationError = decoder.error("out_of_range");
    if (decoder.uint() !== 2n) throw generationError;
    const { registryVersion, hashAlgorithm, profile } =
        decoder.openerFields(false);
    const knownHash = hashAlgorithm === BLAKE3_256;
    const lastKnown = (last: bigint): bigint =>
        registryVersion > REGISTRY_VERSION ? U64_MAX : last;
    const canonical = decoder.option((d) => d.code(lastKnown(CANONICAL_FORM)));
    const lineage = decoder.option((d) => d.lineage(knownHash));
    const nextOffset = decoder.offset;
    const identifier = knownHash
        ? contextHash("gts2 header", data.subarray(offset, nextOffset))
        : undefined;
    return {
        header: { registryVersion, hashAlgorithm, profile, canonical, lineage },
        nextOffset,
        identifier,
    };
}

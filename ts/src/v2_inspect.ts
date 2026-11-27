// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The raw inspection of a frame item whose envelope does not decode (P1 section 3.3 and
//! section 5.3). A frame with a malformed field still has a link and a payload to judge: the
//! identity fields, the stored identifier and the stored chunks are read from the item's own
//! bytes, field by field, and what cannot be read is absent.

import { Decoder, HeaderError } from "./v2_decoder.js";
import { frameIdentity, readStorage } from "./v2_frame.js";
import { FramingError, head, itemEnd } from "./v2_framing.js";
import { storedMatches, type PayloadFields } from "./v2_payload.js";

/** The offsets of the eleven elements of a frame item: its type, its fields and its end. */
export type FieldOffsets = readonly number[];

/**
 * The offsets at which the elements of the frame item at `offset` begin, followed by the
 * offset after the last element.
 *
 * @throws FramingError when an element cannot be delimited.
 */
export function fieldOffsets(data: Uint8Array, offset: number): FieldOffsets {
    const offsets = [offset + 2];
    for (let index = 1; index < 11; index++) {
        offsets.push(itemEnd(data, offsets[index - 1]));
    }
    return offsets;
}

/**
 * The octets of the byte string at `offset` as its chunks, whatever its head encoding. An
 * indefinite string ends at its first chunk that cannot be read. Absent when the item is
 * not a byte string.
 */
export function byteChunks(
    data: Uint8Array,
    offset: number,
): Uint8Array[] | undefined {
    let first;
    try {
        first = head(data, offset);
    } catch (error) {
        if (error instanceof FramingError) return undefined;
        throw error;
    }
    if (first.major !== 2) return undefined;
    if (!first.indefinite) {
        const end = first.end + Number(first.value);
        return end <= data.length ? [data.subarray(first.end, end)] : [];
    }
    const chunks: Uint8Array[] = [];
    for (let start = first.end; data[start] !== 0xff;) {
        let chunk;
        try {
            chunk = head(data, start);
        } catch (error) {
            if (error instanceof FramingError) break;
            throw error;
        }
        const end = chunk.end + Number(chunk.value);
        if (chunk.indefinite || end > data.length) break;
        chunks.push(data.subarray(chunk.end, end));
        start = end;
    }
    return chunks;
}

/** The 32 octets of the identifier stored at `offset`, from its chunks, or absent. */
export function storedIdentifier(
    data: Uint8Array,
    offset: number,
): Uint8Array | undefined {
    const result = new Uint8Array(32);
    let used = 0;
    for (const chunk of byteChunks(data, offset) ?? []) {
        if (chunk.length > result.length - used) return undefined;
        result.set(chunk, used);
        used += chunk.length;
    }
    return used === 32 ? result : undefined;
}

/** What the link of a frame depends on, read from its raw fields. */
export interface InspectedLink {
    transparent: boolean;
    identity: Uint8Array | undefined;
    stored: Uint8Array | undefined;
}

/** The link fields of the frame whose elements begin at `offsets`. */
export function inspectLink(
    data: Uint8Array,
    offsets: FieldOffsets,
): InspectedLink {
    let identity: Uint8Array | undefined;
    try {
        identity = frameIdentity(
            new Decoder(data, offsets[0]),
            undefined,
        ).encoded;
    } catch (error) {
        if (!(error instanceof HeaderError)) throw error;
    }
    return {
        transparent: data[offsets[2]] === 0xf5,
        identity,
        stored: storedIdentifier(data, offsets[8]),
    };
}

/** The payload of a frame with a malformed field: what the stored check and the payload read. */
export interface MalformedPayload {
    storedMatches: boolean | undefined;
    /** The fields of the payload, or absent when one of them cannot be read. */
    fields: PayloadFields | undefined;
}

/** The payload fields of the frame whose elements begin at `offsets`. */
export function malformedPayload(
    data: Uint8Array,
    offsets: FieldOffsets,
    newer: boolean,
): MalformedPayload {
    const attempt = <T>(read: () => T): T | undefined => {
        try {
            return read();
        } catch (error) {
            if (error instanceof HeaderError) return undefined;
            throw error;
        }
    };
    const storage = attempt(() =>
        readStorage(new Decoder(data, offsets[6]), newer, 1n),
    );
    const stored = attempt(() => new Decoder(data, offsets[7]).byteString());
    const chunks = byteChunks(data, offsets[7]);
    const matches =
        storage === undefined || chunks === undefined
            ? undefined
            : storedMatches(storage, chunks);
    const decodedLength = attempt(() => new Decoder(data, offsets[4]).uint());
    const payloadDigest = attempt(() =>
        new Decoder(data, offsets[5]).byteString(32),
    );
    if (
        storage === undefined ||
        stored === undefined ||
        decodedLength === undefined ||
        payloadDigest === undefined
    ) {
        return { storedMatches: matches, fields: undefined };
    }
    return {
        storedMatches: matches,
        fields: {
            storage,
            storedPayload: stored,
            decodedLength,
            payloadDigest,
            content: { length: offsets[4] - offsets[3] },
            encodedSignaturesSize: offsets[10] - offsets[9],
        },
    };
}

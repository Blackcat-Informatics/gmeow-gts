// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frame envelopes (P1 sections 3 and 4): the decode of one deterministic frame
//! at a known item boundary, reporting the first malformed field and its offset.

import {
    compareBytes,
    Decoder,
    HeaderError,
    type HeaderReason,
} from "./v2_decoder.js";

const FRAME = 2n;
const CHUNK = 8n;
const EXTERNAL = 12n;
export const COMMIT = 14n;
export const INDEX = 15n;
export const PARITY = 16n;

/** The first malformed field of a frame and its absolute input offset. */
export class FrameError extends Error {
    constructor(
        readonly reason: HeaderReason,
        readonly offset: number,
    ) {
        super(`${reason} at offset ${offset}`);
    }
}

/** The storage record, including the original bytes of a future storage form. */
export type Storage =
    | { kind: "plain" }
    | { kind: "compressed"; codec: bigint; storedCheck: Uint8Array }
    | { kind: "unknown"; tag: bigint; encoded: Uint8Array };

/** One signature of a frame, in envelope order. */
export interface Signature {
    algorithm: bigint;
    keyId: Uint8Array;
    value: Uint8Array;
}

/** A parsed frame, borrowing the original encoded fields and stored payload. */
export interface Frame {
    frameType: bigint;
    /** Whether a reader must refuse the frame if it does not understand it. */
    critical: boolean;
    /** Whether the frame is transparent to readers of its content. */
    transparent: boolean;
    content: Uint8Array;
    decodedLength: bigint;
    payloadDigest: Uint8Array;
    storage: Storage;
    storedPayload: Uint8Array;
    identifier: Uint8Array;
    signatures: Signature[];
    /** The encoded size of the signatures element in bytes. */
    encodedSignaturesSize: number;
    /** The encoded identity fields of the frame. */
    identityFields: Uint8Array;
    /** The offset of the first byte after the frame. */
    nextOffset: number;
}

function validDecodedLength(frameType: bigint, length: bigint): boolean {
    if (frameType === COMMIT || frameType === EXTERNAL) return length === 0n;
    if (frameType === CHUNK) return length !== 0n;
    return true;
}

function registryCode(decoder: Decoder, last: bigint, newer: boolean): bigint {
    return decoder.code(newer ? (1n << 64n) - 1n : last);
}

function mediaTypeShape(decoder: Decoder): void {
    const error = decoder.error("wrong_length");
    const size = decoder.head(4);
    if (size === 0n) throw error;
    const tag = decoder.uint();
    if ((tag === 1n || tag === 2n) && size !== tag) throw error;
    if (tag === 2n) {
        decoder.text();
    } else if (tag !== 1n) {
        for (let index = 1n; index < size; index++) decoder.closedItem();
    }
}

/** The content record of a frame of `frameType`, or any closed item of a later type. */
export function frameContent(decoder: Decoder, frameType: bigint): void {
    if (frameType === 1n) {
        decoder.record(4);
        decoder.uint();
        const count = decoder.head(4);
        for (let index = 0n; index < count; index++) {
            decoder.record(2);
            decoder.uint();
            decoder.uint();
        }
        decoder.uint();
        decoder.uint();
        return;
    }
    let shape: string;
    if ((frameType >= 2n && frameType <= 4n) || frameType === 13n) shape = "uu";
    else if (frameType >= 5n && frameType <= 7n) shape = "u";
    else if (frameType === 8n) shape = "d";
    else if (frameType === 9n) shape = "dm";
    else if (frameType === 10n) shape = "dumu";
    else if (frameType === 11n) shape = "duu";
    else if (frameType === 12n) shape = "num";
    else if (frameType === 14n || frameType === 15n) shape = "ud";
    else if (frameType === 16n) shape = "uuudd";
    else {
        decoder.closedItem();
        return;
    }
    decoder.record(shape.length);
    for (const field of shape) {
        if (field === "u") {
            decoder.uint();
        } else if (field === "d") {
            decoder.byteString(32);
        } else if (field === "m") {
            mediaTypeShape(decoder);
        } else {
            decoder.record(2);
            const algorithm = decoder.uint();
            decoder.byteString(algorithm === 1n ? 32 : undefined);
        }
    }
}

export function readStorage(
    decoder: Decoder,
    newer: boolean,
    decodedLength: bigint,
): Storage {
    const error = decoder.error("wrong_length");
    const size = decoder.head(4);
    if (size === 0n) throw error;
    const tag = registryCode(decoder, 2n, newer);
    if (tag === 1n && size === 1n) return { kind: "plain" };
    if (tag === 2n && size === 3n) {
        const codec = registryCode(decoder, 1n, newer);
        const storedCheck = decoder.byteString(16);
        if (decodedLength === 0n) {
            throw new HeaderError("forbidden_combination", error.offset);
        }
        return { kind: "compressed", codec, storedCheck };
    }
    if (tag === 1n || tag === 2n) throw error;
    for (let index = 1n; index < size; index++) decoder.closedItem();
    return {
        kind: "unknown",
        tag,
        encoded: decoder.bytes.subarray(error.offset, decoder.offset),
    };
}

/** The signatures list: records of three fields in strictly increasing encoded order. */
export function readSignatures(decoder: Decoder, newer: boolean): Signature[] {
    let previous = decoder.bytes.subarray(0, 0);
    const signatures: Signature[] = [];
    const count = decoder.head(4);
    for (let index = 0n; index < count; index++) {
        const error = decoder.error("unordered_or_repeated");
        decoder.record(3);
        const algorithm = registryCode(decoder, 2n, newer);
        const keyId = decoder.byteString(32);
        const value = decoder.byteString();
        const encoded = decoder.bytes.subarray(error.offset, decoder.offset);
        if (compareBytes(encoded, previous) <= 0) throw error;
        previous = encoded;
        signatures.push({ algorithm, keyId, value });
    }
    return signatures;
}

export interface Identity {
    frameType: bigint;
    critical: boolean;
    transparent: boolean;
    content: Uint8Array;
    lengthOffset: number;
    decodedLength: bigint;
    payloadDigest: Uint8Array;
    encoded: Uint8Array;
}

/**
 * The identity fields of a frame. `newer` is undefined when the fields are read from a
 * frame whose envelope failed: the type is then any `uint` and no flag is checked.
 */
export function frameIdentity(
    decoder: Decoder,
    newer: boolean | undefined,
): Identity {
    const start = decoder.offset;
    const frameType =
        newer === undefined
            ? decoder.uint()
            : registryCode(decoder, PARITY, newer);
    const checkFlags = newer !== undefined && frameType <= PARITY;
    let error = decoder.error("forbidden_combination");
    const critical = decoder.boolean();
    if (checkFlags && critical !== frameType < COMMIT) throw error;
    error = decoder.error("forbidden_combination");
    const transparent = decoder.boolean();
    if (checkFlags && transparent !== (frameType === PARITY)) throw error;
    const contentStart = decoder.offset;
    frameContent(decoder, frameType);
    const content = decoder.bytes.subarray(contentStart, decoder.offset);
    const lengthOffset = decoder.offset;
    const decodedLength = decoder.uint();
    const payloadDigest = decoder.byteString(32);
    return {
        frameType,
        critical,
        transparent,
        content,
        lengthOffset,
        decodedLength,
        payloadDigest,
        encoded: decoder.bytes.subarray(start, decoder.offset),
    };
}

/** The bytes that begin the plain storage of a parity frame (`storage` of `[1]`). */
const PLAIN_STORAGE = Uint8Array.of(0x81, 0x01);
/** The bytes of an empty `signatures` list. */
const NO_SIGNATURES = Uint8Array.of(0x80);

function startsWithAt(
    data: Uint8Array,
    offset: number,
    bytes: Uint8Array,
): boolean {
    return bytes.every((byte, index) => data[offset + index] === byte);
}

export function decodeEnvelope(
    data: Uint8Array,
    offset: number,
    registryVersion: bigint,
    parityOnly: boolean,
): { frame: Frame; lengthOffset: number } {
    const decoder = new Decoder(data, offset);
    const newer = registryVersion > 1n;
    decoder.record(11);
    const kindError = decoder.error("unregistered_value");
    if (decoder.uint() !== FRAME) throw kindError;
    const identity = frameIdentity(decoder, newer);
    let error = decoder.error("forbidden_combination");
    if (
        parityOnly &&
        (identity.frameType !== PARITY ||
            !startsWithAt(data, decoder.offset, PLAIN_STORAGE))
    ) {
        throw error;
    }
    const storage = readStorage(decoder, newer, identity.decodedLength);
    if (identity.frameType === PARITY && storage.kind !== "plain") throw error;
    const storedPayload = decoder.byteString();
    const identifier = decoder.byteString(32);
    error = decoder.error("forbidden_combination");
    if (parityOnly && !startsWithAt(data, decoder.offset, NO_SIGNATURES)) {
        throw error;
    }
    const signatures = readSignatures(decoder, newer);
    if (identity.frameType === PARITY && signatures.length !== 0) throw error;
    return {
        frame: {
            frameType: identity.frameType,
            critical: identity.critical,
            transparent: identity.transparent,
            content: identity.content,
            decodedLength: identity.decodedLength,
            payloadDigest: identity.payloadDigest,
            storage,
            storedPayload,
            identifier,
            signatures,
            encodedSignaturesSize: decoder.offset - error.offset,
            identityFields: identity.encoded,
            nextOffset: decoder.offset,
        },
        lengthOffset: identity.lengthOffset,
    };
}

export function checkTypeLength(frame: Frame, offset: number): Frame {
    if (!validDecodedLength(frame.frameType, frame.decodedLength)) {
        throw new FrameError("out_of_range", offset);
    }
    return frame;
}

export function asFrameError<T>(read: () => T): T {
    try {
        return read();
    } catch (error) {
        if (error instanceof HeaderError) {
            throw new FrameError(error.reason, error.offset);
        }
        throw error;
    }
}

/**
 * Reads one deterministic frame envelope at a known item boundary. Links, payloads,
 * capacity and the meaning of the content are the reader's, after the envelope.
 *
 * @throws FrameError at the first malformed field, or at the decoded length when it
 * is not valid for the frame type.
 */
export function decodeFrame(
    data: Uint8Array,
    offset: number,
    registryVersion: bigint,
): Frame {
    const { frame, lengthOffset } = asFrameError(() =>
        decodeEnvelope(data, offset, registryVersion, false),
    );
    return checkTypeLength(frame, lengthOffset);
}

/**
 * Reads the envelope of a candidate parity frame at `offset` (P5 section 6.5): a frame of type
 * parity with plain storage and no signatures. Nothing else of the envelope is judged here.
 *
 * @throws FrameError at the first field that is not as the type states.
 */
export function decodeParityCandidate(
    data: Uint8Array,
    offset: number,
    registryVersion: bigint,
): { frame: Frame; lengthOffset: number } {
    return asFrameError(() =>
        decodeEnvelope(data, offset, registryVersion, true),
    );
}

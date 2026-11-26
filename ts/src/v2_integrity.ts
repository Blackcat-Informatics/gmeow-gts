// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { constants, zstdDecompressSync } from "node:zlib";
import { blake3 } from "@noble/hashes/blake3.js";

const encoder = new TextEncoder();
const framePrefix = Uint8Array.of(0x87, 0x58, 0x20);
const codecArray = Uint8Array.of(0x81);
const frameCapacity = 1n << 26n;
const readCapacity = 1n << 36n;
const maxUint64 = (1n << 64n) - 1n;

export function contextHash(
    context: string,
    ...parts: Uint8Array[]
): Uint8Array {
    return contextHashParts(context, parts);
}

export function contextHashParts(
    context: string,
    parts: Iterable<Uint8Array>,
): Uint8Array {
    const digest = blake3.create({ context: encoder.encode(context) });
    for (const part of parts) digest.update(part);
    return digest.digest();
}

function equals(left: Uint8Array, right: Uint8Array): boolean {
    return (
        left.length === right.length &&
        left.every((byte, index) => byte === right[index])
    );
}

export interface Storage {
    tag: bigint;
    codec?: bigint;
    encodedCodec?: Uint8Array;
    check?: Uint8Array;
}

export interface FrameBytes {
    identity?: Uint8Array;
    storedId?: Uint8Array;
    transparent: boolean;
    decodedLength?: bigint;
    payloadDigest?: Uint8Array;
    storage?: Storage;
    storedPayload?: Uint8Array;
    payloadChunks?: () => Iterable<Uint8Array>;
    content: Uint8Array;
    signaturesSize: number;
}

export interface FrameLink {
    state: "linked" | "healed" | "broken";
    computed?: Uint8Array;
    effective?: Uint8Array;
}

export function frameIdentifier(
    predecessor: Uint8Array,
    identity: Uint8Array,
): Uint8Array {
    return contextHash("gts2 frame", framePrefix, predecessor, identity);
}

function frameId(
    current: FrameBytes,
    predecessor: Uint8Array,
): Uint8Array | undefined {
    if (current.identity === undefined) return;
    return frameIdentifier(predecessor, current.identity);
}

export function checkLink(
    current: FrameBytes,
    predecessor: Uint8Array,
    next?: FrameBytes,
): FrameLink {
    const computed = frameId(current, predecessor);
    let state: FrameLink["state"] = "broken";
    if (computed !== undefined && current.storedId !== undefined) {
        if (equals(computed, current.storedId)) {
            state = "linked";
        } else if (!current.transparent && next?.storedId !== undefined) {
            const fromComputed = frameId(next, computed);
            if (
                fromComputed !== undefined &&
                equals(fromComputed, next.storedId)
            )
                state = "healed";
        }
    }
    const effective = current.transparent
        ? undefined
        : state === "healed"
          ? computed
          : (current.storedId ?? computed);
    return { state, computed, effective };
}

export function checkStored(frame: FrameBytes): boolean | undefined {
    const storage = frame.storage;
    if (
        storage?.encodedCodec === undefined ||
        storage.check === undefined ||
        frame.payloadChunks === undefined
    )
        return;
    const hash = blake3.create({ context: encoder.encode("gts2 stored") });
    hash.update(codecArray).update(storage.encodedCodec);
    for (const chunk of frame.payloadChunks()) {
        hash.update(chunk);
    }
    return equals(hash.digest().subarray(0, 16), storage.check);
}

type PayloadReason =
    | "stored_size"
    | "decoded_size"
    | "total_decoded"
    | "unknown_codec"
    | "codec_constraint"
    | "decode_failed"
    | "decoded_length"
    | "payload_digest";

export type PayloadCheck =
    | { state: "verified"; decoded?: Uint8Array }
    | {
          state: "damaged" | "unchecked";
          reason?: PayloadReason;
          count?: bigint;
      };

export class PayloadBudget {
    private total = 0n;
    private exhausted = false;

    /** Whether a charge has exhausted the total of the read (the scan charges against it). */
    get totalRefused(): boolean {
        return this.exhausted;
    }

    /**
     * Charges `length` decoded bytes to the total of the read. Returns the count
     * that the total refuses, or `undefined` when the charge is admitted.
     */
    charge(length: bigint): bigint | undefined {
        const total = this.total + length;
        if (this.exhausted || total > readCapacity) {
            this.exhausted = true;
            return total > maxUint64 ? maxUint64 : total;
        }
        this.total = total;
        return undefined;
    }

    check(frame: FrameBytes, includeDecoded = false): PayloadCheck {
        const { decodedLength, payloadDigest, storage, storedPayload } = frame;
        if (
            decodedLength === undefined ||
            payloadDigest === undefined ||
            storage === undefined ||
            storedPayload === undefined
        ) {
            return { state: "damaged" };
        }
        const refusal = this.admit(frame, decodedLength, storedPayload.length);
        if (refusal !== undefined) return refusal;
        const length = Number(decodedLength);
        let decoded = storedPayload;
        if (storage.tag === 1n) {
            if (decoded.length !== length)
                return { state: "damaged", reason: "decoded_length" };
        } else if (storage.tag === 2n && storage.codec === 1n) {
            const header = zstdHeader(length);
            if (
                length === 0 ||
                !equals(storedPayload.subarray(0, header.length), header)
            ) {
                return { state: "damaged", reason: "codec_constraint" };
            }
            const output = decodeZstd(storedPayload, header.length, length);
            if (output === undefined)
                return { state: "damaged", reason: "decode_failed" };
            decoded = output;
        } else {
            return { state: "unchecked", reason: "unknown_codec" };
        }
        if (!equals(contextHash("gts2 payload", decoded), payloadDigest)) {
            return { state: "damaged", reason: "payload_digest" };
        }
        return includeDecoded
            ? { state: "verified", decoded }
            : { state: "verified" };
    }

    private admit(
        frame: FrameBytes,
        length: bigint,
        storedSize: number,
    ): PayloadCheck | undefined {
        if (!this.exhausted) {
            for (const size of [
                storedSize,
                frame.content.length,
                frame.signaturesSize,
            ]) {
                if (BigInt(size) > frameCapacity) {
                    return {
                        state: "unchecked",
                        reason: "stored_size",
                        count: BigInt(size),
                    };
                }
            }
            if (length > frameCapacity) {
                return {
                    state: "unchecked",
                    reason: "decoded_size",
                    count: length,
                };
            }
        }
        const total = this.total + length;
        if (this.exhausted || total > readCapacity) {
            this.exhausted = true;
            return {
                state: "unchecked",
                reason: "total_decoded",
                count: total > maxUint64 ? maxUint64 : total,
            };
        }
        this.total = total;
    }
}

// The per-frame capacity admits only the three single-segment header forms.
export function zstdHeader(length: number): Uint8Array {
    const sizeWidth = length < 256 ? 1 : length <= 65791 ? 2 : 4;
    const descriptor = sizeWidth === 1 ? 0x20 : sizeWidth === 2 ? 0x60 : 0xa0;
    const encodedLength = sizeWidth === 2 ? length - 256 : length;
    const header = new Uint8Array(5 + sizeWidth);
    header.set([0x28, 0xb5, 0x2f, 0xfd, descriptor]);
    for (let byte = 0; byte < sizeWidth; byte++)
        header[5 + byte] = encodedLength >>> (8 * byte);
    return header;
}

// The native decoder accepts concatenated frames. Delimit the block stream only
// after the generated header has matched, to require exactly one complete frame.
function singleZstdFrame(data: Uint8Array, offset: number): boolean {
    while (offset + 3 <= data.length) {
        const block =
            data[offset] | (data[offset + 1] << 8) | (data[offset + 2] << 16);
        const kind = (block >> 1) & 3;
        if (kind === 3) return false;
        offset += 3 + (kind === 1 ? 1 : block >> 3);
        if (offset > data.length) return false;
        if ((block & 1) !== 0) return offset === data.length;
    }
    return false;
}

export function decodeZstd(
    data: Uint8Array,
    offset: number,
    length: number,
): Uint8Array | undefined {
    if (!singleZstdFrame(data, offset)) return;
    try {
        const output = zstdDecompressSync(data, {
            chunkSize: Math.max(64, Math.min(length, 16 * 1024)),
            maxOutputLength: length,
            params: { [constants.ZSTD_d_windowLogMax]: 26 },
        });
        if (output.length === length) return output;
    } catch (error) {
        if (!(error instanceof Error)) throw error;
        // The frame remains delimited. Report corrupt codec input at the payload.
        return;
    }
}

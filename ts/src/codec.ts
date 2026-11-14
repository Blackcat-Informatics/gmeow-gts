// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { gunzipSync, gzipSync } from "node:zlib";
import { decompress as zstdDecompress } from "fzstd";

/** A catalog entry (§5, §8.5). */
export interface Codec {
    name: string;
    /** "encode" | "compress" | "encrypt" */
    cls: string;
}

/** Reason a transform chain could not be reversed. */
export interface CodecError {
    /** Opaque-degradation reason such as "unknown-codec", "missing-key", or "damaged". */
    reason: string;
    /** Human-readable diagnostic detail. */
    detail: string;
    /** True when a known codec failed on corrupt bytes. */
    failed: boolean;
}

/** Convert a structured codec failure into an Error carrying the same fields. */
export function codecError(err: CodecError): Error {
    const e = new Error(err.detail) as Error & CodecError;
    e.reason = err.reason;
    e.detail = err.detail;
    e.failed = err.failed;
    return e;
}

/** True when an unknown thrown value is a structured codec Error. */
export function isCodecError(err: unknown): err is Error & CodecError {
    if (!(err instanceof Error)) return false;
    const e = err as Error & Record<string, unknown>;
    return (
        typeof e.reason === "string" &&
        typeof e.failed === "boolean" &&
        typeof e.detail === "string"
    );
}

/**
 * Largest zstd window, as a power of two, that this reader decodes (128 MiB).
 *
 * The zstd library used here (`fzstd`) returns wrong bytes, with no error, once a match
 * reaches back about 256 MiB, which a frame declaring a window of 2^29 or more allows.
 * It was measured to decode correctly up to a window of 2^27, the same bound as the
 * default decoder of the other reference readers, so a frame that declares more is
 * treated as one this reader cannot decode rather than risk returning miscomputed bytes.
 */
export const MAX_ZSTD_WINDOW_LOG = 27;

const ZSTD_MAGIC = 0xfd2fb528;
const ZSTD_MAX_BLOCK = 128 * 1024;
const ZSTD_DICT_ID_BYTES = [0, 1, 2, 4];
const ZSTD_CONTENT_SIZE_BYTES = [0, 2, 4, 8];

/**
 * Throw a codec error when a frame of a zstd payload declares a window above
 * {@link MAX_ZSTD_WINDOW_LOG}. Walks the frame and block headers without decoding.
 * Anything else wrong with the payload (not zstd, truncated) ends the walk quietly:
 * the zstd library reports it.
 */
export function assertZstdWindowSupported(data: Uint8Array): void {
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    const n = data.length;
    let pos = 0;
    while (pos + 4 <= n) {
        const magic = view.getUint32(pos, true);
        if (magic >= 0x184d2a50 && magic <= 0x184d2a5f) {
            if (pos + 8 > n) return;
            pos += 8 + view.getUint32(pos + 4, true);
            continue;
        }
        if (magic !== ZSTD_MAGIC || pos + 5 > n) return;
        const flags = data[pos + 4];
        pos += 5;
        const singleSegment = (flags & 0x20) !== 0;
        let window = 0;
        if (!singleSegment) {
            if (pos >= n) return;
            const base = 2 ** (10 + (data[pos] >> 3));
            window = base + (base / 8) * (data[pos] & 7);
            pos += 1;
        }
        pos += ZSTD_DICT_ID_BYTES[flags & 3];
        const sizeBytes =
            ZSTD_CONTENT_SIZE_BYTES[flags >> 6] || (singleSegment ? 1 : 0);
        if (pos + sizeBytes > n) return;
        if (singleSegment) {
            window = 0;
            for (let i = sizeBytes - 1; i >= 0; i--) {
                window = window * 256 + data[pos + i];
            }
            if (flags >> 6 === 1) window += 256;
        }
        pos += sizeBytes;
        if (window > 2 ** MAX_ZSTD_WINDOW_LOG) {
            throw codecError({
                reason: "damaged",
                detail:
                    `zstd frame declares a window of ${window} bytes, ` +
                    `above this reader's limit of 2**${MAX_ZSTD_WINDOW_LOG}`,
                failed: true,
            });
        }
        for (;;) {
            if (pos + 3 > n) return;
            const header = data[pos] | (data[pos + 1] << 8) | (data[pos + 2] << 16);
            const blockType = (header >> 1) & 3;
            const size = header >> 3;
            if (blockType === 3 || size > ZSTD_MAX_BLOCK) return;
            pos += 3 + (blockType === 1 ? 1 : size);
            if (pos > n) return;
            if (header & 1) break;
        }
        if (flags & 0x04) pos += 4;
    }
}

function toUint8Array(buf: Buffer): Uint8Array {
    return new Uint8Array(buf.buffer, buf.byteOffset, buf.byteLength);
}

function decodeOne(codec: Codec, data: Uint8Array): Uint8Array {
    if (codec.cls === "encrypt") {
        throw codecError({
            reason: "missing-key",
            detail: `no key for encrypt codec '${codec.name}'`,
            failed: false,
        });
    }
    switch (codec.name) {
        case "identity":
            return data;
        case "gzip": {
            try {
                return toUint8Array(gunzipSync(data));
            } catch (e) {
                throw codecError({
                    reason: "damaged",
                    detail: `gzip decode failed: ${(e as Error).message}`,
                    failed: true,
                });
            }
        }
        case "zstd":
        case "zstd-rsyncable": {
            try {
                assertZstdWindowSupported(data);
                return zstdDecompress(data);
            } catch (e) {
                if (isCodecError(e)) throw e;
                throw codecError({
                    reason: "damaged",
                    detail: `zstd decode failed: ${(e as Error).message}`,
                    failed: true,
                });
            }
        }
        default:
            throw codecError({
                reason: "unknown-codec",
                detail: `unknown codec '${codec.name}'`,
                failed: false,
            });
    }
}

/**
 * Reverse a resolved codec chain, last to first (§6.1, §8.2).
 * The baseline carries no keys, so every encrypt-class codec degrades to
 * missing-key (matching the Python reader with keys=None).
 */
export function decodeChain(chain: Codec[], data: Uint8Array): Uint8Array {
    let current = data;
    for (let i = chain.length - 1; i >= 0; i--) {
        current = decodeOne(chain[i], current);
    }
    return current;
}

/** Baseline codec helpers exposed for tests and future writers. */
export const identity = {
    name: "identity" as const,
    cls: "encode" as const,
    encode: (data: Uint8Array): Uint8Array => data,
    decode: (data: Uint8Array): Uint8Array => data,
};

export const gzip = {
    name: "gzip" as const,
    cls: "compress" as const,
    encode: (data: Uint8Array): Uint8Array => toUint8Array(gzipSync(data)),
    decode: (data: Uint8Array): Uint8Array => toUint8Array(gunzipSync(data)),
};

export const zstd = {
    name: "zstd" as const,
    cls: "compress" as const,
    encode: (_data: Uint8Array): Uint8Array => {
        throw codecError({
            reason: "unsupported",
            detail: "zstd encode is not implemented in the baseline TypeScript engine",
            failed: false,
        });
    },
    decode: (data: Uint8Array): Uint8Array => {
        assertZstdWindowSupported(data);
        return zstdDecompress(data);
    },
};

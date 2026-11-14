// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// A walk over the frame and block headers of a zstd payload. It imports nothing from
// Node, so the Node and the browser decoders share it.

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
const TRUNCATED = "zstd frame is truncated";

/**
 * Walk every frame of a zstd payload without decoding it and name the first reason this
 * reader will not decode it, or return null when the payload is whole frames it accepts.
 *
 * The verdict is the walk's, never the library's: the library accepts more than the
 * format does, so any structure the walk dislikes (unknown magic, truncation, a reserved
 * bit, an invalid or oversized block), a window above {@link MAX_ZSTD_WINDOW_LOG}, or a
 * dictionary (which no engine here implements) is a refusal. Damage inside a block is
 * not found here.
 */
export function zstdFrameProblem(data: Uint8Array): string | null {
    const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
    const n = data.length;
    let pos = 0;
    while (pos < n) {
        if (pos + 4 > n) return TRUNCATED;
        const magic = view.getUint32(pos, true);
        pos += 4;
        if (magic >= 0x184d2a50 && magic <= 0x184d2a5f) {
            if (pos + 4 > n) return TRUNCATED;
            pos += 4 + view.getUint32(pos, true);
            if (pos > n) return TRUNCATED;
            continue;
        }
        if (magic !== ZSTD_MAGIC) {
            return "payload is not a zstd frame (unknown magic number)";
        }
        if (pos >= n) return TRUNCATED;
        const flags = data[pos];
        pos += 1;
        if (flags & 0x08) return "zstd frame header sets a reserved bit";
        const singleSegment = (flags & 0x20) !== 0;
        let window = 0;
        if (!singleSegment) {
            if (pos >= n) return TRUNCATED;
            const base = 2 ** (10 + (data[pos] >> 3));
            window = base + (base / 8) * (data[pos] & 7);
            pos += 1;
        }
        const dictBytes = ZSTD_DICT_ID_BYTES[flags & 3];
        const sizeBytes =
            ZSTD_CONTENT_SIZE_BYTES[flags >> 6] || (singleSegment ? 1 : 0);
        if (pos + dictBytes + sizeBytes > n) return TRUNCATED;
        for (let i = 0; i < dictBytes; i++) {
            if (data[pos + i] !== 0) {
                return "zstd frame names a dictionary, which this reader does not implement";
            }
        }
        pos += dictBytes;
        if (singleSegment) {
            window = 0;
            for (let i = sizeBytes - 1; i >= 0; i--) {
                window = window * 256 + data[pos + i];
            }
            if (flags >> 6 === 1) window += 256;
        }
        pos += sizeBytes;
        if (window > 2 ** MAX_ZSTD_WINDOW_LOG) {
            return (
                `zstd frame declares a window of ${window} bytes, ` +
                `above this reader's limit of 2**${MAX_ZSTD_WINDOW_LOG}`
            );
        }
        for (;;) {
            if (pos + 3 > n) return TRUNCATED;
            const header = data[pos] | (data[pos + 1] << 8) | (data[pos + 2] << 16);
            const blockType = (header >> 1) & 3;
            const size = header >> 3;
            if (blockType === 3 || size > ZSTD_MAX_BLOCK) {
                return "zstd block header is invalid";
            }
            pos += 3 + (blockType === 1 ? 1 : size);
            if (pos > n) return TRUNCATED;
            if (header & 1) break;
        }
        if (flags & 0x04) {
            pos += 4;
            if (pos > n) return TRUNCATED;
        }
    }
    return null;
}

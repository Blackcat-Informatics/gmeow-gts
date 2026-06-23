// SPDX-FileCopyrightText: 2026 Blackcat Informatics(R) Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { Decoder, Tagged } from "cbor";
import { blake3 } from "@noble/hashes/blake3.js";

export const SELF_DESCRIBE_TAG = 55799;
export const MAGIC = "GTS1";
export const VERSION = 1;

export class BrowserWireError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "BrowserWireError";
    }
}

export function isHeaderItem(item: unknown): boolean {
    let inner = item;
    if (item instanceof Tagged) inner = item.value;
    if (!(inner instanceof Map)) return false;
    const hasGts = mapGet(inner, "gts") !== undefined;
    const hasT = mapGet(inner, "t") !== undefined;
    return hasGts && !hasT;
}

export function unwrapHeader(item: unknown): Map<unknown, unknown> {
    let inner = item;
    if (item instanceof Tagged) {
        if (item.tag !== SELF_DESCRIBE_TAG) {
            throw new BrowserWireError(
                `unexpected CBOR tag ${item.tag} on header item`,
            );
        }
        inner = item.value;
    }
    if (inner instanceof Map) return inner;
    throw new BrowserWireError("header item is not a CBOR map");
}

export function decodeFirst(data: Uint8Array): unknown {
    return Decoder.decodeFirstSync(data, { preferMap: true });
}

export function blake3_256(data: Uint8Array): Uint8Array {
    return blake3(data);
}

export function hex(data: Uint8Array): string {
    let out = "";
    for (const b of data) out += b.toString(16).padStart(2, "0");
    return out;
}

export function digestStr(data: Uint8Array): string {
    return "blake3:" + hex(blake3_256(data));
}

export function mapGet(
    m: Map<unknown, unknown> | undefined,
    key: string,
): unknown | undefined {
    if (!m) return undefined;
    if (m.has(key)) return m.get(key);
    for (const [k, v] of m) {
        if (k === key) return v;
    }
    return undefined;
}

export function asText(value: unknown): string | undefined {
    if (typeof value === "string") return value;
    return undefined;
}

export function asBytes(value: unknown): Uint8Array | undefined {
    if (value instanceof Uint8Array) {
        return copyBytes(value);
    }
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    return undefined;
}

export function asInt(value: unknown): number | undefined {
    if (typeof value === "number") {
        if (Number.isInteger(value) && value >= 0) return value;
    }
    if (typeof value === "bigint") {
        if (value >= 0n && value <= Number.MAX_SAFE_INTEGER)
            return Number(value);
    }
    return undefined;
}

export function asInt64(value: unknown): number | undefined {
    if (typeof value === "number") {
        if (Number.isInteger(value)) return value;
    }
    if (typeof value === "bigint") {
        if (
            value >= Number.MIN_SAFE_INTEGER &&
            value <= Number.MAX_SAFE_INTEGER
        ) {
            return Number(value);
        }
    }
    return undefined;
}

export function textOr(value: unknown, def: string): string {
    return asText(value) ?? def;
}

function cloneMap(m: Map<unknown, unknown>): Map<unknown, unknown> {
    return new Map(m);
}

function hashExcluding(
    m: Map<unknown, unknown>,
    excluded: string[],
): Uint8Array {
    const content = cloneMap(m);
    for (const k of excluded) {
        if (content.has(k)) content.delete(k);
    }
    return blake3_256(encodeCanonical(content));
}

export function contentId(frame: Map<unknown, unknown>): Uint8Array {
    return hashExcluding(frame, ["id", "sig"]);
}

export function headerId(header: Map<unknown, unknown>): Uint8Array {
    return hashExcluding(header, ["id"]);
}

export function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
    return true;
}

export function tripleEqual(
    a: { s: number; p: number; o: number },
    b: {
        s: number;
        p: number;
        o: number;
    },
): boolean {
    return a.s === b.s && a.p === b.p && a.o === b.o;
}

export function copyBytes(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
    return bytes.slice();
}

export function concatBytes(
    chunks: ReadonlyArray<Uint8Array<ArrayBufferLike>>,
): Uint8Array<ArrayBuffer> {
    let length = 0;
    for (const c of chunks) length += c.length;
    const out = new Uint8Array(length);
    let offset = 0;
    for (const c of chunks) {
        out.set(c, offset);
        offset += c.length;
    }
    return out;
}

export function toBufferSource(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
    if (
        bytes.byteOffset === 0 &&
        bytes.byteLength === bytes.buffer.byteLength
    ) {
        return bytes as Uint8Array<ArrayBuffer>;
    }
    return copyBytes(bytes);
}

export function encodeCanonical(value: unknown): Uint8Array {
    if (value === null) return new Uint8Array([0xf6]);
    if (value === undefined) return new Uint8Array([0xf7]);
    if (typeof value === "boolean")
        return new Uint8Array([value ? 0xf5 : 0xf4]);
    if (typeof value === "string") {
        const bytes = new TextEncoder().encode(value);
        return concatBytes([encodeMajor(3, BigInt(bytes.length)), bytes]);
    }
    if (typeof value === "number") {
        if (!Number.isInteger(value)) {
            throw new BrowserWireError(
                "canonical CBOR encoder only supports integer numbers",
            );
        }
        return encodeInteger(BigInt(value));
    }
    if (typeof value === "bigint") return encodeInteger(value);
    if (value instanceof ArrayBuffer) {
        const bytes = new Uint8Array(value);
        return concatBytes([encodeMajor(2, BigInt(bytes.length)), bytes]);
    }
    if (value instanceof Uint8Array) {
        const bytes = copyBytes(value);
        return concatBytes([encodeMajor(2, BigInt(bytes.length)), bytes]);
    }
    if (Array.isArray(value)) {
        const body = value.map((v) => encodeCanonical(v));
        return concatBytes([encodeMajor(4, BigInt(value.length)), ...body]);
    }
    if (value instanceof Map) {
        const entries = [...value.entries()].map(([k, v]) => ({
            key: encodeCanonical(k),
            value: encodeCanonical(v),
        }));
        entries.sort((a, b) => compareCborKeys(a.key, b.key));
        const pieces: Uint8Array[] = [encodeMajor(5, BigInt(entries.length))];
        for (const entry of entries) pieces.push(entry.key, entry.value);
        return concatBytes(pieces);
    }
    if (value instanceof Tagged) {
        return concatBytes([
            encodeMajor(6, BigInt(value.tag)),
            encodeCanonical(value.value),
        ]);
    }
    throw new BrowserWireError(
        `unsupported canonical CBOR value: ${typeof value}`,
    );
}

function encodeInteger(value: bigint): Uint8Array {
    if (value >= 0) return encodeMajor(0, value);
    return encodeMajor(1, -1n - value);
}

function encodeMajor(major: number, value: bigint): Uint8Array {
    if (value < 0n) throw new BrowserWireError("negative CBOR length");
    const prefix = major << 5;
    if (value <= 23n) return new Uint8Array([prefix | Number(value)]);
    if (value <= 0xffn) return new Uint8Array([prefix | 24, Number(value)]);
    if (value <= 0xffffn) {
        return new Uint8Array([
            prefix | 25,
            Number((value >> 8n) & 0xffn),
            Number(value & 0xffn),
        ]);
    }
    if (value <= 0xffffffffn) {
        return new Uint8Array([
            prefix | 26,
            Number((value >> 24n) & 0xffn),
            Number((value >> 16n) & 0xffn),
            Number((value >> 8n) & 0xffn),
            Number(value & 0xffn),
        ]);
    }
    if (value <= 0xffffffffffffffffn) {
        const out = new Uint8Array(9);
        out[0] = prefix | 27;
        let temp = value;
        for (let i = 0; i < 8; i++) {
            out[8 - i] = Number(temp & 0xffn);
            temp >>= 8n;
        }
        return out;
    }
    throw new BrowserWireError("CBOR integer exceeds uint64 range");
}

function compareCborKeys(a: Uint8Array, b: Uint8Array): number {
    if (a.length !== b.length) return a.length - b.length;
    for (let i = 0; i < a.length; i++) {
        if (a[i] !== b[i]) return a[i] - b[i];
    }
    return 0;
}

function readLength(
    data: Uint8Array,
    offset: number,
    info: number,
): { length: number; extra: number } {
    switch (info) {
        case 24:
            if (offset >= data.length)
                throw new BrowserWireError("unexpected EOF");
            return { length: data[offset], extra: 1 };
        case 25: {
            if (offset + 2 > data.length)
                throw new BrowserWireError("unexpected EOF");
            return { length: (data[offset] << 8) | data[offset + 1], extra: 2 };
        }
        case 26: {
            if (offset + 4 > data.length)
                throw new BrowserWireError("unexpected EOF");
            const n =
                (data[offset] << 24) |
                (data[offset + 1] << 16) |
                (data[offset + 2] << 8) |
                data[offset + 3];
            return { length: n >>> 0, extra: 4 };
        }
        case 27: {
            if (offset + 8 > data.length)
                throw new BrowserWireError("unexpected EOF");
            let n = 0n;
            for (let i = 0; i < 8; i++)
                n = (n << 8n) | BigInt(data[offset + i]);
            if (n > BigInt(Number.MAX_SAFE_INTEGER)) {
                throw new BrowserWireError("length exceeds safe integer range");
            }
            return { length: Number(n), extra: 8 };
        }
        default:
            throw new BrowserWireError(
                `unsupported additional info for length: ${info}`,
            );
    }
}

export function cborItemLength(data: Uint8Array, offset: number): number {
    if (offset >= data.length) throw new BrowserWireError("EOF");
    const start = offset;
    const stack: { remaining: number }[] = [];

    const complete = () => {
        while (stack.length > 0) {
            const top = stack[stack.length - 1];
            if (top.remaining > 0) top.remaining--;
            if (top.remaining === 0) stack.pop();
            else break;
        }
    };

    for (;;) {
        if (offset >= data.length) throw new BrowserWireError("unexpected EOF");
        const b = data[offset];
        const major = b >> 5;
        const info = b & 0x1f;
        offset++;

        let extra = 0;
        let length = -1;
        if (info <= 23) {
            length = info;
        } else if (info === 24 || info === 25 || info === 26 || info === 27) {
            if (major === 7) {
                extra = info === 24 ? 1 : info === 25 ? 2 : info === 26 ? 4 : 8;
                if (offset + extra > data.length) {
                    throw new BrowserWireError("unexpected EOF");
                }
            } else {
                const res = readLength(data, offset, info);
                length = res.length;
                extra = res.extra;
            }
        } else if (info >= 28 && info <= 30) {
            throw new BrowserWireError(`reserved additional info ${info}`);
        } else if (info === 31) {
            switch (major) {
                case 2:
                case 3:
                    for (;;) {
                        if (offset >= data.length)
                            throw new BrowserWireError("unexpected EOF");
                        const nb = data[offset];
                        if (nb === 0xff) {
                            offset++;
                            break;
                        }
                        const nmajor = nb >> 5;
                        const ninfo = nb & 0x1f;
                        if (nmajor !== major || ninfo === 31) {
                            throw new BrowserWireError(
                                "invalid indefinite string chunk",
                            );
                        }
                        let nlen: number;
                        let nextra = 0;
                        if (ninfo <= 23) nlen = ninfo;
                        else {
                            const res = readLength(data, offset + 1, ninfo);
                            nlen = res.length;
                            nextra = res.extra;
                        }
                        offset += 1 + nextra;
                        if (data.length - offset < nlen) {
                            throw new BrowserWireError("unexpected EOF");
                        }
                        offset += nlen;
                    }
                    complete();
                    if (stack.length === 0) return offset - start;
                    continue;
                case 4:
                case 5:
                    throw new BrowserWireError(
                        `indefinite-length ${major === 5 ? "map" : "array"} not supported`,
                    );
                default:
                    throw new BrowserWireError(
                        `indefinite length for major type ${major}`,
                    );
            }
        }

        offset += extra;

        switch (major) {
            case 0:
            case 1:
            case 7:
                complete();
                break;
            case 2:
            case 3:
                if (data.length - offset < length)
                    throw new BrowserWireError("unexpected EOF");
                offset += length;
                complete();
                break;
            case 4:
                if (length === 0) complete();
                else stack.push({ remaining: length });
                break;
            case 5:
                if (length === 0) complete();
                else stack.push({ remaining: length * 2 });
                break;
            case 6:
                stack.push({ remaining: 1 });
                break;
        }

        if (stack.length === 0) return offset - start;
    }
}

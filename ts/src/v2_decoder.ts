// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The value decoder of the v2 header items: a cursor over the bytes of an item that
//! reads each field of its grammar and reports the first refusal with its offset
//! (the `Decoder` of the reference reader).

/** Why a value does not have its grammar (P1, the typed refusals of the decoder). */
export type HeaderReason =
    | "wrong_type"
    | "wrong_length"
    | "not_deterministic"
    | "out_of_range"
    | "unregistered_value"
    | "inconsistent_counts"
    | "unordered_or_repeated";

/** The first refusal of a decoder: its reason and the offset where it stands. */
export class HeaderError extends Error {
    constructor(
        readonly reason: HeaderReason,
        readonly offset: number,
    ) {
        super(reason);
    }
}

/** The smallest argument that each head width encodes, for the widths of 1, 2, 4 and 8 bytes. */
/** The registry code of the BLAKE3-256 hash. */
export const BLAKE3_256 = 1n;

/** The largest value the head of a code can carry. */
const U64_MAX = (1n << 64n) - 1n;

const SHORTEST = [24n, 256n, 65536n, 4294967296n];

/** A cursor over `bytes` that reads from `offset` (the `Decoder` of the reference reader). */
export class Decoder {
    /** The offset of the next byte to read. */
    offset: number;
    /** The offset at which a field may no longer start, when the reader sets one. */
    fieldStartLimit: number | undefined;

    constructor(
        readonly bytes: Uint8Array,
        offset: number,
    ) {
        this.offset = offset;
        this.fieldStartLimit = undefined;
    }

    /** A refusal of `reason` at the current offset. */
    error(reason: HeaderReason): HeaderError {
        return new HeaderError(reason, this.offset);
    }

    /** The next `size` bytes, consumed. */
    take(size: number): Uint8Array {
        const end = this.offset + size;
        if (!Number.isSafeInteger(end) || end > this.bytes.length) {
            throw this.error("wrong_length");
        }
        const value = this.bytes.subarray(this.offset, end);
        this.offset = end;
        return value;
    }

    /** The argument of the head at the cursor, which must have major type `expected`. */
    head(expected: number): bigint {
        const limit = this.fieldStartLimit;
        if (limit !== undefined && this.offset >= limit) {
            throw new HeaderError("out_of_range", limit);
        }
        const start = this.offset;
        const fail = (reason: HeaderReason): HeaderError =>
            new HeaderError(reason, start);
        const first = this.take(1)[0];
        const major = first >> 5;
        const additional = first & 31;
        let argument: bigint;
        if (additional <= 23) {
            argument = BigInt(additional);
        } else if (additional <= 27) {
            const width = additional - 24;
            let value = 0n;
            for (const byte of this.take(1 << width)) {
                value = (value << 8n) | BigInt(byte);
            }
            if (major <= 4 && value < SHORTEST[width]) {
                throw fail("not_deterministic");
            }
            if (major === 7 && additional === 24 && value < 32n) {
                throw fail("wrong_length");
            }
            argument = value;
        } else if (additional === 31 && major >= 2 && major <= 4) {
            throw fail("not_deterministic");
        } else if (additional === 31 && major === 5) {
            throw fail("wrong_type");
        } else {
            throw fail("wrong_length");
        }
        if (major !== expected) throw fail("wrong_type");
        return argument;
    }

    /** A record of `arity` fields: the head of an array of that length. */
    record(arity: number): void {
        const error = this.error("wrong_length");
        if (this.head(4) !== BigInt(arity)) throw error;
    }

    /** An unsigned integer. */
    uint(): bigint {
        return this.head(0);
    }

    /** A code of an enumeration of `last` codes, from 1 to `last`. */
    code(last: number | bigint): bigint {
        const error = this.error("unregistered_value");
        const value = this.uint();
        if (value < 1n || value > BigInt(last)) throw error;
        return value;
    }

    /**
     * An optional field: `[1]` is absent and `[2, value]` is present (the `option_tag`
     * of P2). Any other array is refused.
     */
    option<T>(read: (decoder: Decoder) => T): T | undefined {
        const error = this.error("wrong_length");
        const arity = this.head(4);
        if (arity === 0n) throw error;
        const tag = this.code(2);
        if (tag !== arity) throw error;
        return tag === 2n ? read(this) : undefined;
    }

    /** The next byte, not consumed. */
    peekByte(): number {
        const byte = this.take(1)[0];
        this.offset -= 1;
        return byte;
    }

    /** A positive unsigned integer. */
    positive(): bigint {
        const error = this.error("out_of_range");
        const value = this.uint();
        if (value === 0n) throw error;
        return value;
    }

    /** A text string; invalid UTF-8 is a wrong type. */
    text(): string {
        const lengthError = this.error("wrong_length");
        const typeError = this.error("wrong_type");
        const length = this.head(3);
        if (length > BigInt(Number.MAX_SAFE_INTEGER)) throw lengthError;
        try {
            return new TextDecoder("utf-8", { fatal: true }).decode(
                this.take(Number(length)),
            );
        } catch (error) {
            if (error instanceof HeaderError) throw error;
            throw typeError;
        }
    }

    /** A boolean: the one-byte simple values false and true. */
    boolean(): boolean {
        const error = this.error("wrong_type");
        const code = this.head(7);
        if (this.offset - error.offset === 1) {
            if (code === 20n) return false;
            if (code === 21n) return true;
        }
        throw error;
    }

    /** One complete item of any shape built from the closed set of types the headers use. */
    closedItem(): void {
        let pending = 1n;
        while (pending !== 0n) {
            const major = this.peekByte() >> 5;
            pending -= 1n;
            switch (major) {
                case 0:
                case 1:
                    this.head(major);
                    break;
                case 2:
                    this.byteString();
                    break;
                case 3:
                    this.text();
                    break;
                case 4:
                    pending += this.head(4);
                    break;
                case 7:
                    this.boolean();
                    break;
                default:
                    this.head(0);
                    return;
            }
        }
    }

    /** The registry version, hash algorithm and optional profile that open a header. */
    openerFields(knownHashOnly: boolean): {
        registryVersion: bigint;
        hashAlgorithm: bigint;
        profile: bigint | undefined;
    } {
        const registryVersion = this.positive();
        const hashAlgorithm = this.code(
            knownHashOnly || registryVersion === 1n ? BLAKE3_256 : U64_MAX,
        );
        const profile = this.option((d) => d.code(U64_MAX));
        return { registryVersion, hashAlgorithm, profile };
    }

    /** A claimed previous head and lineage ordinal. */
    lineage(knownHash: boolean): { previousHead: Uint8Array; ordinal: bigint } {
        this.record(2);
        const previousHead = this.byteString(knownHash ? 32 : undefined);
        return { previousHead, ordinal: this.positive() };
    }

    /** A byte string, whose length must be `expected` when that is given. */
    byteString(expected?: number): Uint8Array {
        const error = this.error("wrong_length");
        const length = this.head(2);
        if (length > BigInt(Number.MAX_SAFE_INTEGER)) throw error;
        const size = Number(length);
        if (expected !== undefined && expected !== size) throw error;
        return this.take(size);
    }
}

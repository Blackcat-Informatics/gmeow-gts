// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { isUtf8 } from "node:buffer";
import { FramingError, head } from "./v2_framing.js";

export class RdfProjectionError extends Error {}

export function requireRdf(ok: unknown, reason: string): asserts ok {
    if (!ok) throw new RdfProjectionError(reason);
}

const decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });
const maxUint64 = (1n << 64n) - 1n;

export function decodeRdfText(bytes: Uint8Array): string {
    requireRdf(isUtf8(bytes), "wrong_type");
    return decoder.decode(bytes);
}

export class RdfCursor {
    private offset = 0;

    constructor(private readonly data: Uint8Array) {}

    private head(major: number, signed = false): bigint {
        let value;
        try {
            value = head(this.data, this.offset);
        } catch (error) {
            if (!(error instanceof FramingError)) throw error;
            throw new RdfProjectionError("wrong_length");
        }
        requireRdf(value.major > 4 || value.shortest, "not_deterministic");
        requireRdf(
            value.major === major || (signed && value.major === 1),
            "wrong_type",
        );
        this.offset = value.end;
        return value.major === 1 ? -1n - value.value : value.value;
    }

    end(): void {
        requireRdf(this.offset === this.data.length, "wrong_length");
    }

    array(): bigint {
        return this.head(4);
    }

    record(arity: number): void {
        requireRdf(this.array() === BigInt(arity), "wrong_length");
    }

    list<T>(read: () => T): T[] {
        const count = this.array();
        const result: T[] = [];
        for (let i = 0n; i < count; i++) result.push(read());
        return result;
    }

    uint(): bigint {
        return this.head(0);
    }

    int(): bigint {
        return this.head(0, true);
    }

    bool(): boolean {
        const octet = this.data[this.offset];
        this.head(7);
        requireRdf(octet === 0xf4 || octet === 0xf5, "wrong_type");
        return octet === 0xf5;
    }

    span(read: () => void): Uint8Array {
        const start = this.offset;
        read();
        return this.data.subarray(start, this.offset);
    }

    positive(): bigint {
        const value = this.uint();
        requireRdf(value > 0n, "out_of_range");
        return value;
    }

    code(max: bigint): number {
        const value = this.uint();
        requireRdf(value > 0n && value <= max, "unregistered_value");
        return Number(value);
    }

    body(major: 2 | 3): Uint8Array {
        const length = this.head(major);
        requireRdf(
            length <= BigInt(this.data.length - this.offset),
            "wrong_length",
        );
        const bytes = this.data.subarray(
            this.offset,
            this.offset + Number(length),
        );
        this.offset += bytes.length;
        return bytes;
    }

    text(): string {
        return decodeRdfText(this.body(3));
    }

    deltas(minimum = 0n): bigint[] {
        let previous = 0n;
        return this.list(() => {
            previous += this.int();
            requireRdf(
                previous >= minimum && previous <= maxUint64,
                "out_of_range",
            );
            return previous;
        });
    }
}

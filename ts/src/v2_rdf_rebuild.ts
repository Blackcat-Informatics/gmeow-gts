// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer, isUtf8 } from "node:buffer";
import { contextHash, contextHashParts } from "./v2_integrity.js";
import { RdfCursor, requireRdf } from "./v2_rdf_cursor.js";
import { hexLexical } from "./v2_rdf_values.js";

const contentContext = "gts2 unit";
const rebuiltLimit = 1n << 37n;
const maxUint64 = (1n << 64n) - 1n;
const memberKinds = [8, 9, 10, 11] as const;
type Kind = (typeof memberKinds)[number];

interface Reference {
    digest: string;
    length: bigint;
}

interface RecordFields extends Reference {
    kind: Kind;
    count: bigint;
}

function readRecord(cursor: RdfCursor, kind: Kind): RecordFields {
    cursor.record(kind === 8 ? 1 : kind === 9 ? 2 : kind === 10 ? 4 : 3);
    const digest = cursor.digest();
    const length = kind >= 10 ? cursor.uint() : 0n;
    if (kind === 9 || kind === 10) {
        const arity = cursor.array();
        requireRdf(arity > 0n, "wrong_length");
        const option = cursor.code(2n);
        requireRdf(arity === BigInt(option), "wrong_length");
        if (option === 2) requireRdf(isUtf8(cursor.body(3)), "wrong_type");
    }
    const count = kind >= 10 ? cursor.uint() : 0n;
    return { kind, digest, length, count };
}

function readEntries(record: RecordFields, body: Uint8Array): Reference[] {
    requireRdf(record.count >= (record.kind === 10 ? 2n : 1n), "out_of_range");
    const cursor = new RdfCursor(body);
    const entries = cursor.list(() => {
        cursor.record(2);
        return { digest: cursor.digest(), length: cursor.uint() };
    });
    cursor.end();
    requireRdf(BigInt(entries.length) === record.count, "inconsistent_counts");
    let sum = 0n;
    for (const entry of entries) {
        sum += entry.length;
        requireRdf(entry.length > 0n && sum <= maxUint64, "out_of_range");
    }
    requireRdf(sum === record.length, "inconsistent_counts");
    return entries;
}

function digestHex(bytes: Uint8Array): string {
    return Buffer.from(
        bytes.buffer,
        bytes.byteOffset,
        bytes.byteLength,
    ).toString("hex");
}

function utf8Width(octet: number): number {
    if (octet >= 0xc2 && octet <= 0xdf) return 2;
    if (octet >= 0xe0 && octet <= 0xef) return 3;
    if (octet >= 0xf0 && octet <= 0xf4) return 4;
    return 0;
}

function validUtf8(parts: readonly Uint8Array[]): boolean {
    const tail = new Uint8Array(4);
    let held = 0;
    for (const bytes of parts) {
        let part = bytes;
        if (held > 0) {
            const width = utf8Width(tail[0]);
            const take = Math.min(width - held, part.length);
            tail.set(part.subarray(0, take), held);
            held += take;
            if (held < width) continue;
            if (!isUtf8(tail.subarray(0, width))) return false;
            held = 0;
            part = part.subarray(take);
        }
        let end = part.length;
        for (let at = end - 1; at >= 0 && end - at < 4; at--) {
            if ((part[at] & 0xc0) === 0x80) continue;
            if (utf8Width(part[at]) > end - at) {
                held = end - at;
                tail.set(part.subarray(at));
                end = at;
            }
            break;
        }
        if (!isUtf8(part.subarray(0, end))) return false;
    }
    return held === 0;
}

export class RebuiltValue {
    readonly key: string;
    private text?: string;
    private octets?: string;
    private utf8?: boolean;

    constructor(
        readonly length: bigint,
        digest: string,
        private readonly parts: Uint8Array[],
    ) {
        this.key = `${digest}/${length}`;
    }

    checkText(): void {
        this.utf8 ??= validUtf8(this.parts);
        requireRdf(this.utf8, "ill_formed_term");
    }

    lexical(hex: boolean, charge: (size: bigint) => void): string {
        const cached = hex ? this.octets : this.text;
        if (cached !== undefined) return cached;
        charge(this.length * (hex ? 2n : 1n));
        if (hex) {
            this.octets = this.parts.map(hexLexical).join("");
            return this.octets;
        }
        const decoder = new TextDecoder("utf-8", {
            fatal: true,
            ignoreBOM: true,
        });
        const chunks = this.parts.map((part) =>
            decoder.decode(part, { stream: true }),
        );
        chunks.push(decoder.decode());
        this.text = chunks.join("");
        return this.text;
    }
}

type Candidate = RebuiltValue | { reason: string };

export class RdfRebuild {
    private readonly units = new Map<string, Uint8Array>();
    private readonly values = new Map<string, Candidate>();
    private charged = 0n;
    private refused = false;
    private textRemaining = 1n << 28n;

    open(): void {
        this.units.clear();
    }

    value(digest: string, length: bigint): Candidate | undefined {
        return this.values.get(`${digest}/${length}`);
    }

    lexical(candidate: RebuiltValue, hex: boolean): string {
        return candidate.lexical(hex, (size) => {
            requireRdf(size <= this.textRemaining, "projection_value_limit");
            this.textRemaining -= size;
        });
    }

    read(type: bigint, content: Uint8Array, payload: Uint8Array): void {
        if (type === 13n) {
            this.readPack(content, payload);
            return;
        }
        const cursor = new RdfCursor(content);
        const record = readRecord(cursor, memberKinds[Number(type - 8n)]);
        cursor.end();
        if (record.kind < 10) {
            this.readUnit(record.digest, payload);
        } else {
            this.declare(record, readEntries(record, payload));
        }
    }

    private readUnit(digest: string, payload: Uint8Array): void {
        if (this.units.has(digest)) return;
        if (digestHex(contextHash(contentContext, payload)) === digest) {
            this.units.set(digest, payload);
        }
    }

    private declare(record: RecordFields, entries: Reference[]): void {
        const admitted =
            !this.refused && record.length <= rebuiltLimit - this.charged;
        if (admitted) this.charged += record.length;
        else this.refused = true;

        // Recipes consume the same reconstruction budget as values.
        if (record.kind !== 11) return;
        const key = `${record.digest}/${record.length}`;
        const previous = this.values.get(key);
        if (previous instanceof RebuiltValue) return;
        const candidate = admitted
            ? this.rebuild(record, entries)
            : { reason: "total_rebuilt" };
        if (previous === undefined || candidate instanceof RebuiltValue)
            this.values.set(key, candidate);
    }

    private rebuild(record: RecordFields, entries: Reference[]): Candidate {
        const parts: Uint8Array[] = [];
        for (const entry of entries) {
            const unit = this.units.get(entry.digest);
            if (unit === undefined) return { reason: "chunk" };
            if (BigInt(unit.length) !== entry.length)
                return { reason: "blob_length" };
            parts.push(unit);
        }
        if (
            digestHex(contextHashParts(contentContext, parts)) !== record.digest
        ) {
            return { reason: "blob_digest" };
        }
        return new RebuiltValue(record.length, record.digest, parts);
    }

    private readPack(content: Uint8Array, payload: Uint8Array): void {
        const fields = new RdfCursor(content);
        fields.record(2);
        const count = fields.positive();
        const unitLength = fields.uint();
        fields.end();

        const cursor = new RdfCursor(payload);
        cursor.record(2);
        const members = cursor.list(() => {
            const arity = cursor.array();
            requireRdf(arity > 0n, "wrong_length");
            const tag = cursor.code(4n);
            requireRdf(arity === 3n, "wrong_length");
            return {
                record: readRecord(cursor, memberKinds[tag - 1]),
                span: cursor.uint(),
            };
        });
        const bytes = cursor.body(2);
        cursor.end();
        requireRdf(BigInt(members.length) === count, "inconsistent_counts");
        let offset = 0;
        let unitBytes = 0n;
        const bodies = members.map(({ record, span }) => {
            requireRdf(
                span <= BigInt(bytes.length - offset),
                "inconsistent_counts",
            );
            const body = bytes.subarray(offset, offset + Number(span));
            offset += body.length;
            if (record.kind < 10) unitBytes += span;
            return body;
        });
        requireRdf(
            offset === bytes.length && unitBytes === unitLength,
            "inconsistent_counts",
        );

        const entries = members.map(({ record }, index) => {
            const body = bodies[index];
            if (record.kind === 8)
                requireRdf(body.length !== 0, "out_of_range");
            return record.kind >= 10 ? readEntries(record, body) : undefined;
        });
        // All units in a pack share its file position, including units whose
        // directory entry follows a declaration that uses them.
        for (const [index, { record }] of members.entries()) {
            if (record.kind < 10) this.readUnit(record.digest, bodies[index]);
        }
        for (const [index, { record }] of members.entries()) {
            const parts = entries[index];
            if (parts !== undefined) this.declare(record, parts);
        }
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer } from "node:buffer";
import {
    frameContentProblem,
    framePayloadLengthProblem,
} from "./v2_framing.js";
import { contextHash, frameIdentifier } from "./v2_integrity.js";
import { mustEncode } from "./wire.js";

export interface V2SegmentOptions {
    profile?: number;
    canonical?: boolean;
    lineage?: { previousHead: Uint8Array; ordinal: bigint };
}

interface RangePeak {
    height: number;
    digest: Uint8Array;
}

const maxUint64 = (1n << 64n) - 1n;
const frameHeader = Uint8Array.of(0x8b, 0x02);
const plainStorage = mustEncode([1]);
const noSignatures = mustEncode([]);
const emptyPayload = new Uint8Array();

function encodeHeader(options: V2SegmentOptions): Uint8Array {
    const profile = options.profile ?? 0;
    if (!Number.isInteger(profile) || profile < 0 || profile > 6) {
        throw new RangeError(`invalid GTS v2 profile ${profile}`);
    }
    const lineage = options.lineage;
    if (
        lineage !== undefined &&
        (!(lineage.previousHead instanceof Uint8Array) ||
            lineage.previousHead.byteLength !== 32 ||
            typeof lineage.ordinal !== "bigint" ||
            lineage.ordinal <= 0n ||
            lineage.ordinal > maxUint64)
    ) {
        throw new RangeError(
            "GTS v2 lineage requires a 32-byte digest and a positive uint64 ordinal",
        );
    }
    return mustEncode([
        1,
        2,
        1,
        1,
        profile === 0 ? [1] : [2, profile],
        options.canonical === true ? [2, 1] : [1],
        lineage === undefined
            ? [1]
            : [2, [lineage.previousHead, lineage.ordinal]],
    ]);
}

function byteStringHead(length: number): Uint8Array {
    const encoded = mustEncode(length);
    // A byte string uses the same length argument as an unsigned integer.
    encoded[0] |= 0x40;
    return encoded;
}

/** Writes a registry version 1 segment with plain, unsigned frames. */
export class V2SegmentWriter {
    readonly #items: Uint8Array[];
    readonly #peaks: RangePeak[] = [];
    #head: Uint8Array;
    #leafCount = 0n;

    constructor(options: V2SegmentOptions = {}) {
        const header = encodeHeader(options);
        this.#items = [header];
        this.#head = contextHash("gts2 header", header);
        this.#pushLeaf(this.#head);
    }

    /** A copy of the current ordinary head identifier. */
    get head(): Uint8Array {
        return this.#head.slice();
    }

    /** An independent snapshot of the encoded segment. */
    bytes(): Uint8Array {
        return Buffer.concat(this.#items);
    }

    /**
     * Append an encoded content record and its decoded payload. The caller
     * supplies the content semantics, including any parity placement rules.
     */
    append(type: number, content: Uint8Array, payload: Uint8Array): Uint8Array {
        if (type === 14)
            throw new RangeError("use commit() to append a GTS v2 commit");
        if (!Number.isInteger(type) || type < 1 || type > 16) {
            throw new RangeError(`invalid GTS v2 frame type ${type}`);
        }
        if (
            !(content instanceof Uint8Array) ||
            !(payload instanceof Uint8Array)
        ) {
            throw new TypeError(
                "GTS v2 content and payload must be byte arrays",
            );
        }
        const ownedContent = Buffer.from(content);
        const problem = frameContentProblem(BigInt(type), ownedContent);
        if (problem !== undefined) {
            throw new TypeError(
                `GTS v2 frame type ${type} content: ${problem}`,
            );
        }
        const stored = Buffer.from(payload);
        const lengthProblem = framePayloadLengthProblem(
            BigInt(type),
            BigInt(stored.length),
        );
        if (lengthProblem !== undefined) {
            throw new RangeError(
                `GTS v2 frame type ${type} payload length: ${lengthProblem}`,
            );
        }
        return this.#append(type, ownedContent, stored);
    }

    /** Commit the opener and preceding ordinary frames, then advance the head. */
    commit(): Uint8Array {
        const root = contextHash(
            "gts2 range root",
            mustEncode([
                this.#leafCount,
                this.#peaks.map((peak) => peak.digest),
            ]),
        );
        return this.#append(
            14,
            mustEncode([this.#leafCount - 1n, root]),
            emptyPayload,
        );
    }

    #append(
        type: number,
        content: Uint8Array,
        payload: Uint8Array,
    ): Uint8Array {
        const transparent = type === 16;
        if (!transparent && this.#leafCount === maxUint64) {
            throw new RangeError(
                "GTS v2 segment exceeds the uint64 leaf limit",
            );
        }
        const identity = Buffer.concat([
            mustEncode(type),
            mustEncode(type < 14),
            mustEncode(transparent),
            content,
            mustEncode(payload.length),
            mustEncode(contextHash("gts2 payload", payload)),
        ]);
        const id = frameIdentifier(this.#head, identity);
        const item = Buffer.concat([
            frameHeader,
            identity,
            plainStorage,
            byteStringHead(payload.length),
            payload,
            mustEncode(id),
            noSignatures,
        ]);
        if (!transparent) {
            this.#pushLeaf(id);
            this.#head = id;
        }
        this.#items.push(item);
        return id.slice();
    }

    #pushLeaf(id: Uint8Array): void {
        const peak = {
            height: 0,
            digest: contextHash("gts2 range leaf", mustEncode([id])),
        };
        let count = this.#peaks.length;
        while (this.#peaks[count - 1]?.height === peak.height) {
            const left = this.#peaks[--count];
            peak.digest = contextHash(
                "gts2 range node",
                mustEncode([left.digest, peak.digest]),
            );
            peak.height++;
        }
        this.#peaks.length = count;
        this.#peaks.push(peak);
        this.#leafCount++;
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer } from "node:buffer";
import { contextHash } from "./v2_integrity.js";
import { mustEncode } from "./wire.js";

const unitBytes = 256 * 1024;
const packBodyBytes = 16 * 1024 * 1024;
const unitContext = "gts2 unit";

export interface ValueReference {
    digest: Uint8Array;
    length: number;
}

type Member =
    | [tag: 1, content: [digest: Uint8Array], length: number]
    | [
          tag: 4,
          content: [digest: Uint8Array, total: number, entries: number],
          length: number,
      ];

/** Emit plain packs before the terms that reference their values. */
export function writeValuePacks(
    values: Iterable<string>,
    append: (content: Uint8Array, payload: Uint8Array) => void,
): Map<string, ValueReference> {
    const references = new Map<string, ValueReference>();
    const units = new Set<string>();
    let directory: Member[] = [];
    let bodies: Uint8Array[] = [];
    let bodyBytes = 0;
    let storedUnits = 0;

    const flush = (): void => {
        if (directory.length === 0) return;
        append(
            mustEncode([directory.length, storedUnits]),
            mustEncode([directory, Buffer.concat(bodies, bodyBytes)]),
        );
        directory = [];
        bodies = [];
        bodyBytes = 0;
        storedUnits = 0;
    };

    const push = (member: Member, body: Uint8Array): void => {
        if (bodyBytes + body.length > packBodyBytes) flush();
        directory.push(member);
        bodies.push(body);
        bodyBytes += body.length;
        if (member[0] === 1) storedUnits += body.length;
    };

    for (const value of values) {
        if (references.has(value)) continue;
        if (value.length === 0)
            throw new TypeError("A packed RDF literal value cannot be empty");
        const bytes = Buffer.from(value, "utf8");
        const entries: [Uint8Array, number][] = [];
        for (let offset = 0; offset < bytes.length; offset += unitBytes) {
            const unit = bytes.subarray(offset, offset + unitBytes);
            const id = contextHash(unitContext, unit);
            entries.push([id, unit.length]);
            const key = Buffer.from(id).toString("hex");
            if (units.has(key)) continue;
            push([1, [id], unit.length], unit);
            units.add(key);
        }
        const digest = contextHash(unitContext, bytes);
        const body = mustEncode(entries);
        push([4, [digest, bytes.length, entries.length], body.length], body);
        references.set(value, { digest, length: bytes.length });
    }
    flush();
    return references;
}

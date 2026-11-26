// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { test } from "node:test";
import {
    decodeReadResult,
    encodeReadResult,
    type ReadResult,
} from "../src/v2_read_result.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const hex = (value: string): Uint8Array =>
    Uint8Array.from(Buffer.from(value, "hex"));

const recorded = readdirSync(P7, { recursive: true })
    .map(String)
    .filter((name) => name.endsWith(".json"))
    .flatMap((name) => {
        const entry = JSON.parse(readFileSync(new URL(name, P7), "utf8"));
        return (entry.cases ?? [])
            .filter(
                (c: { read_result?: unknown }) =>
                    typeof c.read_result === "string",
            )
            .map((c: { read_result: string }) => ({
                name,
                bytes: hex(c.read_result),
            }));
    });

test("every recorded read_result decodes and encodes back to its own bytes", () => {
    assert.ok(recorded.length > 0, "the corpus holds recorded read_results");
    for (const { name, bytes } of recorded) {
        const result = decodeReadResult(bytes);
        assert.deepEqual(encodeReadResult(result), bytes, name);
    }
});

test("a label outside its enumeration is refused, never encoded as a code", () => {
    const [{ bytes }] = recorded.filter(
        (r) => r.name === "canonical-no-resync.json",
    );
    const result: ReadResult = decodeReadResult(bytes);
    assert.equal(result.file.tag, "second_generation");
    if (result.file.tag !== "second_generation") return;
    const [segment] = result.file.segments;
    const bogus: ReadResult = {
        ...result,
        file: {
            ...result.file,
            segments: [
                { ...segment, integrity: "unknown_state" as never },
                ...result.file.segments.slice(1),
            ],
        },
    };
    assert.throws(() => encodeReadResult(bogus), /not in its enumeration/);
});

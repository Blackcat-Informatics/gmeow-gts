// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { test } from "node:test";
import { encodeParityFindings } from "../src/v2_parity_findings.js";
import type { ReadCapacity } from "../src/v2_payload.js";
import { readWire, type WireRead } from "../src/v2_segment.js";

const P5 = new URL("../../../vectors/v2/P5/", import.meta.url);

const bytes = (name: string) => new Uint8Array(readFileSync(new URL(name, P5)));

const hex = (value: Uint8Array) => Buffer.from(value).toString("hex");

function findingsOf(name: string, capacity?: ReadCapacity): string[] {
    const read = readWire(bytes(name), capacity);
    assert.equal(read.segments.length, 1, name);
    return encodeParityFindings(read.segments[0].parityFindings).map(hex);
}

const TAIL_AFTER_OPENER = "182481018101810182020b820219" + "01d681" + "01";

test("a damaged parity frame gives parity_damaged with its position alone, then the tail", () => {
    assert.deepEqual(findingsOf("parity-damaged-each.3.bin"), [
        "181e810181018202048101810181 01".replace(/ /g, ""),
        TAIL_AFTER_OPENER,
    ]);
    assert.deepEqual(findingsOf("parity-base.bin"), []);
});

test("an unexplained shard is reported and the span is repaired around it", () => {
    assert.deepEqual(findingsOf("parity-unexplained.bin"), [
        "181f8202182881018202048101810182 0202".replace(/ /g, ""),
        "18208101810182020481018101820201",
        "18238101810182020481018101820201",
    ]);
});

test("a repaired copy is the input with the accepted repair written in place", () => {
    const read = readWire(bytes("recover-no-signature.signature-00-3f.bin"));
    assert.deepEqual(
        read.repairedCopy,
        bytes("recover-no-signature.signature-00-3f.repaired.bin"),
    );
    assert.equal(readWire(bytes("parity-base.bin")).repairedCopy, undefined);
});

test("a repair that fails the run recheck is repair_failed with verify_failed and no copy", () => {
    const read = readWire(bytes("recover-verify-failed.no-repaired-copy.bin"));
    assert.equal(read.repairedCopy, undefined);
    assert.ok(
        read.segments.some((segment) =>
            segment.parityFindings.some(
                (finding) =>
                    finding.code === "repair_failed" &&
                    finding.reason === "verify_failed",
            ),
        ),
    );
});

function capacityOf(raw: Record<string, number>): ReadCapacity {
    return {
        storedBytesPerFrame: BigInt(raw.stored_bytes_per_frame),
        decodedBytesPerFrame: BigInt(raw.decoded_bytes_per_frame),
        decodedBytesPerRead: BigInt(raw.decoded_bytes_per_read),
        itemsPerRead: BigInt(raw.items_per_read),
    };
}

function contains(haystack: Uint8Array, needle: Uint8Array): boolean {
    return Buffer.from(haystack).includes(Buffer.from(needle));
}

test("every parity finding of a corpus vector stands in its expected read result", () => {
    let checked = 0;
    for (const file of readdirSync(P5).filter((name) =>
        name.endsWith(".json"),
    )) {
        const json = JSON.parse(readFileSync(new URL(file, P5), "utf8"));
        for (const entry of json.cases ?? []) {
            const input = entry.input_file as string | undefined;
            if (
                input === undefined ||
                !existsSync(new URL(input, P5)) ||
                input.endsWith(".zst") ||
                typeof entry.read_result !== "string"
            ) {
                continue;
            }
            const expected = Uint8Array.from(
                Buffer.from(entry.read_result, "hex"),
            );
            const read: WireRead = readWire(
                bytes(input),
                entry.capacity === undefined
                    ? undefined
                    : capacityOf(entry.capacity),
            );
            for (const segment of read.segments) {
                for (const record of encodeParityFindings(
                    segment.parityFindings,
                )) {
                    assert.ok(
                        contains(expected, record),
                        `${input}: result lacks ${hex(record)}`,
                    );
                    checked += 1;
                }
            }
        }
    }
    assert.ok(checked > 20, `only ${checked} findings were checked`);
});

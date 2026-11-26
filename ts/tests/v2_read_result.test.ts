// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { test } from "node:test";
import {
    decodeReadResult,
    encodeReadResult,
    severityOf,
    type Finding,
    type FindingCode,
    type ReadResult,
    type Reason,
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

/** The recorded second-generation result, with `findings` put in place of its file findings. */
function withFileFindings(findings: Finding[]): ReadResult {
    const [{ bytes }] = recorded.filter(
        (r) => r.name === "canonical-no-resync.json",
    );
    const result = decodeReadResult(bytes);
    assert.equal(result.file.tag, "second_generation");
    if (result.file.tag !== "second_generation") throw new Error("unreachable");
    return { ...result, file: { ...result.file, findings } };
}

const finding = (code: FindingCode, reason: Reason | undefined): Finding => ({
    code,
    reason,
    subject: undefined,
    position: undefined,
    offset: undefined,
    length: undefined,
    count: undefined,
});

test("a finding whose code lists no reason is refused with one", () => {
    const withReason = withFileFindings([
        finding("torn_tail", "unknown_codec"),
    ]);
    assert.throws(() => encodeReadResult(withReason), /takes no reason/);
});

test("a finding whose code lists reasons is refused without one, or with one it does not list", () => {
    const absent = withFileFindings([finding("malformed_value", undefined)]);
    assert.throws(() => encodeReadResult(absent), /no reason is not a reason/);
    const unlisted = withFileFindings([
        finding("malformed_value", "payload_digest"),
    ]);
    assert.throws(
        () => encodeReadResult(unlisted),
        /is not a reason of finding/,
    );
});

test("the severity of a finding is the fixed column of its code", () => {
    assert.equal(severityOf("empty_input"), "fatal");
    assert.equal(severityOf("profile_not_checked"), "information");
});

test("a registry version 1 read refuses a finding code of version 2", () => {
    const registryOne = withFileFindings([finding("seal_failed", "aead")]);
    assert.equal(registryOne.registryVersion, 1n);
    assert.throws(
        () => encodeReadResult(registryOne),
        /not in registry version 1/,
    );
});

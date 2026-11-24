// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { inspectFraming } from "../src/v2_framing.js";

test("framing reports unsupported wire generations without rounding", () => {
    const firstItem = Buffer.from("82011bffffffffffffffff", "hex");
    assert.deepEqual(inspectFraming(firstItem), {
        kind: "unsupported",
        generation: 18446744073709551615n,
    });
});

test("a resync prefix requires the fixed marker before exposing its registry", () => {
    const valid =
        "8b050250fcf3eb32a2f0267dd16a53f09bff8792010181018101008101805820" +
        "00".repeat(32);
    const prefix = inspectFraming(Buffer.from(valid, "hex"));
    assert.equal(prefix.kind, "v2");
    const [resync] = prefix.records;
    assert.equal(resync.kind, "resync");
    assert.equal(resync.reason, undefined);

    const invalid = inspectFraming(
        Buffer.from(valid.replace("fcf3", "00f3"), "hex"),
    );
    assert.equal(invalid.kind, "v2");
    assert.deepEqual(
        [...invalid.records],
        [
            {
                kind: "resync",
                offset: 0,
                end: valid.length / 2,
                reason: "out_of_range",
            },
        ],
    );
});

test("frame envelope validation preserves the first field error", () => {
    const cases = [
        ["P1/frame-flags-order", "forbidden_combination"],
        ["P1/envelope-field-before-content-value", "wrong_type"],
        ["P1/frame-compressed-empty", "forbidden_combination"],
        ["P1/stored-check-short", "wrong_length"],
        ["P1/frame-unknown-content-bad-utf8", "wrong_type"],
        ["P1/frame-storage-tag-newer", undefined],
        ["P6/sig-list-grammar-first", "wrong_length"],
        ["P6/sig-list-unordered", "unordered_or_repeated"],
        ["P6/sig-duplicate-records", "unordered_or_repeated"],
    ] as const;
    for (const [fixture, reason] of cases) {
        const path = new URL(
            `../../../vectors/v2/${fixture}.bin`,
            import.meta.url,
        );
        const result = inspectFraming(readFileSync(path));
        assert.equal(result.kind, "v2", fixture);
        const last = [...result.records].at(-1);
        assert.equal(last?.kind, "frame", fixture);
        assert.equal(last.reason, reason, fixture);
    }
});

test("an unreadable opener leaves its frames unchecked until the next opener", () => {
    const file = readFileSync(
        new URL(
            "../../../vectors/v2/P1/frame-flags-order.bin",
            import.meta.url,
        ),
    );
    const malformedHeader = Buffer.from("8701020101f481018101", "hex");
    const result = inspectFraming(
        Buffer.concat([malformedHeader, file.subarray(11), file]),
    );
    assert.equal(result.kind, "v2");
    const records = [...result.records];
    assert.deepEqual(
        records.map(({ kind, reason }) => [kind, reason]),
        [
            ["header", "wrong_type"],
            ["unchecked", undefined],
            ["header", undefined],
            ["frame", "forbidden_combination"],
        ],
    );
});

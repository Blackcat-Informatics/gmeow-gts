// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
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

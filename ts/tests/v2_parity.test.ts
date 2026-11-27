// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { readWire } from "../src/v2_segment.js";

const P5 = new URL("../../../vectors/v2/P5/", import.meta.url);

/** The stage 1 judgement of the in-order parity frame at `position` of the first segment. */
function judgementOf(name: string, position: number) {
    const read = readWire(new Uint8Array(readFileSync(new URL(name, P5))));
    assert.equal(read.stop, undefined, name);
    const [segment] = read.segments;
    assert.ok(segment !== undefined, name);
    return segment.frames.find((frame) => frame.position === position)
        ?.judgement;
}

test("an in-order parity frame whose payload and table hold is verified", () => {
    assert.equal(judgementOf("parity-base.bin", 4), "verified");
});

test("an in-order parity frame whose anchor is not its predecessor is damaged", () => {
    assert.equal(judgementOf("parity-damaged-each.3.bin", 4), "damaged");
});

test("a clean parity frame above the capacity is exceeds_capacity whatever its table digest", () => {
    assert.equal(
        judgementOf("parity-above-capacity.bin", 4),
        "exceeds_capacity",
    );
    assert.equal(
        judgementOf("parity-above-capacity-wrong-table.bin", 4),
        "exceeds_capacity",
    );
});

test("a damaged parity frame above the capacity whose payload is damaged is damaged", () => {
    assert.equal(
        judgementOf("parity-above-capacity-and-damaged.bin", 4),
        "damaged",
    );
});

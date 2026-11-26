// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { classify, ItemWalk, type Boundary } from "../src/v2_walk.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const source = new Uint8Array(
    readFileSync(new URL("canonical-no-resync.source.bin", P7)),
);

function walkAll(data: Uint8Array): Boundary[] {
    const walk = new ItemWalk(data, Number.MAX_SAFE_INTEGER);
    const out: Boundary[] = [];
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        out.push(next);
    }
    return out;
}

test("the item boundaries of an input tile it: each begins where the last ended", () => {
    const items = walkAll(source);
    assert.ok(items.length > 1, "the input holds more than one item");
    let offset = 0;
    for (const item of items) {
        assert.equal(item.offset, offset);
        assert.equal(item.failure, undefined);
        offset = item.end;
    }
    assert.equal(offset, source.length);
});

test("the first item of a segment is a header", () => {
    const [first] = walkAll(source);
    assert.ok(first !== undefined);
    assert.deepEqual(classify(source, first.offset, false), { kind: "header" });
});

test("the walk stops at its item limit and yields nothing after it", () => {
    const walk = new ItemWalk(source, 1);
    assert.notEqual(walk.take(), undefined);
    assert.equal(walk.take(), undefined);
});

test("peek does not consume the boundary that take then returns", () => {
    const walk = new ItemWalk(source, Number.MAX_SAFE_INTEGER);
    const peeked = walk.peek();
    assert.deepEqual(walk.take(), peeked);
});

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { checkClosing, type ClosingItem } from "../src/v2_closing.js";

/** An ordinary item of three bytes, then an offsets item covering it. */
function offsetsFixture(length: number): {
    data: Uint8Array;
    items: ClosingItem[];
} {
    const data = Uint8Array.of(
        0x80,
        0x80,
        0x80, // an ordinary item of three bytes
        0x82,
        0x04,
        0x81,
        length, // offsets: one entry, recording `length`
    );
    return {
        data,
        items: [
            { kind: "other", offset: 0, end: 3 },
            { kind: "offsets", offset: 3, end: 7 },
        ],
    };
}

test("an offsets item that records the length of each item it covers holds", () => {
    const { data, items } = offsetsFixture(3);
    assert.deepEqual(checkClosing(data, items, []), []);
});

test("an offsets item that records a wrong length is an index-offsets finding at its position", () => {
    const { data, items } = offsetsFixture(2);
    assert.deepEqual(checkClosing(data, items, []), [
        { kind: "index_offsets", position: 1 },
    ]);
});

/** A locator that points back `distance` bytes, after an index frame at offset 0. */
function locatorFixture(distance: number): {
    data: Uint8Array;
    items: ClosingItem[];
} {
    const data = Uint8Array.of(
        0x80,
        0x80,
        0x80, // the index frame, three bytes
        0x82,
        0x03,
        0x1b,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        distance, // locator, 64-bit distance
    );
    return {
        data,
        items: [
            { kind: "other", offset: 0, end: 3 },
            { kind: "locator", offset: 3, end: 14 },
        ],
    };
}

test("a locator that points back to the last index frame holds", () => {
    const { data, items } = locatorFixture(3);
    const frames = [{ position: 0, frameType: 15n }];
    assert.deepEqual(checkClosing(data, items, frames), []);
});

test("a locator that points elsewhere is an index-locator finding", () => {
    const { data, items } = locatorFixture(2);
    const frames = [{ position: 0, frameType: 15n }];
    assert.deepEqual(checkClosing(data, items, frames), [
        { kind: "index_locator", position: 1 },
    ]);
});

test("a locator followed by another item is misplaced, not checked", () => {
    const { data, items } = locatorFixture(3);
    const misplaced: ClosingItem[] = [
        ...items,
        { kind: "other", offset: 14, end: 14 },
    ];
    const findings = checkClosing(data, misplaced, [
        { position: 0, frameType: 15n },
    ]);
    assert.equal(findings[0]?.kind, "malformed");
    assert.equal(findings[0]?.position, 1);
});

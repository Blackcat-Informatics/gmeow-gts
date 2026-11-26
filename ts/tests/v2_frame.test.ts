// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeFrame, FrameError } from "../src/v2_frame.js";
import { classify, ItemWalk } from "../src/v2_walk.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const source = new Uint8Array(
    readFileSync(new URL("canonical-no-resync.source.bin", P7)),
);

function frameBoundaries() {
    const walk = new ItemWalk(source, Number.MAX_SAFE_INTEGER);
    const out = [];
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        if (classify(source, next.offset, false).kind === "frame") {
            out.push(next);
        }
    }
    return out;
}

test("every frame of a corpus input decodes and ends where its item ends", () => {
    const frames = frameBoundaries();
    assert.ok(frames.length > 0);
    for (const boundary of frames) {
        const frame = decodeFrame(source, boundary.offset, 1n);
        assert.equal(frame.nextOffset, boundary.end);
        assert.equal(frame.identifier.length, 32);
        assert.equal(frame.payloadDigest.length, 32);
    }
});

test("a frame cut short is refused as a wrong length where the bytes run out", () => {
    const [first] = frameBoundaries();
    assert.ok(first !== undefined);
    const cut = source.subarray(0, first.end - 1);
    assert.throws(
        () => decodeFrame(cut, first.offset, 1n),
        (error: unknown) =>
            error instanceof FrameError && error.reason === "wrong_length",
    );
});

test("a frame whose kind code is not a frame is refused as unregistered at the kind", () => {
    const [first] = frameBoundaries();
    assert.ok(first !== undefined);
    const other = source.slice();
    other[first.offset + 1] = 0x03;
    assert.throws(
        () => decodeFrame(other, first.offset, 1n),
        (error: unknown) =>
            error instanceof FrameError &&
            error.reason === "unregistered_value" &&
            error.offset === first.offset + 1,
    );
});

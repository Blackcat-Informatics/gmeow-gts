// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { readWire } from "../src/v2_segment.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const source = new Uint8Array(
    readFileSync(new URL("canonical-no-resync.source.bin", P7)),
);

test("the corpus input reads as two segments whose frames are linked and verified", () => {
    const read = readWire(source);
    assert.equal(read.stop, undefined);
    assert.equal(read.segments.length, 2);
    assert.equal(read.segments[0]?.opener.kind, "header");
    assert.equal(read.segments[1]?.opener.kind, "resync");
    assert.deepEqual(
        read.segments.map((segment) => segment.frames.length),
        [2, read.segments[1]?.frames.length],
    );
    for (const segment of read.segments) {
        assert.ok(segment.frames.length > 0);
        for (const wire of segment.frames) {
            assert.equal(wire.link.state, "linked");
            assert.equal(wire.payload.state.kind, "verified");
        }
        assert.deepEqual(segment.closing, []);
    }
});

test("a torn tail stops the read at the framing failure", () => {
    const read = readWire(source.subarray(0, source.length - 3));
    assert.equal(read.stop?.kind, "framing");
});

test("a resync item opens a segment without a header before it", () => {
    const read = readWire(source.subarray(209));
    assert.equal(read.segments[0]?.opener.kind, "resync");
});

// The second frame of the first segment, whose first byte the tests overwrite.
const SECOND_FRAME = readWire(source).segments[0]?.items[2]?.offset ?? -1;

test("a lost item is recovered by the resync item after it, and the damage is reported", () => {
    const whole = readWire(source);
    const damaged = source.slice();
    damaged[SECOND_FRAME] = 0xff;
    const read = readWire(damaged);
    assert.equal(read.stop, undefined);
    assert.equal(read.segments.length, 2);
    assert.equal(read.segments[0]?.frames.length, 1);
    assert.equal(read.segments[0]?.framingFailure?.offset, SECOND_FRAME);
    assert.equal(read.segments[0]?.endOffset, 209);
    assert.equal(
        read.segments[1]?.frames.length,
        whole.segments[1]?.frames.length,
    );
    for (const wire of read.segments[1]?.frames ?? []) {
        assert.equal(wire.link.state, "linked");
    }
});

test("a lost item with no resync item after it stops the read at the item", () => {
    const damaged = source.slice(0, 209);
    damaged[SECOND_FRAME] = 0xff;
    const read = readWire(damaged);
    assert.equal(read.segments.length, 1);
    assert.equal(read.stop?.kind, "framing");
    assert.equal(read.stop?.offset, SECOND_FRAME);
});

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fieldOffsets } from "../src/v2_inspect.js";
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

// The element offsets of the first frame of the first segment.
const FIRST_FRAME = readWire(source).segments[0]?.items[1]?.offset ?? -1;

test("a frame with a malformed flag keeps its type and payload, its link is broken, and the read goes on", () => {
    const damaged = source.slice();
    const offsets = fieldOffsets(damaged, FIRST_FRAME);
    // The critical flag of the frame is a simple value; any other simple value is malformed.
    damaged[offsets[1]] = 0xf6;
    const read = readWire(damaged);
    assert.equal(read.stop, undefined);
    assert.equal(read.segments.length, 2);
    const wire = read.segments[0]?.frames[0];
    assert.ok(wire !== undefined);
    assert.ok("error" in wire.envelope);
    assert.equal(wire.frameType, 1n);
    assert.equal(wire.link.state, "broken");
    assert.ok(wire.stored !== undefined);
    assert.equal(wire.payload.state.kind, "verified");
});

test("a frame whose decoded length cannot be read is damaged by its malformed fields", () => {
    const damaged = source.slice();
    const offsets = fieldOffsets(damaged, FIRST_FRAME);
    // A negative integer where the decoded length belongs.
    damaged[offsets[4]] = 0x20;
    const read = readWire(damaged);
    const wire = read.segments[0]?.frames[0];
    assert.ok(wire !== undefined);
    assert.deepEqual(wire.payload.state, {
        kind: "damaged",
        damage: "malformed_fields",
    });
});

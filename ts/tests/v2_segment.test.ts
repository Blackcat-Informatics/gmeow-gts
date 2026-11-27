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

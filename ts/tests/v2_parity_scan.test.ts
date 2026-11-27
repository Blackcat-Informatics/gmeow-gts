// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { DEFAULT_CAPACITY } from "../src/v2_payload.js";
import { readWire } from "../src/v2_segment.js";

const P5 = new URL("../../../vectors/v2/P5/", import.meta.url);
const P6 = new URL("../../../vectors/v2/P6/", import.meta.url);
const bytes = (base: URL, name: string): Uint8Array =>
    new Uint8Array(readFileSync(new URL(name, base)));

test("a parity candidate refused by the total decoded length is recorded on the segment and resumes nothing", () => {
    const read = readWire(bytes(P5, "recover-scan-charge.bin"), {
        ...DEFAULT_CAPACITY,
        decodedBytesPerRead: 1739n,
    });
    const [segment] = read.segments;
    assert.ok(segment !== undefined);
    assert.deepEqual(segment.parityRefusal, {
        position: 2,
        offset: 2634,
        length: 501,
        count: 2086n,
    });
    assert.deepEqual(segment.resumptions, []);
    assert.equal(read.stop?.kind, "framing");
    assert.equal(read.stop?.reason, "undelimitable_input");
});

test("an incomplete item before a parity frame is a damaged region of an undelimitable input", () => {
    const read = readWire(
        bytes(P5, "recovery-parity-in-resync-segment.incomplete-item.bin"),
    );
    assert.equal(read.stop, undefined);
    assert.equal(read.segments.length, 2);
    const [, segment] = read.segments;
    assert.ok(segment !== undefined);
    assert.deepEqual(segment.resumptions, [
        { reason: "undelimitable_input", offset: 274, length: 312 },
    ]);
    assert.equal(segment.items[1]?.kind, "damaged_region");
    assert.notEqual(segment.framingFailure?.reason, "torn_tail");
});

/** The ordinary frames after the damaged region that are not transparent, with their detachment. */
function framesAfterRegion(name: string): boolean[] {
    const read = readWire(bytes(P6, name));
    assert.equal(read.stop, undefined, name);
    const [segment] = read.segments;
    assert.ok(segment !== undefined, name);
    const [region] = segment.resumptions;
    assert.equal(region?.length, 1, name);
    const damaged = segment.items.findIndex(
        (item) => item.kind === "damaged_region",
    );
    assert.equal(damaged, 2, name);
    return segment.frames
        .filter((frame) => !frame.transparent && frame.position > damaged)
        .map((frame) => frame.detached);
}

test("the frame after a parity resumption is detached when its anchor is not the head", () => {
    assert.deepEqual(framesAfterRegion("sig-detached-frame.bin"), [true]);
    assert.deepEqual(framesAfterRegion("sig-resumed-frame-anchor-true.bin"), [
        false,
    ]);
});

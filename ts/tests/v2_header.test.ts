// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { HeaderError } from "../src/v2_decoder.js";
import { decodeFrame } from "../src/v2_frame.js";
import { decodeHeader } from "../src/v2_header.js";
import { frameIdentifier } from "../src/v2_integrity.js";
import { classify, ItemWalk } from "../src/v2_walk.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const source = new Uint8Array(
    readFileSync(new URL("canonical-no-resync.source.bin", P7)),
);

test("the header of a corpus input ends where its item ends and carries an identifier", () => {
    const first = new ItemWalk(source, 1).take();
    assert.ok(first !== undefined);
    const decoded = decodeHeader(source.subarray(0, first.end), first.offset);
    assert.equal(decoded.nextOffset, first.end);
    assert.equal(decoded.identifier?.length, 32);
    assert.equal(decoded.header.hashAlgorithm, 1n);
});

test("the first frame of a corpus input is chained to the identifier of its header", () => {
    const walk = new ItemWalk(source, Number.MAX_SAFE_INTEGER);
    const header = walk.take();
    assert.ok(header !== undefined);
    const headerId = decodeHeader(
        source.subarray(0, header.end),
        header.offset,
    ).identifier;
    assert.ok(headerId !== undefined);
    let frame = walk.take();
    while (
        frame !== undefined &&
        classify(source, frame.offset, false).kind !== "frame"
    ) {
        frame = walk.take();
    }
    assert.ok(frame !== undefined);
    const decoded = decodeFrame(source, frame.offset, 1n);
    const identity = decoded.identityFields;
    assert.deepEqual(frameIdentifier(headerId, identity), decoded.identifier);
});

test("a generation other than 2 is refused as out of range at the generation's offset", () => {
    const header = new ItemWalk(source, 1).take();
    assert.ok(header !== undefined);
    const damaged = source.slice(0, header.end);
    damaged[2] = 0x03;
    assert.throws(
        () => decodeHeader(damaged, header.offset),
        (error: unknown) =>
            error instanceof HeaderError &&
            error.reason === "out_of_range" &&
            error.offset === 2,
    );
});

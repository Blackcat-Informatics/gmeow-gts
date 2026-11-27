// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeFrame, type Frame } from "../src/v2_frame.js";
import { decodeHeader } from "../src/v2_header.js";
import { linkOf, type LinkFields } from "../src/v2_link.js";
import { classify, ItemWalk } from "../src/v2_walk.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const source = new Uint8Array(
    readFileSync(new URL("canonical-no-resync.source.bin", P7)),
);

// The corpus input holds a resync item after its first two frames, so the frames after it
// chain to the resync item's own identifier. The first segment is the two frames before it.
const FIRST_SEGMENT_FRAMES = 2;

/** The header identifier and the frames of the corpus input, in order. */
function chain(): { header: Uint8Array; frames: Frame[] } {
    const walk = new ItemWalk(source, Number.MAX_SAFE_INTEGER);
    const first = walk.take();
    assert.ok(first !== undefined);
    const header = decodeHeader(
        source.subarray(0, first.end),
        first.offset,
    ).identifier;
    assert.ok(header !== undefined);
    const frames: Frame[] = [];
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        if (classify(source, next.offset, false).kind === "frame") {
            frames.push(decodeFrame(source, next.offset, 1n));
        }
    }
    return { header, frames };
}

function fieldsOf(frame: Frame): LinkFields {
    return {
        transparent: frame.transparent,
        identity: frame.identityFields,
        stored: frame.identifier,
    };
}

test("every frame of the first segment of a corpus input is linked to the frame before it", () => {
    const { header, frames } = chain();
    assert.ok(frames.length > 0);
    let predecessor = header;
    for (const frame of frames.slice(0, FIRST_SEGMENT_FRAMES)) {
        const link = linkOf(fieldsOf(frame), predecessor, () => undefined);
        assert.equal(link.state, "linked");
        if (!frame.transparent) {
            assert.deepEqual(link.effectiveIdentifier, frame.identifier);
        }
        predecessor = link.effectiveIdentifier ?? predecessor;
    }
});

test("a frame whose predecessor is not its own is broken when nothing confirms it", () => {
    const { frames } = chain();
    const [frame] = frames;
    assert.ok(frame !== undefined);
    const wrong = new Uint8Array(32);
    const link = linkOf(fieldsOf(frame), wrong, () => undefined);
    assert.equal(link.state, "broken");
    assert.deepEqual(link.effectiveIdentifier, frame.identifier);
});

test("a frame whose computed identifier a following frame confirms is healed to that identifier", () => {
    const { frames } = chain();
    const [frame] = frames;
    assert.ok(frame !== undefined);
    const wrong = new Uint8Array(32);
    const confirmed: Uint8Array[] = [];
    const link = linkOf(fieldsOf(frame), wrong, (identifier) => {
        confirmed.push(identifier);
        return true;
    });
    assert.equal(link.state, "healed");
    assert.equal(confirmed.length, 1);
    assert.deepEqual(link.effectiveIdentifier, link.computedIdentifier);
    assert.notDeepEqual(link.effectiveIdentifier, frame.identifier);
});

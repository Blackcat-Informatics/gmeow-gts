// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { decodeFrame, type Frame } from "../src/v2_frame.js";
import {
    DEFAULT_CAPACITY,
    PayloadBudget,
    storedMatches,
} from "../src/v2_payload.js";
import { classify, ItemWalk } from "../src/v2_walk.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);

function load(name: string): Uint8Array {
    return new Uint8Array(readFileSync(new URL(name, P7)));
}

/** The frames of an input with the offset of each frame's item. */
function framesOf(data: Uint8Array): { offset: number; frame: Frame }[] {
    const walk = new ItemWalk(data, Number.MAX_SAFE_INTEGER);
    const out: { offset: number; frame: Frame }[] = [];
    for (let next = walk.take(); next !== undefined; next = walk.take()) {
        if (classify(data, next.offset, false).kind === "frame") {
            out.push({
                offset: next.offset,
                frame: decodeFrame(data, next.offset, 1n),
            });
        }
    }
    return out;
}

const SOURCE = "canonical-no-resync.source.bin";

test("every frame of the corpus inputs verifies its payload under the default capacity", () => {
    const budget = new PayloadBudget(DEFAULT_CAPACITY);
    let checked = 0;
    for (const name of [SOURCE, "canonical-no-resync.compacted.bin"]) {
        for (const { frame } of framesOf(load(name))) {
            assert.equal(budget.check(frame, undefined).state.kind, "verified");
            checked++;
        }
    }
    assert.ok(checked > 0);
});

test("a stored payload whose bytes change fails its digest as damaged, not verified", () => {
    const data = load(SOURCE);
    const target = framesOf(data).find(({ frame }) => frame.decodedLength > 0n);
    assert.ok(target !== undefined);
    const damaged = data.slice();
    damaged[target.frame.storedPayload.byteOffset - data.byteOffset] ^= 1;
    const frame = decodeFrame(damaged, target.offset, 1n);
    const check = new PayloadBudget(DEFAULT_CAPACITY).check(frame, undefined);
    assert.deepEqual(check.state, {
        kind: "damaged",
        damage: "payload_digest",
    });
});

test("a payload that the total of the read cannot cover is unchecked, and the total stays refused", () => {
    const frames = framesOf(load(SOURCE)).filter(
        ({ frame }) => frame.decodedLength > 0n,
    );
    assert.ok(frames.length >= 2);
    const budget = new PayloadBudget({
        ...DEFAULT_CAPACITY,
        decodedBytesPerRead: 0n,
    });
    const first = budget.check(frames[0].frame, undefined);
    assert.equal(first.state.kind, "unchecked");
    assert.equal(
        first.state.kind === "unchecked" && first.state.refusal.kind,
        "total_decoded",
    );
    assert.equal(
        budget.check(frames[1].frame, undefined).state.kind,
        "unchecked",
    );
    assert.equal(budget.totalRefused, true);
});

test("a payload above the per-frame capacity is refused as decoded size and charges nothing", () => {
    const target = framesOf(load(SOURCE)).find(
        ({ frame }) => frame.decodedLength > 0n,
    );
    assert.ok(target !== undefined);
    const budget = new PayloadBudget({
        ...DEFAULT_CAPACITY,
        decodedBytesPerFrame: 0n,
    });
    const check = budget.check(target.frame, undefined);
    assert.equal(
        check.state.kind === "unchecked" && check.state.refusal.kind,
        "decoded_size",
    );
    assert.equal(budget.totalRefused, false);
});

test("the compressed frames of the overhead vector verify their payloads and stored checks", () => {
    const data = new Uint8Array(
        readFileSync(
            new URL(
                "../../../vectors/v2/P1/frame-overhead-compressed.bin",
                import.meta.url,
            ),
        ),
    );
    const compressed = framesOf(data).filter(
        ({ frame }) => frame.storage.kind === "compressed",
    );
    assert.equal(compressed.length, 2);
    const budget = new PayloadBudget(DEFAULT_CAPACITY);
    const [first] = compressed;
    const check = budget.check(
        first.frame,
        storedMatches(first.frame.storage, [first.frame.storedPayload]),
    );
    assert.equal(check.storedMatches, true);
    assert.equal(check.state.kind, "verified");
    assert.equal(
        check.state.kind === "verified" &&
            new TextDecoder().decode(check.state.decoded),
        "abcabcabcabc",
    );
    for (const { frame } of compressed.slice(1)) {
        assert.equal(budget.check(frame, undefined).state.kind, "verified");
    }
});

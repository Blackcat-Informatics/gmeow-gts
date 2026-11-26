// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { Decoder, HeaderError } from "../src/v2_decoder.js";
import { PayloadBudget } from "../src/v2_integrity.js";
import { decodeResync, ResyncScanner } from "../src/v2_resync.js";
import { ItemWalk } from "../src/v2_walk.js";

const P7 = new URL("../../../vectors/v2/P7/", import.meta.url);
const input = new Uint8Array(
    readFileSync(new URL("idx-resync-opener.bin", P7)),
);

function refusal(read: (decoder: Decoder) => unknown, bytes: number[]) {
    try {
        read(new Decoder(Uint8Array.from(bytes), 0));
    } catch (error) {
        assert.ok(error instanceof HeaderError);
        return `${error.reason}@${error.offset}`;
    }
    return "accepted";
}

test("a head that is not the shortest form is refused at its first byte", () => {
    assert.equal(
        refusal((d) => d.uint(), [0x18, 0x05]),
        "not_deterministic@0",
    );
    assert.equal(
        refusal((d) => d.uint(), [0x18, 0x18]),
        "accepted",
    );
});

test("a head cut short and a head of another major type are refused where they start", () => {
    assert.equal(
        refusal((d) => d.uint(), [0x19, 0x01]),
        "wrong_length@1",
    );
    assert.equal(
        refusal((d) => d.uint(), [0x41, 0x00]),
        "wrong_type@0",
    );
});

test("an option is absent as [1] and present as [2, value]; [0] and a mismatched tag are refused", () => {
    const read = (d: Decoder) => d.option((inner) => inner.uint());
    assert.equal(refusal(read, [0x81, 0x01]), "accepted");
    assert.equal(refusal(read, [0x82, 0x02, 0x07]), "accepted");
    assert.equal(refusal(read, [0x80]), "wrong_length@0");
    assert.equal(refusal(read, [0x82, 0x01, 0x07]), "wrong_length@0");
});

test("the scanner accepts the resync opener of the segment and agrees with the walk", () => {
    const budget = new PayloadBudget();
    const found = new ResyncScanner(input).scan(0, input.length, budget);
    assert.equal(found?.kind, "accepted");
    if (found?.kind !== "accepted") return;
    assert.equal(found.value.claimedOffset, BigInt(found.offset));
    const walk = new ItemWalk(input, Number.MAX_SAFE_INTEGER);
    let boundary = walk.take();
    while (boundary !== undefined && boundary.offset !== found.offset) {
        boundary = walk.take();
    }
    assert.equal(boundary?.end, found.value.nextOffset);
    const walked = decodeResync(input, found.offset, boundary?.end ?? 0);
    assert.deepEqual(walked.identifier, found.value.identifier);
});

test("a candidate whose bytes are damaged is not accepted", () => {
    const damaged = input.slice();
    const budget = new PayloadBudget();
    const found = new ResyncScanner(input).scan(0, input.length, budget);
    assert.ok(found?.kind === "accepted");
    damaged[found.value.nextOffset - 1] ^= 1;
    const again = new ResyncScanner(damaged).scan(
        0,
        damaged.length,
        new PayloadBudget(),
    );
    assert.equal(again, undefined);
});

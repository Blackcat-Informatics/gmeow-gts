// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { readWire } from "../src/v2_segment.js";
import { COMPOSITE_ALGORITHM, type HeldKey } from "../src/v2_signature.js";

const P6 = new URL("../../../vectors/v2/P6/", import.meta.url);
const bytes = (name: string): Uint8Array =>
    new Uint8Array(readFileSync(new URL(name, P6)));

const signer = JSON.parse(
    readFileSync(new URL("sign-composite-reproduce.json", P6), "utf8"),
).cases[0];
const heldKey: HeldKey = {
    keyId: Uint8Array.from(Buffer.from(signer.signer.key_id, "hex")),
    algorithm: COMPOSITE_ALGORITHM,
    publicKey: bytes(signer.public_key_file),
};

/** The results of the signatures of every frame of the vector, in order. */
function results(name: string): (string | undefined)[][] {
    const read = readWire(bytes(name), undefined, [heldKey]);
    assert.equal(read.stop, undefined, name);
    return read.segments.flatMap((segment) =>
        segment.frames.flatMap((frame) =>
            frame.signatures.map((signature) => [
                signature.status,
                signature.reason,
            ]),
        ),
    );
}

/** The status and reason that the corpus states for the vector's single signature. */
function expected(name: string): (string | undefined)[] {
    const description = JSON.parse(
        readFileSync(new URL(name.replace(/\.bin$/, ".json"), P6), "utf8"),
    );
    const [status, reason] = description.cases[0].result.result;
    return [status, reason ?? undefined];
}

test("each signature of a frame is verified against the held keys, with the corpus outcome", () => {
    for (const name of [
        "composite-honest.bin",
        "composite-both.bin",
        "composite-classical-nonce.bin",
        "composite-classical-scalar.bin",
        "composite-classical-small-order.bin",
    ]) {
        assert.deepEqual(results(name), [expected(name)], name);
    }
});

test("two records of one key in one frame give two results in order", () => {
    assert.deepEqual(results("composite-hedged.both.bin"), [
        ["valid", undefined],
        ["valid", undefined],
    ]);
});

test("a frame that is not signed, or whose key is not held, has no verified signature", () => {
    const read = readWire(bytes("composite-honest.bin"));
    const outcomes = read.segments.flatMap((segment) =>
        segment.frames.flatMap((frame) =>
            frame.signatures.map((signature) => [
                signature.status,
                signature.reason,
            ]),
        ),
    );
    assert.deepEqual(outcomes, [["unverified", "key_not_held"]]);
});

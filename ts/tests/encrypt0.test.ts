// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// COSE_Encrypt0 conformance (§9.3): the TypeScript engine reproduces the frozen
// fixed-IV AES-256-GCM seal (vectors/encrypt0/basic.json) byte-for-byte, opens
// it, and round-trips a random-IV seal.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import {
    encrypt0,
    encrypt0WithIv,
    decrypt0,
    recipientKid,
    Encrypt0Error,
} from "../src/cose.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");

function unhex(s: string): Uint8Array {
    return Uint8Array.from(Buffer.from(s, "hex"));
}

test("encrypt0 vector: fixed-IV seal reproduces frozen bytes and opens (§9.3)", () => {
    const c = JSON.parse(
        readFileSync(join(repoRoot, "vectors", "encrypt0", "basic.json"), "utf-8"),
    );
    const key = unhex(c.key);
    const iv = unhex(c.iv);
    const plaintext = unhex(c.plaintext);
    const expected = unhex(c.cose);

    // Fixed IV -> the sealed bytes reproduce the frozen vector exactly.
    assert.deepEqual(encrypt0WithIv(plaintext, c.kid, key, iv), expected);

    // The recipient kid round-trips out of the cleartext header.
    assert.equal(recipientKid(expected), c.kid);

    // The frozen COSE opens back to the plaintext under the content key.
    assert.deepEqual(
        decrypt0(expected, (kid) => (kid === c.kid ? key : null)),
        plaintext,
    );

    // No key -> missing-key; wrong key -> auth-failed.
    assert.throws(
        () => decrypt0(expected, () => null),
        (e: unknown) => e instanceof Encrypt0Error && e.reason === "missing-key",
    );
    assert.throws(
        () => decrypt0(expected, () => new Uint8Array(32)),
        (e: unknown) => e instanceof Encrypt0Error && e.reason === "auth-failed",
    );
});

test("encrypt0 random-IV round-trip", () => {
    const key = new Uint8Array(32).map((_, i) => i);
    const sealed = encrypt0(new TextEncoder().encode("verified id record"), "did:court", key);
    assert.equal(recipientKid(sealed), "did:court");
    assert.deepEqual(
        decrypt0(sealed, (kid) => (kid === "did:court" ? key : null)),
        new TextEncoder().encode("verified id record"),
    );
});

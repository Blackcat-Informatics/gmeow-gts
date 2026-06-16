// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Cross-engine COSE_Sign1 + emojihash conformance: the TypeScript engine must
// reproduce the frozen vectors/cose and vectors/emojihash byte-for-byte.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import * as ed from "@noble/ed25519";

import { signId, verifySig, signatureKid } from "../src/cose.js";
import {
    emojihash,
    emojihashLabels,
    emojiIndices,
} from "../src/emojihash.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");

function unhex(s: string): Uint8Array {
    return Uint8Array.from(Buffer.from(s, "hex"));
}

function vectors(sub: string): string[] {
    const dir = join(repoRoot, "vectors", sub);
    return readdirSync(dir)
        .filter((f) => f.endsWith(".json"))
        .map((f) => join(dir, f));
}

test("COSE_Sign1 vectors round-trip (§9.2)", () => {
    const files = vectors("cose");
    assert.ok(files.length >= 2, "expected COSE vectors");
    for (const path of files) {
        const c = JSON.parse(readFileSync(path, "utf-8"));
        const seed = unhex(c.seed);
        const frameId = unhex(c.frame_id);

        // Deterministic Ed25519: signing reproduces the frozen bytes.
        const got = Buffer.from(signId(frameId, seed, c.kid)).toString("hex");
        assert.equal(got, c.cose, `sign mismatch in ${path}`);

        // The kid round-trips, and the public key matches.
        const cose = unhex(c.cose);
        assert.equal(signatureKid(cose), c.kid);
        assert.equal(Buffer.from(ed.getPublicKey(seed)).toString("hex"), c.pub);

        // Verification succeeds; a tampered id fails.
        assert.equal(verifySig(cose, frameId, unhex(c.pub)), "valid");
        const tampered = new Uint8Array([...frameId, 0xff]);
        assert.equal(verifySig(cose, tampered, unhex(c.pub)), "invalid");
    }
});

test("emojihash vectors", () => {
    const files = vectors("emojihash");
    assert.ok(files.length >= 4, "expected emojihash vectors");
    for (const path of files) {
        const c = JSON.parse(readFileSync(path, "utf-8"));
        const data = unhex(c.data);
        assert.deepEqual(emojiIndices(data, c.length), c.indices, path);
        assert.equal(emojihash(data, c.length), c.emoji);
        assert.equal(emojihashLabels(data, c.length), c.labels);
    }
});

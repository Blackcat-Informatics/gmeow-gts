// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { proofFromJson, verifyProof, verifyProofJson } from "../src/mmr.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const proofsDir = resolve(__dirname, "../../../vectors/proofs");

function proof(name: string): string {
    return readFileSync(join(proofsDir, name), "utf8");
}

test("positive proof fixture verifies", () => {
    const p = proofFromJson(proof("mmr-basic-proof.json"));
    verifyProof(p);
    assert.equal(p.count, 4);
    assert.equal(p.leafIndex, 2);
});

test("negative proof fixture fails", () => {
    const p = proofFromJson(proof("mmr-basic-proof-bad-root.json"));
    assert.throws(() => verifyProof(p), /root/);
});

test("proofFromJson rejects unsafe integer fields", () => {
    const doc = JSON.parse(proof("mmr-basic-proof.json"));
    doc.count = Number.MAX_SAFE_INTEGER + 1;
    assert.throws(
        () => proofFromJson(JSON.stringify(doc)),
        /safe unsigned integer/,
    );
});

test("verify proof rejects invalid direct-call step sides", () => {
    const p = proofFromJson(proof("mmr-basic-proof.json"));
    const invalid = {
        ...p,
        path: [
            { ...p.path[0], side: "center" as unknown as "left" },
            ...p.path.slice(1),
        ],
    };

    assert.throws(() => verifyProof(invalid), /unsupported proof side/);
});

test("verifyProofJson returns a verified proof", () => {
    const p = verifyProofJson(proof("mmr-basic-proof.json"));
    assert.equal(p.root.length, 32);
});

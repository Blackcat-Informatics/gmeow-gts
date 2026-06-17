// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// File-level signing conformance (§9.2): the TypeScript engine reproduces the
// frozen signed GTS (vectors/signed/basic.json) when signing, and verifies it.

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import * as ed from "@noble/ed25519";

import { Writer } from "../src/writer.js";
import { Read } from "../src/reader.js";
import { TermKind } from "../src/model.js";
import { verifySignatures } from "../src/cose.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");

const CAT = "https://example.org/Cat";
const LABEL = "http://www.w3.org/2000/01/rdf-schema#label";

function unhex(s: string): Uint8Array {
    return Uint8Array.from(Buffer.from(s, "hex"));
}

test("signed file vector: writer signing + reader verification (§9.2)", () => {
    const c = JSON.parse(
        readFileSync(join(repoRoot, "vectors", "signed", "basic.json"), "utf-8"),
    );
    const seed = unhex(c.seed);
    const pub = unhex(c.pub);
    const expected = unhex(c.gts);

    // Writer signing reproduces the frozen signed file byte-for-byte.
    const w = new Writer("dist");
    w.signWith(seed, c.kid);
    w.addTerms([
        { kind: TermKind.Iri, value: CAT },
        { kind: TermKind.Iri, value: LABEL },
        { kind: TermKind.Literal, value: "Cat", lang: "en" },
    ]);
    w.addQuads([{ s: 0, p: 1, o: 2 }]);
    assert.deepEqual(w.toBytes(), expected, "writer signing mismatch");

    // Right key -> every signature valid.
    let g = Read(expected, false);
    assert.equal(g.signatures.length, 2);
    verifySignatures(g.signatures, (kid) => (kid === c.kid ? pub : null));
    assert.ok(g.signatures.every((s) => s.status === "valid" && s.kid === c.kid));

    // No key -> unverified.
    g = Read(expected, false);
    verifySignatures(g.signatures, () => null);
    assert.ok(g.signatures.every((s) => s.status === "unverified"));

    // Wrong key -> invalid.
    const wrong = ed.getPublicKey(new Uint8Array(32).fill(7));
    g = Read(expected, false);
    verifySignatures(g.signatures, () => wrong);
    assert.ok(g.signatures.every((s) => s.status === "invalid"));
});

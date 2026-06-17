// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
// OpenPGP key-parsing conformance (§9.2): the TypeScript engine parses the
// armored Ed25519 transport key the same way as the Python reference, and the
// `gts extract-key` CLI reproduces the frozen stdout byte-for-byte.

import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import { parseTransportKey } from "../src/openpgp.js";
import { emojihash } from "../src/emojihash.js";
import { hex } from "../src/wire.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");
const cli = resolve(__dirname, "../bin/gts.js");
const openpgpDir = join(repoRoot, "vectors", "openpgp");

function unhex(s: string): Uint8Array {
    return Uint8Array.from(Buffer.from(s, "hex"));
}

test("parses the frozen Ed25519 key vector (raw, fingerprint, emojihash)", () => {
    const c = JSON.parse(readFileSync(join(openpgpDir, "test-key.json"), "utf-8"));
    const key = parseTransportKey(c.armored);
    assert.equal(hex(key.rawPublic), c.raw_pub);
    assert.equal(key.fingerprint, c.fingerprint);
    assert.equal(emojihash(key.rawPublic), c.emojihash);
});

test("rejects non-armored input", () => {
    assert.throws(() => parseTransportKey("not a key"));
});

test("CLI extract-key reproduces the frozen stdout (§9.2)", () => {
    const c = JSON.parse(readFileSync(join(openpgpDir, "extract-key.json"), "utf-8"));
    const dir = mkdtempSync(join(tmpdir(), "gts-extract-key-"));
    const f = join(dir, "signed.gts");
    writeFileSync(f, unhex(c.gts));

    const r = spawnSync("node", [cli, "extract-key", f], { encoding: "utf8" });
    assert.equal(r.status, 0);
    assert.equal(r.stdout, c.stdout);
});

test("CLI extract-key exits 1 for a file with no embedded key", () => {
    const r = spawnSync(
        "node",
        [cli, "extract-key", join(repoRoot, "vectors", "01-minimal.gts")],
        { encoding: "utf8" },
    );
    assert.equal(r.status, 1);
});

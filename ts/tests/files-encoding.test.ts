// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Regression guard for #5: the files profile must encode `mode` as a decimal
// xsd:integer and `modified` at second precision, matching the Rust/Go/Python
// engines so the same fixture packs to byte-identical output everywhere.

import {
    mkdtempSync,
    writeFileSync,
    chmodSync,
    utimesSync,
    symlinkSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { diff, pack } from "../src/files.js";
import { Read } from "../src/reader.js";
import { toNQuads } from "../src/nquads.js";

test("files profile: second-precision modified, decimal mode (#5)", () => {
    const dir = mkdtempSync(join(tmpdir(), "gts-enc-"));
    const f = join(dir, "a.txt");
    writeFileSync(f, "hello");
    const d = new Date(1700000000 * 1000); // 2023-11-14T22:13:20Z
    utimesSync(f, d, d);
    if (process.platform !== "win32") chmodSync(f, 0o644);

    const nq = toNQuads(Read(pack([dir]), false));

    // modified is RFC 3339 at second precision — never milliseconds (".000Z").
    assert.ok(
        nq.includes('"2023-11-14T22:13:20Z"'),
        "modified must be second-precision RFC 3339",
    );
    assert.ok(!/\.\d{3}Z/.test(nq), "modified must not include milliseconds");

    // mode is the DECIMAL value of the permission bits (0o644 === 420), not the
    // octal string "644". (POSIX only — Windows does not surface 0o644.)
    if (process.platform !== "win32") {
        assert.ok(
            nq.includes('"420"^^<http://www.w3.org/2001/XMLSchema#integer>'),
            "mode must be the decimal integer 420",
        );
        assert.ok(
            !nq.includes('"644"'),
            "mode must not be the octal string 644",
        );
    }
});

test("files profile refuses symlink entries for pack and diff", () => {
    if (process.platform === "win32") return;

    const dir = mkdtempSync(join(tmpdir(), "gts-symlink-"));
    const f = join(dir, "a.txt");
    writeFileSync(f, "hello");
    const archive = Read(pack([dir]), false);
    symlinkSync(f, join(dir, "linked.txt"));

    assert.throws(() => pack([dir]), /symlink/);
    assert.throws(() => diff(archive, dir), /symlink/);
});

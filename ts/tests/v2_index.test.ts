// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { decodeClaim, readIndexPayload } from "../src/v2_index.js";
import { HeaderError } from "../src/v2_decoder.js";

const digest = new Uint8Array(32).fill(7);
const head = Uint8Array.of(0x58, 0x20);

/** A commit claim `[count, root]`. */
function claim(count: number, root: Uint8Array): Uint8Array {
    return Uint8Array.of(0x82, count, ...head, ...root);
}

/**
 * An index payload with no type runs, no graph names, no entries and no pairs; its
 * root is `root` and it has no optional field and no records.
 */
function emptyIndex(root: Uint8Array): Uint8Array {
    return Uint8Array.of(
        0x86, // record of six fields
        0x80, // no type runs
        0x80, // no graph names
        0x84,
        0x00,
        0x00,
        0x00, // record of four: three counts of zero
        0x80, // no pairs
        ...head,
        ...root,
        0x81,
        0x01, // option absent
        0x80, // no records
    );
}

test("a commit claim reads its ordinary count and range root", () => {
    const { count, root } = decodeClaim(claim(3, digest));
    assert.equal(count, 3n);
    assert.deepEqual(root, digest);
});

test("a commit claim whose root is not 32 bytes is refused as a wrong length", () => {
    const short = Uint8Array.of(0x82, 0x03, 0x58, 0x1f, ...new Uint8Array(31));
    assert.throws(
        () => decodeClaim(short),
        (error: unknown) =>
            error instanceof HeaderError && error.reason === "wrong_length",
    );
});

test("an index payload with no runs matches a segment with no runs and reads its root", () => {
    const result = readIndexPayload(
        emptyIndex(digest),
        { newer: false, stopped: false },
        [],
    );
    assert.equal(result.ok, true);
    if (!result.ok) return;
    assert.equal(result.runsMatch, true);
    assert.deepEqual(result.root, digest);
});

test("an index payload whose run disagrees with the segment's runs does not match", () => {
    const payload = Uint8Array.of(
        0x86,
        0x81,
        0x82,
        0x10,
        0x01, // one type run: parity, length one
        0x80,
        0x84,
        0x00,
        0x00,
        0x00,
        0x80,
        ...head,
        ...digest,
        0x81,
        0x01,
        0x80,
    );
    const result = readIndexPayload(
        payload,
        { newer: false, stopped: false },
        [],
    );
    assert.equal(result.ok && result.runsMatch, false);
});

test("a frame code beyond the registry is refused as unregistered at the code", () => {
    const payload = Uint8Array.of(
        0x86,
        0x81,
        0x82,
        0x11,
        0x01, // type run with code 17, past the parity code
        0x80,
        0x84,
        0x00,
        0x00,
        0x00,
        0x80,
        ...head,
        ...digest,
        0x81,
        0x01,
        0x80,
    );
    const result = readIndexPayload(
        payload,
        { newer: false, stopped: false },
        [],
    );
    assert.equal(result.ok, false);
    if (result.ok) return;
    assert.equal(result.kind, "malformed");
    assert.equal(
        result.kind === "malformed" && result.error.reason,
        "unregistered_value",
    );
    assert.equal(result.kind === "malformed" && result.error.offset, 3);
});

test("a code beyond the registry of a later version is unknown, not malformed, before a stop", () => {
    const payload = Uint8Array.of(
        0x86,
        0x81,
        0x82,
        0x11,
        0x01,
        0x80,
        0x84,
        0x00,
        0x00,
        0x00,
        0x80,
        ...head,
        ...digest,
        0x81,
        0x01,
        0x80,
    );
    const result = readIndexPayload(
        payload,
        { newer: true, stopped: false },
        [],
    );
    assert.equal(result.ok, false);
    if (result.ok) return;
    assert.equal(result.kind, "unknown");
});

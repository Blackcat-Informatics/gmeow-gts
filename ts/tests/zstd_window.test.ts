// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { test } from "node:test";
import assert from "node:assert/strict";
import { decodeChain, isCodecError, MAX_ZSTD_WINDOW_LOG } from "../src/codec.js";
import { Read } from "../src/reader.js";
import * as wire from "../src/wire.js";
import { Writer } from "../src/writer.js";

const ZSTD = { name: "zstd", cls: "compress" };
const CONTENT = new TextEncoder().encode("declared window");

/** A zstd frame by hand: window descriptor, one raw block, no content size. */
function windowFrame(windowLog: number, content: Uint8Array): Uint8Array {
    const block = 1 | (content.length << 3);
    return new Uint8Array([
        0x28, 0xb5, 0x2f, 0xfd, 0x00, (windowLog - 10) << 3,
        block & 0xff, (block >> 8) & 0xff, (block >> 16) & 0xff,
        ...content,
    ]);
}

function concat(...parts: Uint8Array[]): Uint8Array {
    return Uint8Array.from(parts.flatMap((p) => [...p]));
}

test("a zstd frame declaring a window up to the limit decodes", () => {
    const out = decodeChain([ZSTD], windowFrame(MAX_ZSTD_WINDOW_LOG, CONTENT));
    assert.deepEqual(out, CONTENT);
});

for (const windowLog of [28, 29, 31]) {
    test(`a zstd frame declaring a window of 2^${windowLog} is refused, not decoded`, () => {
        assert.throws(
            () => decodeChain([ZSTD], windowFrame(windowLog, CONTENT)),
            (err: unknown) =>
                isCodecError(err) &&
                err.reason === "damaged" &&
                err.failed &&
                /window/.test(err.detail),
        );
    });
}

test("a large window in a later frame of the payload is still refused", () => {
    const payload = concat(windowFrame(20, CONTENT), windowFrame(29, CONTENT));
    assert.throws(
        () => decodeChain([ZSTD], payload),
        (err: unknown) => isCodecError(err) && /window/.test(err.detail),
    );
});

test("a blob over the window limit is one opaque node and folding goes on", () => {
    const w = new Writer("generic");
    const header = w.toBytes();
    const frame = new Map<unknown, unknown>();
    frame.set("t", "blob");
    frame.set("x", [2]);
    frame.set("d", windowFrame(29, CONTENT));
    const pub = new Map<unknown, unknown>();
    pub.set("digest", wire.digestStr(CONTENT));
    frame.set("pub", pub);
    frame.set("prev", w.head());
    frame.set("id", wire.contentId(frame));
    const g = Read(concat(header, wire.encode(frame)), true);
    assert.deepEqual(
        g.opaque.map((o) => [o.frameType, o.reason]),
        [["blob", "damaged"]],
    );
    assert.deepEqual(
        g.diagnostics.map((d) => d.code),
        ["DamagedFrame"],
    );
    assert.equal(g.blobs.length, 0);
});

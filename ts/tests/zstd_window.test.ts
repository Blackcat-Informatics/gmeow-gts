// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { test } from "node:test";
import assert from "node:assert/strict";
import { decodeChain, isCodecError, MAX_ZSTD_WINDOW_LOG } from "../src/codec.js";
import { Read } from "../src/reader.js";
import { readStream } from "../src/browser.js";
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

/** A file holding one blob frame: `payload` as its stored bytes, `x` as its chain. */
function fileWithBlob(
    payload: Uint8Array,
    declared: Uint8Array,
    x: number[] | undefined = [2],
): Uint8Array {
    const w = new Writer("generic");
    const header = w.toBytes();
    const frame = new Map<unknown, unknown>();
    frame.set("t", "blob");
    if (x) frame.set("x", x);
    frame.set("d", payload);
    const pub = new Map<unknown, unknown>();
    pub.set("digest", wire.digestStr(declared));
    frame.set("pub", pub);
    frame.set("prev", w.head());
    frame.set("id", wire.contentId(frame));
    return concat(header, wire.encode(frame));
}

function stream(data: Uint8Array): ReadableStream<Uint8Array> {
    return new ReadableStream({
        start(controller) {
            controller.enqueue(data);
            controller.close();
        },
    });
}

function assertOneOpaqueBlob(g: {
    opaque: { frameType: string; reason: string }[];
    diagnostics: { code: string }[];
    blobs: unknown[];
}): void {
    assert.deepEqual(
        g.opaque.map((o) => [o.frameType, o.reason]),
        [["blob", "damaged"]],
    );
    assert.deepEqual(
        g.diagnostics.map((d) => d.code),
        ["DamagedFrame"],
    );
    assert.equal(g.blobs.length, 0);
}

test("a blob over the window limit is one opaque node and folding goes on", () => {
    assertOneOpaqueBlob(
        Read(fileWithBlob(windowFrame(29, CONTENT), CONTENT), true),
    );
});

test("the browser build refuses the same frame", async () => {
    const data = fileWithBlob(windowFrame(29, CONTENT), CONTENT);
    assertOneOpaqueBlob(await readStream(stream(data)));
});

/** A first frame the zstd library accepts but the format does not, then a large window. */
function bypassPayload(): Uint8Array {
    const size = 131073; // one byte over the largest block
    const block = 1 | (size << 3);
    const first = concat(
        new Uint8Array([0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x50]),
        new Uint8Array([block & 0xff, (block >> 8) & 0xff, (block >> 16) & 0xff]),
        new Uint8Array(size),
    );
    return concat(first, windowFrame(29, CONTENT));
}

test("a structure the walk dislikes is refused, not left to the library", () => {
    assert.throws(
        () => decodeChain([ZSTD], bypassPayload()),
        (err: unknown) => isCodecError(err) && err.reason === "damaged",
    );
});

test("a payload that is not whole zstd frames is refused in both builds", async () => {
    const payload = new TextEncoder().encode("not zstd frame data");
    assertOneOpaqueBlob(Read(fileWithBlob(payload, CONTENT), true));
    assertOneOpaqueBlob(await readStream(stream(fileWithBlob(payload, CONTENT))));
});

test("a frame naming a dictionary is refused, since none is implemented", () => {
    // flags: dictionary id of one byte; window descriptor; id 5; one raw block.
    const frame = new Uint8Array([
        0x28, 0xb5, 0x2f, 0xfd, 0x01, 0x00, 0x05, 0x29, 0x00, 0x00, 0x41,
    ]);
    assert.throws(
        () => decodeChain([ZSTD], frame),
        (err: unknown) => isCodecError(err) && /dictionary/.test(err.detail),
    );
});

test("a blob whose bytes do not hash to its declared digest is damaged", async () => {
    // Stored uncompressed (no transform) under a digest of other bytes.
    const data = fileWithBlob(CONTENT, new TextEncoder().encode("other"), undefined);
    assertOneOpaqueBlob(Read(data, true));
    assertOneOpaqueBlob(await readStream(stream(data)));
});

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import cbor from "cbor";
import { V2SegmentWriter } from "../src/index.js";
import { mustEncode } from "../src/wire.js";

type Option<T> = [1] | [2, T];
type Header = [
    1,
    2,
    1,
    1,
    Option<number>,
    Option<1>,
    Option<[Uint8Array, number | bigint]>,
];
type Frame = [
    2,
    number,
    boolean,
    boolean,
    unknown,
    number,
    Uint8Array,
    [1],
    Uint8Array,
    Uint8Array,
    [],
];

test("v2 segment writer reproduces committed and transparent corpus segments", () => {
    for (const name of [
        "P5/commit-valid",
        "P5/range-transparent.no-parity",
        "P5/range-transparent.with-parity",
        "P3/state-prefix-fold",
    ]) {
        const fixture = readFileSync(
            new URL(`../../../vectors/v2/${name}.bin`, import.meta.url),
        );
        const segments: Uint8Array[] = [];
        let writer: V2SegmentWriter | undefined;
        for (const item of cbor.decodeAllSync(fixture) as (Header | Frame)[]) {
            if (item[0] === 1) {
                if (writer !== undefined) segments.push(writer.bytes());
                const [, , , , profile, canonical, lineage] = item;
                writer = new V2SegmentWriter({
                    profile: profile[1],
                    canonical: canonical[0] === 2,
                    lineage:
                        lineage[0] === 2
                            ? {
                                  previousHead: lineage[1][0],
                                  ordinal: BigInt(lineage[1][1]),
                              }
                            : undefined,
                });
                continue;
            }
            assert.ok(writer, name);
            const [, type, , transparent, content, , , , payload, expectedId] =
                item;
            const previous = writer.head;
            const id =
                type === 14
                    ? writer.commit()
                    : writer.append(type, mustEncode(content), payload);
            assert.deepEqual(Buffer.from(id), expectedId, name);
            assert.deepEqual(writer.head, transparent ? previous : id, name);
        }
        assert.ok(writer, name);
        segments.push(writer.bytes());
        assert.deepEqual(Buffer.concat(segments), fixture, name);
    }
});

test("v2 writer owns its inputs and rejected appends leave its prefix intact", () => {
    const writer = new V2SegmentWriter();
    const content = mustEncode([1]);
    const payload = mustEncode([[0]]);
    const id = writer.append(5, content, payload);
    const expectedId = writer.head;
    const expectedBytes = Buffer.from(writer.bytes());
    content.fill(0);
    payload.fill(0);
    id.fill(0);
    writer.head.fill(0);
    writer.bytes().fill(0);
    assert.throws(
        () => writer.append(5, Uint8Array.of(0x81, 0x18, 1), payload),
        /not_deterministic/,
    );
    assert.throws(() => writer.append(14, content, payload), /use commit/);
    assert.deepEqual(writer.head, expectedId);
    assert.deepEqual(writer.bytes(), expectedBytes);
});

test("v2 writer rejects a JavaScript lineage digest supplied as text", () => {
    const previousHead = "x".repeat(32) as unknown as Uint8Array;
    assert.throws(
        () => new V2SegmentWriter({ lineage: { previousHead, ordinal: 1n } }),
        /32-byte digest/,
    );
});

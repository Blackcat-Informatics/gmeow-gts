// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { inspectFraming } from "../src/v2_framing.js";
import { contextHash } from "../src/v2_integrity.js";
import { projectRdf, rdfNQuads } from "../src/v2_rdf_projection.js";
import { V2SegmentWriter } from "../src/v2_writer.js";
import { mustEncode } from "../src/wire.js";

function fold(data: Uint8Array): string {
    const wire = inspectFraming(data, true);
    assert.equal(wire.kind, "v2");
    return rdfNQuads(projectRdf(wire));
}

test("value references select the first usable pair in file order", () => {
    const cases = [
        ["P3/terms-literal-ref-first-candidate.failing-first", undefined],
        ["P3/terms-literal-ref-length-pair.length-4", "missing_value"],
        ["P3/terms-literal-ref-not-utf8", "ill_formed_term"],
        ["P4/rebuild-value-first-verified.first-fails", undefined],
        ["P4/rebuild-value-other-segment.value-resolves", undefined],
        ["P4/rebuild-value-other-segment.unit-in-term-segment", "chunk"],
        ["P4/pack-recipe-trailing-count", "wrong_length"],
    ] as const;
    for (const [fixture, reason] of cases) {
        const input = readFileSync(
            new URL(`../../../vectors/v2/${fixture}.bin`, import.meta.url),
        );
        if (reason === undefined) {
            assert.equal(
                fold(input),
                '<http://e/s> <http://e/p> "abc"^^<http://e/dt> .\n',
                fixture,
            );
        } else {
            assert.throws(() => fold(input), new RegExp(reason), fixture);
        }
    }
});

function appendReferences(
    writer: V2SegmentWriter,
    digest: Uint8Array,
    length: number,
): void {
    const subject = Buffer.from("http://e/s");
    const datatype = Buffer.from("http://e/dt");
    writer.append(
        1,
        mustEncode([
            0,
            [
                [1, 2],
                [5, 2],
                [7, 1],
            ],
            2,
            1,
        ]),
        mustEncode([
            [
                [
                    1,
                    [0, 0],
                    [subject.length, datatype.length],
                    Buffer.concat([subject, datatype]),
                ],
                [5, [digest, digest], [length, length], [1, 1], [1, 1], []],
                [7, [digest], [length]],
            ],
            [],
        ]),
    );
    writer.append(
        2,
        mustEncode([2, 3]),
        mustEncode([[[[1], 3]], [[0, 3]], [2, 1, 1], [0, 0, 0]]),
    );
}

test("a pack can rebuild a value before its units in the directory", () => {
    const writer = new V2SegmentWriter();
    const pieces = [
        Uint8Array.of(0xf0),
        Uint8Array.of(0x9f, 0x90),
        Uint8Array.of(0x88),
    ];
    const digest = contextHash("gts2 unit", ...pieces);
    const units = pieces.map((part) => contextHash("gts2 unit", part));
    const entries = mustEncode(
        pieces.map((part, index) => [units[index], part.length]),
    );
    writer.append(
        13,
        mustEncode([5, 5]),
        mustEncode([
            [
                [4, [digest, 4, 3], entries.length],
                [1, [units[0]], 1],
                [1, [units[0]], 1],
                [2, [units[1], [1]], 2],
                [1, [units[2]], 1],
            ],
            Buffer.concat([entries, Uint8Array.of(0), ...pieces]),
        ]),
    );
    appendReferences(writer, digest, 4);
    assert.equal(
        fold(writer.bytes()),
        '<http://e/s> <http://e/s> "🐈"^^<http://e/dt> .\n' +
            '<http://e/s> <http://e/s> "F09F9088"^^<http://www.w3.org/2001/XMLSchema#hexBinary> .\n',
    );
});

test("recipe declarations charge the rebuild budget even when units are missing", () => {
    const writer = new V2SegmentWriter();
    const value = Buffer.from("ab");
    const digest = contextHash("gts2 unit", value);
    writer.append(8, mustEncode([digest]), value);
    const recipeSize = 1n << 37n;
    writer.append(
        10,
        mustEncode([digest, recipeSize, [1], 2]),
        mustEncode([
            [digest, recipeSize - 1n],
            [digest, 1],
        ]),
    );
    writer.append(11, mustEncode([digest, 2, 1]), mustEncode([[digest, 2]]));
    appendReferences(writer, digest, 2);
    assert.throws(() => fold(writer.bytes()), /total_rebuilt/);
});

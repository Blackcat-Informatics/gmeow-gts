// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { inspectFraming } from "../src/v2_framing.js";
import { projectRdf, rdfNQuads } from "../src/v2_rdf_projection.js";
import { V2SegmentWriter } from "../src/v2_writer.js";
import { mustEncode } from "../src/wire.js";

const xsd = "http://www.w3.org/2001/XMLSchema#";
const iri = Buffer.from("http://e/s");

function fold(data: Uint8Array): string {
    const wire = inspectFraming(data, true);
    assert.equal(wire.kind, "v2");
    return rdfNQuads(projectRdf(wire));
}

test("inline typed values generate exact RDF lexical forms", () => {
    const maxUint = (1n << 64n) - 1n;
    const examples = [
        [[1, false], "false", "boolean"],
        [[1, true], "true", "boolean"],
        [[2, maxUint], "18446744073709551615", "nonNegativeInteger"],
        [[3, maxUint], "18446744073709551615", "integer"],
        [[3, -1n - maxUint], "-18446744073709551616", "integer"],
        [[4, -62135596800n, 0], "0001-01-01T00:00:00Z", "dateTime"],
        [[4, -1, 500000000], "1969-12-31T23:59:59.5Z", "dateTime"],
        [
            [4, 253402300799n, 999999999],
            "9999-12-31T23:59:59.999999999Z",
            "dateTime",
        ],
        [[5, Uint8Array.of()], "", "hexBinary"],
        [[5, Uint8Array.of(0, 255, 65)], "00FF41", "hexBinary"],
    ] as const;
    const writer = new V2SegmentWriter();
    writer.append(
        1,
        mustEncode([
            0,
            [
                [1, 1],
                [6, examples.length],
            ],
            2,
            1,
        ]),
        mustEncode([
            [
                [1, [0], [iri.length], iri],
                [6, examples.map(([value]) => value)],
            ],
            [],
        ]),
    );
    writer.append(
        2,
        mustEncode([2, examples.length]),
        mustEncode([
            [[[1], examples.length]],
            [[0, examples.length]],
            examples.map(() => 1),
            examples.map(() => 0),
        ]),
    );
    assert.deepEqual(
        fold(writer.bytes()).trimEnd().split("\n"),
        examples.map(
            ([, lexical, datatype]) =>
                `<http://e/s> <http://e/s> "${lexical}"^^<${xsd}${datatype}> .`,
        ),
    );
});

test("nested annotation aliases collapse by RDF identity while written repeats fail", () => {
    const datatype = Buffer.from(xsd + "nonNegativeInteger");
    for (const [second, repeat] of [
        [9, false],
        [10, true],
    ] as const) {
        const writer = new V2SegmentWriter();
        writer.append(
            1,
            mustEncode([
                0,
                [
                    [1, 2],
                    [2, 1],
                    [6, 2],
                    [4, 6],
                ],
                2,
                1,
            ]),
            mustEncode([
                [
                    [
                        1,
                        [0, 0],
                        [iri.length, datatype.length],
                        Buffer.concat([iri, datatype]),
                    ],
                    [2, [0], [1], Buffer.from("7"), [1], [1], []],
                    [
                        6,
                        [
                            [2, 7],
                            [2, 7],
                        ],
                    ],
                    [
                        4,
                        [0, 0, 0, 0, 0, 0],
                        [0, 0, 0, 0, 0, 0],
                        [2, 3, 4, 5, 6, 7],
                    ],
                ],
                [],
            ]),
        );
        const first = repeat ? 9 : 8;
        writer.append(
            4,
            mustEncode([2, 2]),
            mustEncode([
                [[[1], 2]],
                [0, 0],
                [0, 0],
                [first, second - first],
                [1, 0],
            ]),
        );
        if (repeat) {
            assert.throws(() => fold(writer.bytes()), /unordered_or_repeated/);
        } else {
            assert.equal(
                fold(writer.bytes()),
                '<http://e/s> <http://e/s> <<( <http://e/s> <http://e/s> <<( <http://e/s> <http://e/s> "7"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> )>> )>> .\n',
            );
        }
    }
});

test("typed-value grammar rejects invalid time bounds and non-boolean simple values", () => {
    for (const [name, reason] of [
        ["terms-value-kind-unregistered", /unregistered_value/],
        ["terms-value-time-range.before", /out_of_range/],
        ["terms-value-time-range.after", /out_of_range/],
        ["terms-value-time-range.nanoseconds", /out_of_range/],
    ] as const) {
        const input = readFileSync(
            new URL(`../../../vectors/v2/P3/${name}.bin`, import.meta.url),
        );
        assert.throws(() => fold(input), reason, name);
    }
    const writer = new V2SegmentWriter();
    writer.append(
        1,
        mustEncode([0, [[6, 1]], 2, 1]),
        Buffer.from("82818206818201f9001480", "hex"),
    );
    assert.throws(() => fold(writer.bytes()), /wrong_type/);
});

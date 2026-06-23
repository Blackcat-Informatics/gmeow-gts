// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { test } from "node:test";
import assert from "node:assert/strict";
import fc from "fast-check";
import { TermKind } from "../src/model.js";
import { toNQuads } from "../src/nquads.js";
import { Read } from "../src/reader.js";
import { Writer } from "../src/writer.js";

const CAT = "https://example.org/Cat";
const LABEL = "http://www.w3.org/2000/01/rdf-schema#label";
const POSITIVE_DECIMAL_INTEGER = /^[1-9][0-9]*$/;

function envInt(name: string, fallback: number): number {
    const raw = process.env[name];
    if (raw === undefined) return fallback;
    if (!POSITIVE_DECIMAL_INTEGER.test(raw)) {
        throw new Error(`${name} must be a positive decimal integer`);
    }
    const value = Number(raw);
    if (!Number.isSafeInteger(value)) {
        throw new Error(`${name} must be a safe positive integer`);
    }
    return value;
}

const propertyParameters = {
    numRuns: envInt("GTS_PROPERTY_RUNS", 50),
    seed: envInt("GTS_PROPERTY_SEED", 20260623),
};

function labelLog(labels: string[]): Uint8Array {
    const writer = new Writer("dist");
    writer.addTerms([
        { kind: TermKind.Iri, value: CAT },
        { kind: TermKind.Iri, value: LABEL },
        ...labels.map((label) => ({
            kind: TermKind.Literal,
            value: label,
            lang: "en",
        })),
    ]);
    writer.addQuads(labels.map((_, idx) => ({ s: 0, p: 1, o: idx + 2 })));
    return writer.toBytes();
}

test("reader refuses arbitrary bytes without throwing", () => {
    fc.assert(
        fc.property(fc.uint8Array({ maxLength: 512 }), (data) => {
            const graph = Read(data, false);

            for (const diagnostic of graph.diagnostics) {
                assert.notEqual(diagnostic.code, "");
                assert.notEqual(diagnostic.detail, "");
            }

            assert.doesNotThrow(() => toNQuads(graph));
        }),
        propertyParameters,
    );
});

test("writer decode, projection, and torn append are deterministic", () => {
    fc.assert(
        fc.property(
            fc.array(fc.string({ unit: "binary", maxLength: 24 }), {
                minLength: 1,
                maxLength: 5,
            }),
            (labels) => {
                const first = labelLog(labels);
                const second = labelLog(labels);

                assert.deepEqual(first, second);

                const graph = Read(first, false);
                assert.deepEqual(
                    graph.diagnostics.map((d) => d.code),
                    [],
                );
                assert.equal(graph.quads.length, labels.length);

                const projection = toNQuads(graph);
                assert.equal(projection, toNQuads(Read(second, false)));

                const torn = new Uint8Array(first.length + 1);
                torn.set(first);
                torn.set([0xa3], first.length);

                const tornGraph = Read(torn, false);
                assert.ok(
                    tornGraph.diagnostics.some(
                        (diagnostic) => diagnostic.code === "TornAppendError",
                    ),
                );
                assert.deepEqual(tornGraph.quads, graph.quads);
                assert.equal(toNQuads(tornGraph), projection);
            },
        ),
        propertyParameters,
    );
});

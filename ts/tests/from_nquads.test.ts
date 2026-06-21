// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { fromNQuads, NQuadsParseError } from "../src/from_nquads.js";
import { Read } from "../src/reader.js";
import { toNQuads } from "../src/nquads.js";
import { RDF_DIR_LANG_STRING, TermKind } from "../src/model.js";
import { Writer } from "../src/writer.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");
const cli = resolve(__dirname, "../bin/gts.js");
const vectorsDir = join(repoRoot, "vectors");
const RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

function sortedLines(text: string): string[] {
    return text
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .sort();
}

function roundTrip(nq: string): string {
    return toNQuads(Read(fromNQuads(nq), false));
}

test("fromNQuads inverts fold output for a corpus vector", () => {
    const src = readFileSync(join(vectorsDir, "11-datatype-defaulting.gts"));
    const nq = toNQuads(Read(src, false));
    assert.deepEqual(sortedLines(roundTrip(nq)), sortedLines(nq));
});

test("fromNQuads preserves named graphs, reifiers, and annotations", () => {
    const w = new Writer("dist");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://ex/s" },
        { kind: TermKind.Iri, value: "https://ex/p" },
        { kind: TermKind.Iri, value: "https://ex/o" },
        { kind: TermKind.Iri, value: "https://ex/g" },
        { kind: TermKind.Iri, value: "https://ex/conf" },
        { kind: TermKind.Literal, value: "0.9" },
    ]);
    w.addQuads([{ s: 0, p: 1, o: 2, g: 3 }]);
    w.addReifies([{ rid: 0, spo: { s: 0, p: 1, o: 2 } }]);
    w.addAnnot([{ s: 0, p: 4, o: 5 }]);
    const nq = toNQuads(Read(w.toBytes(), false));
    assert.deepEqual(sortedLines(roundTrip(nq)), sortedLines(nq));
});

test("fromNQuads preserves language-tagged and datatyped literals", () => {
    const xsdInt = "http://www.w3.org/2001/XMLSchema#integer";
    const nq =
        '<https://ex/s> <https://ex/label> "Cat"@en .\n' +
        `<https://ex/s> <https://ex/n> "42"^^<${xsdInt}> .\n` +
        "_:b0 <https://ex/p> <https://ex/s> .\n";
    assert.deepEqual(sortedLines(roundTrip(nq)), sortedLines(nq));
});

test("fromNQuads preserves directional language literals", () => {
    const nq = '<https://ex/s> <https://ex/label> "RTL"@ar--rtl .\n';
    const graph = Read(fromNQuads(nq), false);
    const literal = graph.terms.find((term) => term.kind === TermKind.Literal);
    assert.equal(literal?.lang, "ar");
    assert.equal(literal?.direction, "rtl");
    assert.equal(
        literal === undefined ? undefined : graph.datatypeIri(literal),
        RDF_DIR_LANG_STRING,
    );
    assert.deepEqual(sortedLines(toNQuads(graph)), sortedLines(nq));
});

test("writer allows multiple reifiers for the same statement", () => {
    const w = new Writer("dist");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://ex/r1" },
        { kind: TermKind.Iri, value: "https://ex/r2" },
        { kind: TermKind.Iri, value: "https://ex/s" },
        { kind: TermKind.Iri, value: "https://ex/p" },
        { kind: TermKind.Iri, value: "https://ex/o" },
    ]);
    w.addQuads([{ s: 2, p: 3, o: 4 }]);
    w.addReifies([
        { rid: 0, spo: { s: 2, p: 3, o: 4 } },
        { rid: 1, spo: { s: 2, p: 3, o: 4 } },
    ]);
    const graph = Read(w.toBytes(), false);
    assert.equal(graph.reifiers.length, 2);
    assert.deepEqual(graph.reifiers.map((entry) => entry.rid).sort(), [0, 1]);
});

test("fromNQuads preserves multiple reifiers for the same statement", () => {
    const nq =
        `<https://ex/r1> <${RDF_REIFIES}> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n` +
        `<https://ex/r2> <${RDF_REIFIES}> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n`;
    const graph = Read(fromNQuads(nq), false);
    assert.equal(graph.reifiers.length, 2);
    assert.deepEqual(sortedLines(toNQuads(graph)), sortedLines(nq));
});

test("fromNQuads handles compact blank-node and language-tag delimiters", () => {
    const nq =
        "<https://ex/s> <https://ex/p> _:b0.\n" +
        '<https://ex/s> <https://ex/label> "Cat"@en.\n';
    const expected =
        "<https://ex/s> <https://ex/p> _:b0 .\n" +
        '<https://ex/s> <https://ex/label> "Cat"@en .\n';
    assert.deepEqual(sortedLines(roundTrip(nq)), sortedLines(expected));
});

test("fromNQuads keeps quoted-triple close delimiters out of tokens", () => {
    const nq =
        `<https://ex/r1> <${RDF_REIFIES}> <<( _:b0 <https://ex/p> _:b1)>> .\n` +
        `<https://ex/r2> <${RDF_REIFIES}> <<( <https://ex/s> <https://ex/p> "Cat"@en)>> .\n`;
    const expected =
        `<https://ex/r1> <${RDF_REIFIES}> <<( _:b0 <https://ex/p> _:b1 )>> .\n` +
        `<https://ex/r2> <${RDF_REIFIES}> <<( <https://ex/s> <https://ex/p> "Cat"@en )>> .\n`;
    assert.deepEqual(sortedLines(roundTrip(nq)), sortedLines(expected));
});

test("fromNQuads rejects malformed statements", () => {
    assert.throws(
        () => fromNQuads("<https://ex/s> <https://ex/p> .\n"),
        NQuadsParseError,
    );
});

test("fromNQuads rejects empty blank-node labels and language tags", () => {
    assert.throws(
        () => fromNQuads("<https://ex/s> <https://ex/p> _: .\n"),
        NQuadsParseError,
    );
    assert.throws(
        () => fromNQuads('<https://ex/s> <https://ex/p> "Cat"@ .\n'),
        NQuadsParseError,
    );
});

test("CLI from-nq writes a GTS that folds to the input N-Quads", () => {
    const tmp = mkdtempSync(join(tmpdir(), "gts-from-nq-"));
    const src = readFileSync(join(vectorsDir, "11-datatype-defaulting.gts"));
    const nq = toNQuads(Read(src, false));
    const nqPath = join(tmp, "in.nq");
    const outPath = join(tmp, "out.gts");
    writeFileSync(nqPath, nq);
    const result = spawnSync("node", [cli, "from-nq", nqPath, "-o", outPath], {
        encoding: "utf8",
    });
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(
        sortedLines(toNQuads(Read(readFileSync(outPath), false))),
        sortedLines(nq),
    );
});

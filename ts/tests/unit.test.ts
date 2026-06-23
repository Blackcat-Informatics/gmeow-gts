// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";
import { Graph, TermKind } from "../src/model.js";
import * as wire from "../src/wire.js";
import { Writer } from "../src/writer.js";
import { Read } from "../src/reader.js";
import { toNQuads } from "../src/nquads.js";
import { decodeChain, gzip, identity, isCodecError } from "../src/codec.js";

const __dirname = dirname(fileURLToPath(import.meta.url));

test("wire.encode round-trips values", () => {
    const m = new Map<unknown, unknown>();
    m.set("a", 1);
    m.set("b", new Uint8Array([0, 1, 2]));
    const encoded = wire.encode(m);
    assert.ok(encoded.length > 0);
    const decoded = wire.decodeFirst(encoded);
    assert.ok(decoded instanceof Map);
    assert.equal(decoded.get("a"), 1);
    const bytes = wire.asBytes(decoded.get("b"));
    assert.ok(bytes);
    // Byte strings encode as CBOR major type 2 (never an RFC 8746 typed
    // array), so the decoded value compares by content.
    assert.deepEqual(new Uint8Array(bytes), new Uint8Array([0, 1, 2]));
});

test("wire.encode emits plain byte strings, not tag-64 typed arrays", () => {
    const encoded = wire.encode(new Uint8Array([0, 1, 2]));
    // 0x43 = major type 2 (byte string), length 3.
    assert.deepEqual(new Uint8Array(encoded), new Uint8Array([0x43, 0, 1, 2]));
});

test("blake3_256 returns 32 bytes", () => {
    const h = wire.blake3_256(new Uint8Array([0]));
    assert.equal(h.length, 32);
});

test("codec identity round-trip", () => {
    const data = new TextEncoder().encode("hello gts");
    assert.deepEqual(identity.decode(identity.encode(data)), data);
});

test("codec gzip round-trip", () => {
    const data = new TextEncoder().encode("hello gts hello gts");
    assert.deepEqual(gzip.decode(gzip.encode(data)), data);
});

test("codec zstd decodes the zstd corpus vector", () => {
    const path = resolve(__dirname, "../../../vectors/02-zstd-frame.gts");
    const g = Read(readFileSync(path), false);
    assert.equal(g.diagnostics.length, 0);
    assert.ok(g.quads.length > 0);
});

test("codec zstd rejects outputs over the safety bound", () => {
    const encoded = Uint8Array.from(
        Buffer.from(
            "KLUv/YBYAQAAAVQAABAAAAEA+/85wAICABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAAAgAQAAIAEAACABAACQAAAA==",
            "base64",
        ),
    );

    assert.throws(
        () => decodeChain([{ name: "zstd", cls: "compress" }], encoded),
        (err: unknown) =>
            isCodecError(err) &&
            err.failed &&
            err.detail.includes("decompressed size exceeds safety bound"),
    );
});

test("writer produces a readable GTS log", () => {
    const w = new Writer("dist");
    const t1 = { kind: TermKind.Iri, value: "https://example.org/Cat" };
    const t2 = { kind: TermKind.Literal, value: "Cat", lang: "en" };
    const t3 = {
        kind: TermKind.Iri,
        value: "http://www.w3.org/2000/01/rdf-schema#label",
    };
    w.addTerms([t1, t2, t3]);
    w.addQuads([{ s: 0, p: 2, o: 1 }]);
    const data = w.toBytes();
    const g = Read(data, false);
    assert.equal(g.terms.length, 3);
    assert.equal(g.quads.length, 1);
    assert.equal(g.segmentProfiles[0], "dist");
    assert.equal(g.diagnostics.length, 0);
});

test("toNQuads serialises a simple graph", () => {
    const g = new Graph();
    g.terms.push(
        { kind: TermKind.Iri, value: "https://example.org/Cat" },
        { kind: TermKind.Literal, value: "Cat", lang: "en" },
        {
            kind: TermKind.Iri,
            value: "http://www.w3.org/2000/01/rdf-schema#label",
        },
    );
    g.quads.push({ s: 0, p: 2, o: 1 });
    const out = toNQuads(g);
    assert.equal(
        out.trim(),
        '<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> "Cat"@en .',
    );
});

test("reader rejects a torn file", () => {
    const w = new Writer("generic");
    w.addTerms([{ kind: TermKind.Iri, value: "https://example.org/A" }]);
    const data = w.toBytes();
    const torn = data.subarray(0, data.length - 4);
    const g = Read(torn, false);
    assert.ok(g.diagnostics.some((d) => d.code === "TornAppendError"));
});

test("public reader reports malformed input diagnostics without throwing", () => {
    const writer = new Writer("generic");
    const torn = new Uint8Array(writer.toBytes().length + 1);
    torn.set(writer.toBytes());
    torn.set([0xa3], writer.toBytes().length);

    const cases: [Uint8Array, string[]][] = [
        [new Uint8Array(), ["EmptyFile"]],
        [Uint8Array.of(0x01), ["DamagedFrame"]],
        [torn, ["TornAppendError"]],
    ];
    for (const [data, expected] of cases) {
        assert.deepEqual(
            Read(data, false).diagnostics.map((d) => d.code),
            expected,
        );
    }
});

test("reader allows clean multi-segment file", () => {
    const w1 = new Writer("dist");
    w1.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/Cat" },
        { kind: TermKind.Literal, value: "Cat", lang: "en" },
        {
            kind: TermKind.Iri,
            value: "http://www.w3.org/2000/01/rdf-schema#label",
        },
    ]);
    w1.addQuads([{ s: 0, p: 2, o: 1 }]);

    const w2 = new Writer("dist");
    w2.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/Dog" },
        { kind: TermKind.Literal, value: "Dog", lang: "en" },
        {
            kind: TermKind.Iri,
            value: "http://www.w3.org/2000/01/rdf-schema#label",
        },
    ]);
    w2.addQuads([{ s: 0, p: 2, o: 1 }]);

    // Concatenate segments manually (Writer produces a full header each time).
    const combined = new Uint8Array(w1.toBytes().length + w2.toBytes().length);
    combined.set(w1.toBytes());
    combined.set(w2.toBytes(), w1.toBytes().length);

    const g = Read(combined, true);
    assert.equal(g.segmentHeads.length, 2);
    assert.equal(g.quads.length, 2);
});

import { existsSync, mkdirSync, mkdtempSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { unpack } from "../src/files.js";

const filesNS = "https://w3id.org/gts/files#";
const rdfType = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

function filesGraphWithPath(path: string): Graph {
    const g = new Graph();
    const subj = g.terms.push({ kind: TermKind.Bnode, value: "f0" }) - 1;
    const fileEntry =
        g.terms.push({ kind: TermKind.Iri, value: filesNS + "FileEntry" }) - 1;
    const typePred = g.terms.push({ kind: TermKind.Iri, value: rdfType }) - 1;
    const pathPred =
        g.terms.push({ kind: TermKind.Iri, value: filesNS + "path" }) - 1;
    const digestPred =
        g.terms.push({ kind: TermKind.Iri, value: filesNS + "digest" }) - 1;
    const sizePred =
        g.terms.push({ kind: TermKind.Iri, value: filesNS + "size" }) - 1;
    const pathVal = g.terms.push({ kind: TermKind.Literal, value: path }) - 1;

    const empty = new Uint8Array(0);
    const digest = wire.digestStr(empty);
    const digestVal =
        g.terms.push({ kind: TermKind.Literal, value: digest }) - 1;
    const sizeVal =
        g.terms.push({ kind: TermKind.Literal, value: String(empty.length) }) -
        1;

    g.quads.push({ s: subj, p: typePred, o: fileEntry });
    g.quads.push({ s: subj, p: pathPred, o: pathVal });
    g.quads.push({ s: subj, p: digestPred, o: digestVal });
    g.quads.push({ s: subj, p: sizePred, o: sizeVal });
    g.setBlob(digest, empty);
    return g;
}

test("reader rejects a malformed transform field", () => {
    const w = new Writer("generic");
    w.addTerms([{ kind: TermKind.Iri, value: "https://example.org/A" }]);

    const badFrame = new Map<unknown, unknown>();
    badFrame.set("t", "terms");
    badFrame.set("x", "not-an-array");
    badFrame.set("d", new Uint8Array([0xa1, 0x61, 0x78, 0x01]));
    badFrame.set("prev", w.head());
    badFrame.set("id", wire.contentId(badFrame));
    const frameBytes = wire.encode(badFrame);

    const data = new Uint8Array(w.toBytes().length + frameBytes.length);
    data.set(w.toBytes());
    data.set(frameBytes, w.toBytes().length);

    const g = Read(data, false);
    assert.ok(
        g.diagnostics.some((d) => d.code === "DamagedFrame"),
        "expected DamagedFrame diagnostic for malformed transform",
    );
});

test("unpack rejects Windows-style path traversal", () => {
    const g = filesGraphWithPath("..\\\\..\\\\etc\\\\passwd");
    const dest = mkdtempSync(join(tmpdir(), "gts-unpack-"));
    assert.throws(() => unpack(g, dest), /path traversal/);
});

test("unpack rejects drive-relative archive paths", () => {
    const g = filesGraphWithPath("C:\\\\secret.txt");
    const dest = mkdtempSync(join(tmpdir(), "gts-unpack-"));
    assert.throws(() => unpack(g, dest), /absolute or drive-relative path/);
});

test("unpack rejects destination symlink escapes", (t) => {
    const g = filesGraphWithPath("link/escape.txt");
    const root = mkdtempSync(join(tmpdir(), "gts-unpack-symlink-"));
    const dest = join(root, "dst");
    const outside = join(root, "outside");
    mkdirSync(dest);
    mkdirSync(outside);
    try {
        symlinkSync(outside, join(dest, "link"), "dir");
    } catch (err) {
        t.skip(`symlink creation unavailable: ${err}`);
        return;
    }

    assert.throws(() => unpack(g, dest), /path escapes destination/);
    assert.equal(existsSync(join(outside, "escape.txt")), false);
});

test("unpack rejects leaf symlink redirects", (t) => {
    const g = filesGraphWithPath("target.txt");
    const root = mkdtempSync(join(tmpdir(), "gts-unpack-leaf-symlink-"));
    const dest = join(root, "dst");
    const outside = join(root, "outside");
    mkdirSync(dest);
    mkdirSync(outside);
    try {
        symlinkSync(join(outside, "escape.txt"), join(dest, "target.txt"));
    } catch (err) {
        t.skip(`symlink creation unavailable: ${err}`);
        return;
    }

    assert.throws(() => unpack(g, dest), /symlink/);
    assert.equal(existsSync(join(outside, "escape.txt")), false);
});

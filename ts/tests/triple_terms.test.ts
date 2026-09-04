// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/**
 * Self-describing triple terms and the multi-valued `reifies` layer (§7.3).
 *
 * `rdf:reifies` is not functional, so a reifier id can never identify a triple.
 * These tests pin the wire consequences: every `reifies` row survives, a triple
 * term states its own `(s, p, o)` in `"tt"`, and the one remaining incoherence —
 * a legacy `"tt"`-less term over an over-bound reifier — is the only shape that
 * still raises `ConflictingReifier`.
 */

import { test } from "node:test";
import assert from "node:assert/strict";

import { Read, ReadFileSegments } from "../src/reader.js";
import { readStream } from "../src/browser.js";
import { Writer } from "../src/writer.js";
import { toNQuads } from "../src/nquads.js";
import { fromNQuads } from "../src/from_nquads.js";
import { compactStreamable } from "../src/compact.js";
import { Graph, TermKind, type Triple } from "../src/model.js";

const RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const LABEL = "http://www.w3.org/2000/01/rdf-schema#label";

function codes(g: Graph): string[] {
    return g.diagnostics.map((d) => d.code);
}

function concat(parts: Uint8Array[]): Uint8Array {
    let length = 0;
    for (const p of parts) length += p.length;
    const out = new Uint8Array(length);
    let offset = 0;
    for (const p of parts) {
        out.set(p, offset);
        offset += p.length;
    }
    return out;
}

function stream(bytes: Uint8Array): ReadableStream<Uint8Array> {
    return new ReadableStream<Uint8Array>({
        start(controller) {
            controller.enqueue(bytes);
            controller.close();
        },
    });
}

function sortedLines(text: string): string[] {
    return text
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0)
        .sort();
}

/** `r rdf:reifies <<(s p "Cat"@en)>>` and `r rdf:reifies <<(s p "Chat"@fr)>>`. */
function twoBindingsOnOneReifier(): Writer {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/r1" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/Cat" }, // 1
        { kind: TermKind.Iri, value: LABEL }, // 2
        { kind: TermKind.Literal, value: "Cat", lang: "en" }, // 3
        { kind: TermKind.Literal, value: "Chat", lang: "fr" }, // 4
    ]);
    w.addReifies([{ rid: 0, spo: { s: 1, p: 2, o: 3 } }]);
    w.addReifies([{ rid: 0, spo: { s: 1, p: 2, o: 4 } }]);
    return w;
}

test("reifies is multi-valued: both bindings survive with no diagnostic", () => {
    const g = Read(twoBindingsOnOneReifier().toBytes(), false);
    assert.deepEqual(codes(g), []);
    assert.deepEqual(g.reifierTriples(0), [
        { s: 1, p: 2, o: 3 },
        { s: 1, p: 2, o: 4 },
    ]);
    assert.deepEqual(
        g.reifiers.map((r) => r.spo),
        [
            { s: 1, p: 2, o: 3 },
            { s: 1, p: 2, o: 4 },
        ],
    );
    // Both rows project, in file order.
    assert.deepEqual(sortedLines(toNQuads(g)), [
        `<https://example.org/r1> <${RDF_REIFIES}> <<( <https://example.org/Cat> <${LABEL}> "Cat"@en )>> .`,
        `<https://example.org/r1> <${RDF_REIFIES}> <<( <https://example.org/Cat> <${LABEL}> "Chat"@fr )>> .`,
    ]);
});

test("a byte-identical reifies row still collapses (§7.8 set semantics)", () => {
    const w = twoBindingsOnOneReifier();
    w.addReifies([{ rid: 0, spo: { s: 1, p: 2, o: 3 } }]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), []);
    assert.equal(g.reifiers.length, 2);
});

test("a reifies row differing only in its graph slot is its own row", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/r1" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 2
        { kind: TermKind.Iri, value: "https://example.org/o" }, // 3
        { kind: TermKind.Iri, value: "https://example.org/g" }, // 4
    ]);
    w.addReifies([
        { rid: 0, spo: { s: 1, p: 2, o: 3 } },
        { rid: 0, spo: { s: 1, p: 2, o: 3 }, g: 4 },
        { rid: 0, spo: { s: 1, p: 2, o: 3 }, g: 4 },
    ]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), []);
    assert.equal(g.reifiers.length, 2);
    // The multi-valued view is by triple, so one distinct triple remains one.
    assert.deepEqual(g.reifierTriples(0), [{ s: 1, p: 2, o: 3 }]);
});

test("ConflictingReifier is only the legacy tt-less indirect term", () => {
    const w = twoBindingsOnOneReifier();
    w.addTerms([{ kind: TermKind.Triple, value: "", reifier: 0 }]); // 5
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), ["ConflictingReifier"]);
    // The shape is a property of the completed fold, not of one frame.
    assert.equal(g.diagnostics[0].frameIndex, undefined);
    // NO row is dropped, and the term still resolves to the first binding.
    assert.equal(g.reifiers.length, 2);
    assert.deepEqual(g.tripleOf(5), { s: 1, p: 2, o: 3 });
});

test("ConflictingReifier is reported once per offending term", () => {
    const w = twoBindingsOnOneReifier();
    w.addTerms([
        { kind: TermKind.Triple, value: "", reifier: 0 }, // 5
        { kind: TermKind.Triple, value: "", reifier: 0 }, // 6
    ]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), ["ConflictingReifier", "ConflictingReifier"]);
});

test("self-describing triple terms sharing a reifier id stay distinct", () => {
    const w = twoBindingsOnOneReifier();
    w.addTerms([
        {
            kind: TermKind.Triple,
            value: "",
            reifier: 0,
            triple: { s: 1, p: 2, o: 3 },
        }, // 5
        {
            kind: TermKind.Triple,
            value: "",
            reifier: 0,
            triple: { s: 1, p: 2, o: 4 },
        }, // 6
    ]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), []);
    assert.deepEqual(g.tripleOf(5), { s: 1, p: 2, o: 3 });
    assert.deepEqual(g.tripleOf(6), { s: 1, p: 2, o: 4 });
});

test("tt is authoritative over a reifier binding that disagrees", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/r1" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 2
        { kind: TermKind.Iri, value: "https://example.org/o1" }, // 3
        { kind: TermKind.Iri, value: "https://example.org/o2" }, // 4
    ]);
    w.addReifies([{ rid: 0, spo: { s: 1, p: 2, o: 3 } }]);
    w.addTerms([
        {
            kind: TermKind.Triple,
            value: "",
            reifier: 0,
            triple: { s: 1, p: 2, o: 4 },
        }, // 5
    ]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), []);
    assert.deepEqual(g.tripleOf(5), { s: 1, p: 2, o: 4 });
    // The legacy indirection survives as provenance and changes nothing.
    assert.equal(g.terms[5].reifier, 0);
});

test("an unreified triple term round-trips as itself", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o" }, // 2
        { kind: TermKind.Triple, value: "", triple: { s: 0, p: 1, o: 2 } }, // 3
        { kind: TermKind.Iri, value: "https://example.org/says" }, // 4
    ]);
    w.addQuads([{ s: 0, p: 4, o: 3 }]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), []);
    // A value, not a statement: no reifier is minted and no row is emitted.
    assert.deepEqual(g.reifiers, []);
    assert.equal(g.terms[3].reifier, undefined);
    assert.deepEqual(g.terms[3].triple, { s: 0, p: 1, o: 2 });
    assert.equal(
        toNQuads(g).trim(),
        "<https://example.org/s> <https://example.org/says> " +
            "<<( <https://example.org/s> <https://example.org/p> " +
            "<https://example.org/o> )>> .",
    );
});

test("a forward-referencing tt is diagnosed and dropped", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        // 2: its own object component names a term that does not exist yet.
        { kind: TermKind.Triple, value: "", triple: { s: 0, p: 1, o: 9 } },
    ]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), ["ForwardReference"]);
    assert.equal(g.terms[2].triple, undefined);
    assert.equal(g.tripleOf(2), undefined);
});

test("a tt violating position constraints is dropped, falling back to rf", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o" }, // 2
        { kind: TermKind.Literal, value: "not a term position" }, // 3
        { kind: TermKind.Iri, value: "https://example.org/r" }, // 4
    ]);
    w.addReifies([{ rid: 4, spo: { s: 0, p: 1, o: 2 } }]);
    w.addTerms([
        // 5: predicate is a literal — falls back to the reifier binding.
        {
            kind: TermKind.Triple,
            value: "",
            reifier: 4,
            triple: { s: 0, p: 3, o: 2 },
        },
        // 6: subject is a literal — no fallback, so it states nothing.
        { kind: TermKind.Triple, value: "", triple: { s: 3, p: 1, o: 2 } },
    ]);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), ["PositionConstraint", "PositionConstraint"]);
    assert.deepEqual(g.tripleOf(5), { s: 0, p: 1, o: 2 });
    assert.equal(g.tripleOf(6), undefined);
});

test("a snapshot's tt components are shifted into the segment id space", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/outer" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/says" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/pad" }, // 2
    ]);
    const snapTerm = (entries: [string, unknown][]): Map<unknown, unknown> =>
        new Map<unknown, unknown>(entries);
    const payload = new Map<unknown, unknown>([
        [
            "terms",
            [
                snapTerm([
                    ["k", 0],
                    ["v", "https://example.org/s"],
                ]),
                snapTerm([
                    ["k", 0],
                    ["v", "https://example.org/p"],
                ]),
                snapTerm([
                    ["k", 0],
                    ["v", "https://example.org/o"],
                ]),
                snapTerm([
                    ["k", 3],
                    ["tt", [0, 1, 2]],
                ]),
            ],
        ],
        // Snapshot-local ids throughout: the reader shifts rows and terms alike.
        ["quads", [[0, 1, 3]]],
    ]);
    w.addFrame("snapshot", payload);
    const g = Read(w.toBytes(), false);
    assert.deepEqual(codes(g), []);
    assert.equal(g.terms.length, 7);
    assert.deepEqual(g.terms[6].triple, { s: 3, p: 4, o: 5 });
    assert.equal(
        toNQuads(g).trim(),
        "<https://example.org/s> <https://example.org/p> " +
            "<<( <https://example.org/s> <https://example.org/p> " +
            "<https://example.org/o> )>> .",
    );
});

test("the union keeps two tt terms sharing a reifier id distinct", () => {
    const segment = (): Uint8Array => {
        const w = new Writer("dist");
        w.addTerms([
            { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
            { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
            { kind: TermKind.Iri, value: "https://example.org/o1" }, // 2
            { kind: TermKind.Iri, value: "https://example.org/o2" }, // 3
            { kind: TermKind.Iri, value: "https://example.org/r" }, // 4
        ]);
        w.addReifies([
            { rid: 4, spo: { s: 0, p: 1, o: 2 } },
            { rid: 4, spo: { s: 0, p: 1, o: 3 } },
        ]);
        w.addTerms([
            {
                kind: TermKind.Triple,
                value: "",
                reifier: 4,
                triple: { s: 0, p: 1, o: 2 },
            }, // 5
            {
                kind: TermKind.Triple,
                value: "",
                reifier: 4,
                triple: { s: 0, p: 1, o: 3 },
            }, // 6
        ]);
        w.addQuads([
            { s: 0, p: 1, o: 5 },
            { s: 0, p: 1, o: 6 },
        ]);
        return w.toBytes();
    };
    const g = Read(concat([segment(), segment()]), true);
    assert.deepEqual(codes(g), []);
    const quoted: Triple[] = [];
    for (let tid = 0; tid < g.terms.length; tid++) {
        if (g.terms[tid].kind !== TermKind.Triple) continue;
        const spo = g.tripleOf(tid);
        assert.notEqual(spo, undefined);
        quoted.push(spo!);
    }
    // Two DISTINCT triple terms over one reifier id, not collapsed into one.
    assert.equal(quoted.length, 2);
    assert.notDeepEqual(quoted[0], quoted[1]);
    assert.equal(g.quads.length, 2);
});

test("the union merges a tt term with a legacy term resolving the same", () => {
    // §7.8 defines triple-term equality as equality of the RESOLVED (s, p, o),
    // so there is one key space: a self-describing term and a legacy indirect
    // term that denote the same triple are the same term.
    const legacy = new Writer("dist");
    legacy.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o" }, // 2
        { kind: TermKind.Iri, value: "https://example.org/r" }, // 3
    ]);
    legacy.addReifies([{ rid: 3, spo: { s: 0, p: 1, o: 2 } }]);
    legacy.addTerms([{ kind: TermKind.Triple, value: "", reifier: 3 }]); // 4
    legacy.addQuads([{ s: 0, p: 1, o: 4 }]);

    const selfDescribing = new Writer("dist");
    selfDescribing.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o" }, // 2
        { kind: TermKind.Triple, value: "", triple: { s: 0, p: 1, o: 2 } }, // 3
    ]);
    selfDescribing.addQuads([{ s: 0, p: 1, o: 3 }]);

    const g = Read(concat([legacy.toBytes(), selfDescribing.toBytes()]), true);
    assert.deepEqual(codes(g), []);
    const quoted = g.terms.filter((t) => t.kind === TermKind.Triple);
    assert.equal(quoted.length, 1, "the two forms intern to ONE union term");
    // The quads therefore coincide and collapse under set semantics.
    assert.equal(g.quads.length, 1);
    const spo = g.tripleOf(g.quads[0].o);
    assert.notEqual(spo, undefined);
    assert.deepEqual(
        [g.terms[spo!.s].value, g.terms[spo!.p].value, g.terms[spo!.o].value],
        [
            "https://example.org/s",
            "https://example.org/p",
            "https://example.org/o",
        ],
    );
});

test("the union sees a legacy term over-bound from another segment", () => {
    const w1 = new Writer("dist");
    w1.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o1" }, // 2
        { kind: TermKind.Iri, value: "https://example.org/r" }, // 3
    ]);
    w1.addReifies([{ rid: 3, spo: { s: 0, p: 1, o: 2 } }]);
    w1.addTerms([{ kind: TermKind.Triple, value: "", reifier: 3 }]); // 4
    w1.addQuads([{ s: 0, p: 1, o: 4 }]);

    const w2 = new Writer("dist");
    w2.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o2" }, // 2
        { kind: TermKind.Iri, value: "https://example.org/r" }, // 3
    ]);
    w2.addReifies([{ rid: 3, spo: { s: 0, p: 1, o: 2 } }]);

    const data = concat([w1.toBytes(), w2.toBytes()]);

    // Neither segment alone is incoherent.
    const perSegment = ReadFileSegments(data);
    assert.deepEqual(codes(perSegment.segments[0]), []);
    assert.deepEqual(codes(perSegment.segments[1]), []);

    // The union is: one legacy term, one reifier, two distinct bindings.
    const g = Read(data, true);
    assert.deepEqual(codes(g), ["ConflictingReifier"]);
    assert.equal(g.reifiers.length, 2);
});

test("browser fold resolves tt and reports the same conflict shape", async () => {
    const distinct = twoBindingsOnOneReifier();
    distinct.addTerms([
        {
            kind: TermKind.Triple,
            value: "",
            reifier: 0,
            triple: { s: 1, p: 2, o: 3 },
        }, // 5
        {
            kind: TermKind.Triple,
            value: "",
            reifier: 0,
            triple: { s: 1, p: 2, o: 4 },
        }, // 6
    ]);
    const ok = await readStream(stream(distinct.toBytes()), {
        allowSegments: false,
    });
    assert.deepEqual(codes(ok), []);
    assert.deepEqual(ok.tripleOf(5), { s: 1, p: 2, o: 3 });
    assert.deepEqual(ok.tripleOf(6), { s: 1, p: 2, o: 4 });

    const legacy = twoBindingsOnOneReifier();
    legacy.addTerms([{ kind: TermKind.Triple, value: "", reifier: 0 }]); // 5
    const conflicted = await readStream(stream(legacy.toBytes()), {
        allowSegments: false,
    });
    assert.deepEqual(codes(conflicted), ["ConflictingReifier"]);
    assert.equal(conflicted.reifiers.length, 2);
    assert.deepEqual(conflicted.tripleOf(5), { s: 1, p: 2, o: 3 });
});

test("fromNQuads interns a bare quoted triple as a self-describing term", () => {
    const nq =
        "<https://example.org/s> <https://example.org/says> " +
        "<<( <https://example.org/s> <https://example.org/p> " +
        "<https://example.org/o> )>> .\n";
    const g = Read(fromNQuads(nq), false);
    assert.deepEqual(codes(g), []);
    // No reifier is minted for a quoted triple used as a value (§7.3).
    assert.deepEqual(g.reifiers, []);
    const quoted = g.terms.filter((t) => t.kind === TermKind.Triple);
    assert.equal(quoted.length, 1);
    assert.notEqual(quoted[0].triple, undefined);
    assert.equal(quoted[0].reifier, undefined);
    assert.equal(g.quads.length, 1);
    assert.deepEqual(sortedLines(toNQuads(g)), sortedLines(nq));
});

test("fromNQuads keeps two rdf:reifies bindings on one reifier", () => {
    const nq =
        `<https://example.org/r1> <${RDF_REIFIES}> ` +
        `<<( <https://example.org/Cat> <${LABEL}> "Cat"@en )>> .\n` +
        `<https://example.org/r1> <${RDF_REIFIES}> ` +
        `<<( <https://example.org/Cat> <${LABEL}> "Chat"@fr )>> .\n`;
    const g = Read(fromNQuads(nq), false);
    assert.deepEqual(codes(g), []);
    assert.equal(g.reifiers.length, 2);
    assert.deepEqual(sortedLines(toNQuads(g)), sortedLines(nq));
});

test("streamable compaction shifts tt into the output id space", () => {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/s" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Iri, value: "https://example.org/o" }, // 2
        { kind: TermKind.Triple, value: "", triple: { s: 0, p: 1, o: 2 } }, // 3
        { kind: TermKind.Iri, value: "https://example.org/says" }, // 4
    ]);
    w.addQuads([{ s: 0, p: 4, o: 3 }]);
    const compacted = compactStreamable(w.toBytes(), {
        timestamp: "2026-01-01T00:00:00Z",
    });
    const after = Read(compacted, true);
    assert.deepEqual(codes(after), []);
    const quoted = after.terms.findIndex((t) => t.kind === TermKind.Triple);
    assert.notEqual(quoted, -1);
    // Ids moved (the leading streaming index occupies the low id space); the
    // stated triple did not.
    const resolved = after.tripleOf(quoted);
    assert.notEqual(resolved, undefined);
    assert.ok(resolved!.s > 0, "tt components were shifted");
    assert.deepEqual(
        [
            after.terms[resolved!.s].value,
            after.terms[resolved!.p].value,
            after.terms[resolved!.o].value,
        ],
        [
            "https://example.org/s",
            "https://example.org/p",
            "https://example.org/o",
        ],
    );
    // The content statement survives the rewrite verbatim.
    assert.ok(
        sortedLines(toNQuads(after)).includes(
            "<https://example.org/s> <https://example.org/says> " +
                "<<( <https://example.org/s> <https://example.org/p> " +
                "<https://example.org/o> )>> .",
        ),
    );
});

// A `reifies` row may name the very TERM that resolves through it — the row
// `(0, (2, 1, 1))` alongside term `2 = k:3 rf=0` is built here through the real
// writer, not hand-forged. Resolution MUST terminate (§7.3): the union interns a
// triple term on its RESOLVED components, and the N-Quads projection walks them,
// so both would recurse forever without a guard.
function selfReachingSegment(): Uint8Array {
    const w = new Writer("generic");
    w.addTerms([
        { kind: TermKind.Iri, value: "https://example.org/r1" }, // 0
        { kind: TermKind.Iri, value: "https://example.org/p" }, // 1
        { kind: TermKind.Triple, value: "", reifier: 0 }, // 2: legacy k:3 rf=0
    ]);
    // The binding puts the triple term itself in subject position.
    w.addReifies([{ rid: 0, spo: { s: 2, p: 1, o: 1 } }]);
    w.addQuads([{ s: 2, p: 1, o: 1 }]);
    return w.toBytes();
}

test("a wire-constructed self-reaching triple term folds and projects", () => {
    const g = Read(selfReachingSegment(), true);
    assert.deepEqual(codes(g), []);
    // The SELF-REACHING TERM ITSELF is the blank node (§7.3: it "renders as the
    // same fresh blank node an unbound triple term already produces"). Resolving
    // one step first and degrading a nested occurrence renders a different graph.
    assert.deepEqual(sortedLines(toNQuads(g)), [
        `<https://example.org/r1> <${RDF_REIFIES}> ` +
            "<<( _:unbound_triple_2 <https://example.org/p> " +
            "<https://example.org/p> )>> .",
        "_:unbound_triple_2 <https://example.org/p> <https://example.org/p> .",
    ]);
});

test("the union terminates on a self-reaching triple term", () => {
    const one = selfReachingSegment();
    const g = Read(concat([one, one]), true);
    // Both copies state no triple, so they intern to ONE term rather than two
    // distinct terms sharing a reifier id; byte-identical rows then collapse
    // under §7.8 set semantics. The reifier is not over-bound, so there is no
    // ConflictingReifier either.
    assert.equal(g.quads.length, 1);
    for (const q of g.quads) {
        assert.equal(g.terms[q.s].kind, TermKind.Triple);
    }
    assert.deepEqual(codes(g), []);
    assert.equal(sortedLines(toNQuads(g)).length, 2);
});

test("per-segment reads terminate on a self-reaching triple term", () => {
    const one = selfReachingSegment();
    const segments = ReadFileSegments(concat([one, one])).segments;
    assert.equal(segments.length, 2);
    for (const seg of segments) {
        assert.equal(sortedLines(toNQuads(seg)).length, 2);
    }
});

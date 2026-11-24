// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer } from "node:buffer";
import { TermKind, type Quad, type Term } from "./model.js";
import { V2SegmentWriter } from "./v2_writer.js";
import { mustEncode } from "./wire.js";

const restart = 64;
const termsPerFrame = 4096;
const rowsPerFrame = 16384;
const frameLimit = 2 ** 26;

export class RdfImportCapacityError extends Error {}

export function inlineTermBytes(value: string, language?: string): number {
    const bytes = Buffer.byteLength(value) + Buffer.byteLength(language ?? "");
    if (bytes >= frameLimit) {
        throw new RdfImportCapacityError(
            "RDF term exceeds the v2 inline payload limit",
        );
    }
    return bytes;
}

function runs<T, K>(
    rows: readonly T[],
    keyOf: (row: T) => K,
): { key: K; start: number; end: number }[] {
    const result: { key: K; start: number; end: number }[] = [];
    for (let start = 0; start < rows.length;) {
        const key = keyOf(rows[start]);
        let end = start + 1;
        while (end < rows.length && keyOf(rows[end]) === key) end++;
        result.push({ key, start, end });
        start = end;
    }
    return result;
}

function frontCode(terms: Term[]): {
    fields: [number[], number[], Uint8Array];
    formed: number;
} {
    const prefixes: number[] = [];
    const lengths: number[] = [];
    const suffixes: Uint8Array[] = [];
    let previous = new Uint8Array();
    let formed = 0;
    for (const [index, term] of terms.entries()) {
        const text = Buffer.from(term.value, "utf8");
        let prefix = 0;
        if (index % restart !== 0) {
            const limit = Math.min(text.length, previous.length);
            while (prefix < limit && previous[prefix] === text[prefix])
                prefix++;
        }
        prefixes.push(prefix);
        lengths.push(text.length - prefix);
        suffixes.push(text.subarray(prefix));
        previous = text;
        formed += prefix;
    }
    return { fields: [prefixes, lengths, Buffer.concat(suffixes)], formed };
}

function literalColumns(
    terms: Term[],
    languages: Map<string, number>,
): number[][] {
    const forms: number[] = [];
    const datatypeIds: number[] = [];
    const languageIds: number[] = [];
    for (const term of terms) {
        if (term.lang === undefined) {
            if (term.datatype === undefined)
                throw new TypeError("RDF import requires an interned datatype");
            forms.push(1);
            datatypeIds.push(term.datatype);
            continue;
        }
        forms.push(
            term.direction === "ltr" ? 3 : term.direction === "rtl" ? 4 : 2,
        );
        let id = languages.get(term.lang);
        if (id === undefined) {
            id = languages.size;
            languages.set(term.lang, id);
        }
        languageIds.push(id);
    }
    return [forms, datatypeIds, languageIds];
}

function termFrame(
    base: number,
    terms: Term[],
): { content: Uint8Array; payload: Uint8Array; formed: number } {
    const languageIds = new Map<string, number>();
    const termRuns: [number, number][] = [];
    let formed = 0;
    const blocks = runs(terms, (term) => term.kind).map(
        ({ key, start, end }) => {
            const code = key + 1;
            termRuns.push([code, end - start]);
            const block = terms.slice(start, end);
            if (key === TermKind.Bnode)
                return [
                    code,
                    block.map((term) => Buffer.from(term.value, "utf8")),
                ];
            if (key === TermKind.Triple) {
                const triples = block.map((term) => {
                    if (term.triple === undefined)
                        throw new TypeError(
                            "RDF import requires explicit triple terms",
                        );
                    return term.triple;
                });
                return [
                    code,
                    triples.map((term) => term.s),
                    triples.map((term) => term.p),
                    triples.map((term) => term.o),
                ];
            }
            const coded = frontCode(block);
            formed += coded.formed;
            const literal =
                key === TermKind.Literal
                    ? literalColumns(block, languageIds)
                    : [];
            return [code, ...coded.fields, ...literal];
        },
    );
    return {
        content: mustEncode([base, termRuns, 2, restart]),
        payload: mustEncode([blocks, [...languageIds.keys()]]),
        formed,
    };
}

function delta(rows: Quad[], column: "s" | "o"): number[] {
    let previous = 0;
    return rows.map((row) => {
        const difference = row[column] - previous;
        previous = row[column];
        return difference;
    });
}

function quadPayload(rows: Quad[]): Uint8Array {
    const graphs = runs(rows, (row) => row.g).map(({ key, start, end }) => [
        key === undefined ? [1] : [2, key],
        end - start,
    ]);
    const predicates = runs(rows, (row) => row.p).map(({ key, start, end }) => [
        key,
        end - start,
    ]);
    return mustEncode([graphs, predicates, delta(rows, "o"), delta(rows, "s")]);
}

/** Write the parser's interned RDF dataset as one committed v2 segment. */
export function writeRdfSegment(terms: Term[], rows: Quad[]): Uint8Array {
    const writer = new V2SegmentWriter();
    let total = 0;
    let formed = 0;
    let items = 2; // Reserve the opener and commit.
    const append = (
        type: number,
        content: Uint8Array,
        payload: Uint8Array,
        copied = 0,
    ): void => {
        if (
            content.length > frameLimit ||
            payload.length > frameLimit ||
            total + payload.length > 2 ** 36 ||
            formed + copied > 2 ** 37 ||
            items === 2 ** 26
        ) {
            throw new RdfImportCapacityError(
                "RDF import exceeds the v2 reader capacity",
            );
        }
        writer.append(type, content, payload);
        total += payload.length;
        formed += copied;
        items++;
    };

    // Bound each initial batch before encoding. Assembly then checks the exact
    // payload size, splitting a batch without renumbering its terms when needed.
    let base = 0;
    let budget = 0;
    const writeTerms = (start: number, end: number): void => {
        const frame = termFrame(start, terms.slice(start, end));
        if (frame.payload.length <= frameLimit) {
            append(1, frame.content, frame.payload, frame.formed);
            return;
        }
        if (end - start === 1) {
            throw new RdfImportCapacityError(
                `RDF term ${start} exceeds the v2 inline payload limit`,
            );
        }
        const middle = start + Math.floor((end - start) / 2);
        writeTerms(start, middle);
        writeTerms(middle, end);
    };
    for (const [index, term] of terms.entries()) {
        const size = inlineTermBytes(term.value, term.lang);
        if (
            index > base &&
            (index - base === termsPerFrame || budget + size > frameLimit)
        ) {
            writeTerms(base, index);
            base = index;
            budget = 0;
        }
        budget += size;
    }
    if (base < terms.length) writeTerms(base, terms.length);

    for (let start = 0; start < rows.length; start += rowsPerFrame) {
        const batch = rows.slice(start, start + rowsPerFrame);
        append(2, mustEncode([2, batch.length]), quadPayload(batch));
    }
    writer.commit();
    return writer.bytes();
}

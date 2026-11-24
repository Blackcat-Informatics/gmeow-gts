// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import { Buffer, constants } from "node:buffer";
import {
    RDF_DIR_LANG_STRING,
    RDF_LANG_STRING,
    TermKind,
    XSD_STRING,
    type Quad,
    type Term,
} from "./model.js";
import { escapeLiteral } from "./nquads.js";
import { validIri, validLanguageTag } from "./rdf12_syntax.js";
import { inspectFraming } from "./v2_framing.js";
import {
    decodeRdfText,
    RdfCursor,
    RdfProjectionError,
    requireRdf,
} from "./v2_rdf_cursor.js";
import { hexDatatype, readTypedValue, typedLiteral } from "./v2_rdf_values.js";
import { RdfRebuild, RebuiltValue } from "./v2_rdf_rebuild.js";

const rdfReifies = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const formedLimit = 1n << 37n;

interface LiteralColumns {
    forms: number[];
    datatypes: bigint[];
    languages: bigint[];
}

type Block =
    | ({
          kind: 1 | 2;
          prefixes: bigint[];
          lengths: bigint[];
          suffixes: Uint8Array;
      } & LiteralColumns)
    | { kind: 3; labels: Uint8Array[] }
    | { kind: 4; subjects: bigint[]; predicates: bigint[]; objects: bigint[] }
    | ({ kind: 5 | 7; digests: string[]; lengths: bigint[] } & LiteralColumns)
    | { kind: 6; count: bigint; encoded: Uint8Array };

function literalColumns(c: RdfCursor, present: boolean): LiteralColumns {
    return {
        forms: present ? c.list(() => c.code(4n)) : [],
        datatypes: present ? c.list(() => c.uint()) : [],
        languages: present ? c.list(() => c.uint()) : [],
    };
}

function readBlock(c: RdfCursor): Block {
    const arity = c.array();
    requireRdf(arity > 0n, "wrong_length");
    const kind = c.code(7n);
    switch (kind) {
        case 1:
        case 2: {
            requireRdf(arity === (kind === 1 ? 4n : 7n), "wrong_length");
            const prefixes = c.list(() => c.uint());
            const lengths = c.list(() => c.uint());
            const suffixes = c.body(2);
            return {
                kind,
                prefixes,
                lengths,
                suffixes,
                ...literalColumns(c, kind === 2),
            };
        }
        case 3:
            requireRdf(arity === 2n, "wrong_length");
            return { kind, labels: c.list(() => c.body(2)) };
        case 4:
            requireRdf(arity === 4n, "wrong_length");
            return {
                kind,
                subjects: c.list(() => c.uint()),
                predicates: c.list(() => c.uint()),
                objects: c.list(() => c.uint()),
            };
        case 5:
        case 7: {
            requireRdf(arity === (kind === 5 ? 6n : 3n), "wrong_length");
            const digests = c.list(() => c.digest());
            const lengths = c.list(() => c.uint());
            return { kind, digests, lengths, ...literalColumns(c, kind === 5) };
        }
        case 6: {
            requireRdf(arity === 2n, "wrong_length");
            const count = c.array();
            // Keep the validated span instead of allocating one object per value.
            const encoded = c.span(() => {
                for (let i = 0n; i < count; i++) readTypedValue(c);
            });
            return { kind, count, encoded };
        }
        default:
            throw new RdfProjectionError("unsupported_term_kind");
    }
}

function checkBlockCount(block: Block, count: bigint): void {
    const matches = (...lengths: number[]): boolean =>
        lengths.every((length) => BigInt(length) === count);
    switch (block.kind) {
        case 1:
        case 2: {
            requireRdf(
                matches(block.prefixes.length, block.lengths.length),
                "inconsistent_counts",
            );
            break;
        }
        case 3:
            requireRdf(matches(block.labels.length), "inconsistent_counts");
            break;
        case 4:
            requireRdf(
                matches(
                    block.subjects.length,
                    block.predicates.length,
                    block.objects.length,
                ),
                "inconsistent_counts",
            );
            break;
        case 6:
            requireRdf(block.count === count, "inconsistent_counts");
            break;
        case 5:
        case 7:
            requireRdf(
                matches(block.digests.length, block.lengths.length),
                "inconsistent_counts",
            );
            break;
    }
    if (block.kind === 2 || block.kind === 5) {
        const typed = block.forms.filter((form) => form === 1).length;
        requireRdf(
            matches(block.forms.length) &&
                block.datatypes.length === typed &&
                block.languages.length === block.forms.length - typed,
            "inconsistent_counts",
        );
    }
}

function checkLanguages(blocks: Block[], languages: string[]): void {
    for (const block of blocks) {
        if (block.kind !== 2 && block.kind !== 5) continue;
        for (const id of block.languages) {
            requireRdf(id < BigInt(languages.length), "out_of_range");
        }
    }
    let used = 0;
    const seen = new Set<string>();
    for (const block of blocks) {
        if (block.kind !== 2 && block.kind !== 5) continue;
        for (const id of block.languages) {
            requireRdf(id <= BigInt(used), "not_deterministic");
            if (id === BigInt(used)) {
                const text = languages[used++];
                requireRdf(!seen.has(text), "not_deterministic");
                seen.add(text);
            }
        }
    }
    requireRdf(used === languages.length, "not_deterministic");
}

function* texts(
    block: Extract<Block, { prefixes: bigint[] }>,
    restart: bigint,
): IterableIterator<Uint8Array> {
    const suffixLength = block.lengths.reduce(
        (sum, length) => sum + length,
        0n,
    );
    requireRdf(
        suffixLength === BigInt(block.suffixes.length),
        "inconsistent_counts",
    );
    let offset = 0;
    let previous: Uint8Array = new Uint8Array();
    for (const [index, prefix] of block.prefixes.entries()) {
        const restarting = BigInt(index) % restart === 0n;
        if (index !== 0)
            requireRdf(prefix <= BigInt(previous.length), "out_of_range");
        requireRdf(!restarting || prefix === 0n, "forbidden_combination");
        const length = block.lengths[index];
        const suffix = block.suffixes.subarray(offset, offset + Number(length));
        const retained = Number(prefix);
        requireRdf(
            restarting ||
                retained === previous.length ||
                suffix.length === 0 ||
                previous[retained] !== suffix[0],
            "not_deterministic",
        );
        previous =
            retained === 0
                ? suffix
                : suffix.length === 0
                  ? previous.subarray(0, retained)
                  : Buffer.concat([previous.subarray(0, retained), suffix]);
        offset += suffix.length;
        yield previous;
    }
}

type Run = { id: bigint; count: bigint };

function readRuns(c: RdfCursor, value: () => bigint, termKinds = false): Run[] {
    let previous: bigint | undefined;
    return c.list(() => {
        c.record(2);
        const id = value();
        if (termKinds) requireRdf(id !== previous, "not_deterministic");
        const count = c.positive();
        requireRdf(id !== previous, "not_deterministic");
        previous = id;
        return { id, count };
    });
}

function runCount(runs: Run[]): bigint {
    return runs.reduce((sum, run) => sum + run.count, 0n);
}

function* expand(runs: Run[]): Generator<bigint, void> {
    for (const { id, count } of runs) {
        for (let i = 0n; i < count; i++) yield id;
    }
}

function graphId(c: RdfCursor): bigint {
    const arity = c.array();
    requireRdf(arity > 0n, "wrong_length");
    const code = c.code(2n);
    requireRdf(arity === BigInt(code), "wrong_length");
    return code === 1 ? -1n : c.uint();
}

export interface RdfProjection {
    terms: Term[];
    quads: Quad[];
    emptyGraphs: number[];
}

interface Definition {
    rdf: number;
    written: number;
}

class Projector {
    readonly result: RdfProjection = { terms: [], quads: [], emptyGraphs: [] };
    readonly rebuild = new RdfRebuild();
    private dictionary: Definition[] = [];
    private readonly written = new Map<string, number>();
    private readonly interned = new Map<string, number>();
    private readonly rows = new Set<string>();
    private scope = -1;
    private formed = 0n;

    open(): void {
        this.scope++;
        this.dictionary = [];
        this.written.clear();
        this.rebuild.open();
    }

    private intern(term: Term): number {
        const metadata = JSON.stringify([
            term.kind,
            term.lang?.length,
            term.direction,
            term.datatype,
            term.triple?.s,
            term.triple?.p,
            term.triple?.o,
            term.kind === TermKind.Bnode ? this.scope : undefined,
        ]);
        // Append the lexical text verbatim so interning never expands escapes.
        const key = metadata + (term.lang ?? "") + term.value;
        const existing = this.interned.get(key);
        if (existing !== undefined) return existing;
        const id = this.result.terms.length;
        this.interned.set(key, id);
        this.result.terms.push(term);
        return id;
    }

    private resolve(wireId: bigint, ...kinds: TermKind[]): number {
        requireRdf(
            wireId >= 0n && wireId < BigInt(this.dictionary.length),
            "out_of_range",
        );
        const { rdf: id } = this.dictionary[Number(wireId)];
        requireRdf(
            kinds.length === 0 || kinds.includes(this.result.terms[id].kind),
            "forbidden_combination",
        );
        return id;
    }

    private node(wireId: bigint): number {
        return this.resolve(wireId, TermKind.Iri, TermKind.Bnode);
    }

    private add(
        term: Term,
        storageKind = term.kind + 1,
        source?: string | readonly bigint[],
    ): void {
        const rdf = this.intern(term);
        const key =
            source === undefined
                ? `${storageKind}/${rdf}`
                : typeof source === "string"
                  ? `${storageKind}/${rdf}/${source}`
                  : `${storageKind}/${source.map((id) => this.dictionary[Number(id)].written).join("/")}`;
        let written = this.written.get(key);
        if (written === undefined) {
            written = this.written.size;
            this.written.set(key, written);
        }
        this.dictionary.push({ rdf, written });
    }

    terms(content: Uint8Array, payload: Uint8Array): void {
        const fields = new RdfCursor(content);
        fields.record(4);
        requireRdf(
            fields.uint() === BigInt(this.dictionary.length),
            "inconsistent_counts",
        );
        const runs = readRuns(fields, () => BigInt(fields.code(7n)), true);
        requireRdf(
            runs.length !== 0 &&
                BigInt(this.dictionary.length) + runCount(runs) <= 1n << 64n,
            "out_of_range",
        );
        fields.code(2n);
        const restart = fields.positive();
        fields.end();

        const c = new RdfCursor(payload);
        c.record(2);
        const blocks = c.list(() => readBlock(c));
        const languages = c.list(() => c.text());
        c.end();
        requireRdf(blocks.length === runs.length, "inconsistent_counts");
        for (const [index, block] of blocks.entries())
            checkBlockCount(block, runs[index].count);
        for (const [index, block] of blocks.entries()) {
            requireRdf(
                BigInt(block.kind) === runs[index].id,
                "forbidden_combination",
            );
        }
        for (const block of blocks) {
            if (block.kind === 1 || block.kind === 2) {
                for (const prefix of block.prefixes) {
                    this.formed += prefix;
                    requireRdf(this.formed <= formedLimit, "total_formed");
                }
            }
        }
        for (const block of blocks) {
            if (block.kind !== 1 && block.kind !== 2) continue;
            const formed = texts(block, restart);
            while (!formed.next().done) {
                // Validate the entire run before checking the language table.
            }
        }
        checkLanguages(blocks, languages);
        for (const block of blocks) this.addBlock(block, restart, languages);
    }

    private addBlock(block: Block, restart: bigint, languages: string[]): void {
        if (block.kind === 3) {
            for (const label of block.labels)
                this.add({
                    kind: TermKind.Bnode,
                    value: Buffer.from(label).toString("hex"),
                });
            return;
        }
        if (block.kind === 4) {
            for (const [index, subject] of block.subjects.entries()) {
                const predicate = block.predicates[index];
                const object = block.objects[index];
                this.add(
                    {
                        kind: TermKind.Triple,
                        value: "",
                        triple: {
                            s: this.node(subject),
                            p: this.resolve(predicate, TermKind.Iri),
                            o: this.resolve(object),
                        },
                    },
                    block.kind,
                    [subject, predicate, object],
                );
            }
            return;
        }
        if (block.kind === 6) {
            const cursor = new RdfCursor(block.encoded);
            for (let index = 0n; index < block.count; index++) {
                const { lexical, datatype } = typedLiteral(
                    readTypedValue(cursor),
                );
                const datatypeId = this.intern({
                    kind: TermKind.Iri,
                    value: datatype,
                });
                this.add(
                    {
                        kind: TermKind.Literal,
                        value: lexical,
                        datatype: datatypeId,
                    },
                    block.kind,
                );
            }
            cursor.end();
            return;
        }
        if ("digests" in block) {
            this.addReferences(block, languages);
            return;
        }

        let column = 0;
        const next = { datatype: 0, language: 0 };
        for (const bytes of texts(block, restart)) {
            const value = decodeRdfText(bytes);
            if (block.kind === 1) {
                this.add({ kind: TermKind.Iri, value });
                continue;
            }
            this.add(this.literal(block, column++, next, languages, value));
        }
    }

    private literal(
        block: LiteralColumns,
        index: number,
        next: { datatype: number; language: number },
        languages: string[],
        value: string,
    ): Term {
        const form = block.forms[index];
        const term: Term = { kind: TermKind.Literal, value };
        if (form === 1) {
            term.datatype = this.resolve(
                block.datatypes[next.datatype++],
                TermKind.Iri,
            );
        } else {
            term.lang = languages[Number(block.languages[next.language++])];
            if (form === 3) term.direction = "ltr";
            if (form === 4) term.direction = "rtl";
        }
        return term;
    }

    private addReferences(
        block: Extract<Block, { digests: string[] }>,
        languages: string[],
    ): void {
        const next = { datatype: 0, language: 0 };
        const hex = block.kind === 7;
        for (const [index, digest] of block.digests.entries()) {
            const candidate = this.rebuild.value(digest, block.lengths[index]);
            if (!hex && candidate instanceof RebuiltValue)
                candidate.checkText();
            const term: Term = hex
                ? { kind: TermKind.Literal, value: "" }
                : this.literal(block, index, next, languages, "");
            if (!(candidate instanceof RebuiltValue)) {
                throw new RdfProjectionError(
                    candidate?.reason ?? "missing_value",
                );
            }
            term.value = this.rebuild.lexical(candidate, hex);
            if (hex)
                term.datatype = this.intern({
                    kind: TermKind.Iri,
                    value: hexDatatype,
                });
            this.add(term, block.kind, candidate.key);
        }
    }

    private appendQuad(row: Quad): void {
        const key = JSON.stringify([row.g, row.s, row.p, row.o]);
        if (this.rows.has(key)) return;
        this.rows.add(key);
        this.result.quads.push(row);
    }

    readRows(kind: bigint, content: Uint8Array, payload: Uint8Array): void {
        const fields = new RdfCursor(content);
        fields.record(kind === 5n ? 1 : 2);
        if (kind !== 5n) fields.code(2n);
        const count = fields.positive();
        fields.end();

        const c = new RdfCursor(payload);
        if (kind === 5n) {
            c.record(1);
            const ids = c.deltas();
            c.end();
            requireRdf(BigInt(ids.length) === count, "inconsistent_counts");
            for (const id of ids) this.result.emptyGraphs.push(this.node(id));
            return;
        }

        c.record(kind === 2n ? 4 : kind === 3n ? 3 : 5);
        const graphs = readRuns(c, () => graphId(c));
        const predicateRuns = kind === 2n ? readRuns(c, () => c.uint()) : [];
        const objectsFirst = kind === 2n ? c.deltas() : undefined;
        const subjects = c.deltas();
        const predicates = kind === 4n ? c.deltas() : [];
        const objects = objectsFirst ?? c.deltas();
        const counts = kind === 4n ? c.deltas(1n) : [];
        c.end();
        requireRdf(
            runCount(graphs) === count &&
                BigInt(subjects.length) === count &&
                BigInt(objects.length) === count,
            "inconsistent_counts",
        );
        if (kind === 2n)
            requireRdf(
                runCount(predicateRuns) === count,
                "inconsistent_counts",
            );
        if (kind === 4n) {
            requireRdf(
                BigInt(predicates.length) === count &&
                    BigInt(counts.length) === count,
                "inconsistent_counts",
            );
            this.checkAnnotations(graphs, subjects, predicates, objects);
        }

        const graph = expand(graphs);
        const predicate = expand(predicateRuns);
        const reifies =
            kind === 3n
                ? this.intern({ kind: TermKind.Iri, value: rdfReifies })
                : undefined;
        for (const [index, subject] of subjects.entries()) {
            const graphId = graph.next().value;
            const predicateId = predicate.next().value ?? predicates[index];
            requireRdf(graphId !== undefined, "inconsistent_counts");
            const g = graphId === -1n ? undefined : this.node(graphId);
            const s = this.node(subject);
            const p = reifies ?? this.resolve(predicateId, TermKind.Iri);
            const o =
                kind === 3n
                    ? this.resolve(objects[index], TermKind.Triple)
                    : this.resolve(objects[index]);
            this.appendQuad({ g, s, p, o });
        }
    }

    private checkAnnotations(
        graphs: Run[],
        subjects: bigint[],
        predicates: bigint[],
        objects: bigint[],
    ): void {
        const seen = new Set<string>();
        const graph = expand(graphs);
        const key = (id: bigint): number | string =>
            this.dictionary[Number(id)]?.written ?? `wire-${id}`;
        for (const [index, subject] of subjects.entries()) {
            const graphId = graph.next().value;
            requireRdf(graphId !== undefined, "inconsistent_counts");
            const identity = JSON.stringify(
                [graphId, subject, predicates[index], objects[index]].map(key),
            );
            requireRdf(!seen.has(identity), "unordered_or_repeated");
            seen.add(identity);
        }
    }

    finish(): RdfProjection {
        this.result.quads.sort(
            (a, b) =>
                (a.g ?? -1) - (b.g ?? -1) ||
                a.p - b.p ||
                a.o - b.o ||
                a.s - b.s,
        );
        const populated = new Set(this.result.quads.map((row) => row.g));
        this.result.emptyGraphs = [...new Set(this.result.emptyGraphs)]
            .filter((id) => !populated.has(id))
            .sort((a, b) => a - b);
        return this.result;
    }
}

export function projectRdf(
    wire: Extract<ReturnType<typeof inspectFraming>, { kind: "v2" }>,
): RdfProjection {
    const projector = new Projector();
    for (const item of wire.records) {
        if (item.kind === "stop")
            throw new RdfProjectionError(
                `${item.reason} at byte ${item.offset}`,
            );
        try {
            requireRdf(
                item.reason === undefined,
                item.reason ?? "malformed item",
            );
            if (item.kind === "header") {
                requireRdf(
                    item.registry === 1n && item.openerId !== undefined,
                    "unsupported or unreadable header",
                );
                projector.open();
                continue;
            }
            if (item.kind === "offsets" || item.kind === "locator") continue;
            requireRdf(
                item.kind === "frame" && item.frameType !== undefined,
                `unreadable ${item.kind} item`,
            );
            requireRdf(
                item.link?.state === "linked" || item.link?.state === "healed",
                "broken or unavailable link",
            );
            requireRdf(item.storedMatches !== false, "stored_check_mismatch");
            requireRdf(
                item.payload?.state === "verified" &&
                    item.payload.decoded !== undefined &&
                    item.content !== undefined,
                "unverified payload",
            );
            if (item.frameType === 1n)
                projector.terms(item.content, item.payload.decoded);
            if (item.frameType >= 2n && item.frameType <= 5n) {
                projector.readRows(
                    item.frameType,
                    item.content,
                    item.payload.decoded,
                );
            }
            if (
                (item.frameType >= 8n && item.frameType <= 11n) ||
                item.frameType === 13n
            ) {
                projector.rebuild.read(
                    item.frameType,
                    item.content,
                    item.payload.decoded,
                );
            }
        } catch (error) {
            if (!(error instanceof RdfProjectionError)) throw error;
            throw new RdfProjectionError(
                `byte ${item.offset}: ${error.message}`,
                { cause: error },
            );
        }
    }
    return projector.finish();
}

function formatLiteral(text: string, suffix: string): string {
    const limit = constants.MAX_STRING_LENGTH - 2 - suffix.length;
    let length = text.length;
    requireRdf(
        length <= limit,
        "N-Quads output exceeds the JavaScript string limit",
    );
    for (
        let index = 0;
        text.length * 6 > limit && index < text.length;
        index++
    ) {
        const code = text.charCodeAt(index);
        if (
            code === 34 ||
            code === 92 ||
            code === 9 ||
            code === 10 ||
            code === 13
        )
            length++;
        else if (code < 32) length += 5;
        requireRdf(
            length <= limit,
            "N-Quads output exceeds the JavaScript string limit",
        );
    }
    // Escaping in slices also bounds the number of regular expression matches
    // that the runtime has to retain at once.
    let literal = '"';
    const batch = 64 * 1024;
    for (let offset = 0; offset < text.length; offset += batch) {
        literal += escapeLiteral(text.slice(offset, offset + batch));
    }
    return literal + '"' + suffix;
}

export function rdfNQuads({
    terms,
    quads,
    emptyGraphs,
}: RdfProjection): string {
    const names = new Map<number, number>();
    const used = new Set(emptyGraphs);
    for (const { s, p, o, g } of quads) {
        used.add(s);
        used.add(p);
        used.add(o);
        if (g !== undefined) used.add(g);
    }
    for (let id = terms.length - 1; id >= 0; id--) {
        if (!used.has(id)) continue;
        const { triple, datatype } = terms[id];
        if (datatype !== undefined) used.add(datatype);
        if (triple !== undefined) {
            used.add(triple.s);
            used.add(triple.p);
            used.add(triple.o);
        }
    }
    let ordinal = 0;
    for (const [id, term] of terms.entries()) {
        if (!used.has(id)) continue;
        if (term.kind === TermKind.Bnode) names.set(id, ordinal);
        ordinal++;
    }
    const iri = (value: string): string => {
        requireRdf(validIri(value), "invalid RDF IRI");
        return `<${value}>`;
    };
    const renderAtom = (id: number): string => {
        const term = terms[id];
        switch (term.kind) {
            case TermKind.Iri:
                return iri(term.value);
            case TermKind.Bnode:
                return `_:b${names.get(id)}`;
            case TermKind.Literal: {
                if (term.lang !== undefined) {
                    requireRdf(
                        validLanguageTag(term.lang),
                        "invalid RDF language tag",
                    );
                    const suffix = `@${term.lang}${term.direction === undefined ? "" : `--${term.direction}`}`;
                    return formatLiteral(term.value, suffix);
                }
                requireRdf(
                    term.datatype !== undefined,
                    "missing literal datatype",
                );
                const datatype = terms[term.datatype].value;
                requireRdf(
                    datatype !== RDF_LANG_STRING &&
                        datatype !== RDF_DIR_LANG_STRING,
                    "RDF datatype requires a language",
                );
                const suffix =
                    datatype === XSD_STRING ? "" : `^^${iri(datatype)}`;
                return formatLiteral(term.value, suffix);
            }
            case TermKind.Triple:
                throw new RdfProjectionError("invalid RDF term role");
        }
    };
    const atoms: (string | undefined)[] = [];
    const atom = (id: number): string => (atoms[id] ??= renderAtom(id));
    const output: string[] = [];
    let length = 0;
    const append = (...parts: string[]): void => {
        for (const part of parts) {
            length += part.length;
            requireRdf(
                length <= constants.MAX_STRING_LENGTH,
                "N-Quads output exceeds the JavaScript string limit",
            );
            output.push(part);
        }
    };
    const render = (id: number): void => {
        let depth = 0;
        for (
            let triple = terms[id].triple;
            triple !== undefined;
            triple = terms[id].triple
        ) {
            append("<<( ", atom(triple.s), " ", atom(triple.p), " ");
            id = triple.o;
            depth++;
        }
        append(atom(id));
        requireRdf(
            depth * 4 <= constants.MAX_STRING_LENGTH - length,
            "N-Quads output exceeds the JavaScript string limit",
        );
        append(" )>>".repeat(depth));
    };
    for (const { s, p, o, g } of quads) {
        append(atom(s), " ", atom(p), " ");
        render(o);
        if (g !== undefined) append(" ", atom(g));
        append(" .\n");
    }
    return output.join("");
}

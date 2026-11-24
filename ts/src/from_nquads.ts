// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

import {
    TermKind,
    RDF_DIR_LANG_STRING,
    RDF_LANG_STRING,
    XSD_STRING,
    type AnnotationEntry,
    type LiteralDirection,
    type Quad,
    type ReifierEntry,
    type Term,
} from "./model.js";
import { Writer } from "./writer.js";
import {
    blankLabelLength,
    validIri,
    validLanguageTag,
} from "./rdf12_syntax.js";
import { inlineTermBytes, writeRdfSegment } from "./v2_rdf_import.js";

const RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

export class NQuadsParseError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "NQuadsParseError";
    }
}

interface Atom {
    kind: TermKind;
    value: string;
    lang?: string;
    direction?: LiteralDirection;
    datatype?: string;
}

interface TripleNode {
    s: Node;
    p: Node;
    o: Node;
}

type Node = { atom: Atom } | { triple: TripleNode };

function atom(
    kind: TermKind,
    value: string,
    lang?: string,
    datatype?: string,
): Node {
    return { atom: { kind, value, lang, datatype } };
}

function isAtom(node: Node, kind?: TermKind): node is { atom: Atom } {
    if (!("atom" in node)) return false;
    return kind === undefined || node.atom.kind === kind;
}

function isTriple(node: Node): node is { triple: TripleNode } {
    return "triple" in node;
}

function isAsciiLetterOrDigit(ch: string): boolean {
    const code = ch.charCodeAt(0);
    return (
        (code >= 48 && code <= 57) ||
        (code >= 65 && code <= 90) ||
        (code >= 97 && code <= 122)
    );
}

function isBNodeChar(ch: string): boolean {
    return isAsciiLetterOrDigit(ch) || ch === "_" || ch === "-" || ch === ".";
}

function isLangChar(ch: string): boolean {
    return isAsciiLetterOrDigit(ch) || ch === "-";
}

class Tokenizer {
    private i = 0;

    constructor(
        private readonly s: string,
        private readonly rdf12 = false,
    ) {}

    private skipWs(): void {
        while (this.i < this.s.length && /[ \t]/.test(this.s[this.i])) {
            this.i++;
        }
    }

    atEnd(): boolean {
        this.skipWs();
        return this.i >= this.s.length || this.s[this.i] === ".";
    }

    finishStatement(): void {
        this.skipWs();
        if (this.s[this.i++] !== ".")
            throw new NQuadsParseError("expected '.' at end of statement");
        this.finishLine();
    }

    versionHint(): void {
        this.i = "VERSION".length;
        this.skipWs();
        if (this.s[this.i] !== '"')
            throw new NQuadsParseError("expected a VERSION string");
        this.literal(false);
        this.finishLine();
    }

    private finishLine(): void {
        this.skipWs();
        if (this.i < this.s.length && this.s[this.i] !== "#") {
            throw new NQuadsParseError(
                "unexpected text after the N-Quads item",
            );
        }
    }

    node(): Node {
        this.skipWs();
        if (this.i >= this.s.length) {
            throw new NQuadsParseError(`unexpected end of line: ${this.s}`);
        }
        if (this.s.startsWith("<<(", this.i))
            return this.rdf12
                ? this.rdfTriple()
                : { triple: this.quotedTriple() };
        switch (this.s[this.i]) {
            case "<":
                return atom(TermKind.Iri, this.iri());
            case "_":
                return atom(TermKind.Bnode, this.bnode());
            case '"':
                return { atom: this.literal() };
            default:
                throw new NQuadsParseError(
                    `unexpected token at ${this.i} in ${this.s}`,
                );
        }
    }

    private iri(): string {
        if (this.s[this.i] !== "<") {
            throw new NQuadsParseError(`bad IRI in ${this.s}`);
        }
        const end = this.s.indexOf(">", this.i + 1);
        if (end < 0) {
            throw new NQuadsParseError(`unterminated IRI in ${this.s}`);
        }
        const value = this.s.slice(this.i + 1, end);
        this.i = end + 1;
        if (!this.rdf12) return value;
        const escaped = new Tokenizer(value, true);
        let iri = "";
        while (escaped.i < value.length) {
            const slash = value.indexOf("\\", escaped.i);
            if (slash === -1) {
                iri += value.slice(escaped.i);
                break;
            }
            iri += value.slice(escaped.i, slash);
            escaped.i = slash + 1;
            if (value[escaped.i] !== "u" && value[escaped.i] !== "U") {
                throw new NQuadsParseError(
                    "IRI escapes must use Unicode scalar notation",
                );
            }
            iri += escaped.escape();
        }
        if (!validIri(iri)) throw new NQuadsParseError("invalid RDF IRI");
        return iri;
    }

    private bnode(): string {
        if (!this.s.startsWith("_:", this.i)) {
            throw new NQuadsParseError(`bad blank node in ${this.s}`);
        }
        this.i += 2;
        const start = this.i;
        if (this.rdf12) {
            this.i += blankLabelLength(this.s.slice(start));
            if (this.i === start)
                throw new NQuadsParseError("invalid blank node label");
            return this.s.slice(start, this.i);
        }
        while (this.i < this.s.length && isBNodeChar(this.s[this.i])) {
            this.i++;
        }
        if (this.i > start && this.s[this.i - 1] === ".") {
            this.i--;
        }
        if (this.i === start) {
            throw new NQuadsParseError(`empty blank node label in ${this.s}`);
        }
        return this.s.slice(start, this.i);
    }

    private literal(suffix = true): Atom {
        this.i++;
        let value = "";
        const boundary = /["\\]/g;
        while (this.i < this.s.length) {
            boundary.lastIndex = this.i;
            const match = boundary.exec(this.s);
            if (match === null) break;
            value += this.s.slice(this.i, match.index);
            this.i = boundary.lastIndex;
            const ch = match[0];
            if (ch === "\\") {
                value += this.escape();
                continue;
            }
            if (ch === '"') {
                if (!suffix) return { kind: TermKind.Literal, value };
                if (this.rdf12) this.skipWs();
                let lang: string | undefined;
                let direction: LiteralDirection | undefined;
                let datatype: string | undefined;
                if (this.s[this.i] === "@") {
                    this.i++;
                    const start = this.i;
                    while (
                        this.i < this.s.length &&
                        isLangChar(this.s[this.i])
                    ) {
                        this.i++;
                    }
                    lang = this.s.slice(start, this.i);
                    if (lang.length === 0) {
                        throw new NQuadsParseError(
                            `empty language tag in ${this.s}`,
                        );
                    }
                    const sep = lang.lastIndexOf("--");
                    if (sep >= 0) {
                        const rawDirection = lang.slice(sep + 2);
                        if (rawDirection !== "ltr" && rawDirection !== "rtl") {
                            throw new NQuadsParseError(
                                `invalid literal direction in ${this.s}`,
                            );
                        }
                        const language = lang.slice(0, sep);
                        if (language.length === 0) {
                            throw new NQuadsParseError(
                                `empty language tag in ${this.s}`,
                            );
                        }
                        lang = language;
                        direction = rawDirection;
                    }
                    if (this.rdf12 && !validLanguageTag(lang)) {
                        throw new NQuadsParseError("invalid RDF language tag");
                    }
                } else if (this.s.startsWith("^^", this.i)) {
                    this.i += 2;
                    this.skipWs();
                    datatype = this.iri();
                    if (
                        this.rdf12 &&
                        (datatype === RDF_LANG_STRING ||
                            datatype === RDF_DIR_LANG_STRING)
                    ) {
                        throw new NQuadsParseError(
                            "the RDF datatype requires a language tag",
                        );
                    }
                }
                return {
                    kind: TermKind.Literal,
                    value,
                    lang,
                    direction,
                    datatype,
                };
            }
        }
        throw new NQuadsParseError(`unterminated literal in ${this.s}`);
    }

    private escape(): string {
        if (this.i >= this.s.length) {
            throw new NQuadsParseError(`bad escape at end of ${this.s}`);
        }
        const ch = this.s[this.i++];
        if (this.rdf12 && ch === "'") return "'";
        switch (ch) {
            case "\\":
                return "\\";
            case '"':
                return '"';
            case "b":
                return "\b";
            case "f":
                return "\f";
            case "n":
                return "\n";
            case "r":
                return "\r";
            case "t":
                return "\t";
            case "u":
            case "U": {
                const width = ch === "u" ? 4 : 8;
                const raw = this.s.slice(this.i, this.i + width);
                if (raw.length !== width || !/^[0-9a-fA-F]+$/.test(raw)) {
                    throw new NQuadsParseError(
                        `bad unicode escape \\${ch}${raw} in ${this.s}`,
                    );
                }
                this.i += width;
                const code = Number.parseInt(raw, 16);
                if (this.rdf12 && code >= 0xd800 && code <= 0xdfff) {
                    throw new NQuadsParseError(
                        "Unicode escapes require a scalar value",
                    );
                }
                try {
                    return String.fromCodePoint(code);
                } catch {
                    throw new NQuadsParseError(
                        `invalid unicode scalar \\${ch}${raw}`,
                    );
                }
            }
            default:
                throw new NQuadsParseError(
                    `unsupported escape \\${ch} in ${this.s}`,
                );
        }
    }

    private quotedTriple(): TripleNode {
        this.i += 3;
        const s = this.node();
        const p = this.node();
        const o = this.node();
        this.skipWs();
        if (!this.s.startsWith(")>>", this.i)) {
            throw new NQuadsParseError(
                `unterminated quoted triple in ${this.s}`,
            );
        }
        this.i += 3;
        return { s, p, o };
    }

    private rdfTriple(): Node {
        const parents: { s: Node; p: Node }[] = [];
        while (this.s.startsWith("<<(", this.i)) {
            this.i += 3;
            this.skipWs();
            if (this.s.startsWith("<<(", this.i))
                throw new NQuadsParseError("invalid triple term subject");
            const s = this.node();
            this.skipWs();
            if (this.s.startsWith("<<(", this.i))
                throw new NQuadsParseError("invalid triple term predicate");
            const p = this.node();
            if (
                !(isAtom(s, TermKind.Iri) || isAtom(s, TermKind.Bnode)) ||
                !isAtom(p, TermKind.Iri)
            ) {
                throw new NQuadsParseError(
                    "invalid triple term subject or predicate",
                );
            }
            parents.push({ s, p });
            this.skipWs();
        }
        let object = this.node();
        for (let index = parents.length - 1; index >= 0; index--) {
            this.skipWs();
            if (!this.s.startsWith(")>>", this.i))
                throw new NQuadsParseError("unterminated triple term");
            this.i += 3;
            object = { triple: { ...parents[index], o: object } };
        }
        return object;
    }
}

class Interner {
    private readonly ids = new Map<string, number>();
    readonly terms: Term[] = [];

    atom(a: Atom): number {
        const key = JSON.stringify([
            "atom",
            a.kind,
            a.value,
            a.lang ?? null,
            a.direction ?? null,
            a.datatype ?? null,
        ]);
        const existing = this.ids.get(key);
        if (existing !== undefined) return existing;

        const term: Term = { kind: a.kind, value: a.value };
        if (a.kind === TermKind.Literal && a.datatype !== undefined) {
            term.datatype = this.atom({
                kind: TermKind.Iri,
                value: a.datatype,
            });
        }
        if (a.lang !== undefined) term.lang = a.lang;
        if (a.direction !== undefined) term.direction = a.direction;

        const id = this.terms.length;
        this.terms.push(term);
        this.ids.set(key, id);
        return id;
    }

    node(n: Node, reifiers: ReifierEntry[]): number {
        if (isAtom(n)) return this.atom(n.atom);
        // A bare quoted triple as a quad component: intern as a SELF-DESCRIBING
        // TRIPLE term carrying its own (s, p, o) (§7.3). An unreified triple
        // term is a value, not a statement — it mints no reifier and emits no
        // `reifies` row, so it round-trips as itself. Interning the components
        // first also keeps every "tt" component id below the term's own id
        // (§7.2 forward-reference rule).
        const s = this.node(n.triple.s, reifiers);
        const p = this.node(n.triple.p, reifiers);
        const o = this.node(n.triple.o, reifiers);
        return this.triple(s, p, o);
    }

    nodeV2(node: Node): number {
        const parents: { s: number; p: number }[] = [];
        while (isTriple(node)) {
            const { s, p, o } = node.triple;
            if (!isAtom(s) || !isAtom(p))
                throw new NQuadsParseError("invalid triple term roles");
            parents.push({ s: this.rdfAtom(s.atom), p: this.rdfAtom(p.atom) });
            node = o;
        }
        let id = this.rdfAtom(node.atom);
        for (let index = parents.length - 1; index >= 0; index--) {
            const { s, p } = parents[index];
            id = this.triple(s, p, id);
        }
        return id;
    }

    private rdfAtom(atom: Atom): number {
        inlineTermBytes(atom.value, atom.lang);
        if (atom.datatype !== undefined) inlineTermBytes(atom.datatype);
        return this.atom(
            atom.kind === TermKind.Literal && atom.lang === undefined
                ? { ...atom, datatype: atom.datatype ?? XSD_STRING }
                : atom,
        );
    }

    private triple(s: number, p: number, o: number): number {
        const key = JSON.stringify(["triple", s, p, o]);
        const existing = this.ids.get(key);
        if (existing !== undefined) return existing;
        const tid = this.terms.length;
        this.terms.push({
            kind: TermKind.Triple,
            value: "",
            triple: { s, p, o },
        });
        this.ids.set(key, tid);
        return tid;
    }
}

/** Record one `r rdf:reifies <<( s p o )>>` statement.
 *
 * `rdf:reifies` is NOT functional (RDF 1.2 / §7.3): `r rdf:reifies
 * <<( s p o1 )>>` and `r rdf:reifies <<( s p o2 )>>` are both assertable and
 * both survive. Only a byte-identical repeat collapses (§7.8).
 */
function setReifier(
    reifiers: ReifierEntry[],
    rid: number,
    spo: { s: number; p: number; o: number },
    g?: number,
): void {
    for (const r of reifiers) {
        if (
            r.rid === rid &&
            r.spo.s === spo.s &&
            r.spo.p === spo.p &&
            r.spo.o === spo.o &&
            r.g === g
        ) {
            return;
        }
    }
    reifiers.push({ rid, spo, ...(g !== undefined ? { g } : {}) });
}

function validateStatement(nodes: Node[], line: string, rdf12: boolean): void {
    if (!(
        isAtom(nodes[0], TermKind.Iri) ||
        isAtom(nodes[0], TermKind.Bnode) ||
        (!rdf12 && isTriple(nodes[0]))
    )) {
        throw new NQuadsParseError(`invalid subject term: ${line}`);
    }
    if (!isAtom(nodes[1], TermKind.Iri)) {
        throw new NQuadsParseError(`predicate must be IRI: ${line}`);
    }
    if (!(
        isAtom(nodes[2], TermKind.Iri) ||
        isAtom(nodes[2], TermKind.Bnode) ||
        isAtom(nodes[2], TermKind.Literal) ||
        isTriple(nodes[2])
    )) {
        throw new NQuadsParseError(`invalid object term: ${line}`);
    }
    if (
        nodes[3] !== undefined &&
        !(isAtom(nodes[3], TermKind.Iri) || isAtom(nodes[3], TermKind.Bnode))
    ) {
        throw new NQuadsParseError(`invalid graph name term: ${line}`);
    }
}

function* statements(text: string, rdf12 = false): IterableIterator<Node[]> {
    for (const raw of text.split(rdf12 ? /\r\n|\r|\n/ : /\r?\n/)) {
        const line = rdf12 ? raw.replace(/^[ \t]+/, "") : raw.trim();
        if (line === "" || line.startsWith("#")) continue;
        const tok = new Tokenizer(line, rdf12);
        if (rdf12 && line.startsWith("VERSION")) {
            tok.versionHint();
            continue;
        }
        const nodes: Node[] = [];
        while (!tok.atEnd()) nodes.push(tok.node());
        if (nodes.length !== 3 && nodes.length !== 4) {
            throw new NQuadsParseError(
                `expected 3 or 4 terms, got ${nodes.length}: ${line}`,
            );
        }
        validateStatement(nodes, line, rdf12);
        if (rdf12) tok.finishStatement();
        yield nodes;
    }
}

/** Parse N-Quads(-star) text into a canonical GTS file. */
export function fromNQuads(text: string): Uint8Array {
    const interner = new Interner();
    const reifiers: ReifierEntry[] = [];
    const pendingQuads: Quad[] = [];

    for (const nodes of statements(text)) {
        const [s, p, o, gname] = nodes;
        if (
            isAtom(s) &&
            isAtom(p, TermKind.Iri) &&
            p.atom.value === RDF_REIFIES &&
            isTriple(o)
        ) {
            const rid = interner.atom(s.atom);
            const gid =
                gname !== undefined
                    ? interner.node(gname, reifiers)
                    : undefined;
            setReifier(
                reifiers,
                rid,
                {
                    s: interner.node(o.triple.s, reifiers),
                    p: interner.node(o.triple.p, reifiers),
                    o: interner.node(o.triple.o, reifiers),
                },
                gid,
            );
            continue;
        }

        const q: Quad = {
            s: interner.node(s, reifiers),
            p: interner.node(p, reifiers),
            o: interner.node(o, reifiers),
        };
        if (gname !== undefined) q.g = interner.node(gname, reifiers);
        pendingQuads.push(q);
    }

    const reifierIds = new Set(reifiers.map((r) => r.rid));
    const quads: Quad[] = [];
    const annotations: AnnotationEntry[] = [];
    for (const q of pendingQuads) {
        if (reifierIds.has(q.s)) {
            annotations.push(q);
        } else {
            quads.push(q);
        }
    }

    const w = new Writer("dist");
    if (interner.terms.length > 0) w.addTerms(interner.terms);
    if (quads.length > 0) w.addQuads(quads);
    if (reifiers.length > 0) w.addReifies(reifiers);
    if (annotations.length > 0) w.addAnnot(annotations);
    return w.toBytes();
}

/** Import RDF 1.2 N-Quads into a committed GTS v2 segment. */
export function fromNQuadsV2(text: string): Uint8Array {
    if (/[\uD800-\uDFFF]/u.test(text))
        throw new NQuadsParseError("input contains a non-scalar Unicode value");
    const interner = new Interner();
    const rows: Quad[] = [];
    const seen = new Set<string>();
    for (const [subject, predicate, object, graph] of statements(text, true)) {
        const s = interner.nodeV2(subject);
        const p = interner.nodeV2(predicate);
        const o = interner.nodeV2(object);
        const g = graph === undefined ? undefined : interner.nodeV2(graph);
        const key = `${s}/${p}/${o}/${g ?? ""}`;
        if (seen.has(key)) continue;
        seen.add(key);
        rows.push({ s, p, o, g });
    }
    return writeRdfSegment(interner.terms, rows);
}

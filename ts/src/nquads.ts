// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { Graph, TermKind } from "./model.js";

const RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

function escape(lex: string): string {
    let out = "";
    for (const ch of lex) {
        switch (ch) {
            case "\\":
                out += "\\\\";
                break;
            case '"':
                out += '\\"';
                break;
            case "\n":
                out += "\\n";
                break;
            case "\r":
                out += "\\r";
                break;
            case "\t":
                out += "\\t";
                break;
            default: {
                const code = ch.codePointAt(0) ?? 0;
                if (code < 0x20) {
                    out += `\\u${code.toString(16).padStart(4, "0").toUpperCase()}`;
                } else {
                    out += ch;
                }
            }
        }
    }
    return out;
}

function render(g: Graph, tid: number, open: readonly number[] = []): string {
    if (tid < 0 || tid >= g.terms.length) {
        return `_:out_of_range_${tid}`;
    }
    // A `reifies` row may name the very term that resolves through it (§7.3),
    // so a projection MUST NOT recurse blindly: a self-reaching term degrades
    // to the same blank node an unbound triple term already produces.
    if (open.includes(tid)) {
        return `_:unbound_triple_${tid}`;
    }
    const t = g.terms[tid];
    switch (t.kind) {
        case TermKind.Iri:
            return `<${t.value}>`;
        case TermKind.Bnode:
            if (t.value !== "") return `_:${t.value}`;
            return `_:b${tid}`;
        case TermKind.Literal: {
            let lit = `"${escape(t.value)}"`;
            if (t.lang) {
                lit += `@${t.lang}`;
                if (t.direction === "ltr" || t.direction === "rtl") {
                    lit += `--${t.direction}`;
                }
            } else if (t.datatype !== undefined)
                lit += `^^${render(g, t.datatype, open)}`;
            return lit;
        }
        case TermKind.Triple: {
            // Quoted triple (RDF 1.2 triple term): its own "tt" is
            // authoritative; a legacy "tt"-less term still resolves through its
            // reifier (§7.3).
            const spo = g.tripleOf(tid);
            if (spo) {
                const inner = [...open, tid];
                return `<<( ${render(g, spo.s, inner)} ${render(g, spo.p, inner)} ${render(g, spo.o, inner)} )>>`;
            }
            // Degraded but syntactically valid: a triple term that states no
            // triple (no "tt", and no bound reifier) becomes a blank node.
            return `_:unbound_triple_${tid}`;
        }
    }
}

/** Serialise a folded Graph to N-Quads text. */
export function toNQuads(g: Graph): string {
    const lines: string[] = [];
    for (const q of g.quads) {
        const triple = `${render(g, q.s)} ${render(g, q.p)} ${render(g, q.o)}`;
        if (q.g !== undefined) {
            lines.push(`${triple} ${render(g, q.g)} .`);
        } else {
            lines.push(`${triple} .`);
        }
    }
    for (const r of g.reifiers) {
        const quoted = `<<( ${render(g, r.spo.s)} ${render(g, r.spo.p)} ${render(g, r.spo.o)} )>>`;
        const triple = `${render(g, r.rid)} <${RDF_REIFIES}> ${quoted}`;
        if (r.g !== undefined) {
            lines.push(`${triple} ${render(g, r.g)} .`);
        } else {
            lines.push(`${triple} .`);
        }
    }
    for (const a of g.annotations) {
        const triple = `${render(g, a.s)} ${render(g, a.p)} ${render(g, a.o)}`;
        if (a.g !== undefined) {
            lines.push(`${triple} ${render(g, a.g)} .`);
        } else {
            lines.push(`${triple} .`);
        }
    }
    if (lines.length === 0) return "";
    return lines.join("\n") + "\n";
}

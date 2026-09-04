// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

/** Well-known datatype IRIs used by the literal-defaulting rule (§7.1). */
export const XSD_STRING = "http://www.w3.org/2001/XMLSchema#string";
export const RDF_LANG_STRING =
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
export const RDF_DIR_LANG_STRING =
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/** The kind of an RDF term, matching the wire "k" field (§7.1). */
export enum TermKind {
    Iri = 0,
    Literal = 1,
    Bnode = 2,
    Triple = 3,
}

/** Parse the wire "k" value; an unknown kind defaults to IRI (§7.1). */
export function termKindFromWire(k: number): TermKind {
    switch (k) {
        case 1:
            return TermKind.Literal;
        case 2:
            return TermKind.Bnode;
        case 3:
            return TermKind.Triple;
        default:
            return TermKind.Iri;
    }
}

/** RDF 1.2 base direction tokens for language-tagged literals. */
export type LiteralDirection = "ltr" | "rtl";

/** A single RDF term carried by append-order id. */
export interface Term {
    kind: TermKind;
    /** IRI string, literal lexical form, or blank-node label (scope-local). */
    value: string;
    /** Term-id of the literal's datatype IRI, when explicit. */
    datatype?: number;
    /** Literal language tag (BCP 47). */
    lang?: string;
    /** RDF 1.2 initial text direction for language-tagged literals. */
    direction?: LiteralDirection;
    /** Term-id of the reifier of a quoted triple (kind == Triple).
     *
     * LEGACY indirection (wire `"rf"`), retained for files written before the
     * self-describing form; superseded by `triple` when both are present
     * (§7.3).
     */
    reifier?: number;
    /** The quoted triple's OWN (s, p, o) term-ids (wire `"tt"`).
     *
     * AUTHORITATIVE when present. Because `rdf:reifies` is not functional
     * (§7.3), a reifier id cannot identify a triple; a triple term therefore
     * carries its own components.
     */
    triple?: Triple;
}

/** A tuple of term-ids; the graph slot is undefined for the default graph. */
export interface Quad {
    s: number;
    p: number;
    o: number;
    g?: number;
}

/** A triple of term-ids. */
export interface Triple {
    s: number;
    p: number;
    o: number;
}

/** A frame the reader could not decode (§7.6). */
export interface OpaqueNode {
    id: Uint8Array;
    frameType: string;
    /** "unknown-codec" | "missing-key" | "damaged" | "unknown-frame-type" */
    reason: string;
    /** "none" | "valid" | "invalid" | "unverified" */
    sigStat: string;
    pubMeta: unknown;
    recipients: unknown[];
}

/** A recorded suppress directive (§11). */
export interface Suppression {
    targets: unknown[];
    reason: string;
    by?: number;
}

/** A machine-observable reader diagnostic (§2.3). */
export interface Diagnostic {
    code: string;
    detail: string;
    frameIndex?: number;
}

/** The verification outcome for a signed frame (§9.2).
 *
 * `cose` retains the raw COSE_Sign1 bytes so streamable compaction (§10.1)
 * can carry the signature detached — forever verifiable against `frameId`
 * even after the frame itself is re-authored into a new chain.
 */
export interface Signature {
    frameId: Uint8Array;
    kid: string;
    /** "none" | "valid" | "invalid" | "unverified" */
    status: string;
    cose?: Uint8Array;
}

/** One segment's layout state (§3.3).
 *
 * `covered`/`head` come from the segment's last intact `index` frame;
 * `tail` counts the legal unpresaged frames after it ("streamable through
 * frame *covered*, accretive tail of *tail* frame(s)"). For an unclaimed
 * (accretive) segment all fields are their zero values.
 */
export interface StreamableInfo {
    claimed: boolean;
    covered: number;
    tail: number;
    head?: Uint8Array;
}

/** A single key/value metadata pair. */
export interface MetaEntry {
    key: string;
    value: unknown;
}

/** A single inline blob. */
export interface BlobEntry {
    digest: string;
    data: Uint8Array;
}

/** Declared blob metadata by digest. */
export interface BlobMetaEntry {
    digest: string;
    meta: unknown;
}

function sameTriple(a: Triple, b: Triple): boolean {
    return a.s === b.s && a.p === b.p && a.o === b.o;
}

/** Reifier-id → triple binding. */
export interface ReifierEntry {
    rid: number;
    spo: Triple;
    g?: number;
}

/** Annotation row. */
export interface AnnotationEntry {
    s: number;
    p: number;
    o: number;
    g?: number;
}

/**
 * The folded result of a GTS log.
 *
 * RDF state is kept alongside reader sidecars such as diagnostics, signatures,
 * opaque nodes, segment heads, and streamable layout observations. Sidecars are
 * observations about the log and should not be replayed as authoring input
 * unless a caller is deliberately preserving evidence.
 */
export class Graph {
    terms: Term[] = [];
    quads: Quad[] = [];
    reifiers: ReifierEntry[] = [];
    annotations: AnnotationEntry[] = [];
    blobs: BlobEntry[] = [];
    blobMeta: BlobMetaEntry[] = [];
    meta: MetaEntry[] = [];
    suppressions: Suppression[] = [];
    opaque: OpaqueNode[] = [];
    signatures: Signature[] = [];
    diagnostics: Diagnostic[] = [];
    segmentHeads: Uint8Array[] = [];
    segmentProfiles: string[] = [];
    segmentMeta: MetaEntry[][] = [];
    /** Per-segment layout state (§3.3), in file order — the
     *  declared-vs-computed streamable claim, its covered boundary, and the
     *  accretive tail. */
    segmentStreamable: StreamableInfo[] = [];

    /** Return the FIRST folded triple binding for `rid`, if present.
     *
     * `rdf:reifies` is not functional (§7.3), so a reifier may bind several
     * triples. This accessor is the legacy single-valued view used only to
     * resolve a `"tt"`-less triple term; use `reifierTriples` for the full
     * multi-valued statement layer.
     */
    reifier(rid: number): Triple | undefined {
        for (const r of this.reifiers) {
            if (r.rid === rid) return r.spo;
        }
        return undefined;
    }

    /** Return every DISTINCT triple bound to `rid`, in file order (§7.3). */
    reifierTriples(rid: number): Triple[] {
        const out: Triple[] = [];
        for (const r of this.reifiers) {
            if (r.rid !== rid) continue;
            if (out.some((seen) => sameTriple(seen, r.spo))) continue;
            out.push(r.spo);
        }
        return out;
    }

    /** Resolve the (s, p, o) a quoted-triple term denotes (§7.3).
     *
     * The term's own `triple` (wire `"tt"`) is authoritative. A legacy
     * `"tt"`-less term falls back to the FIRST binding of its reifier so
     * pre-`"tt"` files keep reading exactly as they did.
     */
    /** Resolve the `(s, p, o)` a quoted-triple term denotes (§7.3).
     *
     * Resolution MUST terminate. A `reifies` row may name the very term that
     * resolves through it, so a term that reaches itself states NO triple for
     * this walk and resolves to `undefined` — the same answer an unbound triple
     * term gives, which every projection renders as a fresh blank node.
     * Guarding here rather than inside each walk keeps the degradation at the
     * RIGHT LEVEL: the self-reaching term itself becomes the blank node,
     * instead of resolving one step and degrading a nested occurrence, which
     * renders a different graph. A `"tt"` cannot cycle, since its components
     * name strictly smaller term-ids (§7.2).
     */
    tripleOf(termId: number): Triple | undefined {
        const t = this.terms[termId];
        if (!t) return undefined;
        if (t.triple !== undefined) return t.triple;
        if (t.reifier === undefined) return undefined;
        const spo = this.reifier(t.reifier);
        if (spo === undefined) return undefined;
        const seen = new Set<number>();
        for (const c of [spo.s, spo.p, spo.o]) {
            if (this.resolutionReaches(c, termId, seen)) return undefined;
        }
        return spo;
    }

    /** Does resolving `from` walk back to `anchor`? */
    private resolutionReaches(
        from: number,
        anchor: number,
        seen: Set<number>,
    ): boolean {
        if (from === anchor) return true;
        if (seen.has(from)) return false;
        seen.add(from);
        const t = this.terms[from];
        if (!t || t.kind !== TermKind.Triple) return false;
        const spo =
            t.triple ??
            (t.reifier !== undefined ? this.reifier(t.reifier) : undefined);
        if (spo === undefined) return false;
        return [spo.s, spo.p, spo.o].some((c) =>
            this.resolutionReaches(c, anchor, seen),
        );
    }

    /** Append a reifier row unless the identical row is already present.
     *
     * The `reifies` frame is a MULTI-VALUED statement layer: distinct triples
     * on one reifier id all survive, each keeping its own graph slot. Only
     * byte-identical rows collapse (§7.8 set semantics).
     */
    setReifier(rid: number, spo: Triple, g?: number): void {
        for (const r of this.reifiers) {
            if (r.rid === rid && sameTriple(r.spo, spo) && r.g === g) {
                return;
            }
        }
        this.reifiers.push({ rid, spo, ...(g !== undefined ? { g } : {}) });
    }

    /** Set a meta key, replacing in place. */
    setMeta(key: string, value: unknown): void {
        for (const m of this.meta) {
            if (m.key === key) {
                m.value = value;
                return;
            }
        }
        this.meta.push({ key, value });
    }

    /** Record a blob's declared metadata, replacing in place. */
    setBlobMeta(digest: string, meta: unknown): void {
        for (const bm of this.blobMeta) {
            if (bm.digest === digest) {
                bm.meta = meta;
                return;
            }
        }
        this.blobMeta.push({ digest, meta });
    }

    /** Store an inline blob under its digest, replacing in place. */
    setBlob(digest: string, data: Uint8Array): void {
        for (const b of this.blobs) {
            if (b.digest === digest) {
                b.data = data;
                return;
            }
        }
        this.blobs.push({ digest, data });
    }

    /** The effective datatype IRI of a literal, applying §7.1 defaulting. */
    datatypeIri(t: Term): string {
        if (t.datatype !== undefined) {
            const dt = this.terms[t.datatype];
            if (dt && dt.value) return dt.value;
            return XSD_STRING;
        }
        if (t.lang) {
            return t.direction === "ltr" || t.direction === "rtl"
                ? RDF_DIR_LANG_STRING
                : RDF_LANG_STRING;
        }
        return XSD_STRING;
    }
}

/** Diagnose the one incoherent shape that survives §7.3, in place.
 *
 * `rdf:reifies` is not functional, so a reifier id bound to several triples is
 * ordinary RDF 1.2 and is NOT a conflict. The single remaining incoherence is a
 * `"tt"`-less (legacy indirect) quoted-triple TERM whose reifier id binds MORE
 * THAN ONE distinct triple: the file is asking for one term with two meanings.
 * `ConflictingReifier` is reported once per offending term; NO reifier row is
 * dropped, and the term keeps resolving to the first binding in file order so
 * the rendering of legacy files never changes.
 *
 * Runs on a fold that is about to be handed to a consumer (a whole read, or one
 * segment of a per-segment read), never twice over the same terms. The
 * diagnostic carries no frame index: the shape is a property of the completed
 * fold, not of any one frame.
 *
 * Returns the diagnostics it appended, so a progressive reader can republish
 * them to its event sink.
 */
export function checkConflictingReifiers(g: Graph): Diagnostic[] {
    const added: Diagnostic[] = [];
    for (let tid = 0; tid < g.terms.length; tid++) {
        const term = g.terms[tid];
        if (term.kind !== TermKind.Triple || term.triple !== undefined) {
            continue;
        }
        if (term.reifier === undefined) continue;
        const bindings = g.reifierTriples(term.reifier);
        if (bindings.length <= 1) continue;
        const diagnostic: Diagnostic = {
            code: "ConflictingReifier",
            detail:
                `legacy triple term ${tid} resolves through reifier ` +
                `${term.reifier}, which binds ${bindings.length} distinct ` +
                "triples; state the triple with 'tt' (§7.3)",
        };
        g.diagnostics.push(diagnostic);
        added.push(diagnostic);
    }
    return added;
}

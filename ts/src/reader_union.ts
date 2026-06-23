// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import {
    Graph,
    TermKind,
    type Quad,
    type Suppression,
    type Triple,
} from "./model.js";

interface InternKey {
    typ: number; // 0=iri, 1=lit, 2=bnode, 3=qt
    a: string;
    b: string;
    c: string;
    d?: string;
    seg?: number;
    rf?: number;
    bnodeTid?: number;
    bnodeLabeled?: boolean;
}

function asText(value: unknown): string | undefined {
    if (typeof value === "string") return value;
    return undefined;
}

function asInt(value: unknown): number | undefined {
    if (typeof value === "number") {
        if (Number.isInteger(value) && value >= 0) return value;
    }
    if (typeof value === "bigint") {
        if (value >= 0n && value <= Number.MAX_SAFE_INTEGER) {
            return Number(value);
        }
    }
    return undefined;
}

function textOr(value: unknown, def: string): string {
    return asText(value) ?? def;
}

class SegmentUnioner {
    out = new Graph();
    intern = new Map<string, number>();

    keyString(k: InternKey): string {
        return JSON.stringify(k);
    }

    keyFor(seg: Graph, segIdx: number, tid: number): InternKey {
        const t = seg.terms[tid];
        switch (t.kind) {
            case TermKind.Iri:
                return { typ: 0, a: t.value, b: "", c: "" };
            case TermKind.Literal:
                return {
                    typ: 1,
                    a: t.value,
                    b: seg.datatypeIri(t),
                    c: t.lang ?? "",
                    d: t.direction ?? "",
                };
            case TermKind.Bnode:
                if (t.value !== "") {
                    return {
                        typ: 2,
                        a: t.value,
                        b: "",
                        c: "",
                        seg: segIdx,
                        bnodeLabeled: true,
                    };
                }
                return {
                    typ: 2,
                    a: "",
                    b: "",
                    c: "",
                    seg: segIdx,
                    bnodeTid: tid,
                };
            case TermKind.Triple: {
                let rf: number | undefined;
                if (t.reifier !== undefined) {
                    rf = this.mapTerm(seg, segIdx, t.reifier);
                }
                return { typ: 3, a: "", b: "", c: "", rf };
            }
        }
    }

    mapTerm(seg: Graph, segIdx: number, tid: number): number {
        const key = this.keyFor(seg, segIdx, tid);
        const ks = this.keyString(key);
        if (this.intern.has(ks)) return this.intern.get(ks)!;
        const t = seg.terms[tid];
        let datatype: number | undefined;
        if (t.datatype !== undefined) {
            datatype = this.mapTerm(seg, segIdx, t.datatype);
        }
        let reifier: number | undefined;
        if (t.reifier !== undefined) {
            reifier = this.mapTerm(seg, segIdx, t.reifier);
        }
        let value = t.value;
        if (t.kind === TermKind.Bnode) {
            if (value !== "") {
                value = `s${segIdx}.${value}`;
            } else {
                value = `s${segIdx}._anon${this.out.terms.length}`;
            }
        }
        this.out.terms.push({
            kind: t.kind,
            value,
            datatype,
            lang: t.lang,
            direction: t.direction,
            reifier,
        });
        const newId = this.out.terms.length - 1;
        this.intern.set(ks, newId);
        return newId;
    }

    remapSuppression(
        seg: Graph,
        segIdx: number,
        sup: Suppression,
    ): Suppression {
        const n = seg.terms.length;
        const newTargets: unknown[] = [];
        for (const target of sup.targets) {
            if (!(target instanceof Map)) {
                newTargets.push(target);
                continue;
            }
            const kind = textOr(target.get("kind"), "");
            if (kind === "frame" || kind === "blob") {
                newTargets.push(target);
                continue;
            }
            const newMap = new Map<unknown, unknown>();
            for (const [k, v] of target) {
                newMap.set(k, v);
                const key = asText(k) ?? "";
                if ((kind === "term" || kind === "reifier") && key === "id") {
                    const tid = asInt(v);
                    if (tid !== undefined && tid < n) {
                        newMap.set(k, this.mapTerm(seg, segIdx, tid));
                    }
                } else if (kind === "quad" && key === "q") {
                    const ids = Array.isArray(v) ? v : undefined;
                    if (ids) {
                        newMap.set(
                            k,
                            ids.map((x) => {
                                const tid = asInt(x);
                                if (tid !== undefined && tid < n) {
                                    return this.mapTerm(seg, segIdx, tid);
                                }
                                return x;
                            }),
                        );
                    }
                }
            }
            newTargets.push(newMap);
        }
        const out: Suppression = { targets: newTargets, reason: sup.reason };
        if (sup.by !== undefined && sup.by < n) {
            out.by = this.mapTerm(seg, segIdx, sup.by);
        }
        return out;
    }
}

function unionQuadKey(q: Quad): string {
    return q.g === undefined
        ? `${q.s},${q.p},${q.o}`
        : `${q.s},${q.p},${q.o},${q.g}`;
}

export function unionSegments(segments: Graph[]): Graph {
    const u = new SegmentUnioner();
    const seen = new Set<string>();
    for (let segIdx = 0; segIdx < segments.length; segIdx++) {
        const seg = segments[segIdx];
        for (const q of seg.quads) {
            const uq: Quad = {
                s: u.mapTerm(seg, segIdx, q.s),
                p: u.mapTerm(seg, segIdx, q.p),
                o: u.mapTerm(seg, segIdx, q.o),
            };
            if (q.g !== undefined) uq.g = u.mapTerm(seg, segIdx, q.g);
            const key = unionQuadKey(uq);
            if (!seen.has(key)) {
                seen.add(key);
                u.out.quads.push(uq);
            }
        }
        for (const r of seg.reifiers) {
            const newRf = u.mapTerm(seg, segIdx, r.rid);
            const spo: Triple = {
                s: u.mapTerm(seg, segIdx, r.spo.s),
                p: u.mapTerm(seg, segIdx, r.spo.p),
                o: u.mapTerm(seg, segIdx, r.spo.o),
            };
            u.out.setReifier(newRf, spo);
        }
        for (const a of seg.annotations) {
            u.out.annotations.push({
                s: u.mapTerm(seg, segIdx, a.s),
                p: u.mapTerm(seg, segIdx, a.p),
                o: u.mapTerm(seg, segIdx, a.o),
            });
        }
        for (const b of seg.blobs) u.out.setBlob(b.digest, b.data);
        for (const bm of seg.blobMeta) u.out.setBlobMeta(bm.digest, bm.meta);
        for (const m of seg.meta) u.out.setMeta(m.key, m.value);
        u.out.segmentMeta.push(...seg.segmentMeta);
        for (const sup of seg.suppressions) {
            u.out.suppressions.push(u.remapSuppression(seg, segIdx, sup));
        }
        u.out.opaque.push(...seg.opaque);
        u.out.signatures.push(...seg.signatures);
        u.out.diagnostics.push(...seg.diagnostics);
        u.out.segmentHeads.push(...seg.segmentHeads);
        u.out.segmentProfiles.push(...seg.segmentProfiles);
        u.out.segmentStreamable.push(...seg.segmentStreamable);
    }
    return u.out;
}

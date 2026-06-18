// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { Graph, type Diagnostic } from "./model.js";
import { Read } from "./reader.js";
import * as wire from "./wire.js";

/** Declared media type for a nested GTS CBOR Sequence blob. */
export const GTS_MEDIA_TYPE = "application/vnd.blackcat.gts+cbor-seq";

export interface ReadNestedOptions {
    maxDepth?: number;
    maxDecodedBytes?: number;
}

/** A root fold plus nested folds addressed by containing blob digest. */
export class NestedReadResult {
    constructor(
        readonly graph: Graph,
        readonly subgraphs: Map<string, Graph>,
        readonly diagnostics: Diagnostic[],
    ) {}

    /** Return the nested graph carried by digest, when one was exposed. */
    subgraph(digest: string): Graph | undefined {
        return this.subgraphs.get(digest);
    }
}

/** Read a GTS file and boundedly recurse into nested-GTS blobs.
 *
 * Baseline readers treat nested GTS as ordinary blobs. Full Reader callers use
 * this helper to expose subgraphs by blob digest while enforcing the recursion
 * and decoded-size budgets required by the v1 security policy.
 */
export function readNested(
    data: Uint8Array,
    options: ReadNestedOptions = {},
): NestedReadResult {
    const maxDepth = Math.max(0, options.maxDepth ?? 3);
    const maxDecodedBytes = Math.max(
        0,
        options.maxDecodedBytes ?? 16 * 1024 * 1024,
    );

    const budget = { remaining: maxDecodedBytes };
    const seen = new Set<string>();
    const subgraphs = new Map<string, Graph>();
    const root = visit(data, 0, maxDepth, budget, seen, subgraphs);
    const diagnostics = [
        ...root.diagnostics,
        ...[...subgraphs.values()].flatMap((graph) => graph.diagnostics),
    ];
    return new NestedReadResult(root, subgraphs, diagnostics);
}

function visit(
    data: Uint8Array,
    depth: number,
    maxDepth: number,
    budget: { remaining: number },
    seen: Set<string>,
    subgraphs: Map<string, Graph>,
): Graph {
    const graph = Read(data, true);
    for (const entry of graph.blobMeta) {
        if (blobMediaType(entry.meta) !== GTS_MEDIA_TYPE) continue;
        if (seen.has(entry.digest)) continue;
        const nestedBytes = blobBytes(graph, entry.digest);
        if (!nestedBytes) continue;
        if (depth >= maxDepth) {
            graph.diagnostics.push({
                code: "RecursionLimit",
                detail: `nested GTS blob ${entry.digest} exceeds max depth ${maxDepth}`,
            });
            continue;
        }
        if (nestedBytes.length > budget.remaining) {
            graph.diagnostics.push({
                code: "RecursionLimit",
                detail:
                    `nested GTS decoded-size budget exceeded at ${entry.digest}: ` +
                    `${nestedBytes.length} > ${budget.remaining}`,
            });
            continue;
        }
        budget.remaining -= nestedBytes.length;
        seen.add(entry.digest);
        const child = visit(
            nestedBytes,
            depth + 1,
            maxDepth,
            budget,
            seen,
            subgraphs,
        );
        if (child.segmentHeads.length === 0) {
            graph.diagnostics.push({
                code: "DamagedFrame",
                detail: `nested GTS blob ${entry.digest} could not be parsed`,
            });
            continue;
        }
        subgraphs.set(entry.digest, child);
    }
    return graph;
}

function blobBytes(graph: Graph, digest: string): Uint8Array | undefined {
    return graph.blobs.find((blob) => blob.digest === digest)?.data;
}

function blobMediaType(meta: unknown): string {
    if (!(meta instanceof Map)) return "";
    return wire.asText(wire.mapGet(meta, "mt")) ?? "";
}

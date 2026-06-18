// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import type { Diagnostic } from "../src/model.js";
import { readNested } from "../src/nested.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");

interface NestedSecurityVector {
    id: string;
    max_depth: number;
    expected_diagnostics: string[];
}

function hasDiagnostic(diagnostics: Diagnostic[], code: string): boolean {
    return diagnostics.some((diag) => diag.code === code);
}

function readHexFixture(name: string): Uint8Array {
    const hex = readFileSync(
        join(repoRoot, "vectors", "security", name),
        "utf-8",
    ).replace(/\s+/g, "");
    return Uint8Array.from(Buffer.from(hex, "hex"));
}

function loadVector(): NestedSecurityVector {
    return JSON.parse(
        readFileSync(
            join(
                repoRoot,
                "vectors",
                "security",
                "nested-recursion-limit.json",
            ),
            "utf-8",
        ),
    ) as NestedSecurityVector;
}

test("readNested exposes subgraphs by blob digest", () => {
    const result = readNested(
        readHexFixture("nested-recursion-limit.gts.hex"),
        {
            maxDepth: 3,
            maxDecodedBytes: 16 * 1024 * 1024,
        },
    );
    const child = result.graph.blobs[0];

    const subgraph = result.subgraph(child.digest);
    assert.ok(subgraph);
    assert.equal(subgraph.blobs.length, 1);
    assert.equal(hasDiagnostic(result.diagnostics, "RecursionLimit"), false);
});

test("nested-recursion-limit security vector", () => {
    const vector = loadVector();
    const result = readNested(
        readHexFixture("nested-recursion-limit.gts.hex"),
        {
            maxDepth: vector.max_depth,
            maxDecodedBytes: 16 * 1024 * 1024,
        },
    );

    const child = result.graph.blobs[0];
    const childGraph = result.subgraph(child.digest);
    assert.ok(childGraph);
    const grandchild = childGraph.blobs[0];
    assert.equal(result.subgraph(grandchild.digest), undefined);
    for (const code of vector.expected_diagnostics) {
        assert.equal(hasDiagnostic(result.diagnostics, code), true);
    }
});

test("readNested stops at decoded-size budget", () => {
    const result = readNested(
        readHexFixture("nested-recursion-limit.gts.hex"),
        {
            maxDepth: 3,
            maxDecodedBytes: 0,
        },
    );

    assert.equal(result.subgraphs.size, 0);
    assert.equal(hasDiagnostic(result.diagnostics, "RecursionLimit"), true);
});

test("readNested skips duplicate nested blob digests", () => {
    const fixture = readHexFixture("nested-duplicate-digest.gts.hex");
    const baseline = readNested(fixture, {
        maxDepth: 3,
        maxDecodedBytes: 16 * 1024 * 1024,
    });
    const childA = baseline.graph.blobs[0];
    const childB = baseline.graph.blobs[1];
    const childAGraph = baseline.subgraph(childA.digest);
    assert.ok(childAGraph);
    const sharedGrandchild = childAGraph.blobs[0];
    const exactBudget =
        childA.data.length + childB.data.length + sharedGrandchild.data.length;

    const result = readNested(fixture, {
        maxDepth: 3,
        maxDecodedBytes: exactBudget,
    });

    assert.equal(result.subgraphs.size, 3);
    assert.equal(hasDiagnostic(result.diagnostics, "RecursionLimit"), false);
});

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import assert from "node:assert/strict";

import { Graph, TermKind } from "../src/model.js";
import {
    evaluateProfilePolicy,
    FILES_NS,
    signatureTrust,
    TrustPolicy,
    type ProfileFinding,
} from "../src/policy.js";
import { COMPACTION } from "../src/stream.js";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../../");
const EX = "https://example.org/";
const KID = "did:example:issuer";

interface ProfileSecurityVector {
    id: string;
    expected_findings: string[];
}

function findingCodes(findings: ProfileFinding[]): Set<string> {
    return new Set(findings.map((finding) => finding.code));
}

function hasFinding(findings: ProfileFinding[], code: string): boolean {
    return findingCodes(findings).has(code);
}

function signedEvidenceGraph(kid = KID): Graph {
    const graph = new Graph();
    const head = new Uint8Array([1, 2, 3]);
    graph.segmentProfiles.push("evidence");
    graph.segmentHeads.push(head);
    graph.signatures.push({ frameId: head, kid, status: "valid" });
    return graph;
}

function loadVector(): ProfileSecurityVector {
    return JSON.parse(
        readFileSync(
            join(repoRoot, "vectors", "security", "profile-policy.json"),
            "utf-8",
        ),
    ) as ProfileSecurityVector;
}

test("valid signature does not imply trusted signer or true claim", () => {
    const graph = signedEvidenceGraph();
    assert.ok(graph.signatures.every((sig) => sig.status === "valid"));

    for (const item of signatureTrust(graph)) {
        assert.equal(item.trusted, false);
    }
    assert.equal(
        hasFinding(
            evaluateProfilePolicy(graph),
            "ProfileSignerTrustNotEvaluated",
        ),
        true,
    );

    const trusted = new TrustPolicy({
        trustedSigners: [KID],
        requireTrustedSigner: true,
    });
    assert.ok(signatureTrust(graph, trusted).every((item) => item.trusted));
    assert.equal(
        evaluateProfilePolicy(graph, trusted).some(
            (finding) => finding.severity === "error",
        ),
        false,
    );
});

test("evidence profile requires signatures and head commitment", () => {
    const graph = new Graph();
    graph.segmentProfiles.push("evidence");
    graph.segmentHeads.push(new Uint8Array([1, 2, 3]));

    const findings = evaluateProfilePolicy(graph);
    assert.equal(hasFinding(findings, "ProfileSignatureRequired"), true);
    assert.equal(hasFinding(findings, "EvidenceHeadCommitmentRequired"), true);
});

test("opaque profile requires pseudonymous recipient kids", () => {
    const graph = new Graph();
    graph.segmentProfiles.push("opaque");
    graph.opaque.push({
        id: new Uint8Array(),
        frameType: "meta",
        reason: "missing-key",
        sigStat: "unverified",
        pubMeta: undefined,
        recipients: [new Map<unknown, unknown>([["kid", "did:court"]])],
    });

    assert.equal(
        hasFinding(evaluateProfilePolicy(graph), "OpaqueRecipientKidPublic"),
        true,
    );

    graph.opaque[0].recipients = [
        new Map<unknown, unknown>([
            ["kid", "anon:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
        ]),
    ];
    assert.equal(
        hasFinding(evaluateProfilePolicy(graph), "OpaqueRecipientKidPublic"),
        false,
    );

    graph.opaque[0].recipients = [
        new Map<unknown, unknown>([["kid", "did:court"]]),
    ];
    const custom = new TrustPolicy({ pseudonymousKidPattern: "^did:court$" });
    assert.equal(
        hasFinding(
            evaluateProfilePolicy(graph, custom),
            "OpaqueRecipientKidPublic",
        ),
        false,
    );
});

test("profile and stream vocabulary findings", () => {
    const files = new Graph();
    files.segmentProfiles.push("generic");
    files.terms.push(
        { kind: TermKind.Iri, value: FILES_NS + "FileEntry" },
        { kind: TermKind.Iri, value: EX + "relatedTo" },
        { kind: TermKind.Literal, value: "x.txt" },
    );
    files.quads.push({ s: 0, p: 1, o: 2 });
    assert.equal(
        hasFinding(evaluateProfilePolicy(files), "ProfileVocabularyUndeclared"),
        true,
    );

    const streamGraph = new Graph();
    streamGraph.segmentProfiles.push("generic");
    streamGraph.terms.push(
        { kind: TermKind.Iri, value: EX + "rewrite" },
        { kind: TermKind.Iri, value: COMPACTION },
        { kind: TermKind.Literal, value: "agent" },
    );
    streamGraph.quads.push({ s: 0, p: 1, o: 2 });
    assert.equal(
        hasFinding(
            evaluateProfilePolicy(streamGraph),
            "StreamVocabularyWithoutLayout",
        ),
        true,
    );
});

test("profile-policy security vector", () => {
    const vector = loadVector();
    const seen = new Set<string>();

    for (const finding of evaluateProfilePolicy(signedEvidenceGraph())) {
        seen.add(finding.code);
    }
    for (const finding of evaluateProfilePolicy(
        signedEvidenceGraph(),
        new TrustPolicy({
            trustedSigners: ["did:example:someone-else"],
            requireTrustedSigner: true,
        }),
    )) {
        seen.add(finding.code);
    }

    const opaque = new Graph();
    opaque.segmentProfiles.push("opaque");
    opaque.opaque.push({
        id: new Uint8Array(),
        frameType: "meta",
        reason: "missing-key",
        sigStat: "unverified",
        pubMeta: undefined,
        recipients: [new Map<unknown, unknown>([["kid", "did:court"]])],
    });
    for (const finding of evaluateProfilePolicy(opaque)) {
        seen.add(finding.code);
    }

    for (const code of vector.expected_findings) {
        assert.equal(seen.has(code), true, code);
    }
});

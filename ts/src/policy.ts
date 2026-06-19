// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

import { Graph, type Signature, TermKind } from "./model.js";
import { SEALED_SOURCE, STREAM_NS } from "./stream.js";
import * as wire from "./wire.js";

/** Vocabulary namespace used by the optional-standard files profile. */
export const FILES_NS = "https://w3id.org/gts/files#";
/** Documented high-privacy recipient kid shape for the opaque profile. */
export const DEFAULT_PSEUDONYMOUS_KID_PATTERN = "^anon:[0-9a-fA-F]{32,}$";

const PROFILE_VOCABS = new Map<string, string>([["files", FILES_NS]]);

/** Deployment trust anchors and high-privacy recipient-id options. */
export interface TrustPolicyOptions {
    /** Signer kid values accepted by the deployment. */
    trustedSigners?: Iterable<string>;
    /** Require at least one trusted valid signer for evidence/opaque profiles. */
    requireTrustedSigner?: boolean;
    /** Recipient kid pattern for high-privacy opaque-profile checks. */
    pseudonymousKidPattern?: string;
}

/** Deployment trust anchors and high-privacy recipient-id rules. */
export class TrustPolicy {
    readonly trustedSigners: Set<string>;
    readonly requireTrustedSigner: boolean;
    readonly pseudonymousKidPattern: string;

    constructor(options: TrustPolicyOptions = {}) {
        this.trustedSigners = new Set(options.trustedSigners ?? []);
        this.requireTrustedSigner = options.requireTrustedSigner ?? false;
        this.pseudonymousKidPattern =
            options.pseudonymousKidPattern ?? DEFAULT_PSEUDONYMOUS_KID_PATTERN;
    }

    /** True when kid is a deployment-trusted signer. */
    isTrusted(kid: string | undefined): boolean {
        return kid !== undefined && kid !== "" && this.trustedSigners.has(kid);
    }

    /** True when kid satisfies the high-privacy opaque-profile shape. */
    isPseudonymousRecipient(kid: string): boolean {
        if (this.pseudonymousKidPattern !== DEFAULT_PSEUDONYMOUS_KID_PATTERN) {
            return customPseudonymousPatternMatches(
                this.pseudonymousKidPattern,
                kid,
            );
        }
        if (!kid.startsWith("anon:")) return false;
        const hex = kid.slice("anon:".length);
        return hex.length >= 32 && [...hex].every(isAsciiHex);
    }
}

/** A signature's cryptographic status plus deployment-trust result. */
export interface SignatureTrust {
    /** Signed frame id. */
    frameId: Uint8Array;
    /** Resolved signer kid, when present. */
    kid: string | undefined;
    /** Reader cryptographic status. */
    status: string;
    /** True when status is valid and kid is deployment-trusted. */
    trusted: boolean;
}

export type Severity = "error" | "warning" | "info";

/** One profile or trust-policy finding. */
export interface ProfileFinding {
    /** Stable machine-readable finding code. */
    code: string;
    /** Error, warning, or info. */
    severity: Severity;
    /** Human-readable finding text. */
    detail: string;
    /** Profile that triggered the finding, when applicable. */
    profile?: string;
    /** Segment index for segment-scoped checks. */
    segmentIndex?: number;
}

/** Evaluate deployment trust for already-verified signature statuses. */
export function signatureTrust(
    graph: Graph,
    policy = new TrustPolicy(),
): SignatureTrust[] {
    return graph.signatures.map((sig) => ({
        frameId: sig.frameId,
        kid: sig.kid,
        status: sig.status,
        trusted: sig.status === "valid" && policy.isTrusted(sig.kid),
    }));
}

/** Run supported profile checks without changing core reader validity. */
export function evaluateProfilePolicy(
    graph: Graph,
    policy = new TrustPolicy(),
    segmentIndex?: number,
): ProfileFinding[] {
    const declared = declaredProfiles(graph);
    const findings: ProfileFinding[] = [
        ...profileVocabFindings(graph, declared, segmentIndex),
        ...streamVocabFindings(graph, segmentIndex),
    ];
    for (const profile of [...declared].sort()) {
        if (profile === "evidence" || profile === "opaque") {
            findings.push(
                ...signaturePolicyFindings(
                    graph,
                    profile,
                    policy,
                    segmentIndex,
                ),
            );
        }
        if (profile === "evidence") {
            findings.push(...evidenceHeadFindings(graph, segmentIndex));
        }
        if (profile === "opaque") {
            findings.push(
                ...opaqueRecipientFindings(graph, policy, segmentIndex),
            );
        }
    }
    return findings;
}

function declaredProfiles(graph: Graph): Set<string> {
    if (graph.segmentProfiles.length === 0) return new Set(["generic"]);
    return new Set(graph.segmentProfiles);
}

function finding(
    code: string,
    severity: Severity,
    detail: string,
    profile?: string,
    segmentIndex?: number,
): ProfileFinding {
    return { code, severity, detail, profile, segmentIndex };
}

function signaturePolicyFindings(
    graph: Graph,
    profile: string,
    policy: TrustPolicy,
    segmentIndex?: number,
): ProfileFinding[] {
    if (graph.signatures.length === 0) {
        if (profile === "evidence" && hasSealedSource(graph)) return [];
        return [
            finding(
                "ProfileSignatureRequired",
                "error",
                `profile '${profile}' requires signed frames`,
                profile,
                segmentIndex,
            ),
        ];
    }

    const findings: ProfileFinding[] = [];
    const invalid = graph.signatures.filter((sig) => sig.status === "invalid");
    if (invalid.length > 0) {
        findings.push(
            finding(
                "ProfileSignatureInvalid",
                "error",
                `profile '${profile}' has ${invalid.length} invalid signature(s)`,
                profile,
                segmentIndex,
            ),
        );
    }
    const unverified = graph.signatures.filter(
        (sig) => sig.status === "unverified",
    );
    if (unverified.length > 0) {
        findings.push(
            finding(
                "ProfileSignatureUnverified",
                "error",
                `profile '${profile}' has ${unverified.length} unresolved signature(s)`,
                profile,
                segmentIndex,
            ),
        );
    }

    const trust = signatureTrust(graph, policy);
    const valid = trust.filter((sig) => sig.status === "valid");
    const trusted = valid.filter((sig) => sig.trusted);
    if (policy.requireTrustedSigner && trusted.length === 0) {
        findings.push(
            finding(
                "ProfileSignerUntrusted",
                "error",
                `profile '${profile}' has no deployment-trusted valid signer`,
                profile,
                segmentIndex,
            ),
        );
    } else if (valid.length > 0 && policy.trustedSigners.size === 0) {
        findings.push(
            finding(
                "ProfileSignerTrustNotEvaluated",
                "warning",
                `profile '${profile}' signatures are cryptographically valid; ` +
                    "no deployment trust policy was supplied",
                profile,
                segmentIndex,
            ),
        );
    }
    return findings;
}

function evidenceHeadFindings(
    graph: Graph,
    segmentIndex?: number,
): ProfileFinding[] {
    if (hasSealedSource(graph) || graph.segmentHeads.length === 0) return [];
    let signed = signedHeads(graph.signatures, "valid");
    if (signed.length === 0)
        signed = signedHeads(graph.signatures, "unverified");
    if (signed.length === 0) {
        return [
            finding(
                "EvidenceHeadCommitmentRequired",
                "error",
                "profile 'evidence' requires a signed segment head commitment",
                "evidence",
                segmentIndex,
            ),
        ];
    }
    const hasCommitment = graph.segmentHeads.some((head) =>
        signed.some((signedHead) => bytesEqual(head, signedHead)),
    );
    if (hasCommitment) return [];
    return [
        finding(
            "EvidenceHeadCommitmentRequired",
            "error",
            "profile 'evidence' requires a signed segment head commitment",
            "evidence",
            segmentIndex,
        ),
    ];
}

function signedHeads(signatures: Signature[], status: string): Uint8Array[] {
    return signatures
        .filter((sig) => sig.status === status)
        .map((sig) => sig.frameId);
}

function hasSealedSource(graph: Graph): boolean {
    return graph.quads.some(
        (quad) => termIriValue(graph, quad.p) === SEALED_SOURCE,
    );
}

function opaqueRecipientFindings(
    graph: Graph,
    policy: TrustPolicy,
    segmentIndex?: number,
): ProfileFinding[] {
    const findings: ProfileFinding[] = [];
    for (const node of graph.opaque) {
        for (const recipient of node.recipients) {
            if (!(recipient instanceof Map)) {
                findings.push(
                    finding(
                        "OpaqueRecipientKidMissing",
                        "error",
                        "opaque-profile recipient lacks a string kid",
                        "opaque",
                        segmentIndex,
                    ),
                );
                continue;
            }
            const kid = wire.asText(wire.mapGet(recipient, "kid"));
            if (kid === undefined) {
                findings.push(
                    finding(
                        "OpaqueRecipientKidMissing",
                        "error",
                        "opaque-profile recipient lacks a string kid",
                        "opaque",
                        segmentIndex,
                    ),
                );
                continue;
            }
            if (!policy.isPseudonymousRecipient(kid)) {
                findings.push(
                    finding(
                        "OpaqueRecipientKidPublic",
                        "error",
                        `opaque-profile high-privacy recipient kid must match ` +
                            `'${policy.pseudonymousKidPattern}', got '${kid}'`,
                        "opaque",
                        segmentIndex,
                    ),
                );
            }
        }
    }
    return findings;
}

function profileVocabFindings(
    graph: Graph,
    declared: Set<string>,
    segmentIndex?: number,
): ProfileFinding[] {
    const used = usedVocabs(graph);
    const findings: ProfileFinding[] = [];
    for (const [profile, vocab] of PROFILE_VOCABS) {
        const declares = declared.has(profile);
        const uses = used.has(vocab);
        if (uses && !declares) {
            findings.push(
                finding(
                    "ProfileVocabularyUndeclared",
                    "error",
                    `segment uses ${vocab} vocabulary but does not declare '${profile}'`,
                    profile,
                    segmentIndex,
                ),
            );
        }
        if (declares && !uses) {
            findings.push(
                finding(
                    "ProfileVocabularyUnused",
                    "warning",
                    `segment declares '${profile}' but uses no ${vocab} vocabulary`,
                    profile,
                    segmentIndex,
                ),
            );
        }
    }
    return findings;
}

function usedVocabs(graph: Graph): Set<string> {
    const out = new Set<string>();
    const vocabs = new Set(PROFILE_VOCABS.values());
    for (const quad of graph.quads) {
        for (const id of quadIds(quad)) {
            const ns = namespace(termIriValue(graph, id));
            if (vocabs.has(ns)) out.add(ns);
        }
    }
    return out;
}

function streamVocabFindings(
    graph: Graph,
    segmentIndex?: number,
): ProfileFinding[] {
    if (graph.segmentStreamable.some((info) => info.claimed)) return [];
    for (const quad of graph.quads) {
        for (const id of quadIds(quad)) {
            if (termIriValue(graph, id).startsWith(STREAM_NS)) {
                return [
                    finding(
                        "StreamVocabularyWithoutLayout",
                        "warning",
                        `segment uses ${STREAM_NS} vocabulary but does not claim ` +
                            "layout 'streamable'",
                        "stream",
                        segmentIndex,
                    ),
                ];
            }
        }
    }
    return [];
}

function quadIds(quad: {
    s: number;
    p: number;
    o: number;
    g?: number;
}): number[] {
    const ids = [quad.s, quad.p, quad.o];
    if (quad.g !== undefined) ids.push(quad.g);
    return ids;
}

function termIriValue(graph: Graph, id: number): string {
    if (id < 0 || id >= graph.terms.length) return "";
    const term = graph.terms[id];
    if (term.kind !== TermKind.Iri) return "";
    return term.value;
}

function namespace(iri: string): string {
    const hash = iri.lastIndexOf("#");
    if (hash >= 0) return iri.slice(0, hash + 1);
    const slash = iri.lastIndexOf("/");
    if (slash >= 0) return iri.slice(0, slash + 1);
    return iri;
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
    if (a.length !== b.length) return false;
    return a.every((value, index) => value === b[index]);
}

function isAsciiHex(ch: string): boolean {
    return /^[0-9a-fA-F]$/.test(ch);
}

function customPseudonymousPatternMatches(
    pattern: string,
    kid: string,
): boolean {
    if (!pattern.startsWith("^") || !pattern.endsWith("$"))
        return kid === pattern;
    const literal = anchoredLiteralPattern(pattern.slice(1, -1));
    return literal !== undefined && kid === literal;
}

function anchoredLiteralPattern(inner: string): string | undefined {
    let literal = "";
    let escaped = false;
    for (const ch of inner) {
        if (escaped) {
            literal += ch;
            escaped = false;
            continue;
        }
        if (ch === "\\") {
            escaped = true;
            continue;
        }
        if (".[\\]{}()*+?|^$".includes(ch)) return undefined;
        literal += ch;
    }
    if (escaped) literal += "\\";
    return literal;
}

# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Trust and profile-policy checks layered above core GTS validity.

The reader verifies bytes, hashes, signatures, and decryptability. This module
keeps deployment trust and profile conformance separate from those mechanics:
``Signature.status == "valid"`` means the COSE signature verified under a
resolved key, not that the signer is authorized or the claim is true.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Literal

from gts.model import Graph, TermKind

Severity = Literal["error", "warning", "info"]

FILES_NS = "https://w3id.org/gts/files#"
PROFILE_VOCABS: dict[str, str] = {"files": FILES_NS}
_DEFAULT_PSEUDONYMOUS_KID = r"^anon:[0-9a-fA-F]{32,}$"


@dataclass(frozen=True)
class TrustPolicy:
    """Deployment trust anchors and high-privacy recipient-id rules.

    ``trusted_signers`` are signer ``kid`` values the caller accepts for
    profile-level authorization. Cryptographic validity is still computed by the
    reader against a :class:`~gts.crypto.KeyProvider`; this policy only says
    whether a valid signer is authorized for the deployment.
    """

    trusted_signers: frozenset[str] = field(default_factory=frozenset)
    require_trusted_signer: bool = False
    pseudonymous_kid_pattern: str = _DEFAULT_PSEUDONYMOUS_KID

    def is_trusted(self, kid: str | None) -> bool:
        """True when ``kid`` is a deployment-trusted signer."""
        return kid is not None and kid in self.trusted_signers

    def is_pseudonymous_recipient(self, kid: str) -> bool:
        """True when ``kid`` satisfies the high-privacy opaque-profile shape."""
        return re.fullmatch(self.pseudonymous_kid_pattern, kid) is not None


@dataclass(frozen=True)
class SignatureTrust:
    """A signature's cryptographic status plus deployment-trust result."""

    frame_id: bytes
    kid: str | None
    status: str
    trusted: bool


@dataclass(frozen=True)
class ProfileFinding:
    """One profile or trust-policy finding."""

    code: str
    severity: Severity
    detail: str
    profile: str | None = None
    segment_index: int | None = None


def signature_trust(
    graph: Graph, policy: TrustPolicy | None = None
) -> list[SignatureTrust]:
    """Evaluate deployment trust for already-verified signature statuses."""
    policy = policy or TrustPolicy()
    return [
        SignatureTrust(
            frame_id=sig.frame_id,
            kid=sig.kid,
            status=sig.status,
            trusted=sig.status == "valid" and policy.is_trusted(sig.kid),
        )
        for sig in graph.signatures
    ]


def evaluate_profile_policy(
    graph: Graph,
    policy: TrustPolicy | None = None,
    *,
    segment_index: int | None = None,
) -> list[ProfileFinding]:
    """Run supported profile checks without changing core reader validity."""
    policy = policy or TrustPolicy()
    findings: list[ProfileFinding] = []
    declared = set(graph.segment_profiles) or {"generic"}

    findings.extend(_profile_vocab_findings(graph, declared, segment_index))
    findings.extend(_stream_vocab_findings(graph, segment_index))

    for profile in sorted(declared):
        if profile in {"evidence", "opaque"}:
            findings.extend(
                _signature_policy_findings(graph, profile, policy, segment_index)
            )
        if profile == "evidence":
            findings.extend(_evidence_head_findings(graph, segment_index))
        if profile == "opaque":
            findings.extend(_opaque_recipient_findings(graph, policy, segment_index))
    return findings


def _signature_policy_findings(
    graph: Graph,
    profile: str,
    policy: TrustPolicy,
    segment_index: int | None,
) -> list[ProfileFinding]:
    findings: list[ProfileFinding] = []
    if not graph.signatures:
        if profile == "evidence" and _has_sealed_source(graph):
            return findings
        return [
            ProfileFinding(
                "ProfileSignatureRequired",
                "error",
                f"profile '{profile}' requires signed frames",
                profile,
                segment_index,
            )
        ]

    invalid = [sig for sig in graph.signatures if sig.status == "invalid"]
    if invalid:
        findings.append(
            ProfileFinding(
                "ProfileSignatureInvalid",
                "error",
                f"profile '{profile}' has {len(invalid)} invalid signature(s)",
                profile,
                segment_index,
            )
        )

    unverified = [sig for sig in graph.signatures if sig.status == "unverified"]
    if unverified:
        findings.append(
            ProfileFinding(
                "ProfileSignatureUnverified",
                "error",
                f"profile '{profile}' has {len(unverified)} unresolved signature(s)",
                profile,
                segment_index,
            )
        )

    trust = signature_trust(graph, policy)
    valid = [sig for sig in trust if sig.status == "valid"]
    trusted = [sig for sig in valid if sig.trusted]
    if policy.require_trusted_signer and not trusted:
        findings.append(
            ProfileFinding(
                "ProfileSignerUntrusted",
                "error",
                f"profile '{profile}' has no deployment-trusted valid signer",
                profile,
                segment_index,
            )
        )
    elif valid and not policy.trusted_signers:
        findings.append(
            ProfileFinding(
                "ProfileSignerTrustNotEvaluated",
                "warning",
                f"profile '{profile}' signatures are cryptographically valid; "
                "no deployment trust policy was supplied",
                profile,
                segment_index,
            )
        )
    return findings


def _evidence_head_findings(
    graph: Graph, segment_index: int | None
) -> list[ProfileFinding]:
    if _has_sealed_source(graph):
        return []
    heads = set(graph.segment_heads)
    signed_heads = {
        sig.frame_id for sig in graph.signatures if sig.status == "valid"
    } or {sig.frame_id for sig in graph.signatures if sig.status == "unverified"}
    if heads and heads.isdisjoint(signed_heads):
        return [
            ProfileFinding(
                "EvidenceHeadCommitmentRequired",
                "error",
                "profile 'evidence' requires a signed segment head commitment",
                "evidence",
                segment_index,
            )
        ]
    return []


def _has_sealed_source(graph: Graph) -> bool:
    from gts.stream import SEALED_SOURCE

    n = len(graph.terms)
    return any(
        0 <= p < n
        and graph.term(p).kind is TermKind.IRI
        and graph.term(p).value == SEALED_SOURCE
        for _s, p, _o, _g in graph.quads
    )


def _opaque_recipient_findings(
    graph: Graph,
    policy: TrustPolicy,
    segment_index: int | None,
) -> list[ProfileFinding]:
    findings: list[ProfileFinding] = []
    for node in graph.opaque:
        for recipient in node.recipients or []:
            kid = recipient.get("kid")
            if not isinstance(kid, str):
                findings.append(
                    ProfileFinding(
                        "OpaqueRecipientKidMissing",
                        "error",
                        "opaque-profile recipient lacks a string kid",
                        "opaque",
                        segment_index,
                    )
                )
            elif not policy.is_pseudonymous_recipient(kid):
                findings.append(
                    ProfileFinding(
                        "OpaqueRecipientKidPublic",
                        "error",
                        "opaque-profile high-privacy recipient kid must match "
                        f"{policy.pseudonymous_kid_pattern!r}, got {kid!r}",
                        "opaque",
                        segment_index,
                    )
                )
    return findings


def _namespace(iri: str) -> str:
    if "#" in iri:
        return iri[: iri.rfind("#") + 1]
    if "/" in iri:
        return iri[: iri.rfind("/") + 1]
    return iri


def _used_vocabs(graph: Graph) -> set[str]:
    out: set[str] = set()
    vocabs = set(PROFILE_VOCABS.values())
    n = len(graph.terms)
    for s, p, o, g in graph.quads:
        refs = (s, p, o) if g is None else (s, p, o, g)
        for tid in refs:
            if not (0 <= tid < n):
                continue
            term = graph.term(tid)
            if term.kind is TermKind.IRI and term.value:
                ns = _namespace(term.value)
                if ns in vocabs:
                    out.add(ns)
    return out


def _profile_vocab_findings(
    graph: Graph, declared: set[str], segment_index: int | None
) -> list[ProfileFinding]:
    findings: list[ProfileFinding] = []
    used = _used_vocabs(graph)
    for profile, vocab in PROFILE_VOCABS.items():
        declares = profile in declared
        uses = vocab in used
        if uses and not declares:
            findings.append(
                ProfileFinding(
                    "ProfileVocabularyUndeclared",
                    "error",
                    f"segment uses {vocab} vocabulary but does not declare '{profile}'",
                    profile,
                    segment_index,
                )
            )
        if declares and not uses:
            findings.append(
                ProfileFinding(
                    "ProfileVocabularyUnused",
                    "warning",
                    f"segment declares '{profile}' but uses no {vocab} vocabulary",
                    profile,
                    segment_index,
                )
            )
    return findings


def _stream_vocab_findings(
    graph: Graph, segment_index: int | None
) -> list[ProfileFinding]:
    from gts.stream import STREAM_NS

    claimed = bool(graph.segment_streamable and graph.segment_streamable[0].claimed)
    if claimed:
        return []
    n = len(graph.terms)
    uses = any(
        term.kind is TermKind.IRI
        and term.value is not None
        and term.value.startswith(STREAM_NS)
        for s, p, o, g in graph.quads
        for tid in ((s, p, o) if g is None else (s, p, o, g))
        if 0 <= tid < n
        for term in (graph.term(tid),)
    )
    if not uses:
        return []
    return [
        ProfileFinding(
            "StreamVocabularyWithoutLayout",
            "warning",
            f"segment uses {STREAM_NS} vocabulary but does not claim layout "
            "'streamable' (§13.3)",
            "stream",
            segment_index,
        )
    ]

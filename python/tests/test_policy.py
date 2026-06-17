# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Trust-policy and profile-policy checks stay above cryptographic validity."""

from __future__ import annotations

import json
import os
from pathlib import Path

from gts import InMemoryKeys, Signer, Term, TermKind, Writer, read
from gts.policy import TrustPolicy, evaluate_profile_policy, signature_trust

EX = "https://example.org/"
VECTORS_DIR = Path(__file__).resolve().parents[2] / "vectors" / "security"


def _claim_terms() -> list[Term]:
    return [
        Term(TermKind.IRI, EX + "claim"),
        Term(TermKind.IRI, EX + "says"),
        Term(TermKind.LITERAL, "the moon is made of cheese"),
    ]


def _signed_graph(profile: str = "evidence") -> tuple[bytes, Signer, InMemoryKeys]:
    signer = Signer.generate("did:example:issuer")
    keys = InMemoryKeys()
    keys.trust(signer)
    writer = Writer(profile=profile, signer=signer)
    writer.add_terms(_claim_terms())
    writer.add_quads([(0, 1, 2, None)])
    return writer.to_bytes(), signer, keys


def test_valid_signature_does_not_imply_trusted_signer_or_true_claim() -> None:
    data, signer, keys = _signed_graph()
    graph = read(data, keys=keys)

    assert {sig.status for sig in graph.signatures} == {"valid"}
    assert graph.quads == [(0, 1, 2, None)]
    assert [item.trusted for item in signature_trust(graph)] == [False, False]

    findings = evaluate_profile_policy(graph)
    assert any(f.code == "ProfileSignerTrustNotEvaluated" for f in findings)

    trusted = TrustPolicy(
        trusted_signers=frozenset({signer.kid}), require_trusted_signer=True
    )
    assert all(item.trusted for item in signature_trust(graph, trusted))
    assert not any(
        f.severity == "error" for f in evaluate_profile_policy(graph, trusted)
    )


def test_evidence_profile_requires_signatures_and_head_commitment() -> None:
    writer = Writer(profile="evidence")
    writer.add_terms(_claim_terms())
    writer.add_quads([(0, 1, 2, None)])

    findings = evaluate_profile_policy(read(writer.to_bytes()))
    assert {f.code for f in findings if f.severity == "error"} >= {
        "ProfileSignatureRequired",
        "EvidenceHeadCommitmentRequired",
    }


def test_trusted_signer_policy_can_reject_valid_but_unauthorized_signer() -> None:
    data, _signer, keys = _signed_graph()
    graph = read(data, keys=keys)

    findings = evaluate_profile_policy(
        graph,
        TrustPolicy(
            trusted_signers=frozenset({"did:example:someone-else"}),
            require_trusted_signer=True,
        ),
    )
    assert any(f.code == "ProfileSignerUntrusted" for f in findings)


def test_opaque_profile_requires_pseudonymous_recipient_kids() -> None:
    data, signer, verifier = _signed_graph(profile="opaque")
    # Add one encrypted frame with a public, stable recipient id.
    writer = Writer(profile="opaque", signer=signer)
    writer.add_frame(
        "meta", payload={"sealed": True}, encrypt=("did:court", os.urandom(32))
    )
    graph = read(data + writer.to_bytes(), keys=verifier)

    findings = evaluate_profile_policy(graph)
    assert any(f.code == "OpaqueRecipientKidPublic" for f in findings)


def test_opaque_profile_accepts_pseudonymous_recipient_and_records_sigstat() -> None:
    signer = Signer.generate("did:example:notary")
    verifier = InMemoryKeys(verifiers={signer.kid: signer.key.public_key()})
    writer = Writer(profile="opaque", signer=signer)
    writer.add_frame(
        "meta",
        payload={"sealed": True},
        encrypt=("anon:" + "a" * 32, os.urandom(32)),
    )
    graph = read(writer.to_bytes(), keys=verifier)

    assert graph.opaque
    assert graph.opaque[0].sigstat == "valid"
    findings = evaluate_profile_policy(graph)
    assert not any(f.code == "OpaqueRecipientKidPublic" for f in findings)


def test_profile_policy_security_vector_descriptor() -> None:
    vector = json.loads((VECTORS_DIR / "profile-policy.json").read_text())
    assert vector["id"] == "profile-policy"
    assert "OpaqueRecipientKidPublic" in vector["expected_findings"]

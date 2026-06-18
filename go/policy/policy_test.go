// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package policy

import (
	"crypto/ed25519"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"go.blackcatinformatics.ca/gts/cose"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/stream"
	"go.blackcatinformatics.ca/gts/writer"
)

const (
	ex  = "https://example.org/"
	kid = "did:example:issuer"
)

type profileSecurityVector struct {
	ID               string   `json:"id"`
	ExpectedFindings []string `json:"expected_findings"`
}

func iri(value string) model.Term {
	return model.Term{Kind: model.Iri, Value: value}
}

func lit(value string) model.Term {
	return model.Term{Kind: model.Literal, Value: value}
}

func claimTerms() []model.Term {
	return []model.Term{
		iri(ex + "claim"),
		iri(ex + "says"),
		lit("the moon is made of cheese"),
	}
}

func signedGraph(profile string) *model.Graph {
	priv := ed25519.NewKeyFromSeed([]byte{
		3, 3, 3, 3, 3, 3, 3, 3,
		3, 3, 3, 3, 3, 3, 3, 3,
		3, 3, 3, 3, 3, 3, 3, 3,
		3, 3, 3, 3, 3, 3, 3, 3,
	})
	pub := priv.Public().(ed25519.PublicKey)
	w := writer.New(profile)
	w.SignWith(priv, kid)
	w.AddTerms(claimTerms())
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	graph := reader.Read(w.ToBytes(), false, nil)
	cose.VerifySignatures(graph.Signatures, func(candidate string) (ed25519.PublicKey, bool) {
		if candidate == kid {
			return pub, true
		}
		return nil, false
	})
	return graph
}

func findingCodes(findings []ProfileFinding) map[string]struct{} {
	out := map[string]struct{}{}
	for _, finding := range findings {
		out[finding.Code] = struct{}{}
	}
	return out
}

func hasFinding(findings []ProfileFinding, code string) bool {
	_, ok := findingCodes(findings)[code]
	return ok
}

func TestValidSignatureDoesNotImplyTrustedSignerOrTrueClaim(t *testing.T) {
	graph := signedGraph("evidence")
	for _, sig := range graph.Signatures {
		if sig.Status != "valid" {
			t.Fatalf("expected valid signature, got %q", sig.Status)
		}
	}
	for _, item := range SignatureTrustForGraph(graph, nil) {
		if item.Trusted {
			t.Fatalf("default policy should not trust signer %q", item.Kid)
		}
	}
	if !hasFinding(EvaluateProfilePolicy(graph, nil, nil), "ProfileSignerTrustNotEvaluated") {
		t.Fatalf("missing ProfileSignerTrustNotEvaluated")
	}
	trusted := NewTrustPolicy([]string{kid}, true)
	for _, item := range SignatureTrustForGraph(graph, trusted) {
		if !item.Trusted {
			t.Fatalf("trusted policy should trust signer %q", item.Kid)
		}
	}
	for _, finding := range EvaluateProfilePolicy(graph, trusted, nil) {
		if finding.Severity == SeverityError {
			t.Fatalf("trusted graph should not have policy error %s", finding.Code)
		}
	}
}

func TestPolicyDefaultsDoNotMutateInput(t *testing.T) {
	policy := &TrustPolicy{}
	graph := &model.Graph{SegmentProfiles: []string{"opaque"}}

	EvaluateProfilePolicy(graph, policy, nil)
	SignatureTrustForGraph(graph, policy)

	if policy.TrustedSigners != nil {
		t.Fatalf("policy defaulting should not mutate TrustedSigners")
	}
	if policy.PseudonymousKidPattern != "" {
		t.Fatalf("policy defaulting should not mutate PseudonymousKidPattern")
	}
}

func TestEvidenceProfileRequiresSignaturesAndHeadCommitment(t *testing.T) {
	w := writer.New("evidence")
	w.AddTerms(claimTerms())
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})

	findings := EvaluateProfilePolicy(reader.Read(w.ToBytes(), false, nil), nil, nil)
	if !hasFinding(findings, "ProfileSignatureRequired") {
		t.Fatalf("missing ProfileSignatureRequired")
	}
	if !hasFinding(findings, "EvidenceHeadCommitmentRequired") {
		t.Fatalf("missing EvidenceHeadCommitmentRequired")
	}
}

func TestOpaqueProfileRequiresPseudonymousRecipientKids(t *testing.T) {
	graph := &model.Graph{SegmentProfiles: []string{"opaque"}}
	graph.Opaque = append(graph.Opaque, model.OpaqueNode{
		FrameType:  "meta",
		Reason:     "missing-key",
		SigStat:    "unverified",
		Recipients: []interface{}{map[interface{}]interface{}{"kid": "did:court"}},
	})

	if !hasFinding(EvaluateProfilePolicy(graph, nil, nil), "OpaqueRecipientKidPublic") {
		t.Fatalf("missing OpaqueRecipientKidPublic")
	}

	graph.Opaque[0].Recipients = []interface{}{
		map[interface{}]interface{}{"kid": "anon:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
	}
	if hasFinding(EvaluateProfilePolicy(graph, nil, nil), "OpaqueRecipientKidPublic") {
		t.Fatalf("pseudonymous recipient should be accepted")
	}

	custom := DefaultTrustPolicy()
	custom.PseudonymousKidPattern = "^did:court$"
	graph.Opaque[0].Recipients = []interface{}{map[interface{}]interface{}{"kid": "did:court"}}
	if hasFinding(EvaluateProfilePolicy(graph, custom, nil), "OpaqueRecipientKidPublic") {
		t.Fatalf("custom anchored literal recipient pattern should be accepted")
	}
}

func TestProfileAndStreamVocabularyFindings(t *testing.T) {
	files := writer.New("generic")
	files.AddTerms([]model.Term{
		iri(FilesNS + "FileEntry"),
		iri(ex + "relatedTo"),
		lit("x.txt"),
	})
	files.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	if !hasFinding(
		EvaluateProfilePolicy(reader.Read(files.ToBytes(), false, nil), nil, nil),
		"ProfileVocabularyUndeclared",
	) {
		t.Fatalf("missing ProfileVocabularyUndeclared")
	}

	streamGraph := writer.New("generic")
	streamGraph.AddTerms([]model.Term{
		iri(ex + "rewrite"),
		iri(stream.Compaction),
		lit("agent"),
	})
	streamGraph.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	if !hasFinding(
		EvaluateProfilePolicy(reader.Read(streamGraph.ToBytes(), false, nil), nil, nil),
		"StreamVocabularyWithoutLayout",
	) {
		t.Fatalf("missing StreamVocabularyWithoutLayout")
	}
}

func TestProfilePolicySecurityVector(t *testing.T) {
	path := filepath.Join("..", "..", "vectors", "security", "profile-policy.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var vector profileSecurityVector
	if err := json.Unmarshal(data, &vector); err != nil {
		t.Fatal(err)
	}

	seen := map[string]struct{}{}
	for _, finding := range EvaluateProfilePolicy(signedGraph("evidence"), nil, nil) {
		seen[finding.Code] = struct{}{}
	}
	for _, finding := range EvaluateProfilePolicy(
		signedGraph("evidence"),
		NewTrustPolicy([]string{"did:example:someone-else"}, true),
		nil,
	) {
		seen[finding.Code] = struct{}{}
	}
	opaqueGraph := &model.Graph{SegmentProfiles: []string{"opaque"}}
	opaqueGraph.Opaque = append(opaqueGraph.Opaque, model.OpaqueNode{
		FrameType:  "meta",
		Reason:     "missing-key",
		Recipients: []interface{}{map[interface{}]interface{}{"kid": "did:court"}},
	})
	for _, finding := range EvaluateProfilePolicy(opaqueGraph, nil, nil) {
		seen[finding.Code] = struct{}{}
	}

	for _, code := range vector.ExpectedFindings {
		if _, ok := seen[code]; !ok {
			t.Fatalf("missing expected security-vector finding %q", code)
		}
	}
}

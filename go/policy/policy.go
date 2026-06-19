// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package policy implements deployment trust and profile-policy diagnostics.
//
// Core reader validity is deliberately separate from these checks:
// cryptographic signature status says whether bytes verified under a resolved
// key, while this package decides whether that signer is trusted for a
// deployment and whether profile-specific requirements are satisfied.
package policy

import (
	"bytes"
	"fmt"
	"strings"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/stream"
	"go.blackcatinformatics.ca/gts/wire"
)

// FilesNS is the vocabulary namespace used by the optional-standard files profile.
const FilesNS = "https://w3id.org/gts/files#"

// DefaultPseudonymousKidPattern is the documented high-privacy recipient kid shape.
const DefaultPseudonymousKidPattern = "^anon:[0-9a-fA-F]{32,}$"

var profileVocabs = map[string]string{"files": FilesNS}

// TrustPolicy holds deployment trust anchors and high-privacy recipient-id rules.
//
// TrustedSigners are signer kid values accepted by the deployment. Cryptographic
// validity is still computed by COSE verification before this policy runs.
type TrustPolicy struct {
	// TrustedSigners are signer kid values accepted by the deployment.
	TrustedSigners map[string]struct{}
	// RequireTrustedSigner upgrades "valid but untrusted" evidence/opaque files to errors.
	RequireTrustedSigner bool
	// PseudonymousKidPattern is the high-privacy recipient-id rule for opaque profiles.
	PseudonymousKidPattern string
}

// NewTrustPolicy builds a deployment policy from trusted signer ids.
func NewTrustPolicy(trustedSigners []string, requireTrustedSigner bool) *TrustPolicy {
	policy := DefaultTrustPolicy()
	policy.RequireTrustedSigner = requireTrustedSigner
	for _, signer := range trustedSigners {
		policy.TrustedSigners[signer] = struct{}{}
	}
	return policy
}

// DefaultTrustPolicy returns the default deployment policy.
func DefaultTrustPolicy() *TrustPolicy {
	return &TrustPolicy{
		TrustedSigners:         map[string]struct{}{},
		PseudonymousKidPattern: DefaultPseudonymousKidPattern,
	}
}

// IsTrusted reports whether kid is a deployment-trusted signer.
func (p *TrustPolicy) IsTrusted(kid string) bool {
	if p == nil || kid == "" {
		return false
	}
	_, ok := p.TrustedSigners[kid]
	return ok
}

// IsPseudonymousRecipient reports whether kid satisfies the opaque-profile rule.
func (p *TrustPolicy) IsPseudonymousRecipient(kid string) bool {
	policy := p
	if policy == nil {
		policy = DefaultTrustPolicy()
	}
	if policy.PseudonymousKidPattern != DefaultPseudonymousKidPattern {
		return customPseudonymousPatternMatches(policy.PseudonymousKidPattern, kid)
	}
	hex, ok := strings.CutPrefix(kid, "anon:")
	if !ok || len(hex) < 32 {
		return false
	}
	for _, ch := range hex {
		if !isASCIIHex(ch) {
			return false
		}
	}
	return true
}

// SignatureTrust is a signature's cryptographic status plus deployment-trust result.
type SignatureTrust struct {
	// FrameID is the signed frame id.
	FrameID []byte
	// Kid is the resolved signer key id, when present.
	Kid string
	// Status is the reader's cryptographic status.
	Status string
	// Trusted says whether Status is valid and Kid is deployment-trusted.
	Trusted bool
}

// Severity is the profile-policy finding severity.
type Severity string

const (
	// SeverityError means a profile-aware publication or verification tool should fail.
	SeverityError Severity = "error"
	// SeverityWarning means the graph is readable, but the profile signal is incomplete.
	SeverityWarning Severity = "warning"
	// SeverityInfo is an informational result.
	SeverityInfo Severity = "info"
)

// ProfileFinding is one profile or trust-policy finding.
type ProfileFinding struct {
	// Code is the stable machine-readable finding code.
	Code string
	// Severity is error, warning, or info.
	Severity Severity
	// Detail is the human-readable finding text.
	Detail string
	// Profile is the profile that triggered the finding, when applicable.
	Profile string
	// SegmentIndex is set for segment-scoped profile checks.
	SegmentIndex *int
}

// SignatureTrustForGraph evaluates deployment trust for already-verified signatures.
func SignatureTrustForGraph(graph *model.Graph, policy *TrustPolicy) []SignatureTrust {
	policy = policyOrDefault(policy)
	out := make([]SignatureTrust, 0, len(graph.Signatures))
	for _, sig := range graph.Signatures {
		out = append(out, SignatureTrust{
			FrameID: sig.FrameID,
			Kid:     sig.Kid,
			Status:  sig.Status,
			Trusted: sig.Status == "valid" && policy.IsTrusted(sig.Kid),
		})
	}
	return out
}

// EvaluateProfilePolicy runs supported profile checks without changing core reader validity.
func EvaluateProfilePolicy(
	graph *model.Graph,
	policy *TrustPolicy,
	segmentIndex *int,
) []ProfileFinding {
	policy = policyOrDefault(policy)
	declared := declaredProfiles(graph)
	findings := []ProfileFinding{}
	findings = append(findings, profileVocabFindings(graph, declared, segmentIndex)...)
	findings = append(findings, streamVocabFindings(graph, segmentIndex)...)
	for _, profile := range sortedProfiles(declared) {
		if profile == "evidence" || profile == "opaque" {
			findings = append(
				findings,
				signaturePolicyFindings(graph, profile, policy, segmentIndex)...,
			)
		}
		if profile == "evidence" {
			findings = append(findings, evidenceHeadFindings(graph, segmentIndex)...)
		}
		if profile == "opaque" {
			findings = append(findings, opaqueRecipientFindings(graph, policy, segmentIndex)...)
		}
	}
	return findings
}

func policyOrDefault(policy *TrustPolicy) *TrustPolicy {
	if policy == nil {
		return DefaultTrustPolicy()
	}
	trustedSigners := map[string]struct{}{}
	for signer := range policy.TrustedSigners {
		trustedSigners[signer] = struct{}{}
	}
	pseudonymousKidPattern := policy.PseudonymousKidPattern
	if pseudonymousKidPattern == "" {
		pseudonymousKidPattern = DefaultPseudonymousKidPattern
	}
	return &TrustPolicy{
		TrustedSigners:         trustedSigners,
		RequireTrustedSigner:   policy.RequireTrustedSigner,
		PseudonymousKidPattern: pseudonymousKidPattern,
	}
}

func declaredProfiles(graph *model.Graph) map[string]struct{} {
	if len(graph.SegmentProfiles) == 0 {
		return map[string]struct{}{"generic": {}}
	}
	out := make(map[string]struct{}, len(graph.SegmentProfiles))
	for _, profile := range graph.SegmentProfiles {
		out[profile] = struct{}{}
	}
	return out
}

func sortedProfiles(profiles map[string]struct{}) []string {
	out := make([]string, 0, len(profiles))
	for profile := range profiles {
		out = append(out, profile)
	}
	for i := 1; i < len(out); i++ {
		for j := i; j > 0 && out[j] < out[j-1]; j-- {
			out[j], out[j-1] = out[j-1], out[j]
		}
	}
	return out
}

func finding(code string, severity Severity, detail string, profile string, segmentIndex *int) ProfileFinding {
	return ProfileFinding{
		Code:         code,
		Severity:     severity,
		Detail:       detail,
		Profile:      profile,
		SegmentIndex: segmentIndex,
	}
}

func signaturePolicyFindings(
	graph *model.Graph,
	profile string,
	policy *TrustPolicy,
	segmentIndex *int,
) []ProfileFinding {
	if len(graph.Signatures) == 0 {
		if profile == "evidence" && hasSealedSource(graph) {
			return nil
		}
		return []ProfileFinding{finding(
			"ProfileSignatureRequired",
			SeverityError,
			fmt.Sprintf("profile %q requires signed frames", profile),
			profile,
			segmentIndex,
		)}
	}
	findings := []ProfileFinding{}
	invalid := 0
	unverified := 0
	for _, sig := range graph.Signatures {
		if sig.Status == "invalid" {
			invalid++
		}
		if sig.Status == "unverified" {
			unverified++
		}
	}
	if invalid > 0 {
		findings = append(findings, finding(
			"ProfileSignatureInvalid",
			SeverityError,
			fmt.Sprintf("profile %q has %d invalid signature(s)", profile, invalid),
			profile,
			segmentIndex,
		))
	}
	if unverified > 0 {
		findings = append(findings, finding(
			"ProfileSignatureUnverified",
			SeverityError,
			fmt.Sprintf("profile %q has %d unresolved signature(s)", profile, unverified),
			profile,
			segmentIndex,
		))
	}
	trust := SignatureTrustForGraph(graph, policy)
	valid := 0
	trusted := 0
	for _, sig := range trust {
		if sig.Status == "valid" {
			valid++
			if sig.Trusted {
				trusted++
			}
		}
	}
	if policy.RequireTrustedSigner && trusted == 0 {
		findings = append(findings, finding(
			"ProfileSignerUntrusted",
			SeverityError,
			fmt.Sprintf("profile %q has no deployment-trusted valid signer", profile),
			profile,
			segmentIndex,
		))
	} else if valid > 0 && len(policy.TrustedSigners) == 0 {
		findings = append(findings, finding(
			"ProfileSignerTrustNotEvaluated",
			SeverityWarning,
			fmt.Sprintf("profile %q signatures are cryptographically valid; no deployment trust policy was supplied", profile),
			profile,
			segmentIndex,
		))
	}
	return findings
}

func evidenceHeadFindings(graph *model.Graph, segmentIndex *int) []ProfileFinding {
	if hasSealedSource(graph) {
		return nil
	}
	if len(graph.SegmentHeads) == 0 {
		return nil
	}
	heads := signedHeads(graph.Signatures, "valid")
	if len(heads) == 0 {
		heads = signedHeads(graph.Signatures, "unverified")
	}
	if len(heads) == 0 {
		return []ProfileFinding{finding(
			"EvidenceHeadCommitmentRequired",
			SeverityError,
			"profile 'evidence' requires a signed segment head commitment",
			"evidence",
			segmentIndex,
		)}
	}
	for _, head := range graph.SegmentHeads {
		for _, signed := range heads {
			if bytes.Equal(head, signed) {
				return nil
			}
		}
	}
	return []ProfileFinding{finding(
		"EvidenceHeadCommitmentRequired",
		SeverityError,
		"profile 'evidence' requires a signed segment head commitment",
		"evidence",
		segmentIndex,
	)}
}

func signedHeads(signatures []model.Signature, status string) [][]byte {
	out := [][]byte{}
	for _, sig := range signatures {
		if sig.Status == status {
			out = append(out, sig.FrameID)
		}
	}
	return out
}

func hasSealedSource(graph *model.Graph) bool {
	for _, quad := range graph.Quads {
		if termIRIValue(graph, quad.P) == stream.SealedSource {
			return true
		}
	}
	return false
}

func opaqueRecipientFindings(
	graph *model.Graph,
	policy *TrustPolicy,
	segmentIndex *int,
) []ProfileFinding {
	findings := []ProfileFinding{}
	for _, node := range graph.Opaque {
		for _, recipient := range node.Recipients {
			entries, ok := recipient.(map[interface{}]interface{})
			if !ok {
				findings = append(findings, finding(
					"OpaqueRecipientKidMissing",
					SeverityError,
					"opaque-profile recipient lacks a string kid",
					"opaque",
					segmentIndex,
				))
				continue
			}
			kidRaw, ok := wire.MapGet(entries, "kid")
			kid, kidOK := wire.AsText(kidRaw)
			if !ok || !kidOK {
				findings = append(findings, finding(
					"OpaqueRecipientKidMissing",
					SeverityError,
					"opaque-profile recipient lacks a string kid",
					"opaque",
					segmentIndex,
				))
				continue
			}
			if !policy.IsPseudonymousRecipient(kid) {
				findings = append(findings, finding(
					"OpaqueRecipientKidPublic",
					SeverityError,
					fmt.Sprintf(
						"opaque-profile high-privacy recipient kid must match %q, got %q",
						policy.PseudonymousKidPattern,
						kid,
					),
					"opaque",
					segmentIndex,
				))
			}
		}
	}
	return findings
}

func termIRIValue(graph *model.Graph, termID int) string {
	if termID < 0 || termID >= len(graph.Terms) {
		return ""
	}
	term := graph.Terms[termID]
	if term.Kind != model.Iri {
		return ""
	}
	return term.Value
}

func usedVocabs(graph *model.Graph) map[string]struct{} {
	out := map[string]struct{}{}
	for _, quad := range graph.Quads {
		ids := []int{quad.S, quad.P, quad.O}
		if quad.G != nil {
			ids = append(ids, *quad.G)
		}
		for _, id := range ids {
			ns := namespace(termIRIValue(graph, id))
			for _, vocab := range profileVocabs {
				if ns == vocab {
					out[vocab] = struct{}{}
				}
			}
		}
	}
	return out
}

func profileVocabFindings(
	graph *model.Graph,
	declared map[string]struct{},
	segmentIndex *int,
) []ProfileFinding {
	used := usedVocabs(graph)
	findings := []ProfileFinding{}
	for profile, vocab := range profileVocabs {
		_, declares := declared[profile]
		_, uses := used[vocab]
		if uses && !declares {
			findings = append(findings, finding(
				"ProfileVocabularyUndeclared",
				SeverityError,
				fmt.Sprintf("segment uses %s vocabulary but does not declare %q", vocab, profile),
				profile,
				segmentIndex,
			))
		}
		if declares && !uses {
			findings = append(findings, finding(
				"ProfileVocabularyUnused",
				SeverityWarning,
				fmt.Sprintf("segment declares %q but uses no %s vocabulary", profile, vocab),
				profile,
				segmentIndex,
			))
		}
	}
	return findings
}

func streamVocabFindings(graph *model.Graph, segmentIndex *int) []ProfileFinding {
	for _, info := range graph.SegmentStreamable {
		if info.Claimed {
			return nil
		}
	}
	for _, quad := range graph.Quads {
		ids := []int{quad.S, quad.P, quad.O}
		if quad.G != nil {
			ids = append(ids, *quad.G)
		}
		for _, id := range ids {
			if strings.HasPrefix(termIRIValue(graph, id), stream.NS) {
				return []ProfileFinding{finding(
					"StreamVocabularyWithoutLayout",
					SeverityWarning,
					fmt.Sprintf("segment uses %s vocabulary but does not claim layout 'streamable'", stream.NS),
					"stream",
					segmentIndex,
				)}
			}
		}
	}
	return nil
}

func namespace(iri string) string {
	if i := strings.LastIndex(iri, "#"); i >= 0 {
		return iri[:i+1]
	}
	if i := strings.LastIndex(iri, "/"); i >= 0 {
		return iri[:i+1]
	}
	return iri
}

func isASCIIHex(ch rune) bool {
	return (ch >= '0' && ch <= '9') ||
		(ch >= 'a' && ch <= 'f') ||
		(ch >= 'A' && ch <= 'F')
}

func customPseudonymousPatternMatches(pattern string, kid string) bool {
	inner, ok := strings.CutPrefix(pattern, "^")
	if !ok {
		return kid == pattern
	}
	inner, ok = strings.CutSuffix(inner, "$")
	if !ok {
		return kid == pattern
	}
	literal, ok := anchoredLiteralPattern(inner)
	return ok && kid == literal
}

func anchoredLiteralPattern(inner string) (string, bool) {
	var b strings.Builder
	escaped := false
	for _, ch := range inner {
		if escaped {
			b.WriteRune(ch)
			escaped = false
			continue
		}
		if ch == '\\' {
			escaped = true
			continue
		}
		switch ch {
		case '.', '[', ']', '{', '}', '(', ')', '*', '+', '?', '|', '^', '$':
			return "", false
		default:
			b.WriteRune(ch)
		}
	}
	if escaped {
		b.WriteRune('\\')
	}
	return b.String(), true
}

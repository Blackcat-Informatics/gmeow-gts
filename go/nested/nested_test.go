// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package nested

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
	"go.blackcatinformatics.ca/gts/writer"
)

const ex = "https://example.org/"

type nestedSecurityVector struct {
	ID                  string   `json:"id"`
	MaxDepth            int      `json:"max_depth"`
	ExpectedDiagnostics []string `json:"expected_diagnostics"`
}

func iri(value string) model.Term {
	return model.Term{Kind: model.Iri, Value: value}
}

func lit(value string) model.Term {
	return model.Term{Kind: model.Literal, Value: value}
}

func tinyGraph(label string) []byte {
	w := writer.New("dist")
	w.AddTerms([]model.Term{
		iri(ex + label),
		iri(ex + "label"),
		lit(label),
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	return w.ToBytes()
}

func bundle(child []byte) []byte {
	w := writer.New("bundle")
	w.AddBlob(child, GTSMediaType, "")
	return w.ToBytes()
}

func labeledBundle(child []byte, label string) []byte {
	w := writer.New("bundle")
	w.AddTerms([]model.Term{
		iri(ex + label),
		iri(ex + "label"),
		lit(label),
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	w.AddBlob(child, GTSMediaType, "")
	return w.ToBytes()
}

func hasDiagnostic(diags []model.Diagnostic, code string) bool {
	for _, diag := range diags {
		if diag.Code == code {
			return true
		}
	}
	return false
}

func loadNestedSecurityVector(t *testing.T) nestedSecurityVector {
	t.Helper()
	path := filepath.Join("..", "..", "vectors", "security", "nested-recursion-limit.json")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var vector nestedSecurityVector
	if err := json.Unmarshal(data, &vector); err != nil {
		t.Fatal(err)
	}
	return vector
}

func TestReadNestedExposesSubgraphByBlobDigest(t *testing.T) {
	child := tinyGraph("child")
	outer := bundle(child)

	result := ReadNested(outer, 3, 16*1024*1024)

	subgraph, ok := result.Subgraph(wire.DigestStr(child))
	if !ok {
		t.Fatalf("missing nested subgraph")
	}
	if len(subgraph.Quads) != 1 {
		t.Fatalf("expected one child quad, got %d", len(subgraph.Quads))
	}
	if hasDiagnostic(result.Diagnostics, "RecursionLimit") {
		t.Fatalf("unexpected RecursionLimit diagnostic")
	}
}

func TestNestedRecursionSecurityVector(t *testing.T) {
	vector := loadNestedSecurityVector(t)
	grandchild := tinyGraph("grandchild")
	child := bundle(grandchild)
	outer := bundle(child)

	result := ReadNested(outer, vector.MaxDepth, 16*1024*1024)

	if _, ok := result.Subgraph(wire.DigestStr(child)); !ok {
		t.Fatalf("first nested child should be exposed")
	}
	if _, ok := result.Subgraph(wire.DigestStr(grandchild)); ok {
		t.Fatalf("grandchild should be blocked by max depth %d", vector.MaxDepth)
	}
	for _, code := range vector.ExpectedDiagnostics {
		if !hasDiagnostic(result.Diagnostics, code) {
			t.Fatalf("missing expected diagnostic %q", code)
		}
	}
}

func TestReadNestedStopsAtDecodedSizeBudget(t *testing.T) {
	child := tinyGraph("oversized")
	outer := bundle(child)

	result := ReadNested(outer, 3, len(child)-1)

	if _, ok := result.Subgraph(wire.DigestStr(child)); ok {
		t.Fatalf("oversized child should not be exposed")
	}
	if !hasDiagnostic(result.Diagnostics, "RecursionLimit") {
		t.Fatalf("missing RecursionLimit diagnostic")
	}
}

func TestReadNestedSkipsDuplicateDigest(t *testing.T) {
	grandchild := tinyGraph("shared-grandchild")
	childA := labeledBundle(grandchild, "child-a")
	childB := labeledBundle(grandchild, "child-b")
	w := writer.New("bundle")
	w.AddBlob(childA, GTSMediaType, "")
	w.AddBlob(childB, GTSMediaType, "")

	result := ReadNested(w.ToBytes(), 3, len(childA)+len(childB)+len(grandchild))

	if len(result.Subgraphs) != 3 {
		t.Fatalf("expected three subgraphs with one shared grandchild, got %d", len(result.Subgraphs))
	}
	if hasDiagnostic(result.Diagnostics, "RecursionLimit") {
		t.Fatalf("duplicate digest should not consume the decoded-size budget twice")
	}
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package writer

import (
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
)

func TestRoundTripTermsAndQuads(t *testing.T) {
	w := New("generic")
	terms := []model.Term{
		{Kind: model.Iri, Value: "https://example.org/Cat"},
		{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	}
	w.AddTerms(terms)
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})

	data := w.ToBytes()
	g := reader.Read(data, true, nil)
	if len(g.Diagnostics) > 0 {
		t.Fatalf("unexpected diagnostics: %v", g.Diagnostics)
	}
	if len(g.Terms) != 3 {
		t.Fatalf("expected 3 terms, got %d", len(g.Terms))
	}
	if len(g.Quads) != 1 {
		t.Fatalf("expected 1 quad, got %d", len(g.Quads))
	}
	if g.SegmentProfiles[0] != "generic" {
		t.Fatalf("expected profile generic, got %q", g.SegmentProfiles[0])
	}
}

func TestBlobDedupInWriter(t *testing.T) {
	w := New("files")
	terms := []model.Term{
		{Kind: model.Iri, Value: "https://w3id.org/gts/files#FileEntry"},
		{Kind: model.Iri, Value: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"},
		{Kind: model.Bnode, Value: "e0"},
		{Kind: model.Literal, Value: "a.txt"},
	}
	w.AddTerms(terms)
	w.AddQuads([]model.Quad{{S: 2, P: 1, O: 0}})
	payload := []byte("shared")
	w.AddBlob(payload, "text/plain", "")
	w.AddBlob(payload, "text/plain", "")

	data := w.ToBytes()
	g := reader.Read(data, true, nil)
	if len(g.Blobs) != 1 {
		t.Fatalf("expected one blob after dedup in writer, got %d", len(g.Blobs))
	}
}

// §7.3: a triple term serialises its OWN (s, p, o) under the "tt" key, and a
// legacy reifier indirection still serialises under "rf".
func TestTermToWireCarriesSelfDescribingTripleTerm(t *testing.T) {
	reifier := 0
	term := model.Term{
		Kind:    model.Triple,
		Reifier: &reifier,
		Triple:  &model.Triple3{S: 1, P: 2, O: 3},
	}
	entries := termToWire(&term)
	got, ok := entries["tt"].([]interface{})
	if !ok {
		t.Fatalf("wire entry \"tt\" = %#v, want a three-element id array", entries["tt"])
	}
	if len(got) != 3 || got[0] != int64(1) || got[1] != int64(2) || got[2] != int64(3) {
		t.Fatalf("wire entry \"tt\" = %#v, want [1 2 3]", got)
	}
	if entries["rf"] != int64(0) {
		t.Fatalf("wire entry \"rf\" = %#v, want the legacy indirection preserved", entries["rf"])
	}

	// An unreified triple term writes "tt" and nothing else.
	plain := model.Term{Kind: model.Triple, Triple: &model.Triple3{S: 1, P: 2, O: 3}}
	if _, ok := termToWire(&plain)["rf"]; ok {
		t.Fatal("an unreified triple term must not mint a reifier on the wire")
	}
}

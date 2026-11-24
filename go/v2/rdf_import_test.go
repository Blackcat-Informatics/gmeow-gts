// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"slices"
	"strings"
	"testing"

	"github.com/fxamacker/cbor/v2"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/v2"
)

func TestImportRDFWritesQuadsAndEmptyGraph(t *testing.T) {
	datatype := 7
	graph := 6
	language := "EN"
	terms := []v2.RDFTerm{
		{Kind: model.Triple, Triple: &model.Triple3{S: 1, P: 3, O: 2}},
		{Kind: model.Bnode, Value: "opaque\xff"},
		{Kind: model.Literal, Value: "value", Lang: &language, Direction: "rtl"},
		{Kind: model.Iri, Value: "urn:p"},
		{Kind: model.Literal, Value: "plain"},
		{Kind: model.Literal, Value: "plain", Datatype: &datatype},
		{Kind: model.Iri, Value: "urn:graph"},
		{Kind: model.Iri, Value: model.XSDString},
		{Kind: model.Iri, Value: "urn:empty"},
		{Kind: model.Iri, Value: "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"},
	}
	rows := []model.Quad{
		{S: 1, P: 3, O: 4},
		{S: 1, P: 3, O: 5}, // The explicit xsd:string is the same literal.
		{S: 1, P: 9, O: 0, G: &graph},
	}
	data, err := v2.ImportRDF(terms, rows, []int{8, 8})
	if err != nil {
		t.Fatal(err)
	}
	read := v2.ReadWire(data, v2.DefaultReadCapacity())
	if read.Dispatch != v2.SecondGeneration || read.Stop != "" || len(read.Segments) != 1 {
		t.Fatalf("unexpected wire result: %s, %s, %d segments", read.Dispatch, read.Stop, len(read.Segments))
	}
	var frameTypes []uint64
	for _, item := range read.Segments[0].Items[1:] {
		f := item.Frame
		if f == nil || f.Link != v2.Linked || f.Payload.State != v2.Verified || f.Malformed != "" {
			t.Fatalf("invalid frame at byte %d", item.Offset)
		}
		frameTypes = append(frameTypes, f.Type)
		if f.Type == 2 {
			var content []uint64
			if err := cbor.Unmarshal(f.Content, &content); err != nil || !slices.Equal(content, []uint64{2, 2}) {
				t.Fatalf("quad record %v, error %v", content, err)
			}
		}
		if f.Type == 5 {
			var content []uint64
			var column [][]int
			if err := cbor.Unmarshal(f.Content, &content); err != nil || !slices.Equal(content, []uint64{1}) {
				t.Fatalf("empty graph record %v, error %v", content, err)
			}
			if err := cbor.Unmarshal(f.Payload.Decoded, &column); err != nil ||
				len(column) != 1 || len(column[0]) != 1 || column[0][0] < 0 {
				t.Fatalf("empty graph IDs %v, error %v", column, err)
			}
		}
	}
	if !slices.Equal(frameTypes, []uint64{1, 2, 5, 14}) {
		t.Fatalf("frame types %v", frameTypes)
	}
}

func TestImportRDFRejectsInconsistentDatasets(t *testing.T) {
	graph := 3
	terms := []v2.RDFTerm{
		{Kind: model.Iri, Value: "urn:s"},
		{Kind: model.Iri, Value: "urn:p"},
		{Kind: model.Triple, Triple: &model.Triple3{S: 0, P: 1, O: 2}},
		{Kind: model.Iri, Value: "urn:g"},
		{Kind: model.Iri, Value: "urn:g"},
		{Kind: model.Literal, Value: "x"},
	}
	for _, tc := range []struct {
		quad   model.Quad
		empty  []int
		reason string
	}{
		{model.Quad{S: 0, P: 1, O: 2}, nil, "cycle"},
		{model.Quad{S: 0, P: 5, O: 5}, nil, "predicate"},
		{model.Quad{S: 0, P: 1, O: 5, G: &graph}, []int{4}, "also names a graph"},
	} {
		data, err := v2.ImportRDF(terms, []model.Quad{tc.quad}, tc.empty)
		if data != nil || err == nil || !strings.Contains(err.Error(), tc.reason) {
			t.Fatalf("expected %q and no output, got %d bytes and %v", tc.reason, len(data), err)
		}
	}
}

func TestImportRDFPreservesExplicitEmptyLanguage(t *testing.T) {
	emptyLanguage := ""
	terms := []v2.RDFTerm{
		{Kind: model.Iri, Value: "urn:s"},
		{Kind: model.Iri, Value: "urn:p"},
		{Kind: model.Literal, Value: "value", Lang: &emptyLanguage},
	}
	data, err := v2.ImportRDF(terms, []model.Quad{{S: 0, P: 1, O: 2}}, nil)
	if err != nil {
		t.Fatal(err)
	}
	read := v2.ReadWire(data, v2.DefaultReadCapacity())
	var dictionary []cbor.RawMessage
	frame := read.Segments[0].Items[1].Frame
	if err := cbor.Unmarshal(frame.Payload.Decoded, &dictionary); err != nil || len(dictionary) != 2 {
		t.Fatalf("invalid dictionary payload: %v", err)
	}
	var languages []string
	if err := cbor.Unmarshal(dictionary[1], &languages); err != nil || !slices.Equal(languages, []string{""}) {
		t.Fatalf("expected one empty language, got %q, error %v", languages, err)
	}
}

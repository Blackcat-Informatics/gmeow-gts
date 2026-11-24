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

func TestImportRDFWritesSharedLargeLiteralValues(t *testing.T) {
	shared := strings.Repeat("é", 1<<19)
	other := shared + "!"
	inline := strings.Repeat("x", (1<<20)-1)
	datatype, language := 2, "EN"
	terms := []v2.RDFTerm{
		{Kind: model.Iri, Value: "urn:s"},
		{Kind: model.Iri, Value: "urn:p"},
		{Kind: model.Iri, Value: "urn:datatype"},
		{Kind: model.Literal, Value: shared, Datatype: &datatype},
		{Kind: model.Literal, Value: shared, Lang: &language, Direction: "rtl"},
		{Kind: model.Literal, Value: other, Datatype: &datatype},
		{Kind: model.Literal, Value: inline},
	}
	quads := []model.Quad{{S: 0, P: 1, O: 3}, {S: 0, P: 1, O: 4}, {S: 0, P: 1, O: 5}, {S: 0, P: 1, O: 6}}
	data, err := v2.ImportRDF(terms, quads, nil)
	if err != nil {
		t.Fatal(err)
	}
	read := v2.ReadWire(data, v2.DefaultReadCapacity())
	projection, err := read.WholeRDFProjection()
	if err != nil {
		t.Fatal(err)
	}
	var frameTypes []uint64
	storageCounts := make(map[int]int)
	for _, item := range read.Segments[0].Items[1:] {
		frame := item.Frame
		frameTypes = append(frameTypes, frame.Type)
		if frame.Type == 1 {
			var fields []cbor.RawMessage
			if err := cbor.Unmarshal(frame.Content, &fields); err != nil || len(fields) != 4 {
				t.Fatalf("terms content error %v", err)
			}
			var runs [][2]int
			if err := cbor.Unmarshal(fields[1], &runs); err != nil {
				t.Fatal(err)
			}
			for _, run := range runs {
				storageCounts[run[0]] += run[1]
			}
		}
	}
	// One shared full-size unit, its two value declarations, and one tail unit.
	if !slices.Equal(frameTypes, []uint64{8, 11, 8, 11, 1, 2, 14}) {
		t.Fatalf("frame types %v", frameTypes)
	}
	if storageCounts[2] != 1 || storageCounts[5] != 3 {
		t.Fatalf("unexpected literal storage counts %v", storageCounts)
	}

	folded, rows, _ := projection.Dataset()
	if len(rows) != len(quads) {
		t.Fatalf("projected %d quads, want %d", len(rows), len(quads))
	}
	for index, row := range rows {
		got, want := folded[row.O], terms[quads[index].O]
		if got.Value != want.Value || got.Direction != want.Direction {
			t.Fatalf("literal %d changed during round trip", index)
		}
		if want.Lang != nil {
			if got.Lang == nil || *got.Lang != *want.Lang || got.Datatype != nil {
				t.Fatalf("literal %d changed language form", index)
			}
		} else {
			datatypeIRI := model.XSDString
			if want.Datatype != nil {
				datatypeIRI = terms[*want.Datatype].Value
			}
			if got.Datatype == nil || folded[*got.Datatype].Value != datatypeIRI || got.Lang != nil {
				t.Fatalf("literal %d changed datatype", index)
			}
		}
	}
}

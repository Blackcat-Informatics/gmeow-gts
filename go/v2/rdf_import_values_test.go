// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
)

func TestRDFWriterReservesValueBudgetForOversizedLiteral(t *testing.T) {
	segment, err := NewSegmentWriter(SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	inline := strings.Repeat("a", rdfLongLiteralBytes)
	oversized := inline + strings.Repeat("b", 256)
	capacity := DefaultReadCapacity()
	capacity.StoredBytesPerFrame = rdfLongLiteralBytes + 128
	writer := rdfSegmentWriter{
		segment: segment, capacity: capacity, items: 2, valueTextRemaining: uint64(len(oversized)),
	}
	terms := []rdfTerm{
		{kind: model.Iri, text: model.XSDString},
		{kind: model.Literal, text: inline, form: 1},
		{kind: model.Literal, text: oversized, form: 1},
	}
	if err := writer.writeLiteralValues(terms, true); err != nil {
		t.Fatal(err)
	}
	if _, ok := writer.lexicalValues[oversized]; !ok || len(writer.lexicalValues) != 1 {
		t.Fatal("the optional literal used capacity reserved for the oversized literal")
	}
	if err := writer.terms(0, terms); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadWire(segment.Bytes(), capacity).WholeRDFProjectionWithValueLimit(uint64(len(oversized))); err != nil {
		t.Fatal(err)
	}
}

func TestRDFWriterKeepsOptionalLiteralInlineWhenItemsAreLimited(t *testing.T) {
	capacity := DefaultReadCapacity()
	capacity.ItemsPerRead = 4
	text := strings.Repeat("x", rdfLongLiteralBytes)
	terms := []rdfTerm{
		{kind: model.Iri, text: model.XSDString},
		{kind: model.Literal, text: text, form: 1},
	}
	data, err := writeRDFSegment(terms, []rdfQuad{{graph: -1, object: 1}}, nil, capacity, uint64(len(text)), true)
	if err != nil {
		t.Fatal(err)
	}
	read := ReadWire(data, capacity)
	p, err := read.WholeRDFProjectionWithValueLimit(0)
	if err != nil {
		t.Fatal(err)
	}
	folded, rows, _ := p.Dataset()
	if len(rows) != 1 || folded[rows[0].O].Value != text {
		t.Fatal("the reader did not recover the inline literal")
	}
}

func TestRDFWriterKeepsFrontCodedBatchWithinDecodedCapacity(t *testing.T) {
	capacity := DefaultReadCapacity()
	capacity.StoredBytesPerFrame = 128
	capacity.DecodedBytesPerRead = 150
	prefix := "urn:" + strings.Repeat("x", 75)
	terms := []rdfTerm{
		{kind: model.Iri, text: prefix + "a"},
		{kind: model.Iri, text: prefix + "b"},
	}
	data, err := writeRDFSegment(terms, nil, nil, capacity, 0, true)
	if err != nil {
		t.Fatal(err)
	}
	read := ReadWire(data, capacity)
	if _, err := read.WholeRDFProjection(); err != nil {
		t.Fatal(err)
	}
	items := read.Segments[0].Items
	if len(items) != 3 || items[1].Frame.Type != termsFrameType {
		t.Fatal("the front-coded terms must share a frame within the decoded budget")
	}
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package fromnquads_test

import (
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/fromnquads"
	"go.blackcatinformatics.ca/gts/v2"
)

func TestFromNQuadsV2PreservesRDF12Terms(t *testing.T) {
	const reifies = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"
	text := "VERSION \"1.2\"\r\n" +
		"# Comment\r" +
		"<urn:\\u0061> <urn:p> \"\\u00E9\\'#\" @EN--rtl <urn:g>. # Another comment\n" +
		"_:é.a <" + reifies + "> <<( _:é.a <urn:p> <<( <urn:x> <urn:p> \"deep\"@en--ltr )>> )>> _:é.a.\n" +
		"<urn:s> <urn:p> \"same\".\n" +
		"<urn:s> <urn:p> \"same\" ^^ <http://www.w3.org/2001/XMLSchema#string>."
	data, err := fromnquads.ImportV2(text)
	if err != nil {
		t.Fatal(err)
	}
	result := v2.ReadWire(data, v2.DefaultReadCapacity())
	projection, err := result.WholeRDFProjection()
	if err != nil {
		t.Fatal(err)
	}
	output, err := projection.NQuads()
	if err != nil || strings.Count(output, "\n") != 3 ||
		!strings.Contains(output, "<urn:a> <urn:p> \"é'#\"@EN--rtl <urn:g> .") {
		t.Fatalf("projection = %q, error %v", output, err)
	}
	terms, quads, _ := projection.Dataset()
	found := false
	for _, quad := range quads {
		if terms[quad.P].Value != reifies {
			continue
		}
		outer := terms[quad.O].Triple
		if outer == nil || quad.G == nil || *quad.G != quad.S || outer.S != quad.S {
			t.Fatal("reification did not preserve the graph and quoted blank node")
		}
		inner := terms[outer.O].Triple
		if inner == nil || terms[inner.O].Direction != "ltr" {
			t.Fatal("reification did not preserve the nested directional literal")
		}
		found = true
	}
	if !found {
		t.Fatal("missing rdf:reifies statement")
	}
	for _, item := range result.Segments[0].Items {
		if item.Frame != nil && (item.Frame.Type == 3 || item.Frame.Type == 4) {
			t.Fatalf("import rewrote ordinary statements as frame type %d", item.Frame.Type)
		}
	}
}

func TestFromNQuadsV2RefusesInvalidRDF12(t *testing.T) {
	for _, tc := range []struct {
		text, reason string
	}{
		{"# \xff", "valid UTF-8"},
		{"<urn:s> <urn:p> <urn:o>", "expected '.'"},
		{"<urn:s> <urn:p> <urn:o> . .", "unexpected text"},
		{"<relative> <urn:p> <urn:o> .", "invalid RDF IRI"},
		{"<urn:\\t> <urn:p> <urn:o> .", "IRI escape"},
		{"<urn:\\uD800> <urn:p> <urn:o> .", "invalid unicode scalar"},
		{"_:a <urn:p> _:", "invalid blank node label"},
		{"_:-a <urn:p> <urn:o> .", "invalid blank node label"},
		{"<urn:s> _:p <urn:o> .", "predicate must be an IRI"},
		{"<urn:s> <urn:p> \"x\"@en- .", "invalid RDF language tag"},
		{"<urn:s> <urn:p> \"x\"@en--RTL .", "invalid literal direction"},
		{"<urn:s> <urn:p> \"x\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#langString> .", "requires a language"},
		{"<urn:s> <urn:p> <<( <urn:s> <urn:p> <urn:o> .", "expected ')>>'"},
		{"VERSION \"1.2\"@en", "unexpected text"},
	} {
		data, err := fromnquads.ImportV2(tc.text)
		if data != nil || err == nil || !strings.Contains(err.Error(), tc.reason) {
			t.Errorf("%q gave %d bytes and %v, want %q", tc.text, len(data), err, tc.reason)
		}
	}
}

func TestFromNQuadsV2HandlesDeepTripleTerms(t *testing.T) {
	const depth = 16_384
	text := "<urn:s> <urn:p> " + strings.Repeat("<<( <urn:s> <urn:p> ", depth) +
		"<urn:o>" + strings.Repeat(" )>>", depth) + " ."
	data, err := fromnquads.ImportV2(text)
	if err != nil {
		t.Fatal(err)
	}
	projection, err := v2.ReadWire(data, v2.DefaultReadCapacity()).WholeRDFProjection()
	if err != nil {
		t.Fatal(err)
	}
	terms, quads, _ := projection.Dataset()
	if len(quads) != 1 {
		t.Fatalf("got %d quads, want 1", len(quads))
	}
	level, id := 0, quads[0].O
	for terms[id].Triple != nil {
		id = terms[id].Triple.O
		level++
	}
	if level != depth || terms[id].Value != "urn:o" {
		t.Fatalf("object depth %d ends at %q", level, terms[id].Value)
	}
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package rdfcodecs_test

import (
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/fromnquads"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/rdfcodecs"
	"go.blackcatinformatics.ca/gts/v2"
)

func projectRDFText(t *testing.T, importText func(string, string) ([]byte, error), text string) *v2.RDFProjection {
	t.Helper()
	data, err := importText(text, "")
	if err != nil {
		t.Fatal(err)
	}
	result, err := v2.ReadWire(data, v2.DefaultReadCapacity()).WholeRDFProjection()
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func TestTriGV2PreservesGraphStateAndBlankIdentity(t *testing.T) {
	source := "<urn:empty> {} <urn:populated> {}\n" +
		"# The next line has a bare carriage return.\r" +
		"<urn:populated> { <urn:s\\u00A0> <urn:p> \"Cat\"@EN--rtl }\n" +
		"_:gts_0 <urn:own> [ <urn:p> <urn:o> ] .\n"
	projected := projectRDFText(t, rdfcodecs.ImportTriGV2, source)
	text, err := projected.TriG()
	if err != nil || !strings.Contains(text, "<urn:empty> {}") ||
		strings.Contains(text, "<urn:populated> {}") || !strings.Contains(text, "<urn:s\u00a0> <urn:p> \"Cat\"@EN--rtl") {
		t.Fatalf("TriG = %q, error %v", text, err)
	}
	roundTripped := projectRDFText(t, rdfcodecs.ImportTriGV2, text)
	terms, rows, empty := roundTripped.Dataset()
	if len(rows) != 3 || len(empty) != 1 || terms[empty[0]].Value != "urn:empty" {
		t.Fatalf("got %d quads and empty graph IDs %v after round-trip", len(rows), empty)
	}
	found := false
	for _, row := range rows {
		if terms[row.P].Value == "urn:own" {
			found = true
			if row.S == row.O || terms[row.S].Kind != model.Bnode || terms[row.O].Kind != model.Bnode {
				t.Fatal("an explicit label and a generated blank node became the same node")
			}
		}
	}
	if !found {
		t.Fatal("the explicit blank node statement was lost")
	}
}

func TestTriGV2DistinguishesReifiersAndTripleTerms(t *testing.T) {
	input := "@prefix ex: <urn:> . <urn:g> { <urn:x> <urn:q> << ex:s ex:p ex:o~ex:r >> . " +
		"<urn:x> <urn:q> <<( <urn:s> <urn:p> <urn:o> )>> }"
	projected := projectRDFText(t, rdfcodecs.ImportTriGV2, input)
	text, err := projected.NQuads()
	if err != nil || strings.Count(text, "\n") != 3 ||
		!strings.Contains(text, "<urn:x> <urn:q> <urn:r> <urn:g> .") ||
		!strings.Contains(text, "<urn:r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<(") {
		t.Fatalf("N-Quads = %q, error %v", text, err)
	}
}

func TestTriGV2RejectsReifiersInRawTripleTermsAndStandaloneCollections(t *testing.T) {
	for _, input := range []string{
		"<urn:s> <urn:p> <<( <urn:x> <urn:q> << <urn:a> <urn:b> <urn:c> >> )>> .",
		"( <urn:a> ) .",
	} {
		if data, err := rdfcodecs.ImportTriGV2(input, ""); data != nil || err == nil {
			t.Errorf("import of %q returned %d bytes and error %v", input, len(data), err)
		}
	}
}

func TestTriGV2RejectsPropertyListsAsGraphLabels(t *testing.T) {
	input := "GRAPH [ <urn:p> <urn:o> ] {}"
	data, err := rdfcodecs.ImportTriGV2(input, "")
	if data != nil || err == nil || !strings.Contains(err.Error(), "graph block name") {
		t.Fatalf("import returned %d bytes and error %v", len(data), err)
	}
}

func TestRDFXMLV2ProjectsCollectionsAndDirectionalLiterals(t *testing.T) {
	input := "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" " +
		"xmlns:its=\"http://www.w3.org/2005/11/its\" xml:base=\"https://example.org/\">" +
		"<rdf:Description rdf:about=\"s\"><rdf:li xml:lang=\"EN\" its:dir=\"rtl\">word</rdf:li>" +
		"<rdf:li rdf:resource=\"o\"/></rdf:Description></rdf:RDF>"
	projected := projectRDFText(t, rdfcodecs.ImportRDFXMLV2, input)
	xml, err := rdfcodecs.ExportRDFXMLV2(projected)
	if err != nil || !strings.Contains(xml, "its:dir=\"rtl\"") || !strings.Contains(xml, "<rdf:_2") {
		t.Fatalf("RDF/XML = %q, error %v", xml, err)
	}
	roundTripped := projectRDFText(t, rdfcodecs.ImportRDFXMLV2, xml)
	quads, err := roundTripped.NQuads()
	if err != nil || strings.Count(quads, "\n") != 2 ||
		!strings.Contains(quads, "#_1> \"word\"@EN--rtl") || !strings.Contains(quads, "#_2> <https://example.org/o>") {
		t.Fatalf("N-Quads = %q, error %v", quads, err)
	}
}

func TestRDFXMLV2RefusesUnrepresentableText(t *testing.T) {
	for _, tc := range []struct {
		input, reason string
	}{
		{"<urn:s> <urn:p> \"\\u0000\" .", "forbidden XML character"},
		{"<urn:s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#li> <urn:o> .", "property element"},
		{"<urn:s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#foo/bar> <urn:o> .", "XML QName"},
	} {
		projected := projectRDFText(t, func(text, _ string) ([]byte, error) {
			return fromnquads.ImportV2(text)
		}, tc.input)
		xml, err := rdfcodecs.ExportRDFXMLV2(projected)
		if xml != "" || err == nil || !strings.Contains(err.Error(), tc.reason) {
			t.Errorf("RDF/XML = %q, error %v", xml, err)
		}
	}
}

func TestRDFXMLV2PreservesTripleContextAndReifiers(t *testing.T) {
	input := "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" " +
		"xmlns:ex=\"urn:\" xml:base=\"https://example.org/\">" +
		"<rdf:Description rdf:about=\"outer\">" +
		"<ex:p rdf:parseType=\"Triple\"><rdf:Description xml:base=\"https://inner.example/\" " +
		"xml:lang=\"EN\" rdf:about=\"s\"><ex:q>word</ex:q></rdf:Description></ex:p>" +
		"<ex:id rdf:ID=\"statement\" rdf:resource=\"o\"/>" +
		"<ex:annotation rdf:annotationNodeID=\"e\u0301\">literal</ex:annotation>" +
		"</rdf:Description></rdf:RDF>"
	projected := projectRDFText(t, rdfcodecs.ImportRDFXMLV2, input)
	xml, err := rdfcodecs.ExportRDFXMLV2(projected)
	if err != nil {
		t.Fatal(err)
	}
	reloaded := projectRDFText(t, rdfcodecs.ImportRDFXMLV2, xml)
	text, err := reloaded.NQuads()
	if err != nil || strings.Count(text, "\n") != 8 ||
		!strings.Contains(text, "<<( <https://inner.example/s> <urn:q> \"word\"@EN )>>") ||
		!strings.Contains(text, "<https://example.org/#statement> <http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate> <urn:id> .") ||
		!strings.Contains(text, "#reifies> <<( <https://example.org/outer> <urn:annotation> \"literal\" )>>") {
		t.Fatalf("N-Quads = %q, error %v", text, err)
	}
}

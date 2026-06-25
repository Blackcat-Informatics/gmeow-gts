// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package rdfcodecs

import (
	"sort"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/writer"
)

func graphNQuads(data []byte) string {
	return nquads.ToNQuads(reader.Read(data, true, nil))
}

func sortedRDFLines(text string) []string {
	lines := []string{}
	for _, line := range strings.Split(text, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "@prefix") || strings.HasPrefix(line, "<?xml") ||
			strings.HasPrefix(line, "<rdf:RDF") || strings.HasPrefix(line, "</rdf:RDF") ||
			strings.HasPrefix(line, "<rdf:Description") || strings.HasPrefix(line, "</rdf:Description") {
			continue
		}
		lines = append(lines, line)
	}
	sort.Strings(lines)
	return lines
}

func sampleGraph(t *testing.T, namedGraph bool) *model.Graph {
	t.Helper()
	w := writer.New("dist")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://ex/s"},
		{Kind: model.Iri, Value: "https://ex/p"},
		{Kind: model.Iri, Value: "https://ex/o"},
		{Kind: model.Iri, Value: "https://ex/g"},
		{Kind: model.Iri, Value: "https://ex/confidence"},
		{Kind: model.Literal, Value: "0.9"},
	})
	graph := (*int)(nil)
	if namedGraph {
		gid := 3
		graph = &gid
	}
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2, G: graph}})
	w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 0, P: 1, O: 2}}})
	w.AddAnnot([]model.Triple3{{S: 0, P: 4, O: 5}})
	return reader.Read(w.ToBytes(), true, nil)
}

func TestNTriplesRoundTripAndNamedGraphRefusal(t *testing.T) {
	text := "<https://ex/s> <https://ex/p> <https://ex/o> .\n" +
		"<https://ex/s> <https://ex/label> \"Cat\"@en .\n"
	data, err := FromNTriples(text)
	if err != nil {
		t.Fatal(err)
	}
	out, err := ToNTriples(reader.Read(data, true, nil))
	if err != nil {
		t.Fatal(err)
	}
	if sortedRDFLines(out)[0] != "<https://ex/s> <https://ex/label> \"Cat\"@en ." {
		t.Fatalf("unexpected N-Triples output:\n%s", out)
	}

	if _, err := FromNTriples("<https://ex/s> <https://ex/p> <https://ex/o> <https://ex/g> .\n"); err == nil {
		t.Fatal("expected named graph N-Triples input to fail")
	}
	for _, fn := range []struct {
		name string
		call func(*model.Graph) (string, error)
	}{
		{"N-Triples", ToNTriples},
		{"Turtle", ToTurtle},
		{"RDF/XML", ToRDFXML},
	} {
		if _, err := fn.call(sampleGraph(t, true)); err == nil || !strings.Contains(err.Error(), "named graph") {
			t.Fatalf("%s named graph refusal = %v", fn.name, err)
		}
	}
}

func TestTurtleParserAcceptsSharedGrammar(t *testing.T) {
	turtle := `@base <https://ex/> .
@prefix ex: <https://ex/ns#> .

<s> a ex:Thing ;
    ex:label "Cat"@en ;
    ex:related ex:a, ex:b ;
    ex:nested [ ex:name "Kit" ] ;
    ex:list ( ex:a ex:b ) .
`
	out := graphNQuads(mustBytes(FromTurtle(turtle)))
	for _, want := range []string{
		"<https://ex/s>",
		"<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
		"\"Cat\"@en",
		"<https://ex/ns#related> <https://ex/ns#a>",
		"<https://ex/ns#related> <https://ex/ns#b>",
		"<https://ex/ns#name> \"Kit\"",
		"<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>",
	} {
		if !strings.Contains(out, want) {
			t.Fatalf("missing %q in\n%s", want, out)
		}
	}
	if _, err := FromTurtle("GRAPH <https://ex/g> { <https://ex/s> <https://ex/p> <https://ex/o> . }"); err == nil {
		t.Fatal("expected Turtle to reject graph blocks")
	}
}

func TestTriGParserAcceptsNamedGraphsAndTripleTerms(t *testing.T) {
	trig := `PREFIX ex: <https://ex/>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>

GRAPH ex:g {
  ex:s ex:p ex:o ;
       ex:label "Cat"@en .
}
ex:r rdf:reifies <<( ex:s ex:p ex:o )>> .
ex:r ex:confidence "0.9" .
<< ex:s ex:p ex:o >> ex:source ex:doc .
`
	out := graphNQuads(mustBytes(FromTriG(trig)))
	for _, want := range []string{
		"<https://ex/s> <https://ex/p> <https://ex/o> <https://ex/g> .",
		"\"Cat\"@en <https://ex/g>",
		"<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>",
		"<<( <https://ex/s> <https://ex/p> <https://ex/o> )>>",
		"<https://ex/confidence> \"0.9\"",
		"<https://ex/source> <https://ex/doc>",
	} {
		if !strings.Contains(out, want) {
			t.Fatalf("missing %q in\n%s", want, out)
		}
	}

	trigText, err := ToTriG(sampleGraph(t, true))
	if err != nil {
		t.Fatal(err)
	}
	imported := graphNQuads(mustBytes(FromTriG(trigText)))
	if sortedRDFLines(imported)[0] == "" {
		t.Fatalf("TriG roundtrip produced empty graph:\n%s", imported)
	}
}

func TestRDFXMLParserAcceptsCommittedShapes(t *testing.T) {
	cases := []struct {
		name string
		xml  string
		want []string
	}{
		{
			name: "rdf-element-not-mandatory",
			xml: `<Book xmlns="http://example.org/terms#">
  <title>Dogs in Hats</title>
</Book>`,
			want: []string{
				"<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
				"<http://example.org/terms#Book>",
				"<http://example.org/terms#title> \"Dogs in Hats\"",
			},
		},
		{
			name: "xml-base-language-direction-and-attribute-property",
			xml: `<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ex="http://example.org/" xmlns:its="http://www.w3.org/2005/11/its" xml:base="http://example.org/base/" xml:lang="en" its:dir="ltr" rdf:version="1.2">
  <rdf:Description rdf:about="item" ex:name="bar"/>
</rdf:RDF>`,
			want: []string{
				"<http://example.org/base/item>",
				"<http://example.org/name> \"bar\"@en--ltr",
			},
		},
		{
			name: "parse-type-resource-collection-and-literal",
			xml: `<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:eg="http://example.org/eg#">
  <rdf:Description rdf:about="http://example.org/eg#eric">
    <rdf:type rdf:parseType="Resource">
      <eg:intersectionOf rdf:parseType="Collection">
  <rdf:Description rdf:about="http://example.org/eg#Person"/>
  <rdf:Description rdf:about="http://example.org/eg#Male"/>
      </eg:intersectionOf>
    </rdf:type>
  </rdf:Description>
  <rdf:Description rdf:about="http://example.org/doc">
    <eg:markup rdf:parseType="Literal"><span xmlns="http://www.w3.org/1999/xhtml">Hi</span></eg:markup>
  </rdf:Description>
</rdf:RDF>`,
			want: []string{
				"<http://example.org/eg#eric>",
				"<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:",
				"<http://example.org/eg#intersectionOf> _:",
				"<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>",
				"<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>",
				"<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>",
				"<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral>",
				"span",
			},
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			out := graphNQuads(mustBytes(FromRDFXML(tc.xml)))
			for _, want := range tc.want {
				if !strings.Contains(out, want) {
					t.Fatalf("missing %q in\n%s", want, out)
				}
			}
		})
	}
}

func TestRDFXMLReificationAndSerializationRoundTrip(t *testing.T) {
	rdfxml := `<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ex="http://example.org/stuff/1.0/" rdf:version="1.2">
  <rdf:Description rdf:about="http://example.org/">
    <ex:prop rdf:annotation="http://example.org/triple1">blah</ex:prop>
    <ex:triple rdf:parseType="Triple">
      <rdf:Description rdf:about="http://example.org/stuff/1.0/s">
  <ex:p rdf:resource="http://example.org/stuff/1.0/o"/>
      </rdf:Description>
    </ex:triple>
  </rdf:Description>
</rdf:RDF>`
	data := mustBytes(FromRDFXML(rdfxml))
	out := graphNQuads(data)
	for _, want := range []string{
		"<http://example.org/stuff/1.0/prop> \"blah\"",
		"<http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies>",
		"<<( <http://example.org/> <http://example.org/stuff/1.0/prop> \"blah\" )>>",
		"<<( <http://example.org/stuff/1.0/s> <http://example.org/stuff/1.0/p> <http://example.org/stuff/1.0/o> )>>",
	} {
		if !strings.Contains(out, want) {
			t.Fatalf("missing %q in\n%s", want, out)
		}
	}

	rendered, err := ToRDFXML(reader.Read(data, true, nil))
	if err != nil {
		t.Fatal(err)
	}
	imported := graphNQuads(mustBytes(FromRDFXML(rendered)))
	if len(sortedRDFLines(imported)) == 0 {
		t.Fatalf("RDF/XML roundtrip produced empty graph:\n%s", rendered)
	}

	bad := `<?xml version="1.0"?><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ex="http://example.org/"><rdf:Description rdf:about="http://example.org/"><ex:p rdf:parseType="Triple"><rdf:Description rdf:about="http://example.org/s"/></ex:p></rdf:Description></rdf:RDF>`
	if _, err := FromRDFXML(bad); err == nil {
		t.Fatal("expected malformed RDF/XML triple parseType to fail")
	}
}

func mustBytes(data []byte, err error) []byte {
	if err != nil {
		panic(err)
	}
	return data
}

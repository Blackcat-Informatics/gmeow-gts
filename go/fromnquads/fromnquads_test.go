// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package fromnquads

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/wire"
	"go.blackcatinformatics.ca/gts/writer"
	"go.blackcatinformatics.ca/gts/xsd"
)

func sortedLines(text string) []string {
	var lines []string
	for _, line := range strings.Split(strings.TrimSpace(text), "\n") {
		line = strings.TrimSpace(line)
		if line != "" {
			lines = append(lines, line)
		}
	}
	sort.Strings(lines)
	return lines
}

func vectorsDir(t *testing.T) string {
	t.Helper()
	dir, err := filepath.Abs("../../vectors")
	if err != nil {
		t.Fatal(err)
	}
	return dir
}

func roundTrip(t *testing.T, nq string) string {
	t.Helper()
	data, err := FromNQuads(nq)
	if err != nil {
		t.Fatal(err)
	}
	return nquads.ToNQuads(reader.Read(data, false, nil))
}

func TestFromNQuadsInvertsFoldOutputForCorpusVector(t *testing.T) {
	data, err := os.ReadFile(filepath.Join(vectorsDir(t), "11-datatype-defaulting.gts"))
	if err != nil {
		t.Fatal(err)
	}
	nq := nquads.ToNQuads(reader.Read(data, false, nil))
	if got, want := sortedLines(roundTrip(t, nq)), sortedLines(nq); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestFromNQuadsPreservesNamedGraphsReifiersAndAnnotations(t *testing.T) {
	w := writer.New("dist")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://ex/s"},
		{Kind: model.Iri, Value: "https://ex/p"},
		{Kind: model.Iri, Value: "https://ex/o"},
		{Kind: model.Iri, Value: "https://ex/g"},
		{Kind: model.Iri, Value: "https://ex/conf"},
		{Kind: model.Literal, Value: "0.9"},
	})
	gid := 3
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2, G: &gid}})
	w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 0, P: 1, O: 2}, G: &gid}})
	w.AddAnnot([]model.AnnotationEntry{{S: 0, P: 4, O: 5, G: &gid}})
	nq := nquads.ToNQuads(reader.Read(w.ToBytes(), false, nil))
	if got, want := sortedLines(roundTrip(t, nq)), sortedLines(nq); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestFromNQuadsPreservesLanguageTaggedAndDatatypedLiterals(t *testing.T) {
	xsdInt := "http://www.w3.org/2001/XMLSchema#integer"
	nq := "<https://ex/s> <https://ex/label> \"Cat\"@en .\n" +
		"<https://ex/s> <https://ex/n> \"42\"^^<" + xsdInt + "> .\n" +
		"_:b0 <https://ex/p> <https://ex/s> .\n"
	if got, want := sortedLines(roundTrip(t, nq)), sortedLines(nq); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestFromNQuadsPreservesDirectionalLanguageLiterals(t *testing.T) {
	nq := "<https://ex/s> <https://ex/label> \"RTL\"@ar--rtl .\n"
	graph := reader.Read(mustFromNQuads(t, nq), false, nil)
	var literal *model.Term
	for i := range graph.Terms {
		if graph.Terms[i].Kind == model.Literal {
			literal = &graph.Terms[i]
			break
		}
	}
	if literal == nil {
		t.Fatal("directional literal not found")
	}
	if literal.Lang != "ar" || literal.Direction != "rtl" {
		t.Fatalf("directional literal lost metadata: %#v", literal)
	}
	if got := graph.DatatypeIRI(literal); got != model.RDFDirLangString {
		t.Fatalf("datatype = %q, want %q", got, model.RDFDirLangString)
	}
	if got, want := sortedLines(nquads.ToNQuads(graph)), sortedLines(nq); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestFromNQuadsAddsIllTypedLiteralMetadata(t *testing.T) {
	boolIRI := xsd.Namespace + "boolean"
	customIRI := "https://example.org/customDatatype"
	nq := "<https://ex/s> <https://ex/bad> \"maybe\"^^<" + boolIRI + "> .\n" +
		"<https://ex/s> <https://ex/good> \"true\"^^<" + boolIRI + "> .\n" +
		"<https://ex/s> <https://ex/custom> \"not our syntax\"^^<" + customIRI + "> .\n"

	graph := reader.Read(mustFromNQuads(t, nq), false, nil)
	items := xsd.IllTypedLiterals(graph)
	if len(items) != 1 {
		t.Fatalf("ill typed literals = %#v, want one recognized invalid literal", items)
	}
	if diag := items[0].Diagnostic(); diag.Code != xsd.IllTypedLiteralCode {
		t.Fatalf("diagnostic code = %q, want %q", diag.Code, xsd.IllTypedLiteralCode)
	}

	sidecar := metaMap(t, graph, xsd.IllTypedLiteralMetaKey)
	version, ok := wire.AsInt(sidecar["version"])
	if !ok || version != 1 {
		t.Fatalf("sidecar version = %#v, want 1", sidecar["version"])
	}
	rows, ok := sidecar["items"].([]interface{})
	if !ok || len(rows) != 1 {
		t.Fatalf("sidecar items = %#v, want one row", sidecar["items"])
	}
	row, ok := rows[0].(map[interface{}]interface{})
	if !ok {
		t.Fatalf("sidecar row type = %T, want map", rows[0])
	}
	termID, ok := wire.AsInt(row["term"])
	if !ok || termID != items[0].TermID {
		t.Fatalf("row term = %#v, want %d", row["term"], items[0].TermID)
	}
	if got, _ := wire.AsText(row["datatype"]); got != boolIRI {
		t.Fatalf("row datatype = %q, want %q", got, boolIRI)
	}
	if got, _ := wire.AsText(row["lexical"]); got != "maybe" {
		t.Fatalf("row lexical = %q, want maybe", got)
	}
	if got, _ := wire.AsText(row["reason"]); got == "" {
		t.Fatal("row reason is empty")
	}
	if got := nquads.ToNQuads(graph); !strings.Contains(got, "\"not our syntax\"^^<"+customIRI+">") {
		t.Fatalf("unsupported datatype was not preserved in N-Quads:\n%s", got)
	}
}

func TestWriterAllowsMultipleReifiersForSameStatement(t *testing.T) {
	w := writer.New("dist")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://ex/r1"},
		{Kind: model.Iri, Value: "https://ex/r2"},
		{Kind: model.Iri, Value: "https://ex/s"},
		{Kind: model.Iri, Value: "https://ex/p"},
		{Kind: model.Iri, Value: "https://ex/o"},
	})
	w.AddQuads([]model.Quad{{S: 2, P: 3, O: 4}})
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 2, P: 3, O: 4}},
		{RID: 1, SPO: model.Triple3{S: 2, P: 3, O: 4}},
	})
	graph := reader.Read(w.ToBytes(), false, nil)
	if len(graph.Reifiers) != 2 {
		t.Fatalf("reifier count = %d, want 2", len(graph.Reifiers))
	}
}

func TestFromNQuadsPreservesMultipleReifiersForSameStatement(t *testing.T) {
	nq := "<https://ex/r1> <" + rdfReifies + "> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n" +
		"<https://ex/r2> <" + rdfReifies + "> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .\n"
	graph := reader.Read(mustFromNQuads(t, nq), false, nil)
	if len(graph.Reifiers) != 2 {
		t.Fatalf("reifier count = %d, want 2", len(graph.Reifiers))
	}
	if got, want := sortedLines(nquads.ToNQuads(graph)), sortedLines(nq); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestFromNQuadsCompactBlankNodeAndLanguageTagDelimiters(t *testing.T) {
	nq := "<https://ex/s> <https://ex/p> _:b0.\n" +
		"<https://ex/s> <https://ex/label> \"Cat\"@en.\n"
	expected := "<https://ex/s> <https://ex/p> _:b0 .\n" +
		"<https://ex/s> <https://ex/label> \"Cat\"@en .\n"
	if got, want := sortedLines(roundTrip(t, nq)), sortedLines(expected); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func mustFromNQuads(t *testing.T, nq string) []byte {
	t.Helper()
	data, err := FromNQuads(nq)
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func metaMap(t *testing.T, graph *model.Graph, key string) map[interface{}]interface{} {
	t.Helper()
	for _, entry := range graph.Meta {
		if entry.Key != key {
			continue
		}
		sidecar, ok := entry.Value.(map[interface{}]interface{})
		if !ok {
			t.Fatalf("metadata %s type = %T, want map", key, entry.Value)
		}
		return sidecar
	}
	t.Fatalf("metadata %s not found in %#v", key, graph.Meta)
	return nil
}

func TestFromNQuadsQuotedTripleAdjacentDelimiters(t *testing.T) {
	nq := "<https://ex/r1> <" + rdfReifies + "> <<( _:b0 <https://ex/p> _:b1)>> .\n" +
		"<https://ex/r2> <" + rdfReifies + "> <<( <https://ex/s> <https://ex/p> \"Cat\"@en)>> .\n"
	expected := "<https://ex/r1> <" + rdfReifies + "> <<( _:b0 <https://ex/p> _:b1 )>> .\n" +
		"<https://ex/r2> <" + rdfReifies + "> <<( <https://ex/s> <https://ex/p> \"Cat\"@en )>> .\n"
	if got, want := sortedLines(roundTrip(t, nq)), sortedLines(expected); strings.Join(got, "\n") != strings.Join(want, "\n") {
		t.Fatalf("round-trip mismatch\ngot:\n%s\nwant:\n%s", strings.Join(got, "\n"), strings.Join(want, "\n"))
	}
}

func TestFromNQuadsRejectsMalformedStatements(t *testing.T) {
	if _, err := FromNQuads("<https://ex/s> <https://ex/p> .\n"); err == nil {
		t.Fatal("expected malformed N-Quads to fail")
	}
}

func TestFromNQuadsRejectsEmptyBlankNodeLabelAndLanguageTag(t *testing.T) {
	for _, input := range []string{
		"<https://ex/s> <https://ex/p> _: .\n",
		"<https://ex/s> <https://ex/p> \"Cat\"@ .\n",
	} {
		if _, err := FromNQuads(input); err == nil {
			t.Fatalf("expected malformed N-Quads to fail: %q", input)
		}
	}
}

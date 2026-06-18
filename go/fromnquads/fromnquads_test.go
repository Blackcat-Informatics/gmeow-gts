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
	"go.blackcatinformatics.ca/gts/writer"
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
	w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 0, P: 1, O: 2}}})
	w.AddAnnot([]model.Triple3{{S: 0, P: 4, O: 5}})
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

func TestFromNQuadsRejectsMalformedStatements(t *testing.T) {
	if _, err := FromNQuads("<https://ex/s> <https://ex/p> .\n"); err == nil {
		t.Fatal("expected malformed N-Quads to fail")
	}
}

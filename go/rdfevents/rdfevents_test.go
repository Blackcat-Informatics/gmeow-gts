// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package rdfevents

import (
	"bytes"
	"context"
	"fmt"
	"sort"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/writer"
)

func termIDPtr(id TermID) *TermID {
	return &id
}

func intPtr(id int) *int {
	return &id
}

func sortedNQuads(text string) string {
	var lines []string
	for _, line := range strings.Split(strings.TrimSpace(text), "\n") {
		line = strings.TrimSpace(line)
		if line != "" {
			lines = append(lines, line)
		}
	}
	sort.Strings(lines)
	return strings.Join(lines, "\n")
}

func eventTestGraph() *model.Graph {
	graphID := 3
	reifierID := 6
	frameIndex := 12
	return &model.Graph{
		Terms: []model.Term{
			{Kind: model.Iri, Value: "https://ex/s"},
			{Kind: model.Iri, Value: "https://ex/p"},
			{Kind: model.Literal, Value: "Cat", Lang: "en", Direction: "rtl"},
			{Kind: model.Iri, Value: "https://ex/g"},
			{Kind: model.Iri, Value: "https://ex/confidence"},
			{Kind: model.Literal, Value: "0.9"},
			{Kind: model.Triple, Reifier: &reifierID},
		},
		Quads:       []model.Quad{{S: 0, P: 1, O: 2, G: &graphID}},
		Reifiers:    []model.ReifierEntry{{RID: 6, SPO: model.Triple3{S: 0, P: 1, O: 2}}},
		Annotations: []model.Triple3{{S: 6, P: 4, O: 5}},
		Diagnostics: []model.Diagnostic{{
			Code:       "TestDiagnostic",
			Detail:     "diagnostic detail",
			FrameIndex: &frameIndex,
		}},
	}
}

type recordingSink struct {
	NopSink
	beforeReferences bool
	limit            int
	events           []string
}

func (s *recordingSink) DeclarationsBeforeReferences() bool {
	return s.beforeReferences
}

func (s *recordingSink) TripleTermNestingLimit() int {
	if s.limit == 0 {
		return defaultTripleTermNestingLimit
	}
	return s.limit
}

func (s *recordingSink) StartScope(scope ScopeID) error {
	s.events = append(s.events, "start")
	return nil
}

func (s *recordingSink) Term(term Term) error {
	s.events = append(s.events, fmt.Sprintf("term:%d", term.ID))
	return nil
}

func (s *recordingSink) Quad(Quad) error {
	s.events = append(s.events, "quad")
	return nil
}

func (s *recordingSink) Reifier(id TermID, triple Triple) error {
	s.events = append(s.events, fmt.Sprintf("reifier:%d", id))
	return nil
}

func (s *recordingSink) Annotation(Triple) error {
	s.events = append(s.events, "annotation")
	return nil
}

func (s *recordingSink) Diagnostic(d Diagnostic) error {
	s.events = append(s.events, "diagnostic:"+d.Code)
	return nil
}

func (s *recordingSink) EndScope(scope ScopeID) error {
	s.events = append(s.events, "end")
	return nil
}

func (s *recordingSink) Finish() error {
	s.events = append(s.events, "finish")
	return nil
}

func TestGraphSourceDrivesFoldedGraphEvents(t *testing.T) {
	sink := &recordingSink{}
	if err := NewGraphSource(eventTestGraph()).WithScope(7).Drive(sink); err != nil {
		t.Fatal(err)
	}
	want := []string{
		"start",
		"term:0",
		"term:1",
		"term:2",
		"term:3",
		"term:4",
		"term:5",
		"term:6",
		"reifier:6",
		"quad",
		"annotation",
		"diagnostic:TestDiagnostic",
		"end",
		"finish",
	}
	if strings.Join(sink.events, "\n") != strings.Join(want, "\n") {
		t.Fatalf("events:\n%s\nwant:\n%s", strings.Join(sink.events, "\n"), strings.Join(want, "\n"))
	}
}

func TestGraphSinkMaterializesGraphWithoutChangingNQuads(t *testing.T) {
	source := eventTestGraph()
	sink := NewGraphSink()
	if err := NewGraphSource(source).Drive(sink); err != nil {
		t.Fatal(err)
	}
	materialized, err := sink.Graph()
	if err != nil {
		t.Fatal(err)
	}
	if got, want := sortedNQuads(nquads.ToNQuads(materialized)), sortedNQuads(nquads.ToNQuads(source)); got != want {
		t.Fatalf("materialized N-Quads changed\ngot:\n%s\nwant:\n%s", got, want)
	}
	if got := len(materialized.Diagnostics); got != 1 {
		t.Fatalf("diagnostic count = %d, want 1", got)
	}
	if materialized.Diagnostics[0].FrameIndex == nil || *materialized.Diagnostics[0].FrameIndex != 12 {
		t.Fatalf("diagnostic frame index not preserved: %#v", materialized.Diagnostics[0])
	}
	literal := materialized.Terms[2]
	if literal.Lang != "en" || literal.Direction != "rtl" {
		t.Fatalf("literal language/direction not preserved: %#v", literal)
	}
}

func TestDeclarationOrderEmitterDeclaresDependenciesFirst(t *testing.T) {
	reifier := 1
	graph := &model.Graph{
		Terms: []model.Term{
			{Kind: model.Triple, Reifier: &reifier},
			{Kind: model.Iri, Value: "https://ex/reifier"},
			{Kind: model.Iri, Value: "https://ex/s"},
			{Kind: model.Iri, Value: "https://ex/p"},
			{Kind: model.Iri, Value: "https://ex/o"},
		},
		Reifiers: []model.ReifierEntry{{RID: 1, SPO: model.Triple3{S: 2, P: 3, O: 4}}},
		Quads:    []model.Quad{{S: 0, P: 3, O: 4}},
	}
	sink := &recordingSink{beforeReferences: true}
	if err := NewGraphSource(graph).Drive(sink); err != nil {
		t.Fatal(err)
	}
	joined := strings.Join(sink.events, ",")
	wantPrefix := "start,term:1,term:2,term:3,term:4,reifier:1,term:0"
	if !strings.HasPrefix(joined, wantPrefix) {
		t.Fatalf("declaration order = %s, want prefix %s", joined, wantPrefix)
	}
}

func TestDeclarationOrderEmitterSelfReifyingTripleTerm(t *testing.T) {
	reifier := 0
	graph := &model.Graph{
		Terms: []model.Term{
			{Kind: model.Triple, Reifier: &reifier},
			{Kind: model.Iri, Value: "https://ex/s"},
			{Kind: model.Iri, Value: "https://ex/p"},
			{Kind: model.Iri, Value: "https://ex/o"},
		},
		Reifiers: []model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}}},
	}
	sink := &recordingSink{beforeReferences: true}
	if err := NewGraphSource(graph).Drive(sink); err != nil {
		t.Fatal(err)
	}
	joined := strings.Join(sink.events, ",")
	wantPrefix := "start,term:1,term:2,term:3,term:0,reifier:0"
	if !strings.HasPrefix(joined, wantPrefix) {
		t.Fatalf("declaration order = %s, want prefix %s", joined, wantPrefix)
	}
}

func TestGraphSinkRejectsDuplicateDeclarations(t *testing.T) {
	sink := NewGraphSink()
	if err := sink.StartScope(0); err != nil {
		t.Fatal(err)
	}
	if err := sink.Term(Term{ID: 1, Kind: TermIRI, Value: "https://ex/a"}); err != nil {
		t.Fatal(err)
	}
	err := sink.Term(Term{ID: 1, Kind: TermIRI, Value: "https://ex/b"})
	if !IsKind(err, ErrorDuplicateDeclaration) {
		t.Fatalf("error = %v, want %s", err, ErrorDuplicateDeclaration)
	}
}

func TestGraphSinkRejectsUnresolvedReferencesAtFinish(t *testing.T) {
	sink := NewGraphSink()
	if err := sink.StartScope(0); err != nil {
		t.Fatal(err)
	}
	if err := sink.Term(Term{ID: 0, Kind: TermIRI, Value: "https://ex/s"}); err != nil {
		t.Fatal(err)
	}
	if err := sink.Quad(Quad{Subject: 0, Predicate: 0, Object: 99}); err != nil {
		t.Fatal(err)
	}
	if err := sink.EndScope(0); err != nil {
		t.Fatal(err)
	}
	err := sink.Finish()
	if !IsKind(err, ErrorUnresolvedReference) {
		t.Fatalf("error = %v, want %s", err, ErrorUnresolvedReference)
	}
}

func TestGraphSinkEnforcesQuotedTripleNestingLimit(t *testing.T) {
	sink := NewGraphSinkWithOptions(GraphSinkOptions{TripleTermNestingLimit: 0})
	if err := sink.StartScope(0); err != nil {
		t.Fatal(err)
	}
	for _, term := range []Term{
		{ID: 0, Kind: TermIRI, Value: "https://ex/s"},
		{ID: 1, Kind: TermIRI, Value: "https://ex/p"},
		{ID: 2, Kind: TermIRI, Value: "https://ex/o"},
		{ID: 3, Kind: TermTriple, Triple: &Triple{Subject: 0, Predicate: 1, Object: 2}, Reifier: termIDPtr(3)},
		{ID: 4, Kind: TermTriple, Triple: &Triple{Subject: 3, Predicate: 1, Object: 2}, Reifier: termIDPtr(4)},
	} {
		if err := sink.Term(term); err != nil {
			t.Fatal(err)
		}
	}
	if err := sink.EndScope(0); err != nil {
		t.Fatal(err)
	}
	err := sink.Finish()
	if !IsKind(err, ErrorTripleNestingLimit) {
		t.Fatalf("error = %v, want %s", err, ErrorTripleNestingLimit)
	}
}

func TestReadToEventsFoldsReaderInputAndDrivesSink(t *testing.T) {
	w := writer.New("dist")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://ex/s"},
		{Kind: model.Iri, Value: "https://ex/p"},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	data := w.ToBytes()

	sink := NewGraphSink()
	graph, err := ReadToEvents(context.Background(), bytes.NewReader(data), reader.Options{}, sink)
	if err != nil {
		t.Fatal(err)
	}
	materialized, err := sink.Graph()
	if err != nil {
		t.Fatal(err)
	}
	if got, want := sortedNQuads(nquads.ToNQuads(materialized)), sortedNQuads(nquads.ToNQuads(graph)); got != want {
		t.Fatalf("reader event N-Quads changed\ngot:\n%s\nwant:\n%s", got, want)
	}
}

func TestGraphSinkRejectsEventsAfterEndScope(t *testing.T) {
	sink := NewGraphSink()
	if err := sink.StartScope(0); err != nil {
		t.Fatal(err)
	}
	if err := sink.EndScope(0); err != nil {
		t.Fatal(err)
	}
	err := sink.Term(Term{ID: 0, Kind: TermIRI, Value: "https://ex/s"})
	if !IsKind(err, ErrorClosedScope) {
		t.Fatalf("error = %v, want %s", err, ErrorClosedScope)
	}
}

func TestGraphSinkMaterializesImplicitTripleTermReifier(t *testing.T) {
	sink := NewGraphSink()
	if err := sink.StartScope(0); err != nil {
		t.Fatal(err)
	}
	for _, term := range []Term{
		{ID: 0, Kind: TermIRI, Value: "https://ex/s"},
		{ID: 1, Kind: TermIRI, Value: "https://ex/p"},
		{ID: 2, Kind: TermIRI, Value: "https://ex/o"},
		{ID: 3, Kind: TermTriple, Triple: &Triple{Subject: 0, Predicate: 1, Object: 2}},
	} {
		if err := sink.Term(term); err != nil {
			t.Fatal(err)
		}
	}
	if err := sink.EndScope(0); err != nil {
		t.Fatal(err)
	}
	if err := sink.Finish(); err != nil {
		t.Fatal(err)
	}
	graph, err := sink.Graph()
	if err != nil {
		t.Fatal(err)
	}
	if triple, ok := graph.Reifier(3); !ok || triple != (model.Triple3{S: 0, P: 1, O: 2}) {
		t.Fatalf("implicit reifier = %#v, %v", triple, ok)
	}
}

func TestEventSourceRejectsUnresolvableTripleTerm(t *testing.T) {
	graph := &model.Graph{
		Terms: []model.Term{
			{Kind: model.Iri, Value: "https://ex/s"},
			{Kind: model.Triple, Reifier: intPtr(1)},
		},
	}
	err := NewGraphSource(graph).Drive(NopSink{})
	if !IsKind(err, ErrorInvalidSource) {
		t.Fatalf("error = %v, want %s", err, ErrorInvalidSource)
	}
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"bytes"
	"context"
	"reflect"
	"sort"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/writer"
)

// The vocabulary shared by the §7.3 triple-term tests: one reifier resource and
// one statement whose object varies, so a single reifier can bind two DISTINCT
// triples exactly as a non-functional rdf:reifies allows.
func tripleTermTerms() []model.Term {
	return []model.Term{
		{Kind: model.Iri, Value: "https://example.org/r1"},                     // 0 reifier
		{Kind: model.Iri, Value: "https://example.org/Cat"},                    // 1 subject
		{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"}, // 2 predicate
		{Kind: model.Literal, Value: "Cat", Lang: "en"},                        // 3 object A
		{Kind: model.Literal, Value: "Chat", Lang: "fr"},                       // 4 object B
		{Kind: model.Iri, Value: "https://example.org/says"},                   // 5 spare predicate
	}
}

func tripleTermDiagCodes(g *model.Graph) []string {
	codes := []string{}
	for _, d := range g.Diagnostics {
		codes = append(codes, d.Code)
	}
	return codes
}

// §7.3: rdf:reifies is not functional, so both bindings survive with no
// diagnostic and each projects its own statement.
func TestReifiesIsMultiValued(t *testing.T) {
	w := writer.New("generic")
	w.AddTerms(tripleTermTerms())
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 4}},
	})

	g := Read(w.ToBytes(), true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
	if len(g.Reifiers) != 2 {
		t.Fatalf("reifier rows = %#v, want both retained", g.Reifiers)
	}
	if g.Reifiers[0].SPO != (model.Triple3{S: 1, P: 2, O: 3}) ||
		g.Reifiers[1].SPO != (model.Triple3{S: 1, P: 2, O: 4}) {
		t.Fatalf("reifier rows lost file order: %#v", g.Reifiers)
	}
	lines := strings.Count(nquads.ToNQuads(g), "\n")
	if lines != 2 {
		t.Fatalf("projected %d N-Quads lines, want 2:\n%s", lines, nquads.ToNQuads(g))
	}
}

// §7.8: only a byte-identical repeat of a row collapses.
func TestReifiesIdenticalRowsCollapse(t *testing.T) {
	graphSlot := 5
	w := writer.New("generic")
	w.AddTerms(tripleTermTerms())
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}, G: &graphSlot},
	})

	g := Read(w.ToBytes(), true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
	// The repeat collapses; the graph-scoped row is a different row and stays.
	if len(g.Reifiers) != 2 {
		t.Fatalf("reifier rows = %#v, want the duplicate collapsed and the graph-scoped row kept", g.Reifiers)
	}
	if g.Reifiers[0].G != nil || g.Reifiers[1].G == nil || *g.Reifiers[1].G != graphSlot {
		t.Fatalf("graph slots not preserved per row: %#v", g.Reifiers)
	}
}

// §7.3: the ONE surviving conflict shape is a "tt"-less (legacy) k:3 term whose
// reifier binds more than one distinct triple. No row is dropped and the term
// still resolves to the first binding.
func TestConflictingReifierOnlyForLegacyTripleTerm(t *testing.T) {
	reifier := 0
	terms := append(tripleTermTerms(), model.Term{Kind: model.Triple, Reifier: &reifier})
	w := writer.New("generic")
	w.AddTerms(terms)
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 4}},
	})
	w.AddQuads([]model.Quad{{S: 1, P: 5, O: 6}})

	g := Read(w.ToBytes(), true, nil)
	if got, want := tripleTermDiagCodes(g), []string{"ConflictingReifier"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("diagnostics = %v, want exactly %v", got, want)
	}
	if g.Diagnostics[0].FrameIndex != nil {
		t.Fatalf("frame index = %d, want none/null", *g.Diagnostics[0].FrameIndex)
	}
	if len(g.Reifiers) != 2 {
		t.Fatalf("reifier rows = %#v, want NO row dropped", g.Reifiers)
	}
	spo, ok := g.TripleOf(6)
	if !ok || spo != (model.Triple3{S: 1, P: 2, O: 3}) {
		t.Fatalf("legacy term resolved to %#v (%v), want the FIRST binding", spo, ok)
	}
}

// The same over-bound reifier is NOT a conflict once the term states its own
// triple: "tt" is authoritative and terminal.
func TestSelfDescribingTripleTermOverBoundReifierIsNotAConflict(t *testing.T) {
	reifier := 0
	terms := append(tripleTermTerms(), model.Term{
		Kind:    model.Triple,
		Reifier: &reifier,
		Triple:  &model.Triple3{S: 1, P: 2, O: 4},
	})
	w := writer.New("generic")
	w.AddTerms(terms)
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 4}},
	})

	g := Read(w.ToBytes(), true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
	spo, ok := g.TripleOf(6)
	if !ok || spo != (model.Triple3{S: 1, P: 2, O: 4}) {
		t.Fatalf("term resolved to %#v (%v), want its own 'tt'", spo, ok)
	}
}

// A "tt"-less term over a reifier with ONE binding is ordinary legacy content.
func TestLegacyTripleTermWithSingleBindingIsClean(t *testing.T) {
	reifier := 0
	terms := append(tripleTermTerms(), model.Term{Kind: model.Triple, Reifier: &reifier})
	w := writer.New("generic")
	w.AddTerms(terms)
	w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}}})

	g := Read(w.ToBytes(), true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
}

// §7.3: an unreified triple term is a value, not a statement — it round-trips
// as itself, minting no reifier and emitting no `reifies` row.
func TestUnreifiedTripleTermRoundTrips(t *testing.T) {
	terms := append(tripleTermTerms(), model.Term{
		Kind:   model.Triple,
		Triple: &model.Triple3{S: 1, P: 2, O: 3},
	})
	w := writer.New("generic")
	w.AddTerms(terms)
	w.AddQuads([]model.Quad{{S: 1, P: 5, O: 6}})

	g := Read(w.ToBytes(), true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
	if len(g.Reifiers) != 0 {
		t.Fatalf("reifier rows = %#v, want none", g.Reifiers)
	}
	if g.Terms[6].Triple == nil || *g.Terms[6].Triple != (model.Triple3{S: 1, P: 2, O: 3}) {
		t.Fatalf("term 'tt' = %#v, want (1,2,3)", g.Terms[6].Triple)
	}
	if g.Terms[6].Reifier != nil {
		t.Fatalf("term reifier = %d, want none minted", *g.Terms[6].Reifier)
	}
	want := "<https://example.org/Cat> <https://example.org/says> " +
		"<<( <https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> \"Cat\"@en )>> .\n"
	if got := nquads.ToNQuads(g); got != want {
		t.Fatalf("projection =\n%s\nwant\n%s", got, want)
	}

	// Re-authoring the folded terms reproduces the same self-describing term.
	w2 := writer.New("generic")
	w2.AddTerms(g.Terms)
	w2.AddQuads(g.Quads)
	again := Read(w2.ToBytes(), true, nil)
	if !reflect.DeepEqual(again.Terms, g.Terms) {
		t.Fatalf("re-authored terms differ\ngot:  %#v\nwant: %#v", again.Terms, g.Terms)
	}
}

// §7.5 union: a triple term's identity is its OWN (s, p, o), so two DISTINCT
// triple terms sharing a reifier id stay distinct through the union.
func TestDistinctTripleTermsSharingReifierStayDistinct(t *testing.T) {
	build := func(object int) []byte {
		reifier := 0
		terms := append(tripleTermTerms(), model.Term{
			Kind:    model.Triple,
			Reifier: &reifier,
			Triple:  &model.Triple3{S: 1, P: 2, O: object},
		})
		w := writer.New("generic")
		w.AddTerms(terms)
		w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: object}}})
		w.AddQuads([]model.Quad{{S: 1, P: 5, O: 6}})
		return w.ToBytes()
	}
	data := append(build(3), build(4)...)

	g := Read(data, true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
	var tripleTerms []model.Term
	for _, term := range g.Terms {
		if term.Kind == model.Triple {
			tripleTerms = append(tripleTerms, term)
		}
	}
	if len(tripleTerms) != 2 {
		t.Fatalf("union kept %d triple terms, want 2 distinct: %#v", len(tripleTerms), tripleTerms)
	}
	if *tripleTerms[0].Triple == *tripleTerms[1].Triple {
		t.Fatalf("the two triple terms collapsed onto %#v", *tripleTerms[0].Triple)
	}
	if len(g.Quads) != 2 {
		t.Fatalf("quads = %#v, want one per distinct triple term", g.Quads)
	}
}

// §7.8: a quoted triple term's identity is the equality of its RESOLVED
// subject, predicate, and object. A self-describing term and a legacy
// "tt"-less term that resolve to the SAME triple are therefore the SAME term
// and intern together — there is no separate legacy key space.
func TestTripleTermsInternOnResolvedTriple(t *testing.T) {
	reifier := 0
	seg := &model.Graph{
		Terms: []model.Term{
			{Kind: model.Iri, Value: "https://example.org/r1"},                     // 0 reifier
			{Kind: model.Iri, Value: "https://example.org/Cat"},                    // 1
			{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"}, // 2
			{Kind: model.Literal, Value: "Cat", Lang: "en"},                        // 3
			{Kind: model.Triple, Reifier: &reifier},                                // 4 legacy
			{Kind: model.Triple, Triple: &model.Triple3{S: 1, P: 2, O: 3}},         // 5 self-describing
			{Kind: model.Iri, Value: "https://example.org/says"},                   // 6
		},
		Reifiers: []model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}}},
		Quads:    []model.Quad{{S: 1, P: 6, O: 4}, {S: 1, P: 6, O: 5}},
	}

	got := unionSegments([]*model.Graph{seg})
	tripleTerms := 0
	for _, term := range got.Terms {
		if term.Kind == model.Triple {
			tripleTerms++
		}
	}
	if tripleTerms != 1 {
		t.Fatalf("union kept %d triple terms, want the two same-resolution forms interned as ONE: %#v",
			tripleTerms, got.Terms)
	}
	// Both quads name the same object value, so they collapse under §7.8.
	if len(got.Quads) != 1 {
		t.Fatalf("union produced %d quads, want 1: %#v", len(got.Quads), got.Quads)
	}
}

// Two legacy "tt"-less terms on one reifier resolve identically (both to the
// first binding), so they intern together; a term that states no triple at all
// keeps the single unresolved identity an unbound quoted triple has always had.
func TestUnresolvedTripleTermsShareOneIdentity(t *testing.T) {
	reifier := 0
	seg := &model.Graph{
		Terms: []model.Term{
			{Kind: model.Iri, Value: "https://example.org/r1"},
			{Kind: model.Triple, Reifier: &reifier},
			{Kind: model.Triple, Reifier: &reifier},
		},
		Quads: []model.Quad{{S: 1, P: 0, O: 0}, {S: 2, P: 0, O: 0}},
	}
	got := unionSegments([]*model.Graph{seg})
	if len(got.Terms) != 2 {
		t.Fatalf("union produced %d terms, want 2: %#v", len(got.Terms), got.Terms)
	}
	if len(got.Quads) != 1 {
		t.Fatalf("union produced %d quads, want 1: %#v", len(got.Quads), got.Quads)
	}
}

// A `reifies` row may name the very term that resolves through it. That is
// legal wire content, so the union must terminate on it rather than recurse
// forever (the reader's no-panic contract, see FuzzRead).
// sortedLines splits N-Quads text into its non-empty lines, sorted.
func sortedLines(text string) []string {
	var out []string
	for _, line := range strings.Split(text, "\n") {
		if trimmed := strings.TrimSpace(line); trimmed != "" {
			out = append(out, trimmed)
		}
	}
	sort.Strings(out)
	return out
}

func selfReachingSegment() []byte {
	reifier := 0
	w := writer.New("generic")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://example.org/r1"},
		{Kind: model.Iri, Value: "https://example.org/p"},
		{Kind: model.Triple, Reifier: &reifier},
	})
	// The binding puts the triple term itself in subject position.
	w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 2, P: 1, O: 1}}})
	w.AddQuads([]model.Quad{{S: 2, P: 1, O: 1}})
	return w.ToBytes()
}

// The N-Quads projection walks a triple term's resolved components, so it too
// must terminate on the self-reaching shape (§7.3): the term degrades to the
// blank node an unbound triple term already produces.
func TestProjectionSurvivesSelfReachingTripleTerm(t *testing.T) {
	g := Read(selfReachingSegment(), true, nil)
	if len(g.Diagnostics) != 0 {
		t.Fatalf("unexpected diagnostics: %#v", g.Diagnostics)
	}
	got := sortedLines(nquads.ToNQuads(g))
	// The SELF-REACHING TERM ITSELF is the blank node. Resolving it one step
	// first and degrading a nested occurrence would render a different graph,
	// and is what made this engine disagree with Rust on identical bytes.
	want := []string{
		"<https://example.org/r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> " +
			"<<( _:unbound_triple_2 <https://example.org/p> <https://example.org/p> )>> .",
		"_:unbound_triple_2 <https://example.org/p> <https://example.org/p> .",
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("projection = %#v, want %#v", got, want)
	}
}

func TestUnionSurvivesSelfReachingTripleTerm(t *testing.T) {
	data := append(selfReachingSegment(), selfReachingSegment()...)

	g := Read(data, true, nil) // must terminate, and must not panic
	if len(g.Terms) == 0 {
		t.Fatal("union dropped every term")
	}
	// A self-reaching term states no triple, so both segments' copies resolve
	// to the same "no triple" answer and intern together — byte-identical rows
	// collapse under §7.8 set semantics, leaving one quad.
	if len(g.Quads) != 1 {
		t.Fatalf("union produced %d quads, want 1: %#v", len(g.Quads), g.Quads)
	}
	for _, q := range g.Quads {
		if g.Terms[q.S].Kind != model.Triple {
			t.Fatalf("quad subject %d is not the triple term: %#v", q.S, g.Terms[q.S])
		}
	}
}

// §7.2/§7.5: every "tt" component must name an earlier term; a violating "tt"
// is diagnosed and dropped whole.
func TestTripleTermForwardReferenceIsDiagnosedAndDropped(t *testing.T) {
	terms := append(tripleTermTerms(), model.Term{
		Kind:   model.Triple,
		Triple: &model.Triple3{S: 1, P: 2, O: 99},
	})
	w := writer.New("generic")
	w.AddTerms(terms)

	g := Read(w.ToBytes(), true, nil)
	if got, want := tripleTermDiagCodes(g), []string{"ForwardReference"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("diagnostics = %v, want %v", got, want)
	}
	if g.Terms[6].Triple != nil {
		t.Fatalf("'tt' = %#v, want dropped", g.Terms[6].Triple)
	}
}

// §7.4: tt[1] must be an IRI and tt[0] must not be a literal; a violating "tt"
// is diagnosed and dropped, and the term falls back to its "rf".
func TestTripleTermPositionConstraints(t *testing.T) {
	reifier := 0
	cases := []struct {
		name   string
		triple model.Triple3
	}{
		{"literal-predicate", model.Triple3{S: 1, P: 3, O: 2}},
		{"literal-subject", model.Triple3{S: 3, P: 2, O: 1}},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			triple := tc.triple
			terms := append(tripleTermTerms(), model.Term{
				Kind:    model.Triple,
				Reifier: &reifier,
				Triple:  &triple,
			})
			w := writer.New("generic")
			w.AddTerms(terms)
			w.AddReifies([]model.ReifierEntry{{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}}})

			g := Read(w.ToBytes(), true, nil)
			if got, want := tripleTermDiagCodes(g), []string{"PositionConstraint"}; !reflect.DeepEqual(got, want) {
				t.Fatalf("diagnostics = %v, want %v", got, want)
			}
			if g.Terms[6].Triple != nil {
				t.Fatalf("'tt' = %#v, want dropped", g.Terms[6].Triple)
			}
			// Dropped "tt" degrades to the legacy reifier fallback.
			spo, ok := g.TripleOf(6)
			if !ok || spo != (model.Triple3{S: 1, P: 2, O: 3}) {
				t.Fatalf("fallback resolution = %#v (%v), want the reifier binding", spo, ok)
			}
		})
	}
}

// A snapshot's term ids are segment-local; each "tt" component shifts with the
// rest of the id space.
func TestSnapshotShiftsTripleTermComponents(t *testing.T) {
	w := writer.New("generic")
	w.AddTerms(tripleTermTerms())
	w.AddFrame("snapshot", map[interface{}]interface{}{
		"terms": []interface{}{
			map[interface{}]interface{}{"k": int64(0), "v": "https://example.org/Dog"},
			map[interface{}]interface{}{"k": int64(1), "v": "Dog", "l": "en"},
			map[interface{}]interface{}{
				"k":  int64(3),
				"tt": []interface{}{int64(0), int64(0), int64(1)},
			},
		},
	}, nil, nil, nil)

	g := Read(w.ToBytes(), true, nil)
	if got := tripleTermDiagCodes(g); len(got) != 0 {
		t.Fatalf("diagnostics = %v, want none", got)
	}
	// Snapshot-local ids 0/0/1 land at 6/6/7 in the enclosing id space; the
	// snapshot's own predicate slot is the shifted IRI at 6.
	if g.Terms[8].Triple == nil || *g.Terms[8].Triple != (model.Triple3{S: 6, P: 6, O: 7}) {
		t.Fatalf("shifted 'tt' = %#v, want (6,6,7)", g.Terms[8].Triple)
	}
}

// The streaming fold reaches the same §7.3 conclusion as the buffered read.
func TestStreamingFoldReportsConflictingReifier(t *testing.T) {
	reifier := 0
	terms := append(tripleTermTerms(), model.Term{Kind: model.Triple, Reifier: &reifier})
	w := writer.New("generic")
	w.AddTerms(terms)
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 4}},
	})
	data := w.ToBytes()

	var counts streamingEventCounts
	streamed, err := ReadToSink(context.Background(), bytes.NewReader(data), Options{AllowSegments: true}, &counts)
	if err != nil {
		t.Fatalf("ReadToSink returned error: %v", err)
	}
	full := Read(data, true, nil)
	if !reflect.DeepEqual(streamed.Diagnostics, full.Diagnostics) {
		t.Fatalf("diagnostics differ\nstreamed: %#v\nfull:     %#v", streamed.Diagnostics, full.Diagnostics)
	}
	if counts.Diagnostics != len(streamed.Diagnostics) {
		t.Fatalf("diagnostic event count %d != result diagnostics %d", counts.Diagnostics, len(streamed.Diagnostics))
	}
}

// The §7.3 check runs once per consumer handoff, never twice over the same
// terms: a per-segment read reports each offending term exactly once.
func TestConflictingReifierReportedOncePerSegmentRead(t *testing.T) {
	reifier := 0
	terms := append(tripleTermTerms(), model.Term{Kind: model.Triple, Reifier: &reifier})
	w := writer.New("generic")
	w.AddTerms(terms)
	w.AddReifies([]model.ReifierEntry{
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 3}},
		{RID: 0, SPO: model.Triple3{S: 1, P: 2, O: 4}},
	})

	fs := ReadFileSegments(w.ToBytes())
	if fs.Fatal != nil {
		t.Fatalf("fatal: %#v", fs.Fatal)
	}
	if len(fs.Segments) != 1 {
		t.Fatalf("segments = %d, want 1", len(fs.Segments))
	}
	if got, want := tripleTermDiagCodes(fs.Segments[0]), []string{"ConflictingReifier"}; !reflect.DeepEqual(got, want) {
		t.Fatalf("diagnostics = %v, want %v", got, want)
	}
}

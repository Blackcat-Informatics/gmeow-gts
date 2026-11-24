// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/v2"
	"go.blackcatinformatics.ca/gts/wire"
)

func projectFixture(t *testing.T, name string) (*v2.RDFProjection, error) {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("..", "..", "vectors", "v2", "P3", name+".bin"))
	if err != nil {
		t.Fatal(err)
	}
	return v2.ReadWire(data, v2.DefaultReadCapacity()).WholeRDFProjection()
}

func TestWholeRDFProjectionDatasetRoundTrip(t *testing.T) {
	projection, err := projectFixture(t, "project__round-trip-rdf")
	if err != nil {
		t.Fatal(err)
	}
	terms, quads, empty := projection.Dataset()
	if len(quads) != 5 || len(empty) != 1 || terms[empty[0]].Value != "http://e/g3" {
		t.Fatalf("projection has %d quads and empty graphs %v, terms %v", len(quads), empty, terms)
	}
	var nested bool
	for _, quad := range quads {
		if triple := terms[quad.O].Triple; triple != nil {
			nested = terms[triple.O].Triple != nil && terms[quad.S].Kind == model.Bnode
		}
	}
	if !nested {
		t.Fatal("projection lost the nested triple term or its blank subject")
	}
	data, err := v2.ImportRDF(terms, quads, empty)
	if err != nil {
		t.Fatal(err)
	}
	readBack, err := v2.ReadWire(data, v2.DefaultReadCapacity()).WholeRDFProjection()
	if err != nil {
		t.Fatal(err)
	}
	backTerms, backQuads, backEmpty := readBack.Dataset()
	if len(backQuads) != len(quads) || len(backEmpty) != 1 || backTerms[backEmpty[0]].Value != terms[empty[0]].Value {
		t.Fatalf("reimported projection has %d quads and empty graphs %v", len(backQuads), backEmpty)
	}
}

func TestWholeRDFProjectionIdentities(t *testing.T) {
	for _, tc := range []struct {
		name  string
		count int
		text  string
	}{
		{"project__reifier-annotation", 2, `<http://e/r> <http://e/q> <http://e/v> <http://e/g> .`},
		{"project__annotation-equals-quad", 1, `<http://e/r>`},
		{"project__binding-quad-collision", 1, `#reifies> <<( <http://e/s> <http://e/p> <http://e/o> )>>`},
		{"project__hidden-lines", 3, `<http://e/o2>`},
		{"project__language-case", 2, `"x"@EN`},
		{"replayed-segment-blank-node.blank-segment-twice", 2, `_:b`},
	} {
		t.Run(tc.name, func(t *testing.T) {
			projection, err := projectFixture(t, tc.name)
			if err != nil {
				t.Fatal(err)
			}
			text, err := projection.NQuads()
			if err != nil || strings.Count(text, "\n") != tc.count || !strings.Contains(text, tc.text) {
				t.Fatalf("N-Quads = %q, error %v", text, err)
			}
			if tc.name == "replayed-segment-blank-node.blank-segment-twice" {
				terms, quads, _ := projection.Dataset()
				if terms[quads[0].S].Value == terms[quads[1].S].Value {
					t.Fatal("equal labels in separate segments must name different blank nodes")
				}
			}
		})
	}
}

func TestWholeRDFProjectionKeepsEmptyLanguage(t *testing.T) {
	projection, err := projectFixture(t, "project__dirlangstring-and-empty-language.empty-language")
	if err != nil {
		t.Fatal(err)
	}
	terms, quads, _ := projection.Dataset()
	if len(quads) != 1 || terms[quads[0].O].Lang == nil || *terms[quads[0].O].Lang != "" {
		t.Fatalf("expected an explicit empty language, got %v", terms)
	}
	if text, err := projection.NQuads(); text != "" || err == nil || !strings.Contains(err.Error(), "invalid RDF language tag") {
		t.Fatalf("N-Quads = %q, error %v", text, err)
	}
}

func requireFrameRefusal(t *testing.T, kind uint64, content, payload []byte, reason string) {
	t.Helper()
	writer, err := v2.NewSegmentWriter(v2.SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(kind, content, payload); err != nil {
		t.Fatal(err)
	}
	projection, err := v2.ReadWire(writer.Bytes(), v2.DefaultReadCapacity()).WholeRDFProjection()
	if projection != nil || err == nil || !strings.Contains(err.Error(), reason) {
		t.Fatalf("projection = %v, error %v, want %s", projection, err, reason)
	}
}

func TestRDFProjectionRejectsMalformedColumnBeforeCountMismatch(t *testing.T) {
	content, err := wire.Encode([]int{2, 2})
	if err != nil {
		t.Fatal(err)
	}
	// The graph run covers one row. The subject column is also ill typed.
	payload, err := wire.Encode([]any{[][]any{{[]int{1}, 1}}, [][]int{{1, 2}}, []int{1, 0}, []any{false, 0}})
	if err != nil {
		t.Fatal(err)
	}
	requireFrameRefusal(t, 2, content, payload, "wrong_type")
	requireFrameRefusal(t, 2, content, []byte{0xff}, "wrong_length")
}

func TestRDFProjectionRejectsDuplicateAnnotationsBeforeResolvingTerms(t *testing.T) {
	content, err := wire.Encode([]int{2, 3})
	if err != nil {
		t.Fatal(err)
	}
	// The first subject is undefined. The two later rows repeat each other.
	payload, err := wire.Encode([]any{[][]any{{[]int{1}, 3}}, []int{1, -1, 0}, []int{0, 0, 0}, []int{0, 0, 0}, []int{1, 0, 0}})
	if err != nil {
		t.Fatal(err)
	}
	requireFrameRefusal(t, 4, content, payload, "unordered_or_repeated")
}

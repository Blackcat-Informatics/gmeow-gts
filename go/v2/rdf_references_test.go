// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"bytes"
	"strings"
	"testing"

	"github.com/zeebo/blake3"
	"go.blackcatinformatics.ca/gts/v2"
	"go.blackcatinformatics.ca/gts/wire"
)

func TestRDFProjectionSelectsFirstUsableValue(t *testing.T) {
	for _, tc := range []struct{ part, name, reason string }{
		{"P3", "terms-literal-ref-first-candidate.failing-first", ""},
		{"P3", "terms-literal-ref-length-pair.length-4", "missing_value"},
		{"P3", "terms-literal-ref-not-utf8", "ill_formed_term"},
		{"P4", "rebuild-value-first-verified.first-fails", ""},
		{"P4", "rebuild-value-other-segment.value-resolves", ""},
		{"P4", "rebuild-value-other-segment.unit-in-term-segment", "chunk"},
		{"P4", "pack-recipe-trailing-count", "wrong_length"},
	} {
		t.Run(tc.name, func(t *testing.T) {
			projection, err := projectCorpus(t, tc.part, tc.name)
			if tc.reason != "" {
				if err == nil || !strings.HasSuffix(err.Error(), ": "+tc.reason) {
					t.Fatalf("projection error = %v, want %s", err, tc.reason)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			want := "<http://e/s> <http://e/p> \"abc\"^^<http://e/dt> .\n"
			if got, err := projection.NQuads(); err != nil || got != want {
				t.Fatalf("N-Quads = %q, error %v", got, err)
			}
		})
	}
}

func fixtureUnitDigest(body []byte) []byte {
	hasher := blake3.NewDeriveKey("gts2 unit")
	_, _ = hasher.Write(body)
	return hasher.Sum(nil)
}

func appendReferencedLiterals(t *testing.T, writer *v2.SegmentWriter, digest []byte) {
	t.Helper()
	const datatype = "http://e/dt"
	appendProjectionFrame(t, writer, 1, []any{0, [][]int{{1, 2}, {5, 2}, {7, 1}}, 2, 1}, []any{
		[][]any{
			{1, []int{0, 0}, []int{10, len(datatype)}, []byte("http://e/s" + datatype)},
			{5, [][]byte{digest, digest}, []int{2, 2}, []int{1, 1}, []int{1, 1}, []int{}},
			{7, [][]byte{digest}, []int{2}},
		},
		[]string{},
	})
	appendProjectionFrame(t, writer, 2, []int{2, 3}, []any{
		[][]any{{[]int{1}, 3}}, [][]int{{0, 3}}, []int{2, 1, 1}, []int{0, 0, 0},
	})
}

func TestRDFProjectionRebuildsPackValueAcrossUnits(t *testing.T) {
	writer, err := v2.NewSegmentWriter(v2.SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	first, last := []byte{0xc3}, []byte{0xa9}
	unitFirst, unitLast := fixtureUnitDigest(first), fixtureUnitDigest(last)
	digest := fixtureUnitDigest([]byte("é"))
	entries, err := wire.Encode([][]any{{unitFirst, 1}, {unitLast, 1}})
	if err != nil {
		t.Fatal(err)
	}
	// The value comes before every unit in the directory. One digest has
	// a damaged copy followed by a verified copy.
	appendProjectionFrame(t, writer, 13, []int{4, 3}, []any{
		[][]any{
			{4, []any{digest, 2, 2}, len(entries)},
			{1, [][]byte{unitFirst}, 1},
			{1, [][]byte{unitFirst}, 1},
			{2, []any{unitLast, []int{1}}, 1},
		},
		bytes.Join([][]byte{entries, {0}, first, last}, nil),
	})
	appendReferencedLiterals(t, writer, digest)
	wireResult := v2.ReadWire(writer.Bytes(), v2.DefaultReadCapacity())
	if _, err := wireResult.WholeRDFProjectionWithValueLimit(5); err == nil || !strings.HasSuffix(err.Error(), ": projection_value_limit") {
		t.Fatalf("projection error = %v, want projection_value_limit", err)
	}
	// Two UTF-8 bytes plus four hex digits. The repeated lexical definition
	// uses the same materialized text.
	projection, err := wireResult.WholeRDFProjectionWithValueLimit(6)
	if err != nil {
		t.Fatal(err)
	}
	want := "<http://e/s> <http://e/s> \"é\"^^<http://e/dt> .\n" +
		"<http://e/s> <http://e/s> \"C3A9\"^^<http://www.w3.org/2001/XMLSchema#hexBinary> .\n"
	if got, err := projection.NQuads(); err != nil || got != want {
		t.Fatalf("N-Quads = %q, error %v", got, err)
	}
}

func TestRDFProjectionRebuildBudgetIncludesRecipes(t *testing.T) {
	for _, tc := range []struct {
		name string
		size uint64
	}{
		{"charged even with missing units", 1 << 37},
		{"refusal persists for small values", (1 << 37) + 1},
	} {
		t.Run(tc.name, func(t *testing.T) {
			writer, err := v2.NewSegmentWriter(v2.SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			body := []byte("ab")
			digest := fixtureUnitDigest(body)
			appendProjectionFrame(t, writer, 8, [][]byte{digest}, body)
			appendProjectionFrame(t, writer, 10, []any{digest, tc.size, []int{1}, 2}, [][]any{
				{digest, tc.size - 1}, {digest, 1},
			})
			appendProjectionFrame(t, writer, 11, []any{digest, 2, 1}, [][]any{{digest, 2}})
			appendReferencedLiterals(t, writer, digest)
			_, err = v2.ReadWire(writer.Bytes(), v2.DefaultReadCapacity()).WholeRDFProjection()
			if err == nil || !strings.HasSuffix(err.Error(), ": total_rebuilt") {
				t.Fatalf("projection error = %v, want total_rebuilt", err)
			}
		})
	}
}

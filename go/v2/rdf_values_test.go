// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"math"
	"strings"
	"testing"

	"github.com/fxamacker/cbor/v2"
	"go.blackcatinformatics.ca/gts/v2"
	"go.blackcatinformatics.ca/gts/wire"
)

func appendProjectionFrame(t *testing.T, writer *v2.SegmentWriter, kind uint64, content, payload any) {
	t.Helper()
	fields, err := wire.Encode(content)
	if err != nil {
		t.Fatal(err)
	}
	body, err := wire.Encode(payload)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Append(kind, fields, body); err != nil {
		t.Fatal(err)
	}
}

func TestRDFProjectionGeneratesInlineValueLexicalForms(t *testing.T) {
	minInteger := cbor.RawMessage{0x3b, 255, 255, 255, 255, 255, 255, 255, 255}
	cases := []struct {
		value             []any
		lexical, datatype string
	}{
		{[]any{1, false}, "false", "boolean"},
		{[]any{1, true}, "true", "boolean"},
		{[]any{2, uint64(math.MaxUint64)}, "18446744073709551615", "nonNegativeInteger"},
		{[]any{3, uint64(math.MaxUint64)}, "18446744073709551615", "integer"},
		{[]any{3, minInteger}, "-18446744073709551616", "integer"},
		{[]any{4, int64(-62135596800), 0}, "0001-01-01T00:00:00Z", "dateTime"},
		{[]any{4, -1, 500000000}, "1969-12-31T23:59:59.5Z", "dateTime"},
		{[]any{4, int64(253402300799), 999999999}, "9999-12-31T23:59:59.999999999Z", "dateTime"},
		{[]any{5, []byte{}}, "", "hexBinary"},
		{[]any{5, []byte{0, 0xff, 0x41}}, "00FF41", "hexBinary"},
	}
	writer, err := v2.NewSegmentWriter(v2.SegmentOptions{})
	if err != nil {
		t.Fatal(err)
	}
	values, objects, subjects := make([][]any, len(cases)), make([]int, len(cases)), make([]int, len(cases))
	for i, tc := range cases {
		values[i], objects[i] = tc.value, 1
	}
	appendProjectionFrame(t, writer, 1, []any{0, [][]int{{1, 1}, {6, len(cases)}}, 2, 1}, []any{
		[][]any{{1, []int{0}, []int{10}, []byte("http://e/s")}, {6, values}},
		[]string{},
	})
	appendProjectionFrame(t, writer, 2, []int{2, len(cases)}, []any{
		[][]any{{[]int{1}, len(cases)}}, [][]int{{0, len(cases)}}, objects, subjects,
	})
	projection, err := v2.ReadWire(writer.Bytes(), v2.DefaultReadCapacity()).WholeRDFProjection()
	if err != nil {
		t.Fatal(err)
	}
	got, err := projection.NQuads()
	if err != nil || strings.Count(got, "\n") != len(cases) {
		t.Fatalf("N-Quads = %q, error %v", got, err)
	}
	for _, tc := range cases {
		want := `"` + tc.lexical + `"^^<http://www.w3.org/2001/XMLSchema#` + tc.datatype + `> .`
		if !strings.Contains(got, want) {
			t.Errorf("missing %s from %q", want, got)
		}
	}
}

func TestRDFProjectionAnnotationStorageIdentity(t *testing.T) {
	for _, tc := range []struct {
		name          string
		first, second int
		repeated      bool
	}{
		{"literal aliases", 2, 3, false},
		{"nested aliases", 8, 9, false},
		{"inline repeat", 3, 4, true},
		{"nested inline repeat", 9, 10, true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			writer, err := v2.NewSegmentWriter(v2.SegmentOptions{})
			if err != nil {
				t.Fatal(err)
			}
			const datatype = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
			appendProjectionFrame(t, writer, 1, []any{0, [][]int{{1, 2}, {2, 1}, {6, 2}, {4, 6}}, 2, 1}, []any{
				[][]any{
					{1, []int{0, 0}, []int{10, len(datatype)}, []byte("http://e/s" + datatype)},
					{2, []int{0}, []int{1}, []byte("7"), []int{1}, []int{1}, []int{}},
					{6, [][]int{{2, 7}, {2, 7}}},
					{4, make([]int, 6), make([]int, 6), []int{2, 3, 4, 5, 6, 7}},
				},
				[]string{},
			})
			// Each of IDs 8 to 10 wraps one of the three literal definitions
			// through two levels of triple terms.
			appendProjectionFrame(t, writer, 4, []int{2, 2}, []any{
				[][]any{{[]int{1}, 2}}, []int{0, 0}, []int{0, 0}, []int{tc.first, tc.second - tc.first}, []int{1, 0},
			})
			projection, err := v2.ReadWire(writer.Bytes(), v2.DefaultReadCapacity()).WholeRDFProjection()
			if tc.repeated {
				if err == nil || !strings.Contains(err.Error(), "unordered_or_repeated") {
					t.Fatalf("projection error = %v, want repeated annotation", err)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			_, quads, _ := projection.Dataset()
			if len(quads) != 1 {
				t.Fatalf("projection has %d quads for RDF aliases", len(quads))
			}
		})
	}
}

func TestRDFProjectionRefusesMalformedInlineValues(t *testing.T) {
	for _, tc := range []struct{ fixture, reason string }{
		{"terms-value-kind-unregistered", "unregistered_value"},
		{"terms-value-time-range.before", "out_of_range"},
		{"terms-value-time-range.after", "out_of_range"},
		{"terms-value-time-range.nanoseconds", "out_of_range"},
	} {
		t.Run(tc.fixture, func(t *testing.T) {
			_, err := projectFixture(t, tc.fixture)
			if err == nil || !strings.Contains(err.Error(), tc.reason) {
				t.Fatalf("projection error = %v, want %s", err, tc.reason)
			}
		})
	}
}

func TestRDFProjectionDoesNotReadFloatAsInlineBoolean(t *testing.T) {
	content, err := wire.Encode([]any{0, [][]int{{6, 1}}, 2, 1})
	if err != nil {
		t.Fatal(err)
	}
	payload, err := wire.Encode([]any{
		[][]any{{6, [][]any{{1, cbor.RawMessage{0xf9, 0x00, 0x14}}}}},
		[]string{},
	})
	if err != nil {
		t.Fatal(err)
	}
	requireFrameRefusal(t, 1, content, payload, "wrong_type")
}

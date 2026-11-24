// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2_test

import (
	"os"
	"path/filepath"
	"slices"
	"testing"

	"go.blackcatinformatics.ca/gts/v2"
)

func TestClosingClaimsUseObservedItemsAndLastTypedIndex(t *testing.T) {
	cases := []struct {
		name   string
		reason v2.Reason
		claim  v2.Reason
	}{
		{"offsets-counts-opaque", "", ""},
		{"offsets-count-short", "", v2.IndexOffsets},
		{"offsets-count-long", "", v2.IndexOffsets},
		{"offsets-length-differs", "", v2.IndexOffsets},
		{"locator-last-index.first", "", v2.IndexLocator},
		{"locator-last-index.second", "", ""},
		{"locator-index-type-long-form", "", v2.IndexLocator},
		{"locator-short-form", v2.NotDeterministic, ""},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			wire := v2.InspectWire(vector(t, tc.name), v2.DefaultReadCapacity())
			items := wire.Segments[0].Items
			item := items[len(items)-1]
			if item.Kind != v2.OffsetsItem && item.Kind != v2.LocatorItem {
				t.Fatalf("expected closing item, got %+v", item)
			}
			if item.Reason != tc.reason || item.ClosingClaimFailure != tc.claim {
				t.Fatalf("closing item = %+v, want reason %q and claim %q", item, tc.reason, tc.claim)
			}
		})
	}
}

func TestClosingPlacementAndGrammarTakePrecedence(t *testing.T) {
	header := vector(t, "gen-record-two")
	offsets := []byte{0x82, 4, 0x81, byte(len(header))}
	locator := []byte{0x82, 3, 0}
	data := slices.Concat(header, offsets, locator, locator, header, offsets, []byte{0x82, 2})
	wire := v2.InspectWire(data, v2.DefaultReadCapacity())
	if wire.Stop != v2.TornTail || len(wire.Segments) != 2 {
		t.Fatalf("closing sequence = %+v", wire)
	}
	for _, item := range wire.Segments[0].Items[1:] {
		if item.Reason != v2.UnorderedOrRepeated || item.ClosingClaimFailure != "" {
			t.Fatalf("misplaced closing item = %+v", item)
		}
	}
	if item := wire.Segments[1].Items[1]; item.Reason != "" || item.ClosingClaimFailure != "" {
		t.Fatalf("final, matched offsets before torn item = %+v", item)
	}

	// An earlier length mismatch must not hide a malformed later member.
	invalid := slices.Concat(header, []byte{0, 0x82, 4, 0x82, 0, 0x61, 'a'})
	item := v2.InspectWire(invalid, v2.DefaultReadCapacity()).Segments[0].Items[2]
	if item.Reason != v2.WrongType || item.ClosingClaimFailure != "" {
		t.Fatalf("malformed length after mismatch = %+v", item)
	}
}

func TestClosingDiagnosticsPreserveRDFProjection(t *testing.T) {
	path := filepath.Join("..", "..", "vectors", "v2", "P3", "project__round-trip-rdf.bin")
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	fold := func(data []byte) string {
		projection, err := v2.ReadWire(data, v2.DefaultReadCapacity()).WholeRDFProjection()
		if err != nil {
			t.Fatal(err)
		}
		out, err := projection.NQuads()
		if err != nil {
			t.Fatal(err)
		}
		return out
	}
	expected := fold(data)
	for _, closing := range [][]byte{{0x82, 4, 0x80}, {0x82, 3, 0}} {
		if got := fold(slices.Concat(data, closing)); got != expected {
			t.Fatalf("closing item %x changed the RDF fold", closing)
		}
	}
}

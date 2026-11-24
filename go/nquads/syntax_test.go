// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package nquads_test

import (
	"testing"

	"go.blackcatinformatics.ca/gts/nquads"
)

func TestIRISyntaxDistinguishesIRIComponents(t *testing.T) {
	for _, tc := range []struct {
		iri  string
		want bool
	}{
		{"urn:é#résumé", true},
		{"https://名.example:99/%20?q=\uE000#x", true},
		{"https://[2001:db8::1]/x", true},
		{"https://[vF.a:b]/x", true},
		{"https://[::1%25zone]/x", false},
		{"https://[v1.%41]/x", false},
		{"https://example/%gg", false},
		{"https://example/#\uE000", false},
		{"https://example/\U0001FFFE", false},
		{"https://example/a>b", false},
		{"relative/path", false},
	} {
		if got := nquads.ValidIRI(tc.iri); got != tc.want {
			t.Errorf("ValidIRI(%q) = %t, want %t", tc.iri, got, tc.want)
		}
	}
}

func TestLanguageTagStructureForNQuads(t *testing.T) {
	for _, tc := range []struct {
		tag  string
		want bool
	}{
		{"EN-GB", true},
		{"zh-cmn-Hans-CN-a-foo-X-7", true},
		{"i-kLIngon", true},
		{"de-1901", true},
		{"x-private", true},
		{"x-", false},
		{"en--rtl", false},
		{"en-abcdefghi", false},
		{"en-a-b", false},
		{"i-Klingon", false},
		{"", false},
	} {
		if got := nquads.WellFormedLanguageTag(tc.tag); got != tc.want {
			t.Errorf("WellFormedLanguageTag(%q) = %t, want %t", tc.tag, got, tc.want)
		}
	}
}

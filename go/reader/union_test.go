// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"testing"

	"go.blackcatinformatics.ca/gts/model"
)

func TestUnionReifiedTripleKeyUsesMappedReifierValue(t *testing.T) {
	reifier := 0
	seg := &model.Graph{
		Terms: []model.Term{
			{Kind: model.Iri, Value: "urn:reifier"},
			{Kind: model.Triple, Reifier: &reifier},
		},
		Quads: []model.Quad{
			{S: 1, P: 0, O: 0},
			{S: 1, P: 0, O: 0},
		},
	}

	got := unionSegments([]*model.Graph{seg})
	if len(got.Terms) != 2 {
		t.Fatalf("union produced %d terms, want 2: %#v", len(got.Terms), got.Terms)
	}
	if len(got.Quads) != 1 {
		t.Fatalf("union produced %d quads, want 1: %#v", len(got.Quads), got.Quads)
	}
}

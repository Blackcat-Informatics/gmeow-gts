// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"strings"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/stream"
)

// quadTermIDs returns every term position of a quad, including the graph
// slot when present (§14.1): a vocabulary IRI used only as a graph name still
// rots a declaration.
func quadTermIDs(q model.Quad) []int {
	if q.G != nil {
		return []int{q.S, q.P, q.O, *q.G}
	}
	return []int{q.S, q.P, q.O}
}

// profileVocabs maps profile names to the spec-owned vocabulary they imply.
var profileVocabs = map[string]string{
	"files": "https://w3id.org/gts/files#",
}

// namespaceOf returns the IRI namespace up to and including the last '#' or '/'.
func namespaceOf(iri string) string {
	if i := strings.LastIndex(iri, "#"); i >= 0 {
		return iri[:i+1]
	}
	if i := strings.LastIndex(iri, "/"); i >= 0 {
		return iri[:i+1]
	}
	return iri
}

// profileCheck implements the §14.1 declared-vs-computed profile checks:
// vocabulary used without its profile declared is an error; a declared-but-
// unused profile is a warning. Returns (message, isError) pairs.
func profileCheck(seg *model.Graph) []struct {
	Msg   string
	IsErr bool
} {
	declared := make(map[string]struct{}, len(seg.SegmentProfiles))
	for _, p := range seg.SegmentProfiles {
		declared[p] = struct{}{}
	}
	used := make(map[string]struct{})
	for _, q := range seg.Quads {
		for _, tid := range quadTermIDs(q) {
			if tid < 0 || tid >= len(seg.Terms) {
				continue // never crash a report over a malformed reference
			}
			term := &seg.Terms[tid]
			if term.Kind != model.Iri || term.Value == "" {
				continue
			}
			ns := namespaceOf(term.Value)
			for _, vocab := range profileVocabs {
				if ns == vocab {
					used[ns] = struct{}{}
				}
			}
		}
	}
	var out []struct {
		Msg   string
		IsErr bool
	}
	for prof, vocab := range profileVocabs {
		_, declares := declared[prof]
		_, uses := used[vocab]
		if uses && !declares {
			out = append(out, struct {
				Msg   string
				IsErr bool
			}{fmt.Sprintf("profile error: segment uses %s vocabulary "+
				"but does not declare '%s'", vocab, prof), true})
		}
		if declares && !uses {
			out = append(out, struct {
				Msg   string
				IsErr bool
			}{fmt.Sprintf("profile warning: segment declares '%s' "+
				"but uses no %s vocabulary", prof, vocab), false})
		}
	}
	return out
}

// streamVocabCheck warns on stream# vocabulary in an unclaimed segment (§13.3).
//
// A warning, never an error: compaction-provenance quads legitimately survive
// nq → gts round trips and re-accretion — the error class is reserved for a
// claimed layout the bytes contradict (the reader's StreamableLayoutError).
func streamVocabCheck(seg *model.Graph) []string {
	claimed := len(seg.SegmentStreamable) > 0 && seg.SegmentStreamable[0].Claimed
	if claimed {
		return nil
	}
	for _, q := range seg.Quads {
		for _, tid := range quadTermIDs(q) {
			if tid < 0 || tid >= len(seg.Terms) {
				continue // never crash a report over a malformed reference
			}
			term := &seg.Terms[tid]
			if term.Kind == model.Iri && strings.HasPrefix(term.Value, stream.NS) {
				return []string{
					fmt.Sprintf("layout warning: segment uses %s vocabulary but does "+
						"not claim layout 'streamable' (§13.3)", stream.NS),
				}
			}
		}
	}
	return nil
}

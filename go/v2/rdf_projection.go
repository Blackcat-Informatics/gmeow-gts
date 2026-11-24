// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"fmt"
	"slices"
	"strconv"

	"go.blackcatinformatics.ca/gts/model"
)

const (
	reifierFrameType    = 3
	annotationFrameType = 4
	rdfReifies          = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"
)

// RDFProjection holds the quads and empty named graphs from a whole RDF projection.
type RDFProjection struct {
	terms       []rdfTerm
	quads       []rdfQuad
	emptyGraphs []int
}

// Dataset returns a detached term table, quad set, and empty graph names
// suitable for ImportRDF. Blank labels are assigned within this projection.
func (p *RDFProjection) Dataset() ([]RDFTerm, []model.Quad, []int) {
	terms := make([]RDFTerm, len(p.terms))
	for index, term := range p.terms {
		value := RDFTerm{Kind: term.kind, Value: term.text}
		switch term.kind {
		case model.Bnode:
			value.Value = blankName(index)
		case model.Triple:
			triple := term.triple
			value.Triple = &triple
		case model.Literal:
			if term.form == 1 {
				datatype := term.datatype
				value.Datatype = &datatype
			} else {
				language := term.lang
				value.Lang = &language
				value.Direction = rdfDirection(term.form)
			}
		}
		terms[index] = value
	}
	quads := make([]model.Quad, len(p.quads))
	for index, row := range p.quads {
		quad := model.Quad{S: row.subject, P: row.predicate, O: row.object}
		if row.graph >= 0 {
			graph := row.graph
			quad.G = &graph
		}
		quads[index] = quad
	}
	return terms, quads, slices.Clone(p.emptyGraphs)
}

func blankName(id int) string { return "b" + strconv.Itoa(id) }

func rdfDirection(form int) string {
	switch form {
	case 3:
		return "ltr"
	case 4:
		return "rtl"
	default:
		return ""
	}
}

type scopedRDFTerm struct {
	rdfTerm
	scope int
}

type rdfDefinition struct {
	rdf     int
	written int
}

type rdfWrittenKey struct {
	kind       uint64
	rdf        int
	components model.Triple3
}

// The wire dictionary keeps both RDF equality and the storage identity used
// to check repeated annotations before projection.
type rdfReadDictionary struct {
	entries []rdfDefinition
	written map[rdfWrittenKey]int
}

func (d *rdfReadDictionary) append(key rdfWrittenKey) {
	id, ok := d.written[key]
	if !ok {
		id = len(d.written)
		d.written[key] = id
	}
	d.entries = append(d.entries, rdfDefinition{rdf: key.rdf, written: id})
}

type rdfProjector struct {
	result   RDFProjection
	interned map[scopedRDFTerm]int
	scope    int
	formed   uint64
}

// WholeRDFProjection projects the RDF rows of wire-checked, header-led registry
// version 1 segments with inline dictionaries. It includes hidden rows.
// Unreadable items or unsupported term storage cause refusal. This operation
// makes no signature, profile, canonicality, or committed-prefix claim.
func (result WireResult) WholeRDFProjection() (*RDFProjection, error) {
	if result.Stop != "" {
		return nil, fmt.Errorf("GTS v2 framing stopped at byte %d: %s", result.StopOffset, result.Stop)
	}
	if result.Dispatch == UnsupportedGeneration {
		return nil, fmt.Errorf("unsupported GTS generation %d", result.Generation)
	}
	if result.Dispatch != SecondGeneration {
		return nil, fmt.Errorf("expected GTS generation 2, found %q", result.Dispatch)
	}
	p := rdfProjector{interned: make(map[scopedRDFTerm]int)}
	for ordinal, segment := range result.Segments {
		p.scope = ordinal
		if err := p.segment(segment); err != nil {
			return nil, fmt.Errorf("segment %d: %w", ordinal, err)
		}
	}
	return p.finish(), nil
}

func (p *rdfProjector) segment(segment Segment) error {
	if len(segment.Items) == 0 || segment.Items[0].Kind != HeaderItem ||
		segment.Opener.ID == nil || segment.Opener.Registry != 1 {
		return fmt.Errorf("RDF projection requires a readable registry version 1 header")
	}
	dictionary := rdfReadDictionary{written: make(map[rdfWrittenKey]int)}
	for _, item := range segment.Items[1:] {
		if item.Kind == LocatorItem || item.Kind == OffsetsItem {
			continue
		}
		if item.Frame == nil || item.Reason != "" {
			return fmt.Errorf("unreadable %s item at byte %d: %s", item.Kind, item.Offset, item.Reason)
		}
		if reason := p.frame(item.Frame, &dictionary); reason != "" {
			return fmt.Errorf("frame type %d at byte %d: %s", item.Frame.Type, item.Offset, reason)
		}
	}
	return nil
}

func (p *rdfProjector) frame(frame *Frame, dictionary *rdfReadDictionary) Reason {
	switch {
	case frame.Malformed != "":
		return frame.Malformed
	case frame.Link != Linked && frame.Link != Healed:
		if frame.Link == "" {
			return "link_unavailable"
		}
		return Reason(frame.Link)
	case frame.StoredMatches != nil && !*frame.StoredMatches:
		return "stored_mismatch"
	case frame.Payload.State != Verified:
		if frame.Payload.Reason != "" {
			return frame.Payload.Reason
		}
		return "payload_unavailable"
	}
	switch frame.Type {
	case termsFrameType:
		return p.readTerms(dictionary, frame.Content, frame.Payload.Decoded)
	case quadsFrameType, reifierFrameType, annotationFrameType, emptyGraphFrameType:
		return p.readRows(dictionary.entries, frame.Type, frame.Content, frame.Payload.Decoded)
	case 6, 7, 8, 9, 10, 11, 12, 13, commitType, 15, parityType:
		// No RDF row comes from these frame types in a whole projection.
		return ""
	default:
		return "unsupported_frame_type"
	}
}

func (p *rdfProjector) intern(term rdfTerm) int {
	key := scopedRDFTerm{rdfTerm: term}
	if term.kind == model.Bnode {
		key.scope = p.scope
	}
	if id, ok := p.interned[key]; ok {
		return id
	}
	id := len(p.result.terms)
	p.interned[key] = id
	p.result.terms = append(p.result.terms, term)
	return id
}

func (p *rdfProjector) resolve(c *rdfCursor, dictionary []rdfDefinition, id uint64, kinds ...model.TermKind) int {
	if c.reason != "" {
		return 0
	}
	if id >= uint64(len(dictionary)) {
		c.reason = OutOfRange
		return 0
	}
	value := dictionary[id].rdf
	c.check(len(kinds) == 0 || slices.Contains(kinds, p.result.terms[value].kind), ForbiddenCombination)
	return value
}

func (p *rdfProjector) node(c *rdfCursor, dictionary []rdfDefinition, id uint64) int {
	return p.resolve(c, dictionary, id, model.Iri, model.Bnode)
}

func (p *rdfProjector) finish() *RDFProjection {
	result := &p.result
	slices.SortFunc(result.quads, compareRDFQuads)
	result.quads = slices.Compact(result.quads)
	populated := make(map[int]struct{})
	for _, row := range result.quads {
		if row.graph >= 0 {
			populated[row.graph] = struct{}{}
		}
	}
	slices.Sort(result.emptyGraphs)
	result.emptyGraphs = slices.DeleteFunc(slices.Compact(result.emptyGraphs), func(id int) bool {
		_, hasRows := populated[id]
		return hasRows
	})
	result.keepUsedTerms()
	return result
}

func (p *RDFProjection) keepUsedTerms() {
	// A term is linked only to earlier terms. Mark its dependencies in
	// reverse order, then compact and assign new identifiers in wire order.
	ids := make([]int, len(p.terms))
	for _, row := range p.quads {
		ids[row.subject], ids[row.predicate], ids[row.object] = 1, 1, 1
		if row.graph >= 0 {
			ids[row.graph] = 1
		}
	}
	for _, id := range p.emptyGraphs {
		ids[id] = 1
	}
	for id := len(p.terms) - 1; id >= 0; id-- {
		if ids[id] == 0 {
			continue
		}
		term := p.terms[id]
		switch term.kind {
		case model.Triple:
			ids[term.triple.S], ids[term.triple.P], ids[term.triple.O] = 1, 1, 1
		case model.Literal:
			if term.form == 1 {
				ids[term.datatype] = 1
			}
		}
	}
	kept := 0
	for id, term := range p.terms {
		if ids[id] == 0 {
			continue
		}
		switch term.kind {
		case model.Triple:
			triple := term.triple
			term.triple = model.Triple3{S: ids[triple.S], P: ids[triple.P], O: ids[triple.O]}
		case model.Literal:
			if term.form == 1 {
				term.datatype = ids[term.datatype]
			}
		}
		ids[id] = kept
		p.terms[kept] = term
		kept++
	}
	clear(p.terms[kept:])
	p.terms = p.terms[:kept]
	for i := range p.quads {
		row := &p.quads[i]
		row.subject, row.predicate, row.object = ids[row.subject], ids[row.predicate], ids[row.object]
		if row.graph >= 0 {
			row.graph = ids[row.graph]
		}
	}
	for i, id := range p.emptyGraphs {
		p.emptyGraphs[i] = ids[id]
	}
}

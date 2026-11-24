// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"cmp"
	"fmt"
	"slices"
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/model"
)

// RDFTerm is a P3 term for import. Kind selects its fields and references
// index the input term table. For a literal, a non-nil Lang preserves the
// supplied language text. Datatype names an IRI for other literals, with
// xsd:string as its default.
type RDFTerm struct {
	Kind      model.TermKind
	Value     string
	Lang      *string
	Direction string
	Datatype  *int
	Triple    *model.Triple3
}

// ImportRDF writes the P3 RDF import of quads and empty named graphs. Each
// input ID indexes terms, and the order of the term table is immaterial. Blank
// labels belong to one segment. A literal without language or datatype means
// xsd:string. Triple terms carry their three components explicitly.
//
// The result is one committed segment with plain storage. Literal text of at
// least 1 MiB can use a value reference, and references also carry text that
// cannot fit an inline frame. The segment fits the default reader and projection
// limits. Unrepresentable terms return an error.
func ImportRDF(terms []RDFTerm, quads []model.Quad, emptyGraphs []int) ([]byte, error) {
	dict := rdfDictionary{
		source: terms, resolved: make([]int, len(terms)), visiting: make([]bool, len(terms)),
		interned: make(map[rdfTerm]int),
	}
	rows, names, err := dict.dataset(quads, emptyGraphs)
	if err != nil {
		return nil, err
	}
	return writeRDF(dict.terms, rows, names)
}

type rdfTerm struct {
	kind       model.TermKind
	text, lang string
	form       int
	datatype   int
	triple     model.Triple3
}

type rdfQuad struct {
	graph, subject, predicate, object int
}

type rdfDictionary struct {
	source   []RDFTerm
	resolved []int // A mapped ID plus one. Zero means unresolved.
	visiting []bool
	terms    []rdfTerm
	interned map[rdfTerm]int
}

func (d *rdfDictionary) dataset(quads []model.Quad, empty []int) ([]rdfQuad, []int, error) {
	rows := make([]rdfQuad, 0, len(quads))
	populated := make(map[int]struct{})
	for index, quad := range quads {
		row, err := d.quad(quad)
		if err != nil {
			return nil, nil, fmt.Errorf("quad %d: %w", index, err)
		}
		rows = append(rows, row)
		if row.graph >= 0 {
			populated[row.graph] = struct{}{}
		}
	}
	slices.SortFunc(rows, compareRDFQuads)
	rows = slices.Compact(rows)

	names := make([]int, 0, len(empty))
	for _, index := range empty {
		id, err := d.reference(index, "empty graph", model.Iri, model.Bnode)
		if err != nil {
			return nil, nil, err
		}
		if _, ok := populated[id]; ok {
			return nil, nil, fmt.Errorf("empty graph term %d also names a graph in a quad", index)
		}
		names = append(names, id)
	}
	slices.Sort(names)
	return rows, slices.Compact(names), nil
}

func compareRDFQuads(a, b rdfQuad) int {
	if a.graph != b.graph {
		return cmp.Compare(a.graph, b.graph)
	}
	if a.predicate != b.predicate {
		return cmp.Compare(a.predicate, b.predicate)
	}
	if a.object != b.object {
		return cmp.Compare(a.object, b.object)
	}
	return cmp.Compare(a.subject, b.subject)
}

func (d *rdfDictionary) quad(quad model.Quad) (rdfQuad, error) {
	row := rdfQuad{graph: -1}
	var err error
	row.subject, err = d.reference(quad.S, "subject", model.Iri, model.Bnode)
	if err != nil {
		return rdfQuad{}, err
	}
	row.predicate, err = d.reference(quad.P, "predicate", model.Iri)
	if err != nil {
		return rdfQuad{}, err
	}
	row.object, err = d.object(quad.O)
	if err != nil {
		return rdfQuad{}, err
	}
	if quad.G != nil {
		row.graph, err = d.reference(*quad.G, "graph", model.Iri, model.Bnode)
	}
	return row, err
}

func (d *rdfDictionary) intern(term rdfTerm) int {
	if id, ok := d.interned[term]; ok {
		return id
	}
	id := len(d.terms)
	d.terms = append(d.terms, term)
	d.interned[term] = id
	return id
}

func (d *rdfDictionary) reference(index int, position string, kinds ...model.TermKind) (int, error) {
	if index < 0 || index >= len(d.source) {
		return 0, fmt.Errorf("%s term ID %d is outside the term table", position, index)
	}
	if !slices.Contains(kinds, d.source[index].Kind) {
		return 0, fmt.Errorf("term %d has a kind that is invalid for %s", index, position)
	}
	return d.atom(index)
}

func (d *rdfDictionary) atom(index int) (int, error) {
	if id := d.resolved[index]; id != 0 {
		return id - 1, nil
	}
	source := d.source[index]
	term := rdfTerm{kind: source.Kind, text: source.Value}
	switch source.Kind {
	case model.Bnode:
	case model.Iri, model.Literal:
		if !utf8.ValidString(source.Value) {
			return 0, fmt.Errorf("term %d text is invalid UTF-8", index)
		}
	default:
		return 0, fmt.Errorf("term %d has an unsupported kind %d", index, source.Kind)
	}
	if source.Kind == model.Literal {
		if err := d.literal(index, source, &term); err != nil {
			return 0, err
		}
	}
	id := d.intern(term)
	d.resolved[index] = id + 1
	return id, nil
}

func (d *rdfDictionary) literal(index int, source RDFTerm, term *rdfTerm) error {
	if source.Lang != nil {
		if source.Datatype != nil || !utf8.ValidString(*source.Lang) {
			return fmt.Errorf("invalid language literal at term %d", index)
		}
		term.lang = *source.Lang
		switch source.Direction {
		case "":
			term.form = 2
		case "ltr":
			term.form = 3
		case "rtl":
			term.form = 4
		default:
			return fmt.Errorf("invalid language direction at term %d", index)
		}
		return nil
	}
	if source.Direction != "" {
		return fmt.Errorf("term %d has direction without a language", index)
	}
	term.form = 1
	if source.Datatype == nil {
		term.datatype = d.intern(rdfTerm{kind: model.Iri, text: model.XSDString})
		return nil
	}
	datatype, err := d.reference(*source.Datatype, "datatype", model.Iri)
	term.datatype = datatype
	return err
}

func (d *rdfDictionary) object(index int) (int, error) {
	// Only a triple's object can be another triple. Follow the chain to
	// a known or atomic term, then intern its parents in reverse order.
	var triples []int
	var object int
	for {
		if index < 0 || index >= len(d.source) {
			return 0, fmt.Errorf("object term ID %d is outside the term table", index)
		}
		if id := d.resolved[index]; id != 0 {
			object = id - 1
			break
		}
		source := d.source[index]
		if source.Kind != model.Triple {
			var err error
			object, err = d.atom(index)
			if err != nil {
				return 0, err
			}
			break
		}
		if source.Triple == nil {
			return 0, fmt.Errorf("triple term %d requires explicit components", index)
		}
		if d.visiting[index] {
			return 0, fmt.Errorf("triple term %d refers to itself through a cycle", index)
		}
		d.visiting[index] = true
		triples = append(triples, index)
		index = source.Triple.O
	}
	for i := len(triples) - 1; i >= 0; i-- {
		index := triples[i]
		triple := d.source[index].Triple
		subject, err := d.reference(triple.S, "triple subject", model.Iri, model.Bnode)
		if err != nil {
			return 0, err
		}
		predicate, err := d.reference(triple.P, "triple predicate", model.Iri)
		if err != nil {
			return 0, err
		}
		object = d.intern(rdfTerm{kind: model.Triple, triple: model.Triple3{S: subject, P: predicate, O: object}})
		d.resolved[index] = object + 1
		d.visiting[index] = false
	}
	return object, nil
}

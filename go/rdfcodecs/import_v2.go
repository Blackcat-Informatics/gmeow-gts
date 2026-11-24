// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package rdfcodecs

import (
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/v2"
)

// ImportTurtleV2 parses Turtle with an initial base IRI and imports its
// default graph as GTS v2. An empty base leaves relative IRIs unresolved.
func ImportTurtleV2(text, baseIRI string) ([]byte, error) {
	return importTriGV2(text, baseIRI, false)
}

// ImportTriGV2 parses TriG with an initial base IRI and imports the dataset
// as GTS v2, including empty named graphs.
func ImportTriGV2(text, baseIRI string) ([]byte, error) {
	return importTriGV2(text, baseIRI, true)
}

func importTriGV2(text, baseIRI string, namedGraphs bool) ([]byte, error) {
	if !utf8.ValidString(text) {
		return nil, codecError("RDF text is not valid UTF-8")
	}
	parser := newTriGParser(text, namedGraphs)
	parser.rdf12 = true
	parser.baseIRI = baseIRI
	if err := parser.parse(); err != nil {
		return nil, err
	}
	return importStatementsV2(parser.statements, parser.graphNames)
}

// ImportRDFXMLV2 parses RDF/XML with an initial base IRI and imports its graph
// as GTS v2. The document may override the base with xml:base.
func ImportRDFXMLV2(text, baseIRI string) ([]byte, error) {
	statements, err := parseRDFXMLStatements(text, baseIRI, true)
	if err != nil {
		return nil, err
	}
	return importStatementsV2(statements, nil)
}

type statementTerms struct {
	terms   []v2.RDFTerm
	atoms   map[rdfNode]int
	triples map[model.Triple3]int
}

func importStatementsV2(statements []rdfStatement, graphs []rdfNode) ([]byte, error) {
	table := statementTerms{atoms: make(map[rdfNode]int), triples: make(map[model.Triple3]int)}
	quads := make([]model.Quad, 0, len(statements))
	populated := make(map[int]struct{})
	for _, statement := range statements {
		quad, err := table.quad(statement)
		if err != nil {
			return nil, err
		}
		quads = append(quads, quad)
		if quad.G != nil {
			populated[*quad.G] = struct{}{}
		}
	}
	var empty []int
	for _, graph := range graphs {
		id, err := table.atom(graph)
		if err != nil {
			return nil, err
		}
		if _, hasRows := populated[id]; !hasRows {
			empty = append(empty, id)
		}
	}
	return v2.ImportRDF(table.terms, quads, empty)
}

func (t *statementTerms) quad(statement rdfStatement) (model.Quad, error) {
	subject, err := t.atom(statement.subject)
	if err != nil {
		return model.Quad{}, err
	}
	predicate, err := t.atom(statement.predicate)
	if err != nil {
		return model.Quad{}, err
	}
	object, err := t.object(statement.object)
	if err != nil {
		return model.Quad{}, err
	}
	quad := model.Quad{S: subject, P: predicate, O: object}
	if statement.graph != nil {
		graph, err := t.atom(*statement.graph)
		if err != nil {
			return model.Quad{}, err
		}
		quad.G = &graph
	}
	return quad, nil
}

func (t *statementTerms) atom(node rdfNode) (int, error) {
	if id, ok := t.atoms[node]; ok {
		return id, nil
	}
	var term v2.RDFTerm
	switch node.kind {
	case nodeIRI:
		if !nquads.ValidIRI(node.value) {
			return 0, codecError("invalid RDF IRI %q", node.value)
		}
		term = v2.RDFTerm{Kind: model.Iri, Value: node.value}
	case nodeBNode:
		label := node.value
		if node.generated {
			// Text blank labels cannot start with a NUL. Keep the parser's
			// generated nodes in a separate namespace within the segment.
			label = "\x00" + label
		}
		term = v2.RDFTerm{Kind: model.Bnode, Value: label}
	case nodeLiteral:
		literal, err := t.literal(node)
		if err != nil {
			return 0, err
		}
		term = literal
	default:
		return 0, codecError("triple terms are only allowed as RDF objects")
	}
	id := len(t.terms)
	t.atoms[node] = id
	t.terms = append(t.terms, term)
	return id, nil
}

func (t *statementTerms) literal(node rdfNode) (v2.RDFTerm, error) {
	term := v2.RDFTerm{Kind: model.Literal, Value: node.value}
	if node.lang != "" {
		if !nquads.WellFormedLanguageTag(node.lang) {
			return v2.RDFTerm{}, codecError("invalid RDF language tag %q", node.lang)
		}
		term.Lang = &node.lang
		term.Direction = node.direction
	} else if node.datatype != "" {
		if node.datatype == model.RDFLangString || node.datatype == model.RDFDirLangString {
			return v2.RDFTerm{}, codecError("datatype %q requires a language", node.datatype)
		}
		datatype, err := t.atom(iriNode(node.datatype))
		if err != nil {
			return v2.RDFTerm{}, err
		}
		term.Datatype = &datatype
	}
	return term, nil
}

func (t *statementTerms) object(node rdfNode) (int, error) {
	var parents [][2]int
	for node.kind == nodeTriple {
		subject, err := t.atom(*node.s)
		if err != nil {
			return 0, err
		}
		predicate, err := t.atom(*node.p)
		if err != nil {
			return 0, err
		}
		parents = append(parents, [2]int{subject, predicate})
		node = *node.o
	}
	object, err := t.atom(node)
	if err != nil {
		return 0, err
	}
	for i := len(parents) - 1; i >= 0; i-- {
		triple := model.Triple3{S: parents[i][0], P: parents[i][1], O: object}
		if id, ok := t.triples[triple]; ok {
			object = id
			continue
		}
		object = len(t.terms)
		t.terms = append(t.terms, v2.RDFTerm{Kind: model.Triple, Triple: &triple})
		t.triples[triple] = object
	}
	return object, nil
}

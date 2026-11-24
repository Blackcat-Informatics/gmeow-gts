// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package rdfcodecs

import (
	"strings"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/v2"
)

// ExportNTriplesV2 writes the projected default graph as N-Triples.
func ExportNTriplesV2(p *v2.RDFProjection) (string, error) {
	return exportDefaultV2(p, "N-Triples")
}

// ExportTurtleV2 writes the projected default graph as Turtle.
func ExportTurtleV2(p *v2.RDFProjection) (string, error) {
	text, err := exportDefaultV2(p, "Turtle")
	return turtleDocument(text), err
}

func exportDefaultV2(p *v2.RDFProjection, format string) (string, error) {
	if p.HasNamedGraphs() {
		return "", codecError("%s cannot serialize a named graph", format)
	}
	return p.NQuads()
}

// ExportRDFXMLV2 writes the projected default graph as RDF/XML. Terms that
// RDF/XML cannot represent cause an error before any XML is returned.
func ExportRDFXMLV2(p *v2.RDFProjection) (string, error) {
	if p.HasNamedGraphs() {
		return "", codecError("RDF/XML cannot serialize a named graph")
	}
	terms, quads, _ := p.Dataset()
	nodes, err := xmlProjectionNodes(terms)
	if err != nil {
		return "", err
	}
	subjects := make(map[string][]property)
	subjectNodes := make(map[string]rdfNode)
	for _, quad := range quads {
		subject, predicate := nodes[quad.S], nodes[quad.P].value
		if err := validateXMLPredicate(predicate); err != nil {
			return "", err
		}
		key := subjectKey(subject)
		subjectNodes[key] = subject
		subjects[key] = append(subjects[key], property{predicate: predicate, object: nodes[quad.O]})
	}
	return writeRDFXMLDocument(subjects, subjectNodes, true)
}

func xmlProjectionNodes(terms []v2.RDFTerm) ([]rdfNode, error) {
	nodes := make([]rdfNode, len(terms))
	// Projection IDs of datatypes and triple components precede the terms
	// that reference them, so the XML views can share those nodes directly.
	for id, term := range terms {
		switch term.Kind {
		case model.Iri:
			if !nquads.ValidIRI(term.Value) {
				return nil, codecError("invalid RDF IRI %q", term.Value)
			}
			nodes[id] = iriNode(term.Value)
		case model.Bnode:
			nodes[id] = bnodeNode(term.Value)
		case model.Literal:
			if !validXMLText(term.Value) {
				return nil, codecError("RDF/XML cannot serialize a literal containing a forbidden XML character")
			}
			var language, datatype string
			if term.Lang != nil {
				language = *term.Lang
				if !nquads.WellFormedLanguageTag(language) {
					return nil, codecError("invalid RDF language tag %q", language)
				}
			} else if term.Datatype != nil {
				datatype = nodes[*term.Datatype].value
				if datatype == model.RDFLangString || datatype == model.RDFDirLangString {
					return nil, codecError("datatype %q requires a language", datatype)
				}
			}
			nodes[id] = literalNode(term.Value, language, term.Direction, datatype)
		case model.Triple:
			triple := term.Triple
			if err := validateXMLPredicate(nodes[triple.P].value); err != nil {
				return nil, err
			}
			nodes[id] = rdfNode{kind: nodeTriple, s: &nodes[triple.S], p: &nodes[triple.P], o: &nodes[triple.O]}
		}
	}
	return nodes, nil
}

func validateXMLPredicate(iri string) error {
	namespace, _, ok := splitPropertyIRI(iri)
	if !ok || namespace == "http://www.w3.org/2000/xmlns/" ||
		(namespace != rdfNS && strings.HasPrefix(namespace, rdfNS)) {
		return codecError("RDF/XML cannot serialize predicate IRI %q as an XML QName", iri)
	}
	if local, rdf := strings.CutPrefix(iri, rdfNS); rdf {
		switch local {
		case "RDF", "ID", "about", "annotation", "annotationNodeID", "parseType", "resource", "nodeID",
			"datatype", "version", "Description", "li", "aboutEach", "aboutEachPrefix", "bagID":
			return codecError("RDF/XML cannot serialize the RDF predicate %q as a property element", iri)
		}
	}
	return nil
}

func validXMLText(value string) bool {
	for _, ch := range value {
		switch {
		case ch == '\t' || ch == '\n' || ch == '\r':
		case ch >= 0x20 && ch <= 0xd7ff:
		case ch >= 0xe000 && ch <= 0xfffd:
		case ch >= 0x10000 && ch <= 0x10ffff:
		default:
			return false
		}
	}
	return true
}

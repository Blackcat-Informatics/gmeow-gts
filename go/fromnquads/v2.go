// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package fromnquads

import (
	"fmt"
	"strings"
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/v2"
)

// ImportV2 imports RDF 1.2 N-Quads as one committed GTS v2 segment.
// Equal statements collapse and quoted triple terms remain RDF values.
// Version directives are syntax hints. An error returns no output bytes.
func ImportV2(text string) ([]byte, error) {
	return importV2(text, true)
}

// ImportTriplesV2 imports RDF 1.2 N-Triples as a committed GTS v2 segment.
// A statement with a named graph is invalid N-Triples input.
func ImportTriplesV2(text string) ([]byte, error) {
	return importV2(text, false)
}

func importV2(text string, allowNamedGraphs bool) ([]byte, error) {
	if !utf8.ValidString(text) {
		return nil, ParseError{"input is not valid UTF-8"}
	}
	interner := newInterner()
	var quads []model.Quad
	for lineNumber := 1; text != ""; lineNumber++ {
		var line string
		line, text = cutRDFLine(text)
		parser := tokenizer{text: line, rdf12: true}
		parser.skipWS()
		if parser.lineFinished() {
			continue
		}
		if parser.consume("VERSION") {
			if err := parser.versionHint(); err != nil {
				return nil, fmt.Errorf("line %d: %w", lineNumber, err)
			}
			continue
		}
		quad, err := parser.rdfQuad(interner)
		if err != nil {
			return nil, fmt.Errorf("line %d: %w", lineNumber, err)
		}
		if !allowNamedGraphs && quad.G != nil {
			return nil, ParseError{fmt.Sprintf("line %d: N-Triples input cannot contain a named graph", lineNumber)}
		}
		quads = append(quads, quad)
	}
	return v2.ImportRDF(rdfTerms(interner.terms), quads, nil)
}

func cutRDFLine(text string) (string, string) {
	end := strings.IndexAny(text, "\r\n")
	if end < 0 {
		return text, ""
	}
	next := end + 1
	if text[end] == '\r' && next < len(text) && text[next] == '\n' {
		next++
	}
	return text[:end], text[next:]
}

func rdfTerms(terms []model.Term) []v2.RDFTerm {
	result := make([]v2.RDFTerm, len(terms))
	for index, term := range terms {
		converted := v2.RDFTerm{Kind: term.Kind, Value: term.Value, Triple: term.Triple, Datatype: term.Datatype}
		if term.Lang != "" {
			language := term.Lang
			converted.Lang = &language
			converted.Direction = term.Direction
		}
		result[index] = converted
	}
	return result
}

func (t *tokenizer) consume(token string) bool {
	if !strings.HasPrefix(t.text[t.pos:], token) {
		return false
	}
	t.pos += len(token)
	return true
}

func (t *tokenizer) lineFinished() bool {
	return t.pos == len(t.text) || t.text[t.pos] == '#'
}

func (t *tokenizer) finishRDFLine() error {
	t.skipWS()
	if t.lineFinished() {
		return nil
	}
	return ParseError{fmt.Sprintf("unexpected text after statement at byte %d", t.pos)}
}

func (t *tokenizer) versionHint() error {
	t.skipWS()
	if _, err := t.stringLiteral(); err != nil {
		return err
	}
	return t.finishRDFLine()
}

func (t *tokenizer) rdfQuad(terms *interner) (model.Quad, error) {
	subject, err := t.rdfResource(terms, false)
	if err != nil {
		return model.Quad{}, err
	}
	predicate, err := t.rdfResource(terms, true)
	if err != nil {
		return model.Quad{}, err
	}
	object, err := t.rdfObject(terms)
	if err != nil {
		return model.Quad{}, err
	}
	quad := model.Quad{S: subject, P: predicate, O: object}
	t.skipWS()
	if t.pos < len(t.text) && (t.text[t.pos] == '<' || t.text[t.pos] == '_') {
		graph, err := t.rdfResource(terms, false)
		if err != nil {
			return model.Quad{}, err
		}
		quad.G = &graph
		t.skipWS()
	}
	if !t.consume(".") {
		return model.Quad{}, ParseError{fmt.Sprintf("expected '.' at byte %d", t.pos)}
	}
	return quad, t.finishRDFLine()
}

func (t *tokenizer) rdfResource(terms *interner, iriOnly bool) (int, error) {
	t.skipWS()
	rest := t.text[t.pos:]
	if strings.HasPrefix(rest, "<") && !strings.HasPrefix(rest, "<<(") ||
		!iriOnly && strings.HasPrefix(rest, "_:") {
		a, err := t.atom()
		if err != nil {
			return 0, err
		}
		return terms.atom(a), nil
	}
	if iriOnly {
		return 0, ParseError{"predicate must be an IRI"}
	}
	return 0, ParseError{"expected an IRI or a blank node"}
}

func (t *tokenizer) rdfObject(terms *interner) (int, error) {
	var parents [][2]int
	t.skipWS()
	for t.consume("<<(") {
		subject, err := t.rdfResource(terms, false)
		if err != nil {
			return 0, err
		}
		predicate, err := t.rdfResource(terms, true)
		if err != nil {
			return 0, err
		}
		parents = append(parents, [2]int{subject, predicate})
		t.skipWS()
	}
	a, err := t.atom()
	if err != nil {
		return 0, err
	}
	object := terms.atom(a)
	for index := len(parents) - 1; index >= 0; index-- {
		t.skipWS()
		if !t.consume(")>>") {
			return 0, ParseError{fmt.Sprintf("expected ')>>' at byte %d", t.pos)}
		}
		parent := parents[index]
		object = terms.triple(parent[0], parent[1], object)
	}
	return object, nil
}

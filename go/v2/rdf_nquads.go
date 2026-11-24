// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

package v2

import (
	"fmt"
	"strings"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
)

// NQuads renders the quads using RDF 1.2 syntax. Empty named graphs remain
// available through Dataset. An unrepresentable term returns an empty string
// and an error.
func (p *RDFProjection) NQuads() (string, error) {
	var output strings.Builder
	for index, row := range p.quads {
		ids := [4]int{row.subject, row.predicate, row.object, row.graph}
		for position, id := range ids {
			if id < 0 {
				break
			}
			if position > 0 {
				output.WriteByte(' ')
			}
			if err := p.writeTerm(&output, id); err != nil {
				return "", fmt.Errorf("quad %d: %w", index, err)
			}
		}
		output.WriteString(" .\n")
	}
	return output.String(), nil
}

func (p *RDFProjection) writeTerm(out *strings.Builder, id int) error {
	// Nested triple terms are possible only in object position. Write their
	// openers in a loop and emit the closing tokens after the final atom.
	depth := 0
	for p.terms[id].kind == model.Triple {
		triple := p.terms[id].triple
		out.WriteString("<<( ")
		if err := p.writeAtom(out, triple.S); err != nil {
			return err
		}
		out.WriteByte(' ')
		if err := p.writeAtom(out, triple.P); err != nil {
			return err
		}
		out.WriteByte(' ')
		id = triple.O
		depth++
	}
	if err := p.writeAtom(out, id); err != nil {
		return err
	}
	for range depth {
		out.WriteString(" )>>")
	}
	return nil
}

func (p *RDFProjection) writeAtom(out *strings.Builder, id int) error {
	term := p.terms[id]
	switch term.kind {
	case model.Iri:
		return writeIRI(out, term.text)
	case model.Bnode:
		out.WriteString("_:")
		out.WriteString(blankName(id))
	case model.Literal:
		return p.writeLiteral(out, term)
	default:
		return fmt.Errorf("term %d breaks an RDF position rule", id)
	}
	return nil
}

func (p *RDFProjection) writeLiteral(out *strings.Builder, term rdfTerm) error {
	out.WriteByte('"')
	out.WriteString(nquads.EscapeLiteral(term.text))
	out.WriteByte('"')
	if term.form != 1 {
		if !nquads.WellFormedLanguageTag(term.lang) {
			return fmt.Errorf("invalid RDF language tag %q", term.lang)
		}
		out.WriteByte('@')
		out.WriteString(term.lang)
		if direction := rdfDirection(term.form); direction != "" {
			out.WriteString("--")
			out.WriteString(direction)
		}
		return nil
	}
	datatype := p.terms[term.datatype].text
	switch datatype {
	case model.RDFLangString, model.RDFDirLangString:
		return fmt.Errorf("datatype %q requires a language", datatype)
	case model.XSDString:
		return nil
	default:
		out.WriteString("^^")
		return writeIRI(out, datatype)
	}
}

func writeIRI(out *strings.Builder, text string) error {
	if !nquads.ValidIRI(text) {
		return fmt.Errorf("invalid RDF IRI %q", text)
	}
	out.WriteByte('<')
	out.WriteString(text)
	out.WriteByte('>')
	return nil
}

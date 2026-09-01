// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package nquads implements the GTS -> N-Quads transform (§14).
package nquads

import (
	"fmt"
	"strings"

	"go.blackcatinformatics.ca/gts/model"
)

const rdfReifies = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

// escape returns an N-Quads safe lexical form (§14).
func escape(lex string) string {
	var out strings.Builder
	for _, ch := range lex {
		switch ch {
		case '\\':
			out.WriteString("\\\\")
		case '"':
			out.WriteString("\\\"")
		case '\n':
			out.WriteString("\\n")
		case '\r':
			out.WriteString("\\r")
		case '\t':
			out.WriteString("\\t")
		default:
			if ch < 0x20 {
				fmt.Fprintf(&out, "\\u%04X", ch)
			} else {
				out.WriteRune(ch)
			}
		}
	}
	return out.String()
}

// render serialises the term at tid to its N-Quads surface form.
func render(g *model.Graph, tid int) string {
	return renderNested(g, tid, nil)
}

// renderNested serialises the term at tid, carrying the triple terms whose
// rendering is still open in open.
//
// A reifies row MAY name the very term that resolves through it (§7.3), so a
// projection MUST NOT recurse blindly: a self-reaching term degrades to the
// same blank node an unbound triple term already produces.
func renderNested(g *model.Graph, tid int, open []int) string {
	if tid < 0 || tid >= len(g.Terms) {
		return fmt.Sprintf("_:out_of_range_%d", tid)
	}
	if model.TermIsOpen(open, tid) {
		return fmt.Sprintf("_:unbound_triple_%d", tid)
	}
	t := &g.Terms[tid]
	switch t.Kind {
	case model.Iri:
		return fmt.Sprintf("<%s>", t.Value)
	case model.Bnode:
		if t.Value != "" {
			return fmt.Sprintf("_:%s", t.Value)
		}
		return fmt.Sprintf("_:b%d", tid)
	case model.Literal:
		lit := fmt.Sprintf("\"%s\"", escape(t.Value))
		if t.Lang != "" {
			if model.IsLiteralDirection(t.Direction) {
				return fmt.Sprintf("%s@%s--%s", lit, t.Lang, t.Direction)
			}
			return fmt.Sprintf("%s@%s", lit, t.Lang)
		}
		if t.Datatype != nil {
			return fmt.Sprintf("%s^^%s", lit, renderNested(g, *t.Datatype, open))
		}
		return lit
	case model.Triple:
		// Quoted triple (RDF 1.2 triple term): its own "tt" is authoritative; a
		// legacy "tt"-less term still resolves through its reifier (§7.3).
		if spo, ok := g.TripleOf(tid); ok {
			inner := model.OpenTerm(open, tid)
			return fmt.Sprintf("<<( %s %s %s )>>",
				renderNested(g, spo.S, inner),
				renderNested(g, spo.P, inner),
				renderNested(g, spo.O, inner))
		}
		// Degraded but syntactically valid: a triple term that states no triple
		// (no "tt", and no bound reifier) becomes a blank node.
		return fmt.Sprintf("_:unbound_triple_%d", tid)
	}
	return ""
}

// ToNQuads serialises a folded Graph to N-Quads text.
func ToNQuads(g *model.Graph) string {
	var lines []string
	for _, q := range g.Quads {
		triple := fmt.Sprintf("%s %s %s", render(g, q.S), render(g, q.P), render(g, q.O))
		if q.G != nil {
			lines = append(lines, fmt.Sprintf("%s %s .", triple, render(g, *q.G)))
		} else {
			lines = append(lines, fmt.Sprintf("%s .", triple))
		}
	}
	for _, r := range g.Reifiers {
		quoted := fmt.Sprintf("<<( %s %s %s )>>", render(g, r.SPO.S), render(g, r.SPO.P), render(g, r.SPO.O))
		triple := fmt.Sprintf("%s <%s> %s", render(g, r.RID), rdfReifies, quoted)
		if r.G != nil {
			lines = append(lines, fmt.Sprintf("%s %s .", triple, render(g, *r.G)))
		} else {
			lines = append(lines, fmt.Sprintf("%s .", triple))
		}
	}
	for _, a := range g.Annotations {
		triple := fmt.Sprintf("%s %s %s", render(g, a.S), render(g, a.P), render(g, a.O))
		if a.G != nil {
			lines = append(lines, fmt.Sprintf("%s %s .", triple, render(g, *a.G)))
		} else {
			lines = append(lines, fmt.Sprintf("%s .", triple))
		}
	}
	if len(lines) == 0 {
		return ""
	}
	return strings.Join(lines, "\n") + "\n"
}

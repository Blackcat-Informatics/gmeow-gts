// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package rdfcodecs provides Go RDF text-codec helpers for folded GTS graphs.
//
// N-Triples, Turtle, TriG, and RDF/XML conversions keep the GTS wire format
// unchanged. Text imports lower into the existing N-Quads importer so term
// interning, RDF 1.2 quoted-triple handling, and XSD diagnostics stay shared.
package rdfcodecs

import (
	"fmt"
	"sort"
	"strings"

	"go.blackcatinformatics.ca/gts/fromnquads"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/nquads"
	"go.blackcatinformatics.ca/gts/reader"
)

const (
	rdfNS        = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
	xsdNS        = "http://www.w3.org/2001/XMLSchema#"
	rdfType      = rdfNS + "type"
	rdfReifies   = rdfNS + "reifies"
	rdfFirst     = rdfNS + "first"
	rdfRest      = rdfNS + "rest"
	rdfNil       = rdfNS + "nil"
	rdfXMLLit    = rdfNS + "XMLLiteral"
	defaultBase  = ""
	errPrefixRDF = "RDF text"
)

// Error reports an RDF text codec error.
type Error struct {
	Detail string
}

func (e Error) Error() string { return e.Detail }

func codecError(format string, args ...interface{}) error {
	return Error{Detail: fmt.Sprintf(format, args...)}
}

// FromNTriples parses N-Triples text into canonical GTS bytes.
func FromNTriples(text string) ([]byte, error) {
	out, err := fromnquads.FromNQuads(text)
	if err != nil {
		return nil, codecError("%s syntax error: %v", errPrefixRDF, err)
	}
	g := reader.Read(out, true, nil)
	for _, q := range g.Quads {
		if q.G != nil {
			return nil, codecError("N-Triples input cannot contain named graph terms")
		}
	}
	return out, nil
}

// FromTurtle parses Turtle text into canonical GTS bytes.
func FromTurtle(text string) ([]byte, error) {
	nq, err := newTriGParser(text, false).parse()
	if err != nil {
		return nil, err
	}
	out, err := fromnquads.FromNQuads(nq)
	if err != nil {
		return nil, codecError("%s syntax error: %v", errPrefixRDF, err)
	}
	return out, nil
}

// FromTriG parses TriG text into canonical GTS bytes.
func FromTriG(text string) ([]byte, error) {
	nq, err := newTriGParser(text, true).parse()
	if err != nil {
		return nil, err
	}
	out, err := fromnquads.FromNQuads(nq)
	if err != nil {
		return nil, codecError("%s syntax error: %v", errPrefixRDF, err)
	}
	return out, nil
}

// ToNTriples serializes a folded default graph to N-Triples.
func ToNTriples(g *model.Graph) (string, error) {
	if err := ensureDefaultGraph(g, "N-Triples"); err != nil {
		return "", err
	}
	return nquads.ToNQuads(g), nil
}

// ToTurtle serializes a folded default graph to Turtle.
func ToTurtle(g *model.Graph) (string, error) {
	if err := ensureDefaultGraph(g, "Turtle"); err != nil {
		return "", err
	}
	body := nquads.ToNQuads(g)
	if body == "" {
		return "", nil
	}
	return fmt.Sprintf("@prefix rdf: <%s> .\n@prefix xsd: <%s> .\n\n%s", rdfNS, xsdNS, body), nil
}

// ToTriG serializes a folded graph to TriG, preserving named graph quads.
func ToTriG(g *model.Graph) (string, error) {
	if g == nil || (len(g.Quads) == 0 && len(g.Reifiers) == 0 && len(g.Annotations) == 0) {
		return "", nil
	}
	lines := []string{fmt.Sprintf("@prefix rdf: <%s> .", rdfNS), ""}
	var openGraph string
	closeGraph := func() {
		if openGraph != "" {
			lines = append(lines, "}")
			openGraph = ""
		}
	}
	for _, q := range g.Quads {
		triple := fmt.Sprintf("%s %s %s .", renderGraphTerm(g, q.S), renderGraphTerm(g, q.P), renderGraphTerm(g, q.O))
		if q.G != nil {
			graph := renderGraphTerm(g, *q.G)
			if openGraph != graph {
				closeGraph()
				lines = append(lines, graph+" {")
				openGraph = graph
			}
			lines = append(lines, "  "+triple)
		} else {
			closeGraph()
			lines = append(lines, triple)
		}
	}
	closeGraph()
	for _, r := range g.Reifiers {
		quoted := fmt.Sprintf("<<( %s %s %s )>>", renderGraphTerm(g, r.SPO.S), renderGraphTerm(g, r.SPO.P), renderGraphTerm(g, r.SPO.O))
		lines = append(lines, fmt.Sprintf("%s rdf:reifies %s .", renderGraphTerm(g, r.RID), quoted))
	}
	for _, a := range g.Annotations {
		lines = append(lines, fmt.Sprintf("%s %s %s .", renderGraphTerm(g, a.S), renderGraphTerm(g, a.P), renderGraphTerm(g, a.O)))
	}
	return strings.Join(lines, "\n") + "\n", nil
}

func ensureDefaultGraph(g *model.Graph, format string) error {
	if g == nil {
		return nil
	}
	for _, q := range g.Quads {
		if q.G != nil {
			return codecError("%s cannot serialize named graph %s", format, renderGraphTerm(g, *q.G))
		}
	}
	return nil
}

type rdfNodeKind int

const (
	nodeIRI rdfNodeKind = iota
	nodeBNode
	nodeLiteral
	nodeTriple
)

type rdfNode struct {
	kind      rdfNodeKind
	value     string
	lang      string
	direction string
	datatype  string
	s         *rdfNode
	p         *rdfNode
	o         *rdfNode
}

func iriNode(value string) rdfNode {
	return rdfNode{kind: nodeIRI, value: value}
}

func bnodeNode(value string) rdfNode {
	return rdfNode{kind: nodeBNode, value: value}
}

func literalNode(value, lang, direction, datatype string) rdfNode {
	return rdfNode{kind: nodeLiteral, value: value, lang: lang, direction: direction, datatype: datatype}
}

func tripleNode(s, p, o rdfNode) rdfNode {
	return rdfNode{kind: nodeTriple, s: &s, p: &p, o: &o}
}

func (n rdfNode) token() string {
	switch n.kind {
	case nodeIRI:
		return fmt.Sprintf("<%s>", n.value)
	case nodeBNode:
		return fmt.Sprintf("_:%s", n.value)
	case nodeLiteral:
		lit := fmt.Sprintf("\"%s\"", escapeLiteral(n.value))
		if n.lang != "" {
			if n.direction == "ltr" || n.direction == "rtl" {
				return fmt.Sprintf("%s@%s--%s", lit, n.lang, n.direction)
			}
			return fmt.Sprintf("%s@%s", lit, n.lang)
		}
		if n.datatype != "" {
			return fmt.Sprintf("%s^^<%s>", lit, n.datatype)
		}
		return lit
	case nodeTriple:
		return fmt.Sprintf("<<( %s %s %s )>>", n.s.token(), n.p.token(), n.o.token())
	default:
		return ""
	}
}

func (n rdfNode) graphName() bool {
	return n.kind == nodeIRI || n.kind == nodeBNode
}

func escapeLiteral(lex string) string {
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

func renderGraphTerm(g *model.Graph, tid int) string {
	if g == nil || tid < 0 || tid >= len(g.Terms) {
		return fmt.Sprintf("_:out_of_range_%d", tid)
	}
	t := &g.Terms[tid]
	switch t.Kind {
	case model.Iri:
		if t.Value == rdfReifies {
			return "rdf:reifies"
		}
		return fmt.Sprintf("<%s>", t.Value)
	case model.Bnode:
		if t.Value != "" {
			return fmt.Sprintf("_:%s", t.Value)
		}
		return fmt.Sprintf("_:b%d", tid)
	case model.Literal:
		lit := fmt.Sprintf("\"%s\"", escapeLiteral(t.Value))
		if t.Lang != "" {
			if model.IsLiteralDirection(t.Direction) {
				return fmt.Sprintf("%s@%s--%s", lit, t.Lang, t.Direction)
			}
			return fmt.Sprintf("%s@%s", lit, t.Lang)
		}
		if t.Datatype != nil {
			return fmt.Sprintf("%s^^%s", lit, renderGraphTerm(g, *t.Datatype))
		}
		return lit
	case model.Triple:
		rid := tid
		if t.Reifier != nil {
			rid = *t.Reifier
		}
		if spo, ok := g.Reifier(rid); ok {
			return fmt.Sprintf("<<( %s %s %s )>>", renderGraphTerm(g, spo.S), renderGraphTerm(g, spo.P), renderGraphTerm(g, spo.O))
		}
		return fmt.Sprintf("_:unbound_triple_%d", tid)
	default:
		return ""
	}
}

func graphTermNode(g *model.Graph, tid int) (rdfNode, error) {
	if g == nil || tid < 0 || tid >= len(g.Terms) {
		return rdfNode{}, codecError("term id %d is out of range", tid)
	}
	t := &g.Terms[tid]
	switch t.Kind {
	case model.Iri:
		return iriNode(t.Value), nil
	case model.Bnode:
		label := t.Value
		if label == "" {
			label = fmt.Sprintf("b%d", tid)
		}
		return bnodeNode(label), nil
	case model.Literal:
		datatype := ""
		if t.Datatype != nil {
			dt, err := graphTermNode(g, *t.Datatype)
			if err != nil {
				return rdfNode{}, err
			}
			if dt.kind != nodeIRI {
				return rdfNode{}, codecError("literal datatype term %d is not an IRI", *t.Datatype)
			}
			datatype = dt.value
		}
		return literalNode(t.Value, t.Lang, t.Direction, datatype), nil
	case model.Triple:
		rid := tid
		if t.Reifier != nil {
			rid = *t.Reifier
		}
		spo, ok := g.Reifier(rid)
		if !ok {
			return rdfNode{}, codecError("triple term %d has no reifier binding", tid)
		}
		s, err := graphTermNode(g, spo.S)
		if err != nil {
			return rdfNode{}, err
		}
		p, err := graphTermNode(g, spo.P)
		if err != nil {
			return rdfNode{}, err
		}
		o, err := graphTermNode(g, spo.O)
		if err != nil {
			return rdfNode{}, err
		}
		return tripleNode(s, p, o), nil
	default:
		return rdfNode{}, codecError("unsupported term kind %d", t.Kind)
	}
}

func sortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for key := range m {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

func hasIRIScheme(value string) bool {
	if len(value) == 0 || !isASCIILetter(rune(value[0])) {
		return false
	}
	for i := 1; i < len(value); i++ {
		ch := value[i]
		if ch == ':' {
			return true
		}
		if !isIRISchemeByte(ch) {
			return false
		}
	}
	return false
}

func isIRISchemeByte(ch byte) bool {
	return isASCIILetter(rune(ch)) || (ch >= '0' && ch <= '9') || ch == '+' || ch == '-' || ch == '.'
}

func isASCIILetter(ch rune) bool {
	return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')
}

func removeDotSegments(path string) string {
	absolute := strings.HasPrefix(path, "/")
	keepTrailingSlash := strings.HasSuffix(path, "/") ||
		strings.HasSuffix(path, "/.") ||
		strings.HasSuffix(path, "/..") ||
		path == "." ||
		path == ".."
	segments := []string{}
	for _, segment := range strings.Split(path, "/") {
		switch segment {
		case "", ".":
		case "..":
			if len(segments) > 0 {
				segments = segments[:len(segments)-1]
			}
		default:
			segments = append(segments, segment)
		}
	}
	normalized := strings.Join(segments, "/")
	if absolute {
		normalized = "/" + normalized
	}
	if keepTrailingSlash && !strings.HasSuffix(normalized, "/") {
		normalized += "/"
	}
	if normalized == "" && absolute {
		normalized = "/"
	}
	return normalized
}

func splitRawPathSuffix(raw string) (string, string) {
	split := strings.IndexAny(raw, "?#")
	if split < 0 {
		return raw, ""
	}
	return raw[:split], raw[split:]
}

func splitBaseForPath(base string) (string, string) {
	schemeEnd := strings.IndexByte(base, ':')
	if schemeEnd < 0 {
		return "", base
	}
	schemePrefix := base[:schemeEnd+1]
	rest := base[schemeEnd+1:]
	if afterSlashes, ok := strings.CutPrefix(rest, "//"); ok {
		authorityEnd := strings.IndexByte(afterSlashes, '/')
		if authorityEnd < 0 {
			return schemePrefix + "//" + afterSlashes, ""
		}
		return schemePrefix + "//" + afterSlashes[:authorityEnd], afterSlashes[authorityEnd:]
	}
	return schemePrefix, rest
}

func resolveRelativeIRI(base, raw string) string {
	if hasIRIScheme(raw) {
		return raw
	}
	baseWithoutFragment := base
	if before, _, ok := strings.Cut(base, "#"); ok {
		baseWithoutFragment = before
	}
	if raw == "" {
		return baseWithoutFragment
	}
	if strings.HasPrefix(raw, "#") {
		return baseWithoutFragment + raw
	}
	baseWithoutQuery := baseWithoutFragment
	if before, _, ok := strings.Cut(baseWithoutFragment, "?"); ok {
		baseWithoutQuery = before
	}
	if strings.HasPrefix(raw, "?") {
		return baseWithoutQuery + raw
	}
	if strings.HasPrefix(raw, "//") {
		if schemeEnd := strings.IndexByte(base, ':'); schemeEnd >= 0 {
			return base[:schemeEnd] + ":" + raw
		}
		return raw
	}
	prefix, basePath := splitBaseForPath(baseWithoutQuery)
	rawPath, suffix := splitRawPathSuffix(raw)
	mergedPath := rawPath
	if !strings.HasPrefix(rawPath, "/") {
		baseDir := "/"
		if basePath != "" {
			if idx := strings.LastIndexByte(basePath, '/'); idx >= 0 {
				baseDir = basePath[:idx+1]
			} else {
				baseDir = ""
			}
		}
		mergedPath = baseDir + rawPath
	}
	return prefix + removeDotSegments(mergedPath) + suffix
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package fromnquads implements the N-Quads -> GTS inverse-of-fold transform.
package fromnquads

import (
	"fmt"
	"strconv"
	"strings"
	"unicode/utf8"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/writer"
	"go.blackcatinformatics.ca/gts/xsd"
)

const rdfReifies = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

// ParseError reports malformed N-Quads(-star) input.
type ParseError struct {
	Detail string
}

func (e ParseError) Error() string { return e.Detail }

type atom struct {
	kind         model.TermKind
	value        string
	lang         string
	direction    string
	datatype     string
	hasLang      bool
	hasDirection bool
	hasDType     bool
}

type tripleNode struct {
	s, p, o node
}

type node struct {
	atom   *atom
	triple *tripleNode
}

func atomNode(kind model.TermKind, value string) node {
	return node{atom: &atom{kind: kind, value: value}}
}

func isAtom(n node, kind model.TermKind) bool {
	return n.atom != nil && n.atom.kind == kind
}

func isASCIILetterOrDigit(b byte) bool {
	return (b >= '0' && b <= '9') || (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z')
}

func isBNodeChar(b byte) bool {
	return isASCIILetterOrDigit(b) || b == '_' || b == '-' || b == '.'
}

func isLangChar(b byte) bool {
	return isASCIILetterOrDigit(b) || b == '-'
}

func isHexDigit(b byte) bool {
	return (b >= '0' && b <= '9') || (b >= 'a' && b <= 'f') || (b >= 'A' && b <= 'F')
}

type tokenizer struct {
	text string
	pos  int
}

func newTokenizer(text string) *tokenizer {
	return &tokenizer{text: text}
}

func (t *tokenizer) skipWS() {
	for t.pos < len(t.text) && (t.text[t.pos] == ' ' || t.text[t.pos] == '\t') {
		t.pos++
	}
}

func (t *tokenizer) atEnd() bool {
	t.skipWS()
	return t.pos >= len(t.text) || t.text[t.pos] == '.'
}

func (t *tokenizer) node() (node, error) {
	t.skipWS()
	if t.pos >= len(t.text) {
		return node{}, ParseError{fmt.Sprintf("unexpected end of line: %q", t.text)}
	}
	if strings.HasPrefix(t.text[t.pos:], "<<(") {
		triple, err := t.quotedTriple()
		if err != nil {
			return node{}, err
		}
		return node{triple: &triple}, nil
	}
	switch t.text[t.pos] {
	case '<':
		value, err := t.iri()
		if err != nil {
			return node{}, err
		}
		return atomNode(model.Iri, value), nil
	case '_':
		value, err := t.bnode()
		if err != nil {
			return node{}, err
		}
		return atomNode(model.Bnode, value), nil
	case '"':
		a, err := t.literal()
		if err != nil {
			return node{}, err
		}
		return node{atom: &a}, nil
	default:
		return node{}, ParseError{fmt.Sprintf("unexpected token at %d in %q", t.pos, t.text)}
	}
}

func (t *tokenizer) iri() (string, error) {
	if t.pos >= len(t.text) || t.text[t.pos] != '<' {
		return "", ParseError{fmt.Sprintf("bad IRI in %q", t.text)}
	}
	start := t.pos + 1
	rel := strings.IndexByte(t.text[start:], '>')
	if rel < 0 {
		return "", ParseError{fmt.Sprintf("unterminated IRI in %q", t.text)}
	}
	end := start + rel
	t.pos = end + 1
	return t.text[start:end], nil
}

func (t *tokenizer) bnode() (string, error) {
	if !strings.HasPrefix(t.text[t.pos:], "_:") {
		return "", ParseError{fmt.Sprintf("bad blank node in %q", t.text)}
	}
	t.pos += 2
	start := t.pos
	for t.pos < len(t.text) && isBNodeChar(t.text[t.pos]) {
		t.pos++
	}
	if t.pos > start && t.text[t.pos-1] == '.' {
		t.pos--
	}
	if t.pos == start {
		return "", ParseError{fmt.Sprintf("empty blank node label in %q", t.text)}
	}
	return t.text[start:t.pos], nil
}

func (t *tokenizer) bumpRune() (rune, bool) {
	if t.pos >= len(t.text) {
		return 0, false
	}
	ch, size := utf8.DecodeRuneInString(t.text[t.pos:])
	if ch == utf8.RuneError && size == 0 {
		return 0, false
	}
	t.pos += size
	return ch, true
}

func (t *tokenizer) literal() (atom, error) {
	if ch, ok := t.bumpRune(); !ok || ch != '"' {
		return atom{}, ParseError{fmt.Sprintf("bad literal in %q", t.text)}
	}
	var value strings.Builder
	for {
		ch, ok := t.bumpRune()
		if !ok {
			return atom{}, ParseError{fmt.Sprintf("unterminated literal in %q", t.text)}
		}
		switch ch {
		case '\\':
			escaped, err := t.escape()
			if err != nil {
				return atom{}, err
			}
			value.WriteRune(escaped)
		case '"':
			a := atom{kind: model.Literal, value: value.String()}
			if t.pos < len(t.text) && t.text[t.pos] == '@' {
				t.pos++
				start := t.pos
				for t.pos < len(t.text) && isLangChar(t.text[t.pos]) {
					t.pos++
				}
				a.lang = t.text[start:t.pos]
				if a.lang == "" {
					return atom{}, ParseError{fmt.Sprintf("empty language tag in %q", t.text)}
				}
				if sep := strings.LastIndex(a.lang, "--"); sep >= 0 {
					rawDirection := a.lang[sep+2:]
					if rawDirection != "ltr" && rawDirection != "rtl" {
						return atom{}, ParseError{fmt.Sprintf("invalid literal direction in %q", t.text)}
					}
					language := a.lang[:sep]
					if language == "" {
						return atom{}, ParseError{fmt.Sprintf("empty language tag in %q", t.text)}
					}
					a.lang = language
					a.direction = rawDirection
					a.hasDirection = true
				}
				a.hasLang = true
			} else if strings.HasPrefix(t.text[t.pos:], "^^") {
				t.pos += 2
				t.skipWS()
				datatype, err := t.iri()
				if err != nil {
					return atom{}, err
				}
				a.datatype = datatype
				a.hasDType = true
			}
			return a, nil
		default:
			value.WriteRune(ch)
		}
	}
}

func (t *tokenizer) escape() (rune, error) {
	ch, ok := t.bumpRune()
	if !ok {
		return 0, ParseError{fmt.Sprintf("bad escape at end of %q", t.text)}
	}
	switch ch {
	case '\\':
		return '\\', nil
	case '"':
		return '"', nil
	case 'b':
		return '\b', nil
	case 'f':
		return '\f', nil
	case 'n':
		return '\n', nil
	case 'r':
		return '\r', nil
	case 't':
		return '\t', nil
	case 'u', 'U':
		width := 4
		if ch == 'U' {
			width = 8
		}
		end := t.pos + width
		if end > len(t.text) {
			return 0, ParseError{fmt.Sprintf("short unicode escape in %q", t.text)}
		}
		raw := t.text[t.pos:end]
		for _, b := range []byte(raw) {
			if !isHexDigit(b) {
				return 0, ParseError{fmt.Sprintf("bad unicode escape \\%c%s in %q", ch, raw, t.text)}
			}
		}
		t.pos = end
		code, err := strconv.ParseInt(raw, 16, 32)
		if err != nil {
			return 0, ParseError{fmt.Sprintf("bad unicode escape \\%c%s: %v", ch, raw, err)}
		}
		r := rune(code)
		if !utf8.ValidRune(r) {
			return 0, ParseError{fmt.Sprintf("invalid unicode scalar \\%c%s", ch, raw)}
		}
		return r, nil
	default:
		return 0, ParseError{fmt.Sprintf("unsupported escape \\%c in %q", ch, t.text)}
	}
}

func (t *tokenizer) quotedTriple() (tripleNode, error) {
	t.pos += 3
	s, err := t.node()
	if err != nil {
		return tripleNode{}, err
	}
	p, err := t.node()
	if err != nil {
		return tripleNode{}, err
	}
	o, err := t.node()
	if err != nil {
		return tripleNode{}, err
	}
	t.skipWS()
	if !strings.HasPrefix(t.text[t.pos:], ")>>") {
		return tripleNode{}, ParseError{fmt.Sprintf("unterminated quoted triple in %q", t.text)}
	}
	t.pos += 3
	return tripleNode{s: s, p: p, o: o}, nil
}

type termKey struct {
	tag          string
	kind         model.TermKind
	value        string
	lang         string
	direction    string
	datatype     string
	hasLang      bool
	hasDirection bool
	hasDType     bool
	s, p, o      int
}

type interner struct {
	ids   map[termKey]int
	terms []model.Term
}

func newInterner() *interner {
	return &interner{ids: map[termKey]int{}}
}

func (i *interner) atom(a atom) int {
	key := termKey{
		tag:          "atom",
		kind:         a.kind,
		value:        a.value,
		lang:         a.lang,
		direction:    a.direction,
		datatype:     a.datatype,
		hasLang:      a.hasLang,
		hasDirection: a.hasDirection,
		hasDType:     a.hasDType,
	}
	if id, ok := i.ids[key]; ok {
		return id
	}
	term := model.Term{Kind: a.kind, Value: a.value}
	if a.kind == model.Literal && a.hasDType {
		dt := i.atom(atom{kind: model.Iri, value: a.datatype})
		term.Datatype = &dt
	}
	if a.hasLang {
		term.Lang = a.lang
	}
	if a.hasDirection {
		term.Direction = a.direction
	}
	id := len(i.terms)
	i.terms = append(i.terms, term)
	i.ids[key] = id
	return id
}

func (i *interner) node(n node, reifiers *[]model.ReifierEntry) int {
	if n.atom != nil {
		return i.atom(*n.atom)
	}
	s := i.node(n.triple.s, reifiers)
	p := i.node(n.triple.p, reifiers)
	o := i.node(n.triple.o, reifiers)
	key := termKey{tag: "triple", s: s, p: p, o: o}
	if id, ok := i.ids[key]; ok {
		return id
	}
	id := len(i.terms)
	rid := id
	i.terms = append(i.terms, model.Term{Kind: model.Triple, Reifier: &rid})
	i.ids[key] = id
	_ = setReifier(reifiers, id, model.Triple3{S: s, P: p, O: o}, nil)
	return id
}

func setReifier(reifiers *[]model.ReifierEntry, rid int, spo model.Triple3, graph *int) error {
	for idx := range *reifiers {
		if (*reifiers)[idx].RID == rid {
			if (*reifiers)[idx].SPO != spo {
				return ParseError{fmt.Sprintf("conflicting rdf:reifies binding for reifier term %d", rid)}
			}
			if sameOptionalInt((*reifiers)[idx].G, graph) {
				return nil
			}
		}
	}
	*reifiers = append(*reifiers, model.ReifierEntry{RID: rid, SPO: spo, G: copyOptionalInt(graph)})
	return nil
}

func sameOptionalInt(a, b *int) bool {
	if a == nil || b == nil {
		return a == nil && b == nil
	}
	return *a == *b
}

func copyOptionalInt(value *int) *int {
	if value == nil {
		return nil
	}
	copied := *value
	return &copied
}

func validateStatement(nodes []node, line string) error {
	if !isSubjectTerm(nodes[0]) {
		return ParseError{fmt.Sprintf("invalid subject term: %q", line)}
	}
	if !isAtom(nodes[1], model.Iri) {
		return ParseError{fmt.Sprintf("predicate must be IRI: %q", line)}
	}
	if !isObjectTerm(nodes[2]) {
		return ParseError{fmt.Sprintf("invalid object term: %q", line)}
	}
	if len(nodes) > 3 && !isGraphNameTerm(nodes[3]) {
		return ParseError{fmt.Sprintf("invalid graph name term: %q", line)}
	}
	return nil
}

func isSubjectTerm(n node) bool {
	return isAtom(n, model.Iri) || isAtom(n, model.Bnode) || n.triple != nil
}

func isObjectTerm(n node) bool {
	return isAtom(n, model.Iri) || isAtom(n, model.Bnode) || isAtom(n, model.Literal) || n.triple != nil
}

func isGraphNameTerm(n node) bool {
	return isAtom(n, model.Iri) || isAtom(n, model.Bnode)
}

// FromNQuads parses N-Quads(-star) text into a canonical GTS file.
func FromNQuads(text string) ([]byte, error) {
	var statements [][]node
	for _, raw := range strings.Split(text, "\n") {
		line := strings.TrimSpace(raw)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		tok := newTokenizer(line)
		var nodes []node
		for !tok.atEnd() {
			n, err := tok.node()
			if err != nil {
				return nil, err
			}
			nodes = append(nodes, n)
		}
		if len(nodes) != 3 && len(nodes) != 4 {
			return nil, ParseError{fmt.Sprintf("expected 3 or 4 terms, got %d: %q", len(nodes), line)}
		}
		if err := validateStatement(nodes, line); err != nil {
			return nil, err
		}
		statements = append(statements, nodes)
	}

	interner := newInterner()
	var reifiers []model.ReifierEntry
	var pendingQuads []model.Quad

	for _, nodes := range statements {
		s, p, o := nodes[0], nodes[1], nodes[2]
		var gname *node
		if len(nodes) == 4 {
			gname = &nodes[3]
		}

		if s.atom != nil && isAtom(p, model.Iri) && p.atom.value == rdfReifies && o.triple != nil {
			rid := interner.atom(*s.atom)
			var graph *int
			if gname != nil {
				gid := interner.node(*gname, &reifiers)
				graph = &gid
			}
			if err := setReifier(&reifiers, rid, model.Triple3{
				S: interner.node(o.triple.s, &reifiers),
				P: interner.node(o.triple.p, &reifiers),
				O: interner.node(o.triple.o, &reifiers),
			}, graph); err != nil {
				return nil, err
			}
			continue
		}

		q := model.Quad{
			S: interner.node(s, &reifiers),
			P: interner.node(p, &reifiers),
			O: interner.node(o, &reifiers),
		}
		if gname != nil {
			gid := interner.node(*gname, &reifiers)
			q.G = &gid
		}
		pendingQuads = append(pendingQuads, q)
	}

	reifierIDs := make(map[int]struct{}, len(reifiers))
	for _, r := range reifiers {
		reifierIDs[r.RID] = struct{}{}
	}
	var quads []model.Quad
	var annotations []model.AnnotationEntry
	for _, q := range pendingQuads {
		if _, ok := reifierIDs[q.S]; ok {
			annotations = append(annotations, model.AnnotationEntry{
				S: q.S,
				P: q.P,
				O: q.O,
				G: copyOptionalInt(q.G),
			})
			continue
		}
		quads = append(quads, q)
	}

	w := writer.New("dist")
	if len(interner.terms) > 0 {
		w.AddTerms(interner.terms)
	}
	if len(quads) > 0 {
		w.AddQuads(quads)
	}
	if len(reifiers) > 0 {
		w.AddReifies(reifiers)
	}
	if len(annotations) > 0 {
		w.AddAnnot(annotations)
	}
	if illTyped := xsd.IllTypedLiteralsInTerms(interner.terms); len(illTyped) > 0 {
		w.AddMeta(map[interface{}]interface{}{
			xsd.IllTypedLiteralMetaKey: xsd.IllTypedLiteralsMetadata(illTyped),
		})
	}
	return w.ToBytes(), nil
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package rdfcodecs

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

type trigParser struct {
	text             string
	pos              int
	prefixes         map[string]string
	nquads           []string
	baseIRI          string
	bnodeCounter     int
	allowNamedGraphs bool
}

func newTriGParser(text string, allowNamedGraphs bool) *trigParser {
	return &trigParser{
		text:             text,
		prefixes:         map[string]string{"rdf": rdfNS},
		allowNamedGraphs: allowNamedGraphs,
	}
}

func (p *trigParser) parse() (string, error) {
	for !p.eof() {
		p.skipWSAndComments()
		if p.eof() {
			break
		}
		if p.consume("@prefix") {
			if err := p.prefixDirective(true); err != nil {
				return "", err
			}
			continue
		}
		if p.consume("@base") {
			if err := p.baseDirective(true); err != nil {
				return "", err
			}
			continue
		}
		if p.consumeKeyword("PREFIX") {
			if err := p.prefixDirective(false); err != nil {
				return "", err
			}
			continue
		}
		if p.consumeKeyword("BASE") {
			if err := p.baseDirective(false); err != nil {
				return "", err
			}
			continue
		}
		if p.consumeKeyword("GRAPH") {
			if !p.allowNamedGraphs {
				return "", codecError("Turtle input cannot contain GRAPH blocks")
			}
			graph, err := p.term(nil)
			if err != nil {
				return "", err
			}
			if err := p.graphBlock(graph); err != nil {
				return "", err
			}
			continue
		}
		if p.consumeChar('{') {
			if !p.allowNamedGraphs {
				return "", codecError("Turtle input cannot contain graph blocks")
			}
			if err := p.defaultGraphBlockAfterOpen(); err != nil {
				return "", err
			}
			continue
		}

		first, err := p.term(nil)
		if err != nil {
			return "", err
		}
		p.skipWSAndComments()
		if p.consumeChar('{') {
			if !p.allowNamedGraphs {
				return "", codecError("Turtle input cannot contain graph blocks")
			}
			if err := p.graphBlockAfterOpen(first); err != nil {
				return "", err
			}
		} else if err := p.statementAfterSubject(first, nil); err != nil {
			return "", err
		}
	}
	if len(p.nquads) == 0 {
		return "", nil
	}
	return strings.Join(p.nquads, "\n") + "\n", nil
}

func (p *trigParser) eof() bool {
	p.skipWSAndComments()
	return p.pos >= len(p.text)
}

func (p *trigParser) skipWSAndComments() {
	for {
		for {
			ch, ok := p.peekChar()
			if !ok || !unicode.IsSpace(ch) {
				break
			}
			p.bumpChar()
		}
		if ch, ok := p.peekChar(); ok && ch == '#' {
			for {
				ch, ok := p.bumpChar()
				if !ok || ch == '\n' {
					break
				}
			}
			continue
		}
		return
	}
}

func (p *trigParser) peekChar() (rune, bool) {
	if p.pos >= len(p.text) {
		return 0, false
	}
	ch, _ := utf8.DecodeRuneInString(p.text[p.pos:])
	return ch, true
}

func (p *trigParser) bumpChar() (rune, bool) {
	if p.pos >= len(p.text) {
		return 0, false
	}
	ch, size := utf8.DecodeRuneInString(p.text[p.pos:])
	p.pos += size
	return ch, true
}

func (p *trigParser) consume(text string) bool {
	p.skipWSAndComments()
	if strings.HasPrefix(p.text[p.pos:], text) {
		p.pos += len(text)
		return true
	}
	return false
}

func (p *trigParser) consumeKeyword(keyword string) bool {
	p.skipWSAndComments()
	rest := p.text[p.pos:]
	if len(rest) < len(keyword) || !strings.EqualFold(rest[:len(keyword)], keyword) {
		return false
	}
	boundary := true
	if len(rest) > len(keyword) {
		ch, _ := utf8.DecodeRuneInString(rest[len(keyword):])
		boundary = unicode.IsSpace(ch) || strings.ContainsRune("{}[]()<>_\";,. ", ch)
	}
	if !boundary {
		return false
	}
	p.pos += len(keyword)
	return true
}

func (p *trigParser) consumeChar(ch rune) bool {
	p.skipWSAndComments()
	if got, ok := p.peekChar(); ok && got == ch {
		p.bumpChar()
		return true
	}
	return false
}

func (p *trigParser) expectChar(ch rune, context string) error {
	if p.consumeChar(ch) {
		return nil
	}
	return codecError("expected %q %s at byte %d", ch, context, p.pos)
}

func (p *trigParser) prefixDirective(requireDot bool) error {
	prefix, err := p.prefixLabel()
	if err != nil {
		return err
	}
	iri, err := p.iri()
	if err != nil {
		return err
	}
	p.prefixes[prefix] = iri
	if requireDot {
		return p.expectChar('.', "after @prefix directive")
	}
	p.consumeChar('.')
	return nil
}

func (p *trigParser) baseDirective(requireDot bool) error {
	iri, err := p.iriRaw()
	if err != nil {
		return err
	}
	if !hasIRIScheme(iri) {
		return codecError("base IRI must be absolute: %q", iri)
	}
	p.baseIRI = iri
	if requireDot {
		return p.expectChar('.', "after @base directive")
	}
	p.consumeChar('.')
	return nil
}

func (p *trigParser) prefixLabel() (string, error) {
	p.skipWSAndComments()
	start := p.pos
	for {
		ch, ok := p.peekChar()
		if !ok {
			break
		}
		if ch == ':' {
			label := p.text[start:p.pos]
			p.bumpChar()
			return label, nil
		}
		if (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z') || unicode.IsDigit(ch) || ch == '_' || ch == '-' {
			p.bumpChar()
			continue
		}
		break
	}
	return "", codecError("expected prefix label at byte %d", start)
}

func (p *trigParser) term(graph *rdfNode) (rdfNode, error) {
	p.skipWSAndComments()
	switch {
	case strings.HasPrefix(p.text[p.pos:], "<<("):
		return p.parenthesizedQuotedTriple(graph)
	case strings.HasPrefix(p.text[p.pos:], "<<"):
		return p.legacyQuotedTriple(graph)
	}
	ch, ok := p.peekChar()
	if !ok {
		return rdfNode{}, codecError("unexpected end of Turtle/TriG input")
	}
	switch ch {
	case '<':
		iri, err := p.iri()
		if err != nil {
			return rdfNode{}, err
		}
		return iriNode(iri), nil
	case '_':
		label, err := p.bnode()
		if err != nil {
			return rdfNode{}, err
		}
		return bnodeNode(label), nil
	case '"':
		return p.literal()
	case '[':
		return p.blankNodePropertyList(graph)
	case '(':
		return p.collection(graph)
	default:
		if lit, ok, err := p.numericLiteral(); ok || err != nil {
			return lit, err
		}
		if lit, ok := p.booleanLiteral(); ok {
			return lit, nil
		}
		iri, err := p.prefixedName()
		if err != nil {
			return rdfNode{}, err
		}
		return iriNode(iri), nil
	}
}

func (p *trigParser) booleanLiteral() (rdfNode, bool) {
	for _, token := range []string{"true", "false"} {
		if !strings.HasPrefix(p.text[p.pos:], token) {
			continue
		}
		end := p.pos + len(token)
		if end < len(p.text) && !isTriGTermDelimiterByte(p.text[end]) {
			continue
		}
		p.pos = end
		return literalNode(token, "", "", xsdNS+"boolean"), true
	}
	return rdfNode{}, false
}

func (p *trigParser) numericLiteral() (rdfNode, bool, error) {
	start := p.pos
	pos := start
	if pos < len(p.text) && (p.text[pos] == '+' || p.text[pos] == '-') {
		pos++
	}
	digitStart := pos
	for pos < len(p.text) && isASCIIDigit(p.text[pos]) {
		pos++
	}
	hasDigitsBeforeDot := pos > digitStart
	hasDot := false
	if pos < len(p.text) && p.text[pos] == '.' && pos+1 < len(p.text) && isASCIIDigit(p.text[pos+1]) {
		hasDot = true
		pos++
		for pos < len(p.text) && isASCIIDigit(p.text[pos]) {
			pos++
		}
	}
	if !hasDigitsBeforeDot && !hasDot {
		return rdfNode{}, false, nil
	}
	hasExponent := false
	if pos < len(p.text) && (p.text[pos] == 'e' || p.text[pos] == 'E') {
		hasExponent = true
		pos++
		if pos < len(p.text) && (p.text[pos] == '+' || p.text[pos] == '-') {
			pos++
		}
		exponentStart := pos
		for pos < len(p.text) && isASCIIDigit(p.text[pos]) {
			pos++
		}
		if pos == exponentStart {
			return rdfNode{}, true, codecError("invalid numeric literal %q", p.text[start:pos])
		}
	}
	if pos < len(p.text) && !isTriGTermDelimiterByte(p.text[pos]) {
		return rdfNode{}, false, nil
	}
	datatype := xsdNS + "integer"
	if hasExponent {
		datatype = xsdNS + "double"
	} else if hasDot {
		datatype = xsdNS + "decimal"
	}
	lexical := p.text[start:pos]
	p.pos = pos
	return literalNode(lexical, "", "", datatype), true, nil
}

func isASCIIDigit(ch byte) bool {
	return ch >= '0' && ch <= '9'
}

func isASCIIHexDigit(ch byte) bool {
	return isASCIIDigit(ch) || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}

func isTriGTermDelimiterByte(ch byte) bool {
	switch ch {
	case '{', '}', '[', ']', '(', ')', '<', '>', ';', ',', '.':
		return true
	default:
		return ch <= ' '
	}
}

func (p *trigParser) predicate() (rdfNode, error) {
	if p.consumeKeyword("a") {
		return iriNode(rdfType), nil
	}
	return p.term(nil)
}

func (p *trigParser) iriRaw() (string, error) {
	p.skipWSAndComments()
	if ch, ok := p.bumpChar(); !ok || ch != '<' {
		return "", codecError("expected IRI at byte %d", p.pos)
	}
	start := p.pos
	for {
		ch, ok := p.bumpChar()
		if !ok {
			return "", codecError("unterminated IRI starting at byte %d", start-1)
		}
		if ch == '>' {
			end := p.pos - 1
			raw := p.text[start:end]
			for _, ch := range raw {
				if ch < 0x20 || unicode.IsSpace(ch) || strings.ContainsRune("<>\"{}|\\^`", ch) {
					return "", codecError("invalid character in IRI starting at byte %d", start-1)
				}
			}
			return raw, nil
		}
	}
}

func (p *trigParser) iri() (string, error) {
	raw, err := p.iriRaw()
	if err != nil {
		return "", err
	}
	return p.resolveIRI(raw), nil
}

func (p *trigParser) resolveIRI(raw string) string {
	if hasIRIScheme(raw) || p.baseIRI == defaultBase {
		return raw
	}
	return resolveRelativeIRI(p.baseIRI, raw)
}

func (p *trigParser) bnode() (string, error) {
	p.skipWSAndComments()
	if !strings.HasPrefix(p.text[p.pos:], "_:") {
		return "", codecError("expected blank node at byte %d", p.pos)
	}
	p.pos += 2
	start := p.pos
	for p.pos < len(p.text) {
		b := p.text[p.pos]
		if (b >= 'A' && b <= 'Z') || (b >= 'a' && b <= 'z') || (b >= '0' && b <= '9') || b == '_' || b == '-' || b == '.' {
			p.pos++
			continue
		}
		break
	}
	if p.pos > start && p.text[p.pos-1] == '.' {
		p.pos--
	}
	if p.pos == start {
		return "", codecError("empty blank node label")
	}
	return p.text[start:p.pos], nil
}

func (p *trigParser) nextBNode() rdfNode {
	id := p.bnodeCounter
	p.bnodeCounter++
	return bnodeNode(fmt.Sprintf("gts_%d", id))
}

func (p *trigParser) literal() (rdfNode, error) {
	p.skipWSAndComments()
	if ch, ok := p.bumpChar(); !ok || ch != '"' {
		return rdfNode{}, codecError("expected literal")
	}
	var value strings.Builder
	for {
		ch, ok := p.bumpChar()
		if !ok {
			return rdfNode{}, codecError("unterminated literal")
		}
		switch ch {
		case '\\':
			escaped, err := p.escape()
			if err != nil {
				return rdfNode{}, err
			}
			value.WriteRune(escaped)
		case '\n', '\r':
			return rdfNode{}, codecError("raw newline in short string literal")
		case '"':
			lang := ""
			datatype := ""
			if got, ok := p.peekChar(); ok && got == '@' {
				p.bumpChar()
				start := p.pos
				for p.pos < len(p.text) {
					b := p.text[p.pos]
					if (b >= 'A' && b <= 'Z') || (b >= 'a' && b <= 'z') || (b >= '0' && b <= '9') || b == '-' {
						p.pos++
						continue
					}
					break
				}
				if p.pos == start {
					return rdfNode{}, codecError("empty language tag")
				}
				lang = p.text[start:p.pos]
			} else if strings.HasPrefix(p.text[p.pos:], "^^") {
				p.pos += 2
				dt, err := p.datatypeIRI()
				if err != nil {
					return rdfNode{}, err
				}
				datatype = dt
			}
			return literalNode(value.String(), lang, "", datatype), nil
		default:
			value.WriteRune(ch)
		}
	}
}

func (p *trigParser) datatypeIRI() (string, error) {
	p.skipWSAndComments()
	if ch, ok := p.peekChar(); ok && ch == '<' {
		return p.iri()
	}
	return p.prefixedName()
}

func (p *trigParser) escape() (rune, error) {
	ch, ok := p.bumpChar()
	if !ok {
		return 0, codecError("bad escape at end of literal")
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
		end := p.pos + width
		if end > len(p.text) || !utf8.ValidString(p.text[p.pos:end]) {
			return 0, codecError("short or invalid unicode escape")
		}
		raw := p.text[p.pos:end]
		for _, b := range []byte(raw) {
			if !isASCIIHexDigit(b) {
				return 0, codecError("bad unicode escape \\%c%s", ch, raw)
			}
		}
		p.pos = end
		code, err := strconv.ParseInt(raw, 16, 32)
		if err != nil {
			return 0, codecError("bad unicode escape: %v", err)
		}
		r := rune(code)
		if !utf8.ValidRune(r) {
			return 0, codecError("invalid unicode scalar \\%c%s", ch, raw)
		}
		return r, nil
	default:
		return 0, codecError("unsupported escape \\%c", ch)
	}
}

func (p *trigParser) parenthesizedQuotedTriple(graph *rdfNode) (rdfNode, error) {
	p.pos += 3
	s, err := p.term(graph)
	if err != nil {
		return rdfNode{}, err
	}
	pred, err := p.predicate()
	if err != nil {
		return rdfNode{}, err
	}
	o, err := p.term(graph)
	if err != nil {
		return rdfNode{}, err
	}
	p.skipWSAndComments()
	if !strings.HasPrefix(p.text[p.pos:], ")>>") {
		return rdfNode{}, codecError("unterminated quoted triple")
	}
	p.pos += 3
	return tripleNode(s, pred, o), nil
}

func (p *trigParser) legacyQuotedTriple(graph *rdfNode) (rdfNode, error) {
	p.pos += 2
	s, err := p.term(graph)
	if err != nil {
		return rdfNode{}, err
	}
	pred, err := p.predicate()
	if err != nil {
		return rdfNode{}, err
	}
	o, err := p.term(graph)
	if err != nil {
		return rdfNode{}, err
	}
	p.skipWSAndComments()
	if !strings.HasPrefix(p.text[p.pos:], ">>") {
		return rdfNode{}, codecError("unterminated quoted triple")
	}
	p.pos += 2
	return tripleNode(s, pred, o), nil
}

func (p *trigParser) prefixedName() (string, error) {
	p.skipWSAndComments()
	start := p.pos
	for {
		ch, ok := p.peekChar()
		if !ok || unicode.IsSpace(ch) || strings.ContainsRune("{}[]()<>\n\t;,", ch) {
			break
		}
		if ch == '.' {
			next := p.pos + 1
			if next >= len(p.text) || p.text[next] <= ' ' || strings.ContainsRune("{}[]()<>\n\t;,", rune(p.text[next])) {
				break
			}
		}
		if ch == ',' {
			break
		}
		p.bumpChar()
	}
	if p.pos == start {
		return "", codecError("expected term at byte %d", p.pos)
	}
	name := p.text[start:p.pos]
	prefix, local, ok := strings.Cut(name, ":")
	if !ok {
		return "", codecError("unsupported bare token %q; use an IRI or prefix", name)
	}
	base, ok := p.prefixes[prefix]
	if !ok {
		return "", codecError("unknown prefix %q", prefix)
	}
	return base + local, nil
}

func (p *trigParser) blankNodePropertyList(graph *rdfNode) (rdfNode, error) {
	if err := p.expectChar('[', "to open blank-node property list"); err != nil {
		return rdfNode{}, err
	}
	subject := p.nextBNode()
	if !p.consumeChar(']') {
		if err := p.predicateObjectList(&subject, graph); err != nil {
			return rdfNode{}, err
		}
		if err := p.expectChar(']', "to close blank-node property list"); err != nil {
			return rdfNode{}, err
		}
	}
	return subject, nil
}

func (p *trigParser) collection(graph *rdfNode) (rdfNode, error) {
	if err := p.expectChar('(', "to open RDF collection"); err != nil {
		return rdfNode{}, err
	}
	items := []rdfNode{}
	for !p.consumeChar(')') {
		if p.eof() {
			return rdfNode{}, codecError("unterminated RDF collection")
		}
		item, err := p.term(graph)
		if err != nil {
			return rdfNode{}, err
		}
		items = append(items, item)
	}
	if len(items) == 0 {
		return iriNode(rdfNil), nil
	}
	cells := make([]rdfNode, len(items))
	for i := range cells {
		cells[i] = p.nextBNode()
	}
	for i, item := range items {
		rest := iriNode(rdfNil)
		if i+1 < len(cells) {
			rest = cells[i+1]
		}
		p.emitStatement(&cells[i], &rdfNode{kind: nodeIRI, value: rdfFirst}, &item, graph)
		p.emitStatement(&cells[i], &rdfNode{kind: nodeIRI, value: rdfRest}, &rest, graph)
	}
	return cells[0], nil
}

func (p *trigParser) graphBlock(graph rdfNode) error {
	if err := p.expectChar('{', "to open graph block"); err != nil {
		return err
	}
	return p.graphBlockAfterOpen(graph)
}

func (p *trigParser) graphBlockAfterOpen(graph rdfNode) error {
	if !graph.graphName() {
		return codecError("graph block name must be an IRI or blank node")
	}
	for !p.consumeChar('}') {
		if p.eof() {
			return codecError("unterminated graph block")
		}
		subject, err := p.term(&graph)
		if err != nil {
			return err
		}
		if err := p.statementAfterSubject(subject, &graph); err != nil {
			return err
		}
	}
	return nil
}

func (p *trigParser) defaultGraphBlockAfterOpen() error {
	for !p.consumeChar('}') {
		if p.eof() {
			return codecError("unterminated graph block")
		}
		subject, err := p.term(nil)
		if err != nil {
			return err
		}
		if err := p.statementAfterSubject(subject, nil); err != nil {
			return err
		}
	}
	return nil
}

func (p *trigParser) statementAfterSubject(subject rdfNode, graph *rdfNode) error {
	if err := p.predicateObjectList(&subject, graph); err != nil {
		return err
	}
	return p.expectChar('.', "to terminate statement")
}

func (p *trigParser) predicateObjectList(subject *rdfNode, graph *rdfNode) error {
	for {
		predicate, err := p.predicate()
		if err != nil {
			return err
		}
		for {
			object, err := p.term(graph)
			if err != nil {
				return err
			}
			p.emitStatement(subject, &predicate, &object, graph)
			if p.consumeChar(',') {
				continue
			}
			break
		}
		if p.consumeChar(';') {
			p.skipWSAndComments()
			if ch, ok := p.peekChar(); ok && (ch == '.' || ch == ']' || ch == '}') {
				break
			}
			continue
		}
		break
	}
	return nil
}

func (p *trigParser) emitStatement(subject, predicate, object, graph *rdfNode) {
	line := fmt.Sprintf("%s %s %s", subject.token(), predicate.token(), object.token())
	if graph != nil {
		line += " " + graph.token()
	}
	p.nquads = append(p.nquads, line+" .")
}

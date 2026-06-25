// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package rdfcodecs

import (
	"bytes"
	"encoding/xml"
	"fmt"
	"io"
	"strings"
	"unicode"

	"go.blackcatinformatics.ca/gts/fromnquads"
	"go.blackcatinformatics.ca/gts/model"
)

const (
	xmlNS = "http://www.w3.org/XML/1998/namespace"
	itsNS = "http://www.w3.org/2005/11/its"
)

// FromRDFXML parses RDF/XML text into canonical GTS bytes.
func FromRDFXML(text string) ([]byte, error) {
	return FromRDFXMLWithBaseIRI(text, "")
}

// FromRDFXMLWithBaseIRI parses RDF/XML text using an explicit document base IRI.
func FromRDFXMLWithBaseIRI(text, baseIRI string) ([]byte, error) {
	root, err := parseXMLDOM(text)
	if err != nil {
		return nil, err
	}
	parser := &rdfXMLParser{baseContext: rdfXMLContext{baseIRI: baseIRI}}
	if err := parser.parseDocument(root, parser.baseContext); err != nil {
		return nil, err
	}
	nq := ""
	if len(parser.nquads) > 0 {
		nq = strings.Join(parser.nquads, "\n") + "\n"
	}
	out, err := fromnquads.FromNQuads(nq)
	if err != nil {
		return nil, codecError("RDF/XML parse error: %v", err)
	}
	return out, nil
}

// ToRDFXML serializes a folded default graph to RDF/XML.
func ToRDFXML(g *model.Graph) (string, error) {
	if err := ensureDefaultGraph(g, "RDF/XML"); err != nil {
		return "", err
	}
	if g == nil {
		return "<?xml version=\"1.0\"?>\n<rdf:RDF xmlns:rdf=\"" + rdfNS + "\" xmlns:xsd=\"" + xsdNS + "\">\n</rdf:RDF>\n", nil
	}

	subjects := map[string][]property{}
	subjectNodes := map[string]rdfNode{}
	add := func(subject rdfNode, predicate string, object rdfNode) error {
		if !subject.graphName() {
			return codecError("RDF/XML cannot serialize non-resource subject %s", subject.token())
		}
		key := subjectKey(subject)
		if _, ok := subjectNodes[key]; !ok {
			subjectNodes[key] = subject
		}
		subjects[key] = append(subjects[key], property{predicate: predicate, object: object})
		return nil
	}

	for _, q := range g.Quads {
		s, err := graphTermNode(g, q.S)
		if err != nil {
			return "", err
		}
		pred, err := graphTermNode(g, q.P)
		if err != nil {
			return "", err
		}
		if pred.kind != nodeIRI {
			return "", codecError("RDF/XML cannot serialize non-IRI predicate %s", pred.token())
		}
		o, err := graphTermNode(g, q.O)
		if err != nil {
			return "", err
		}
		if err := add(s, pred.value, o); err != nil {
			return "", err
		}
	}
	for _, r := range g.Reifiers {
		if r.RID >= 0 && r.RID < len(g.Terms) && g.Terms[r.RID].Kind == model.Triple {
			continue
		}
		s, err := graphTermNode(g, r.RID)
		if err != nil {
			return "", err
		}
		subj, err := graphTermNode(g, r.SPO.S)
		if err != nil {
			return "", err
		}
		pred, err := graphTermNode(g, r.SPO.P)
		if err != nil {
			return "", err
		}
		obj, err := graphTermNode(g, r.SPO.O)
		if err != nil {
			return "", err
		}
		if err := add(s, rdfReifies, tripleNode(subj, pred, obj)); err != nil {
			return "", err
		}
	}
	for _, a := range g.Annotations {
		s, err := graphTermNode(g, a.S)
		if err != nil {
			return "", err
		}
		pred, err := graphTermNode(g, a.P)
		if err != nil {
			return "", err
		}
		if pred.kind != nodeIRI {
			return "", codecError("RDF/XML cannot serialize non-IRI annotation predicate %s", pred.token())
		}
		o, err := graphTermNode(g, a.O)
		if err != nil {
			return "", err
		}
		if err := add(s, pred.value, o); err != nil {
			return "", err
		}
	}

	namespaces := rdfXMLNamespaces(subjects)
	var out strings.Builder
	out.WriteString("<?xml version=\"1.0\"?>\n")
	out.WriteString("<rdf:RDF xmlns:rdf=\"")
	out.WriteString(rdfNS)
	out.WriteString("\" xmlns:xsd=\"")
	out.WriteString(xsdNS)
	out.WriteString("\"")
	for _, ns := range sortedKeys(namespaces) {
		prefix := namespaces[ns]
		if prefix == "rdf" || prefix == "xsd" {
			continue
		}
		out.WriteString(fmt.Sprintf(" xmlns:%s=\"%s\"", prefix, escapeXMLAttr(ns)))
	}
	out.WriteString(">\n")

	for _, key := range sortedKeys(subjects) {
		subject := subjectNodes[key]
		out.WriteString("  <rdf:Description")
		if subject.kind == nodeIRI {
			out.WriteString(fmt.Sprintf(" rdf:about=\"%s\"", escapeXMLAttr(subject.value)))
		} else {
			out.WriteString(fmt.Sprintf(" rdf:nodeID=\"%s\"", escapeXMLAttr(subject.value)))
		}
		out.WriteString(">\n")
		for _, prop := range subjects[key] {
			if err := writeRDFXMLProperty(&out, "    ", prop.predicate, prop.object, namespaces); err != nil {
				return "", err
			}
		}
		out.WriteString("  </rdf:Description>\n")
	}
	out.WriteString("</rdf:RDF>\n")
	return out.String(), nil
}

type xmlName struct {
	raw   string
	space string
	local string
}

func (n xmlName) iri() string {
	return n.space + n.local
}

func (n xmlName) isRDF(local string) bool {
	return n.space == rdfNS && n.local == local
}

func (n xmlName) isXML(local string) bool {
	return n.space == xmlNS && n.local == local
}

func (n xmlName) isITS(local string) bool {
	return n.space == itsNS && n.local == local
}

type xmlAttr struct {
	name  xmlName
	value string
}

type xmlElement struct {
	name     xmlName
	attrs    []xmlAttr
	children []xmlNode
}

type xmlNode struct {
	text    string
	element *xmlElement
}

func parseXMLDOM(text string) (*xmlElement, error) {
	decoder := xml.NewDecoder(strings.NewReader(text))
	stack := []*xmlElement{}
	var root *xmlElement
	for {
		tok, err := decoder.Token()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, codecError("RDF/XML parse error: %v", err)
		}
		switch tok := tok.(type) {
		case xml.StartElement:
			element := &xmlElement{name: convertXMLName(tok.Name, true)}
			for _, attr := range tok.Attr {
				if attr.Name.Space == "xmlns" || attr.Name.Local == "xmlns" {
					continue
				}
				element.attrs = append(element.attrs, xmlAttr{name: convertXMLName(attr.Name, false), value: attr.Value})
			}
			stack = append(stack, element)
		case xml.EndElement:
			if len(stack) == 0 {
				return nil, codecError("RDF/XML parse error: unmatched closing tag")
			}
			element := stack[len(stack)-1]
			stack = stack[:len(stack)-1]
			if len(stack) == 0 {
				if root != nil {
					return nil, codecError("RDF/XML parse error: multiple document elements")
				}
				root = element
			} else {
				parent := stack[len(stack)-1]
				parent.children = append(parent.children, xmlNode{element: element})
			}
		case xml.CharData:
			value := string(tok)
			if len(stack) == 0 {
				if strings.TrimSpace(value) != "" {
					return nil, codecError("RDF/XML parse error: text appears outside the document element")
				}
				continue
			}
			parent := stack[len(stack)-1]
			parent.children = append(parent.children, xmlNode{text: value})
		}
	}
	if len(stack) != 0 {
		return nil, codecError("RDF/XML parse error: document ended before all elements closed")
	}
	if root == nil {
		return nil, codecError("RDF/XML parse error: missing document element")
	}
	return root, nil
}

func convertXMLName(name xml.Name, element bool) xmlName {
	raw := name.Local
	switch name.Space {
	case rdfNS:
		raw = "rdf:" + name.Local
	case xmlNS:
		raw = "xml:" + name.Local
	case itsNS:
		raw = "its:" + name.Local
	default:
		if name.Space != "" && !element {
			raw = "ns:" + name.Local
		}
	}
	return xmlName{raw: raw, space: name.Space, local: name.Local}
}

type rdfXMLContext struct {
	baseIRI   string
	language  string
	direction string
}

func (c rdfXMLContext) child(element *xmlElement) (rdfXMLContext, error) {
	next := c
	if base, ok := element.attrXML("base"); ok {
		if next.baseIRI != "" {
			next.baseIRI = resolveRelativeIRI(next.baseIRI, base)
		} else {
			next.baseIRI = base
		}
	}
	if language, ok := element.attrXML("lang"); ok {
		next.language = language
	}
	if direction, ok := element.attrITS("dir"); ok {
		if direction != "ltr" && direction != "rtl" {
			return rdfXMLContext{}, codecError("RDF/XML parse error: invalid ITS direction %q", direction)
		}
		next.direction = direction
	}
	return next, nil
}

func (e *xmlElement) attrRDF(local string) (string, bool) {
	for _, attr := range e.attrs {
		if attr.name.isRDF(local) {
			return attr.value, true
		}
	}
	return "", false
}

func (e *xmlElement) attrXML(local string) (string, bool) {
	for _, attr := range e.attrs {
		if attr.name.isXML(local) {
			return attr.value, true
		}
	}
	return "", false
}

func (e *xmlElement) attrITS(local string) (string, bool) {
	for _, attr := range e.attrs {
		if attr.name.isITS(local) {
			return attr.value, true
		}
	}
	return "", false
}

func (e *xmlElement) propertyAttrs() []xmlAttr {
	var out []xmlAttr
	for _, attr := range e.attrs {
		if attr.name.space == xmlNS || attr.name.space == itsNS || attr.name.space == "xmlns" {
			continue
		}
		if attr.name.space == rdfNS {
			switch attr.name.local {
			case "about", "ID", "nodeID", "resource", "datatype", "parseType", "type", "version", "annotation":
				continue
			}
		}
		out = append(out, attr)
	}
	return out
}

type rdfXMLParser struct {
	baseContext       rdfXMLContext
	nquads            []string
	bnodeCounter      int
	collectionCounter int
}

func (p *rdfXMLParser) parseDocument(root *xmlElement, context rdfXMLContext) error {
	next, err := context.child(root)
	if err != nil {
		return err
	}
	if root.name.isRDF("RDF") {
		for _, child := range elementChildren(root) {
			if _, err := p.parseNodeElement(child, next); err != nil {
				return err
			}
		}
		return nil
	}
	_, err = p.parseNodeElement(root, next)
	return err
}

func (p *rdfXMLParser) parseNodeElement(element *xmlElement, parent rdfXMLContext) (rdfNode, error) {
	context, err := parent.child(element)
	if err != nil {
		return rdfNode{}, err
	}
	subject, err := p.subjectForNode(element, context)
	if err != nil {
		return rdfNode{}, err
	}
	if !element.name.isRDF("Description") {
		p.insertStatement(subject, iriNode(rdfType), iriNode(element.name.iri()), nil, nil)
	}
	if typeIRI, ok := element.attrRDF("type"); ok {
		p.insertStatement(subject, iriNode(rdfType), iriNode(p.iriRef(typeIRI, context)), nil, nil)
	}
	for _, attr := range element.propertyAttrs() {
		p.insertStatement(subject, iriNode(attr.name.iri()), p.contextLiteral(attr.value, "", context), nil, nil)
	}
	for _, child := range elementChildren(element) {
		if err := p.parsePropertyElement(subject, child, context); err != nil {
			return rdfNode{}, err
		}
	}
	return subject, nil
}

func (p *rdfXMLParser) parsePropertyElement(subject rdfNode, element *xmlElement, parent rdfXMLContext) error {
	context, err := parent.child(element)
	if err != nil {
		return err
	}
	predicate := iriNode(element.name.iri())
	reifier, err := p.optionalReifier(element, context)
	if err != nil {
		return err
	}
	annotation, err := p.optionalAnnotation(element, context)
	if err != nil {
		return err
	}

	if resource, ok := element.attrRDF("resource"); ok {
		object := iriNode(p.iriRef(resource, context))
		p.insertStatement(subject, predicate, object, reifier, annotation)
		p.insertPropertyAttributeStatements(object, element, context)
		return nil
	}
	if nodeID, ok := element.attrRDF("nodeID"); ok {
		object := bnodeNode(nodeID)
		p.insertStatement(subject, predicate, object, reifier, annotation)
		p.insertPropertyAttributeStatements(object, element, context)
		return nil
	}

	if parseType, ok := element.attrRDF("parseType"); ok {
		switch parseType {
		case "Resource":
			object := p.freshBNode()
			p.insertStatement(subject, predicate, object, reifier, annotation)
			p.insertPropertyAttributeStatements(object, element, context)
			for _, child := range elementChildren(element) {
				if err := p.parsePropertyElement(object, child, context); err != nil {
					return err
				}
			}
			return nil
		case "Collection":
			head, err := p.parseCollection(element, context)
			if err != nil {
				return err
			}
			p.insertStatement(subject, predicate, head, reifier, annotation)
			return nil
		case "Literal":
			p.insertStatement(subject, predicate, literalNode(serializeChildrenAsXML(element), "", "", rdfXMLLit), reifier, annotation)
			return nil
		case "Triple":
			triple, err := p.parseTripleElement(element, context)
			if err != nil {
				return err
			}
			p.insertStatement(subject, predicate, triple, reifier, annotation)
			return nil
		default:
			return codecError("RDF/XML parse error: unsupported rdf:parseType %q", parseType)
		}
	}

	children := elementChildren(element)
	if datatype, ok := element.attrRDF("datatype"); ok {
		if len(children) > 0 {
			return codecError("RDF/XML parse error: rdf:datatype property cannot contain node elements")
		}
		p.insertStatement(subject, predicate, literalNode(elementText(element), "", "", p.iriRef(datatype, context)), reifier, annotation)
		return nil
	}
	if len(children) == 1 {
		object, err := p.parseNodeElement(children[0], context)
		if err != nil {
			return err
		}
		p.insertStatement(subject, predicate, object, reifier, annotation)
		return nil
	}
	if len(children) > 1 {
		return codecError("RDF/XML parse error: property element contains more than one node element")
	}
	if len(element.propertyAttrs()) > 0 {
		object := p.freshBNode()
		p.insertStatement(subject, predicate, object, reifier, annotation)
		p.insertPropertyAttributeStatements(object, element, context)
		return nil
	}
	p.insertStatement(subject, predicate, p.contextLiteral(elementText(element), "", context), reifier, annotation)
	return nil
}

func (p *rdfXMLParser) insertPropertyAttributeStatements(subject rdfNode, element *xmlElement, context rdfXMLContext) {
	for _, attr := range element.propertyAttrs() {
		p.insertStatement(subject, iriNode(attr.name.iri()), p.contextLiteral(attr.value, "", context), nil, nil)
	}
}

func (p *rdfXMLParser) parseCollection(element *xmlElement, context rdfXMLContext) (rdfNode, error) {
	items := elementChildren(element)
	if len(items) == 0 {
		return iriNode(rdfNil), nil
	}
	nodes := make([]rdfNode, len(items))
	for i := range nodes {
		nodes[i] = p.freshCollectionBNode()
	}
	for i, item := range items {
		object, err := p.parseNodeElement(item, context)
		if err != nil {
			return rdfNode{}, err
		}
		rest := iriNode(rdfNil)
		if i+1 < len(nodes) {
			rest = nodes[i+1]
		}
		p.insertStatement(nodes[i], iriNode(rdfFirst), object, nil, nil)
		p.insertStatement(nodes[i], iriNode(rdfRest), rest, nil, nil)
	}
	return nodes[0], nil
}

func (p *rdfXMLParser) parseTripleElement(element *xmlElement, context rdfXMLContext) (rdfNode, error) {
	nodes := elementChildren(element)
	if len(nodes) != 1 {
		return rdfNode{}, codecError("RDF/XML parse error: rdf:parseType=\"Triple\" requires one node element")
	}
	tripleSubject, err := p.subjectForNode(nodes[0], context)
	if err != nil {
		return rdfNode{}, err
	}
	properties := elementChildren(nodes[0])
	if len(properties) != 1 {
		return rdfNode{}, codecError("RDF/XML parse error: rdf:parseType=\"Triple\" requires exactly one predicate/object")
	}
	predicate := iriNode(properties[0].name.iri())
	object, err := p.tripleObject(properties[0], context)
	if err != nil {
		return rdfNode{}, err
	}
	return tripleNode(tripleSubject, predicate, object), nil
}

func (p *rdfXMLParser) tripleObject(property *xmlElement, parent rdfXMLContext) (rdfNode, error) {
	context, err := parent.child(property)
	if err != nil {
		return rdfNode{}, err
	}
	if resource, ok := property.attrRDF("resource"); ok {
		return iriNode(p.iriRef(resource, context)), nil
	}
	if nodeID, ok := property.attrRDF("nodeID"); ok {
		return bnodeNode(nodeID), nil
	}
	if parseType, ok := property.attrRDF("parseType"); ok && parseType == "Triple" {
		return p.parseTripleElement(property, context)
	}
	nodes := elementChildren(property)
	if len(nodes) == 1 {
		return p.subjectForNode(nodes[0], context)
	}
	if len(nodes) > 1 {
		return rdfNode{}, codecError("RDF/XML parse error: rdf:parseType=\"Triple\" object has multiple node elements")
	}
	datatype := ""
	if dt, ok := property.attrRDF("datatype"); ok {
		datatype = p.iriRef(dt, context)
	}
	return p.contextLiteral(elementText(property), datatype, context), nil
}

func (p *rdfXMLParser) subjectForNode(element *xmlElement, context rdfXMLContext) (rdfNode, error) {
	if about, ok := element.attrRDF("about"); ok {
		return iriNode(p.iriRef(about, context)), nil
	}
	if id, ok := element.attrRDF("ID"); ok {
		if id == "" {
			return rdfNode{}, codecError("RDF/XML parse error: empty rdf:ID")
		}
		return iriNode(p.rdfIDIRI(id, context)), nil
	}
	if nodeID, ok := element.attrRDF("nodeID"); ok {
		return bnodeNode(nodeID), nil
	}
	return p.freshBNode(), nil
}

func (p *rdfXMLParser) optionalReifier(element *xmlElement, context rdfXMLContext) (*rdfNode, error) {
	id, ok := element.attrRDF("ID")
	if !ok {
		return nil, nil
	}
	if id == "" {
		return nil, codecError("RDF/XML parse error: empty rdf:ID")
	}
	node := iriNode(p.rdfIDIRI(id, context))
	return &node, nil
}

func (p *rdfXMLParser) optionalAnnotation(element *xmlElement, context rdfXMLContext) (*rdfNode, error) {
	annotation, ok := element.attrRDF("annotation")
	if !ok {
		return nil, nil
	}
	node := iriNode(p.iriRef(annotation, context))
	return &node, nil
}

func (p *rdfXMLParser) contextLiteral(lexical, datatype string, context rdfXMLContext) rdfNode {
	if datatype != "" {
		return literalNode(lexical, "", "", datatype)
	}
	if context.language != "" {
		return literalNode(lexical, context.language, context.direction, "")
	}
	return literalNode(lexical, "", "", "")
}

func (p *rdfXMLParser) iriRef(value string, context rdfXMLContext) string {
	if hasIRIScheme(value) || context.baseIRI == "" {
		return value
	}
	return resolveRelativeIRI(context.baseIRI, value)
}

func (p *rdfXMLParser) rdfIDIRI(value string, context rdfXMLContext) string {
	if context.baseIRI == "" {
		return "#" + value
	}
	base := context.baseIRI
	if before, _, ok := strings.Cut(base, "#"); ok {
		base = before
	}
	return base + "#" + value
}

func (p *rdfXMLParser) freshBNode() rdfNode {
	id := p.bnodeCounter
	p.bnodeCounter++
	return bnodeNode(fmt.Sprintf("rdfxml_%d", id))
}

func (p *rdfXMLParser) freshCollectionBNode() rdfNode {
	id := p.collectionCounter
	p.collectionCounter++
	return bnodeNode(fmt.Sprintf("rdfxml_list_%d", id))
}

func (p *rdfXMLParser) insertStatement(subject, predicate, object rdfNode, reifier, annotation *rdfNode) {
	line := fmt.Sprintf("%s %s %s .", subject.token(), predicate.token(), object.token())
	p.nquads = append(p.nquads, line)
	if reifier != nil {
		p.insertReifier(*reifier, subject, predicate, object)
	}
	if annotation != nil {
		p.insertReifier(*annotation, subject, predicate, object)
	}
}

func (p *rdfXMLParser) insertReifier(reifier, subject, predicate, object rdfNode) {
	quoted := tripleNode(subject, predicate, object)
	p.nquads = append(p.nquads, fmt.Sprintf("%s <%s> %s .", reifier.token(), rdfReifies, quoted.token()))
}

func elementChildren(element *xmlElement) []*xmlElement {
	out := []*xmlElement{}
	for i := range element.children {
		if element.children[i].element != nil {
			out = append(out, element.children[i].element)
		}
	}
	return out
}

func elementText(element *xmlElement) string {
	var out strings.Builder
	for _, child := range element.children {
		if child.element == nil {
			out.WriteString(child.text)
		}
	}
	return out.String()
}

func serializeChildrenAsXML(element *xmlElement) string {
	var out strings.Builder
	for _, child := range element.children {
		serializeXMLNode(&out, child)
	}
	return out.String()
}

func serializeXMLNode(out *strings.Builder, node xmlNode) {
	if node.element == nil {
		out.WriteString(escapeXMLText(node.text))
		return
	}
	element := node.element
	out.WriteByte('<')
	out.WriteString(element.name.raw)
	for _, attr := range element.attrs {
		out.WriteByte(' ')
		out.WriteString(attr.name.raw)
		out.WriteString("=\"")
		out.WriteString(escapeXMLAttr(attr.value))
		out.WriteByte('"')
	}
	if len(element.children) == 0 {
		out.WriteString("/>")
		return
	}
	out.WriteByte('>')
	for _, child := range element.children {
		serializeXMLNode(out, child)
	}
	out.WriteString("</")
	out.WriteString(element.name.raw)
	out.WriteByte('>')
}

func subjectKey(subject rdfNode) string {
	if subject.kind == nodeIRI {
		return "I" + subject.value
	}
	return "B" + subject.value
}

func rdfXMLNamespaces(subjects map[string][]property) map[string]string {
	namespaces := map[string]string{rdfNS: "rdf", xsdNS: "xsd"}
	next := 0
	for _, props := range subjects {
		for _, prop := range props {
			ns, _ := splitPropertyIRI(prop.predicate)
			if _, ok := namespaces[ns]; ok {
				continue
			}
			namespaces[ns] = fmt.Sprintf("ns%d", next)
			next++
		}
	}
	return namespaces
}

type property struct {
	predicate string
	object    rdfNode
}

func writeRDFXMLProperty(out *strings.Builder, indent, predicate string, object rdfNode, namespaces map[string]string) error {
	name := serializerQName(predicate, namespaces)
	switch object.kind {
	case nodeIRI:
		out.WriteString(fmt.Sprintf("%s<%s rdf:resource=\"%s\"/>\n", indent, name, escapeXMLAttr(object.value)))
	case nodeBNode:
		out.WriteString(fmt.Sprintf("%s<%s rdf:nodeID=\"%s\"/>\n", indent, name, escapeXMLAttr(object.value)))
	case nodeLiteral:
		out.WriteString(fmt.Sprintf("%s<%s", indent, name))
		if object.lang != "" {
			out.WriteString(fmt.Sprintf(" xml:lang=\"%s\"", escapeXMLAttr(object.lang)))
		}
		if object.direction == "ltr" || object.direction == "rtl" {
			out.WriteString(fmt.Sprintf(" xmlns:its=\"%s\" its:dir=\"%s\"", itsNS, object.direction))
		}
		if object.datatype != "" {
			out.WriteString(fmt.Sprintf(" rdf:datatype=\"%s\"", escapeXMLAttr(object.datatype)))
		}
		out.WriteString(">")
		out.WriteString(escapeXMLText(object.value))
		out.WriteString(fmt.Sprintf("</%s>\n", name))
	case nodeTriple:
		out.WriteString(fmt.Sprintf("%s<%s rdf:parseType=\"Triple\">\n", indent, name))
		if err := writeRDFXMLTripleNode(out, indent+"  ", object, namespaces); err != nil {
			return err
		}
		out.WriteString(fmt.Sprintf("%s</%s>\n", indent, name))
	default:
		return codecError("RDF/XML cannot serialize object %s", object.token())
	}
	return nil
}

func writeRDFXMLTripleNode(out *strings.Builder, indent string, triple rdfNode, namespaces map[string]string) error {
	if triple.kind != nodeTriple {
		return codecError("RDF/XML expected triple node")
	}
	out.WriteString(indent + "<rdf:Description")
	switch triple.s.kind {
	case nodeIRI:
		out.WriteString(fmt.Sprintf(" rdf:about=\"%s\"", escapeXMLAttr(triple.s.value)))
	case nodeBNode:
		out.WriteString(fmt.Sprintf(" rdf:nodeID=\"%s\"", escapeXMLAttr(triple.s.value)))
	default:
		return codecError("RDF/XML cannot serialize triple subject %s", triple.s.token())
	}
	out.WriteString(">\n")
	if triple.p.kind != nodeIRI {
		return codecError("RDF/XML cannot serialize triple predicate %s", triple.p.token())
	}
	if err := writeRDFXMLProperty(out, indent+"  ", triple.p.value, *triple.o, namespaces); err != nil {
		return err
	}
	out.WriteString(indent + "</rdf:Description>\n")
	return nil
}

func serializerQName(iri string, namespaces map[string]string) string {
	ns, local := splitPropertyIRI(iri)
	prefix := namespaces[ns]
	if prefix == "" {
		prefix = "ns"
	}
	return prefix + ":" + local
}

func splitPropertyIRI(iri string) (string, string) {
	split := strings.LastIndexAny(iri, "#/:")
	if split >= 0 {
		split++
	}
	if split < 0 {
		split = 0
	}
	ns, local := iri[:split], iri[split:]
	if local == "" || !isXMLName(local) {
		return iri, "property"
	}
	return ns, local
}

func isXMLName(value string) bool {
	if value == "" {
		return false
	}
	for i, ch := range value {
		if i == 0 {
			if !(ch == '_' || unicode.IsLetter(ch)) {
				return false
			}
			continue
		}
		if !(ch == '_' || ch == '-' || ch == '.' || unicode.IsLetter(ch) || unicode.IsDigit(ch)) {
			return false
		}
	}
	return true
}

func escapeXMLText(value string) string {
	var buf bytes.Buffer
	if err := xml.EscapeText(&buf, []byte(value)); err != nil {
		return value
	}
	return buf.String()
}

func escapeXMLAttr(value string) string {
	return strings.ReplaceAll(escapeXMLText(value), "\"", "&quot;")
}

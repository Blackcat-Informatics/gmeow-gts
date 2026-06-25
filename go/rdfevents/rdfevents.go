// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package rdfevents adapts folded GTS graphs to an RDF event protocol.
//
// The event layer sits above the GTS frame-level streaming reader. It describes
// RDF dataset state: term declarations, quads, reifier bindings, annotations,
// and diagnostics. It does not change the GTS wire format.
package rdfevents

import (
	"context"
	"errors"
	"fmt"
	"io"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/reader"
)

const maxInt = int(^uint(0) >> 1)

// TermID is a scope-local RDF event term identifier.
type TermID uint64

// ScopeID identifies one RDF event scope, such as one folded GTS file.
type ScopeID uint64

// Location is optional source location data attached to a diagnostic.
type Location struct {
	// FrameIndex is a GTS frame index or other source-native record index.
	FrameIndex *uint64
	// Line is a 1-based source line number when available.
	Line *uint64
	// Column is a 1-based source column number when available.
	Column *uint64
}

// Diagnostic is a source diagnostic emitted as part of an RDF event stream.
type Diagnostic struct {
	Code     string
	Detail   string
	Location *Location
}

// Triple is an RDF 1.2 triple value using event term ids.
type Triple struct {
	Subject   TermID
	Predicate TermID
	Object    TermID
}

// Quad is an RDF quad value using event term ids.
type Quad struct {
	Subject   TermID
	Predicate TermID
	Object    TermID
	GraphName *TermID
}

// LiteralDirection is an RDF 1.2 literal base direction.
type LiteralDirection string

// Literal direction values.
const (
	DirectionLTR LiteralDirection = "ltr"
	DirectionRTL LiteralDirection = "rtl"
)

// Valid reports whether d is one of the RDF 1.2 base directions.
func (d LiteralDirection) Valid() bool {
	return d == DirectionLTR || d == DirectionRTL
}

// TermKind identifies an RDF term payload.
type TermKind string

// RDF event term kinds.
const (
	TermIRI       TermKind = "iri"
	TermBlankNode TermKind = "blank-node"
	TermLiteral   TermKind = "literal"
	TermTriple    TermKind = "triple"
)

// Term is a term declaration in an RDF event stream.
type Term struct {
	ID   TermID
	Kind TermKind

	// Value is the IRI string, blank-node label, or literal lexical form.
	Value string

	// Datatype is the datatype IRI term id for literals with explicit datatypes.
	Datatype *TermID
	// Language is the literal language tag.
	Language string
	// Direction is the RDF 1.2 literal base direction.
	Direction LiteralDirection

	// Triple is the quoted triple value for TermTriple declarations.
	Triple *Triple
	// Reifier is the optional reifier term id for TermTriple declarations.
	Reifier *TermID
}

// ErrorKind is a high-level RDF event error category.
type ErrorKind string

// RDF event error categories.
const (
	ErrorSink                 ErrorKind = "sink"
	ErrorInvalidSource        ErrorKind = "invalid-source"
	ErrorDuplicateDeclaration ErrorKind = "duplicate-declaration"
	ErrorUnresolvedReference  ErrorKind = "unresolved-reference"
	ErrorClosedScope          ErrorKind = "closed-scope"
	ErrorTripleNestingLimit   ErrorKind = "triple-nesting-limit"
	ErrorCancelled            ErrorKind = "cancelled"
)

// Error is the concrete error type returned by RDF event sources and sinks.
type Error struct {
	Kind   ErrorKind
	Detail string
	Cause  error
}

// NewError creates an RDF event error.
func NewError(kind ErrorKind, detail string) *Error {
	return &Error{Kind: kind, Detail: detail}
}

func wrapError(kind ErrorKind, detail string, cause error) *Error {
	return &Error{Kind: kind, Detail: detail, Cause: cause}
}

// Error implements error.
func (e *Error) Error() string {
	if e == nil {
		return "<nil>"
	}
	if e.Cause != nil {
		return fmt.Sprintf("%s: %s: %v", e.Kind, e.Detail, e.Cause)
	}
	return fmt.Sprintf("%s: %s", e.Kind, e.Detail)
}

// Unwrap exposes the underlying cause, when present.
func (e *Error) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.Cause
}

// IsKind reports whether err is an RDF event error with kind.
func IsKind(err error, kind ErrorKind) bool {
	var eventErr *Error
	return errors.As(err, &eventErr) && eventErr.Kind == kind
}

// Sink consumes RDF graph events.
type Sink interface {
	StartScope(ScopeID) error
	Term(Term) error
	Quad(Quad) error
	Reifier(TermID, Triple) error
	Annotation(Triple) error
	Diagnostic(Diagnostic) error
	EndScope(ScopeID) error
	Finish() error
}

// DeclarationOrderSink can request that sources emit term dependencies before
// any event that references them.
type DeclarationOrderSink interface {
	DeclarationsBeforeReferences() bool
}

// TripleNestingLimitSink can override the default quoted-triple nesting limit.
type TripleNestingLimitSink interface {
	TripleTermNestingLimit() int
}

// NopSink is a sink implementation that accepts and ignores every event.
type NopSink struct{}

// StartScope implements Sink.
func (NopSink) StartScope(ScopeID) error { return nil }

// Term implements Sink.
func (NopSink) Term(Term) error { return nil }

// Quad implements Sink.
func (NopSink) Quad(Quad) error { return nil }

// Reifier implements Sink.
func (NopSink) Reifier(TermID, Triple) error { return nil }

// Annotation implements Sink.
func (NopSink) Annotation(Triple) error { return nil }

// Diagnostic implements Sink.
func (NopSink) Diagnostic(Diagnostic) error { return nil }

// EndScope implements Sink.
func (NopSink) EndScope(ScopeID) error { return nil }

// Finish implements Sink.
func (NopSink) Finish() error { return nil }

// Source produces RDF graph events.
type Source interface {
	Drive(Sink) error
}

// GraphSource is an RDF event source backed by an already-folded graph.
type GraphSource struct {
	Graph *model.Graph
	Scope ScopeID
}

// NewGraphSource creates a graph-backed source using scope id 0.
func NewGraphSource(graph *model.Graph) GraphSource {
	return GraphSource{Graph: graph}
}

// WithScope returns a copy of s using scope.
func (s GraphSource) WithScope(scope ScopeID) GraphSource {
	s.Scope = scope
	return s
}

// Drive emits graph events into sink.
func (s GraphSource) Drive(sink Sink) error {
	return driveGraph(s.Graph, s.Scope, sink)
}

// ReaderSource folds GTS input and then drives RDF events into a sink.
type ReaderSource struct {
	Context context.Context
	Reader  io.Reader
	Options reader.Options
	Scope   ScopeID
}

// NewReaderSource creates a reader-backed source using scope id 0.
func NewReaderSource(ctx context.Context, r io.Reader, opts reader.Options) ReaderSource {
	return ReaderSource{Context: ctx, Reader: r, Options: opts}
}

// WithScope returns a copy of s using scope.
func (s ReaderSource) WithScope(scope ScopeID) ReaderSource {
	s.Scope = scope
	return s
}

// Drive folds the source input and emits graph events into sink.
func (s ReaderSource) Drive(sink Sink) error {
	_, err := readToEventsWithScope(s.Context, s.Reader, s.Options, s.Scope, sink)
	return err
}

// ReadToEvents folds GTS input and drives RDF events into sink.
func ReadToEvents(ctx context.Context, r io.Reader, opts reader.Options, sink Sink) (*model.Graph, error) {
	return readToEventsWithScope(ctx, r, opts, 0, sink)
}

func readToEventsWithScope(ctx context.Context, r io.Reader, opts reader.Options, scope ScopeID, sink Sink) (*model.Graph, error) {
	if sink == nil {
		return nil, NewError(ErrorSink, "nil sink")
	}
	if ctx == nil {
		ctx = context.Background()
	}
	if err := ctx.Err(); err != nil {
		return nil, wrapError(ErrorCancelled, "context cancelled before RDF event read", err)
	}
	graph, err := reader.ReadFrom(ctx, r, opts)
	if err != nil {
		if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
			return nil, wrapError(ErrorCancelled, "context cancelled during RDF event read", err)
		}
		return nil, wrapError(ErrorInvalidSource, "failed to read GTS input", err)
	}
	if err := ctx.Err(); err != nil {
		return nil, wrapError(ErrorCancelled, "context cancelled before RDF event drive", err)
	}
	if err := driveGraph(graph, scope, sink); err != nil {
		return graph, err
	}
	return graph, nil
}

func driveGraph(graph *model.Graph, scope ScopeID, sink Sink) error {
	if graph == nil {
		return NewError(ErrorInvalidSource, "nil graph")
	}
	if sink == nil {
		return NewError(ErrorSink, "nil sink")
	}
	if err := sink.StartScope(scope); err != nil {
		return sinkError(err)
	}
	if sinkDeclaresBeforeReferences(sink) {
		if err := newDeclarationOrderEmitter(graph, sink).emitAll(); err != nil {
			return err
		}
	} else if err := emitFoldOrder(graph, sink); err != nil {
		return err
	}
	for _, diagnostic := range graph.Diagnostics {
		eventDiagnostic, err := toEventDiagnostic(diagnostic)
		if err != nil {
			return err
		}
		if err := sink.Diagnostic(eventDiagnostic); err != nil {
			return sinkError(err)
		}
	}
	if err := sink.EndScope(scope); err != nil {
		return sinkError(err)
	}
	if err := sink.Finish(); err != nil {
		return sinkError(err)
	}
	return nil
}

func sinkError(err error) error {
	if err == nil {
		return nil
	}
	var eventErr *Error
	if errors.As(err, &eventErr) {
		return err
	}
	return wrapError(ErrorSink, "sink rejected RDF event", err)
}

func sinkDeclaresBeforeReferences(sink Sink) bool {
	if provider, ok := sink.(DeclarationOrderSink); ok {
		return provider.DeclarationsBeforeReferences()
	}
	return false
}

const defaultTripleTermNestingLimit = 64

func sinkTripleTermNestingLimit(sink Sink) int {
	if provider, ok := sink.(TripleNestingLimitSink); ok {
		if limit := provider.TripleTermNestingLimit(); limit >= 0 {
			return limit
		}
	}
	return defaultTripleTermNestingLimit
}

func emitFoldOrder(graph *model.Graph, sink Sink) error {
	for id, term := range graph.Terms {
		eventTerm, err := toEventTerm(graph, id, term)
		if err != nil {
			return err
		}
		if err := sink.Term(eventTerm); err != nil {
			return sinkError(err)
		}
	}
	for _, reifier := range graph.Reifiers {
		if err := validateTermRef(graph, reifier.RID, "reifier"); err != nil {
			return err
		}
		if err := validateTripleRefs(graph, reifier.SPO, "reifier"); err != nil {
			return err
		}
		triple, err := toEventTriple(reifier.SPO)
		if err != nil {
			return err
		}
		if err := sink.Reifier(TermID(reifier.RID), triple); err != nil {
			return sinkError(err)
		}
	}
	for _, quad := range graph.Quads {
		if err := validateQuadRefs(graph, quad); err != nil {
			return err
		}
		eventQuad, err := toEventQuad(quad)
		if err != nil {
			return err
		}
		if err := sink.Quad(eventQuad); err != nil {
			return sinkError(err)
		}
	}
	for _, annotation := range graph.Annotations {
		if err := validateTripleRefs(graph, annotation, "annotation"); err != nil {
			return err
		}
		eventAnnotation, err := toEventTriple(annotation)
		if err != nil {
			return err
		}
		if err := sink.Annotation(eventAnnotation); err != nil {
			return sinkError(err)
		}
	}
	return nil
}

type declarationOrderEmitter struct {
	graph           *model.Graph
	sink            Sink
	limit           int
	emittedTerms    map[int]struct{}
	visitingTerms   map[int]struct{}
	emittedReifiers map[int]struct{}
}

func newDeclarationOrderEmitter(graph *model.Graph, sink Sink) *declarationOrderEmitter {
	return &declarationOrderEmitter{
		graph:           graph,
		sink:            sink,
		limit:           sinkTripleTermNestingLimit(sink),
		emittedTerms:    map[int]struct{}{},
		visitingTerms:   map[int]struct{}{},
		emittedReifiers: map[int]struct{}{},
	}
}

func (e *declarationOrderEmitter) emitAll() error {
	for id := range e.graph.Terms {
		if err := e.emitTerm(id, 0); err != nil {
			return err
		}
	}
	for _, reifier := range e.graph.Reifiers {
		if err := e.emitReifier(reifier.RID, reifier.SPO, 0); err != nil {
			return err
		}
	}
	for _, quad := range e.graph.Quads {
		if err := validateQuadRefs(e.graph, quad); err != nil {
			return err
		}
		eventQuad, err := toEventQuad(quad)
		if err != nil {
			return err
		}
		if err := e.sink.Quad(eventQuad); err != nil {
			return sinkError(err)
		}
	}
	for _, annotation := range e.graph.Annotations {
		if err := validateTripleRefs(e.graph, annotation, "annotation"); err != nil {
			return err
		}
		eventAnnotation, err := toEventTriple(annotation)
		if err != nil {
			return err
		}
		if err := e.sink.Annotation(eventAnnotation); err != nil {
			return sinkError(err)
		}
	}
	return nil
}

func (e *declarationOrderEmitter) emitTerm(id int, depth int) error {
	if _, ok := e.emittedTerms[id]; ok {
		return nil
	}
	if depth > e.limit {
		return NewError(
			ErrorTripleNestingLimit,
			fmt.Sprintf("quoted triple nesting exceeds configured limit %d", e.limit),
		)
	}
	if id < 0 || id >= len(e.graph.Terms) {
		return NewError(ErrorInvalidSource, fmt.Sprintf("term id %d is outside graph terms", id))
	}
	if _, ok := e.visitingTerms[id]; ok {
		return NewError(ErrorInvalidSource, fmt.Sprintf("cycle while declaring term %d", id))
	}
	e.visitingTerms[id] = struct{}{}
	defer delete(e.visitingTerms, id)

	term := e.graph.Terms[id]
	var selfReifierTriple *model.Triple3
	switch term.Kind {
	case model.Literal:
		if term.Datatype != nil {
			if err := e.emitTerm(*term.Datatype, depth); err != nil {
				return err
			}
		}
	case model.Triple:
		if term.Reifier != nil {
			triple, ok := e.graph.Reifier(*term.Reifier)
			if !ok {
				return NewError(
					ErrorInvalidSource,
					fmt.Sprintf("triple term %d does not have a resolvable reifier binding", id),
				)
			}
			if *term.Reifier == id {
				if err := e.emitTripleDeps(triple, depth+1); err != nil {
					return err
				}
				selfReifierTriple = &triple
			} else {
				if err := e.emitReifier(*term.Reifier, triple, depth+1); err != nil {
					return err
				}
			}
		}
	case model.Iri, model.Bnode:
	default:
		return NewError(ErrorInvalidSource, fmt.Sprintf("term %d has unknown kind %d", id, term.Kind))
	}

	eventTerm, err := toEventTerm(e.graph, id, term)
	if err != nil {
		return err
	}
	if err := e.sink.Term(eventTerm); err != nil {
		return sinkError(err)
	}
	e.emittedTerms[id] = struct{}{}
	if selfReifierTriple != nil {
		if _, ok := e.emittedReifiers[id]; !ok {
			e.emittedReifiers[id] = struct{}{}
			eventTriple, err := toEventTriple(*selfReifierTriple)
			if err != nil {
				return err
			}
			if err := e.sink.Reifier(TermID(id), eventTriple); err != nil {
				return sinkError(err)
			}
		}
	}
	return nil
}

func (e *declarationOrderEmitter) emitReifier(reifier int, triple model.Triple3, depth int) error {
	if _, ok := e.emittedReifiers[reifier]; ok {
		return nil
	}
	e.emittedReifiers[reifier] = struct{}{}
	if err := e.emitTerm(reifier, depth); err != nil {
		return err
	}
	if err := e.emitTripleDeps(triple, depth); err != nil {
		return err
	}
	eventTriple, err := toEventTriple(triple)
	if err != nil {
		return err
	}
	if err := e.sink.Reifier(TermID(reifier), eventTriple); err != nil {
		return sinkError(err)
	}
	return nil
}

func (e *declarationOrderEmitter) emitTripleDeps(triple model.Triple3, depth int) error {
	if err := e.emitTerm(triple.S, depth); err != nil {
		return err
	}
	if err := e.emitTerm(triple.P, depth); err != nil {
		return err
	}
	return e.emitTerm(triple.O, depth)
}

func validateTermRef(graph *model.Graph, id int, context string) error {
	if id >= 0 && id < len(graph.Terms) {
		return nil
	}
	return NewError(ErrorInvalidSource, fmt.Sprintf("%s references term id %d outside graph terms", context, id))
}

func validateTripleRefs(graph *model.Graph, triple model.Triple3, context string) error {
	if err := validateTermRef(graph, triple.S, context); err != nil {
		return err
	}
	if err := validateTermRef(graph, triple.P, context); err != nil {
		return err
	}
	return validateTermRef(graph, triple.O, context)
}

func validateQuadRefs(graph *model.Graph, quad model.Quad) error {
	if err := validateTermRef(graph, quad.S, "quad"); err != nil {
		return err
	}
	if err := validateTermRef(graph, quad.P, "quad"); err != nil {
		return err
	}
	if err := validateTermRef(graph, quad.O, "quad"); err != nil {
		return err
	}
	if quad.G != nil {
		return validateTermRef(graph, *quad.G, "quad")
	}
	return nil
}

func toEventID(id int) (TermID, error) {
	if id < 0 {
		return 0, NewError(ErrorInvalidSource, fmt.Sprintf("term id %d is negative", id))
	}
	return TermID(id), nil
}

func toEventIDPtr(id *int) (*TermID, error) {
	if id == nil {
		return nil, nil
	}
	eventID, err := toEventID(*id)
	if err != nil {
		return nil, err
	}
	return &eventID, nil
}

func toEventTerm(graph *model.Graph, id int, term model.Term) (Term, error) {
	eventID, err := toEventID(id)
	if err != nil {
		return Term{}, err
	}
	out := Term{ID: eventID, Value: term.Value}
	switch term.Kind {
	case model.Iri:
		out.Kind = TermIRI
	case model.Bnode:
		out.Kind = TermBlankNode
	case model.Literal:
		out.Kind = TermLiteral
		out.Language = term.Lang
		if model.IsLiteralDirection(term.Direction) {
			out.Direction = LiteralDirection(term.Direction)
		}
		out.Datatype, err = toEventIDPtr(term.Datatype)
		if err != nil {
			return Term{}, err
		}
	case model.Triple:
		if term.Reifier == nil {
			return Term{}, NewError(
				ErrorInvalidSource,
				fmt.Sprintf("triple term %d does not have a reifier id", id),
			)
		}
		triple, ok := graph.Reifier(*term.Reifier)
		if !ok {
			return Term{}, NewError(
				ErrorInvalidSource,
				fmt.Sprintf("triple term %d does not have a resolvable reifier binding", id),
			)
		}
		eventTriple, err := toEventTriple(triple)
		if err != nil {
			return Term{}, err
		}
		out.Kind = TermTriple
		out.Triple = &eventTriple
		out.Reifier, err = toEventIDPtr(term.Reifier)
		if err != nil {
			return Term{}, err
		}
	default:
		return Term{}, NewError(ErrorInvalidSource, fmt.Sprintf("term %d has unknown kind %d", id, term.Kind))
	}
	return out, nil
}

func toEventTriple(triple model.Triple3) (Triple, error) {
	subject, err := toEventID(triple.S)
	if err != nil {
		return Triple{}, err
	}
	predicate, err := toEventID(triple.P)
	if err != nil {
		return Triple{}, err
	}
	object, err := toEventID(triple.O)
	if err != nil {
		return Triple{}, err
	}
	return Triple{Subject: subject, Predicate: predicate, Object: object}, nil
}

func toEventQuad(quad model.Quad) (Quad, error) {
	subject, err := toEventID(quad.S)
	if err != nil {
		return Quad{}, err
	}
	predicate, err := toEventID(quad.P)
	if err != nil {
		return Quad{}, err
	}
	object, err := toEventID(quad.O)
	if err != nil {
		return Quad{}, err
	}
	graphName, err := toEventIDPtr(quad.G)
	if err != nil {
		return Quad{}, err
	}
	return Quad{Subject: subject, Predicate: predicate, Object: object, GraphName: graphName}, nil
}

func toEventDiagnostic(diagnostic model.Diagnostic) (Diagnostic, error) {
	out := Diagnostic{Code: diagnostic.Code, Detail: diagnostic.Detail}
	if diagnostic.FrameIndex != nil {
		if *diagnostic.FrameIndex < 0 {
			return Diagnostic{}, NewError(ErrorInvalidSource, "diagnostic frame index is negative")
		}
		frameIndex := uint64(*diagnostic.FrameIndex)
		out.Location = &Location{FrameIndex: &frameIndex}
	}
	return out, nil
}

// GraphSinkOptions configures GraphSink.
type GraphSinkOptions struct {
	// DeclarationsBeforeReferences asks compatible sources to emit term
	// declarations before any event that references them.
	DeclarationsBeforeReferences bool
	// TripleTermNestingLimit caps quoted-triple nesting. A negative value means
	// the default limit.
	TripleTermNestingLimit int
}

// GraphSink materializes RDF events into a model.Graph.
type GraphSink struct {
	options GraphSinkOptions

	active   bool
	ended    bool
	finished bool

	terms       map[TermID]Term
	termOrder   []TermID
	quads       []Quad
	reifiers    []reifierEvent
	annotations []Triple
	diagnostics []Diagnostic

	graph *model.Graph
}

type reifierEvent struct {
	reifier TermID
	triple  Triple
}

// NewGraphSink creates a materializing RDF event sink.
func NewGraphSink() *GraphSink {
	return NewGraphSinkWithOptions(GraphSinkOptions{TripleTermNestingLimit: -1})
}

// NewGraphSinkWithOptions creates a materializing sink with options.
func NewGraphSinkWithOptions(options GraphSinkOptions) *GraphSink {
	return &GraphSink{options: options}
}

// DeclarationsBeforeReferences implements DeclarationOrderSink.
func (s *GraphSink) DeclarationsBeforeReferences() bool {
	return s.options.DeclarationsBeforeReferences
}

// TripleTermNestingLimit implements TripleNestingLimitSink.
func (s *GraphSink) TripleTermNestingLimit() int {
	if s.options.TripleTermNestingLimit < 0 {
		return defaultTripleTermNestingLimit
	}
	return s.options.TripleTermNestingLimit
}

func (s *GraphSink) init() {
	if s.terms == nil {
		s.terms = map[TermID]Term{}
	}
}

// StartScope implements Sink.
func (s *GraphSink) StartScope(ScopeID) error {
	s.init()
	if s.finished || s.ended {
		return NewError(ErrorClosedScope, "cannot start a scope after sink is closed")
	}
	if s.active {
		return NewError(ErrorInvalidSource, "scope already active")
	}
	s.active = true
	return nil
}

func (s *GraphSink) ensureOpen() error {
	if s.finished || s.ended {
		return NewError(ErrorClosedScope, "event received after scope closed")
	}
	if !s.active {
		return NewError(ErrorInvalidSource, "event received before scope start")
	}
	return nil
}

// Term implements Sink.
func (s *GraphSink) Term(term Term) error {
	if err := s.ensureOpen(); err != nil {
		return err
	}
	if _, exists := s.terms[term.ID]; exists {
		return NewError(ErrorDuplicateDeclaration, fmt.Sprintf("term id %d declared more than once", term.ID))
	}
	s.terms[term.ID] = term
	s.termOrder = append(s.termOrder, term.ID)
	return nil
}

// Quad implements Sink.
func (s *GraphSink) Quad(quad Quad) error {
	if err := s.ensureOpen(); err != nil {
		return err
	}
	s.quads = append(s.quads, quad)
	return nil
}

// Reifier implements Sink.
func (s *GraphSink) Reifier(reifier TermID, triple Triple) error {
	if err := s.ensureOpen(); err != nil {
		return err
	}
	s.reifiers = append(s.reifiers, reifierEvent{reifier: reifier, triple: triple})
	return nil
}

// Annotation implements Sink.
func (s *GraphSink) Annotation(annotation Triple) error {
	if err := s.ensureOpen(); err != nil {
		return err
	}
	s.annotations = append(s.annotations, annotation)
	return nil
}

// Diagnostic implements Sink.
func (s *GraphSink) Diagnostic(diagnostic Diagnostic) error {
	if err := s.ensureOpen(); err != nil {
		return err
	}
	s.diagnostics = append(s.diagnostics, diagnostic)
	return nil
}

// EndScope implements Sink.
func (s *GraphSink) EndScope(ScopeID) error {
	if s.finished || s.ended {
		return NewError(ErrorClosedScope, "scope already closed")
	}
	if !s.active {
		return NewError(ErrorInvalidSource, "end scope before start scope")
	}
	s.active = false
	s.ended = true
	return nil
}

// Finish implements Sink and freezes the materialized graph.
func (s *GraphSink) Finish() error {
	if s.finished {
		return nil
	}
	if s.active {
		return NewError(ErrorInvalidSource, "cannot finish while scope is active")
	}
	if !s.ended {
		return NewError(ErrorInvalidSource, "cannot finish before scope end")
	}

	idToIndex := make(map[TermID]int, len(s.termOrder))
	for idx, id := range s.termOrder {
		idToIndex[id] = idx
	}
	if err := s.checkTripleNesting(); err != nil {
		return err
	}

	graph := &model.Graph{}
	explicitReifiers := map[int]model.Triple3{}
	for _, event := range s.reifiers {
		rid, err := lookupTermID(idToIndex, event.reifier, "reifier")
		if err != nil {
			return err
		}
		triple, err := s.modelTriple(idToIndex, event.triple, "reifier")
		if err != nil {
			return err
		}
		if existing, ok := explicitReifiers[rid]; ok && existing != triple {
			return NewError(ErrorInvalidSource, fmt.Sprintf("reifier %d rebound", event.reifier))
		}
		explicitReifiers[rid] = triple
	}

	impliedReifiers := map[int]model.Triple3{}
	var impliedOrder []int
	for _, id := range s.termOrder {
		eventTerm := s.terms[id]
		term, implied, err := s.modelTerm(idToIndex, id, eventTerm)
		if err != nil {
			return err
		}
		graph.Terms = append(graph.Terms, term)
		if implied != nil {
			if explicit, ok := explicitReifiers[implied.rid]; ok {
				if explicit != implied.triple {
					return NewError(
						ErrorInvalidSource,
						fmt.Sprintf("triple term reifier %d conflicts with explicit reifier event", implied.rid),
					)
				}
				continue
			}
			if existing, ok := impliedReifiers[implied.rid]; ok {
				if existing != implied.triple {
					return NewError(ErrorInvalidSource, fmt.Sprintf("triple term reifier %d rebound", implied.rid))
				}
				continue
			}
			impliedReifiers[implied.rid] = implied.triple
			impliedOrder = append(impliedOrder, implied.rid)
		}
	}
	for _, event := range s.reifiers {
		rid := idToIndex[event.reifier]
		graph.SetReifier(rid, explicitReifiers[rid])
	}
	for _, rid := range impliedOrder {
		graph.SetReifier(rid, impliedReifiers[rid])
	}
	for _, eventQuad := range s.quads {
		quad, err := s.modelQuad(idToIndex, eventQuad)
		if err != nil {
			return err
		}
		graph.Quads = append(graph.Quads, quad)
	}
	for _, eventAnnotation := range s.annotations {
		annotation, err := s.modelTriple(idToIndex, eventAnnotation, "annotation")
		if err != nil {
			return err
		}
		graph.Annotations = append(graph.Annotations, annotation)
	}
	for _, eventDiagnostic := range s.diagnostics {
		diagnostic, err := toModelDiagnostic(eventDiagnostic)
		if err != nil {
			return err
		}
		graph.Diagnostics = append(graph.Diagnostics, diagnostic)
	}
	s.graph = graph
	s.finished = true
	return nil
}

type impliedReifier struct {
	rid    int
	triple model.Triple3
}

func (s *GraphSink) modelTerm(idToIndex map[TermID]int, id TermID, event Term) (model.Term, *impliedReifier, error) {
	switch event.Kind {
	case TermIRI:
		return model.Term{Kind: model.Iri, Value: event.Value}, nil, nil
	case TermBlankNode:
		return model.Term{Kind: model.Bnode, Value: event.Value}, nil, nil
	case TermLiteral:
		term := model.Term{Kind: model.Literal, Value: event.Value, Lang: event.Language}
		if event.Direction != "" {
			if !event.Direction.Valid() {
				return model.Term{}, nil, NewError(ErrorInvalidSource, fmt.Sprintf("literal term %d has invalid direction %q", id, event.Direction))
			}
			term.Direction = string(event.Direction)
		}
		if event.Datatype != nil {
			datatype, err := lookupTermID(idToIndex, *event.Datatype, "literal datatype")
			if err != nil {
				return model.Term{}, nil, err
			}
			term.Datatype = &datatype
		}
		return term, nil, nil
	case TermTriple:
		if event.Triple == nil {
			return model.Term{}, nil, NewError(ErrorInvalidSource, fmt.Sprintf("triple term %d has no triple payload", id))
		}
		triple, err := s.modelTriple(idToIndex, *event.Triple, "triple term")
		if err != nil {
			return model.Term{}, nil, err
		}
		var reifier int
		if event.Reifier != nil {
			reifier, err = lookupTermID(idToIndex, *event.Reifier, "triple term reifier")
			if err != nil {
				return model.Term{}, nil, err
			}
		} else {
			reifier, err = lookupTermID(idToIndex, id, "triple term self reifier")
			if err != nil {
				return model.Term{}, nil, err
			}
		}
		term := model.Term{Kind: model.Triple, Reifier: &reifier}
		return term, &impliedReifier{rid: reifier, triple: triple}, nil
	default:
		return model.Term{}, nil, NewError(ErrorInvalidSource, fmt.Sprintf("term %d has unknown event kind %q", id, event.Kind))
	}
}

func (s *GraphSink) modelTriple(idToIndex map[TermID]int, triple Triple, context string) (model.Triple3, error) {
	subject, err := lookupTermID(idToIndex, triple.Subject, context)
	if err != nil {
		return model.Triple3{}, err
	}
	predicate, err := lookupTermID(idToIndex, triple.Predicate, context)
	if err != nil {
		return model.Triple3{}, err
	}
	object, err := lookupTermID(idToIndex, triple.Object, context)
	if err != nil {
		return model.Triple3{}, err
	}
	return model.Triple3{S: subject, P: predicate, O: object}, nil
}

func (s *GraphSink) modelQuad(idToIndex map[TermID]int, quad Quad) (model.Quad, error) {
	subject, err := lookupTermID(idToIndex, quad.Subject, "quad")
	if err != nil {
		return model.Quad{}, err
	}
	predicate, err := lookupTermID(idToIndex, quad.Predicate, "quad")
	if err != nil {
		return model.Quad{}, err
	}
	object, err := lookupTermID(idToIndex, quad.Object, "quad")
	if err != nil {
		return model.Quad{}, err
	}
	out := model.Quad{S: subject, P: predicate, O: object}
	if quad.GraphName != nil {
		graphName, err := lookupTermID(idToIndex, *quad.GraphName, "quad graph name")
		if err != nil {
			return model.Quad{}, err
		}
		out.G = &graphName
	}
	return out, nil
}

// Graph returns the materialized graph after Finish succeeds.
func (s *GraphSink) Graph() (*model.Graph, error) {
	if !s.finished || s.graph == nil {
		return nil, NewError(ErrorCancelled, "graph sink has not finished")
	}
	return s.graph, nil
}

func lookupTermID(idToIndex map[TermID]int, id TermID, context string) (int, error) {
	if id > TermID(maxInt) {
		return 0, NewError(ErrorUnresolvedReference, fmt.Sprintf("%s references oversized term id %d", context, id))
	}
	idx, ok := idToIndex[id]
	if !ok {
		return 0, NewError(ErrorUnresolvedReference, fmt.Sprintf("%s references undeclared term id %d", context, id))
	}
	return idx, nil
}

func (s *GraphSink) checkTripleNesting() error {
	visiting := map[TermID]struct{}{}
	for _, id := range s.termOrder {
		if err := s.checkTermNesting(id, 0, visiting); err != nil {
			return err
		}
	}
	return nil
}

func (s *GraphSink) checkTermNesting(id TermID, depth int, visiting map[TermID]struct{}) error {
	if depth > s.TripleTermNestingLimit() {
		return NewError(
			ErrorTripleNestingLimit,
			fmt.Sprintf("quoted triple nesting exceeds configured limit %d", s.TripleTermNestingLimit()),
		)
	}
	eventTerm, ok := s.terms[id]
	if !ok {
		return NewError(ErrorUnresolvedReference, fmt.Sprintf("triple nesting references undeclared term id %d", id))
	}
	if eventTerm.Kind != TermTriple {
		return nil
	}
	if _, ok := visiting[id]; ok {
		return NewError(ErrorInvalidSource, fmt.Sprintf("cycle while checking triple term %d", id))
	}
	visiting[id] = struct{}{}
	defer delete(visiting, id)
	if eventTerm.Triple == nil {
		return NewError(ErrorInvalidSource, fmt.Sprintf("triple term %d has no triple payload", id))
	}
	nextDepth := depth + 1
	if err := s.checkTermNesting(eventTerm.Triple.Subject, nextDepth, visiting); err != nil {
		return err
	}
	if err := s.checkTermNesting(eventTerm.Triple.Predicate, nextDepth, visiting); err != nil {
		return err
	}
	return s.checkTermNesting(eventTerm.Triple.Object, nextDepth, visiting)
}

func toModelDiagnostic(diagnostic Diagnostic) (model.Diagnostic, error) {
	out := model.Diagnostic{Code: diagnostic.Code, Detail: diagnostic.Detail}
	if diagnostic.Location != nil && diagnostic.Location.FrameIndex != nil {
		if *diagnostic.Location.FrameIndex > uint64(maxInt) {
			return model.Diagnostic{}, NewError(ErrorInvalidSource, "diagnostic frame index exceeds int range")
		}
		frameIndex := int(*diagnostic.Location.FrameIndex)
		out.FrameIndex = &frameIndex
	}
	return out, nil
}

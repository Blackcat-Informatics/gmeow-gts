"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfFold,
	#superclass : #Object,
	#instVars : [ 'graph', 'pool', 'dictionary', 'blanks', 'values', 'lines', 'suppression', 'textBytes' ],
	#category : #'Gts-Core'
}

{ #category : #folding }
GtsV2RdfFold class >> read: wire [
	^ self read: wire visible: true
]

{ #category : #folding }
GtsV2RdfFold class >> read: wire visible: visible [
	^ self new read: wire visible: visible
]

{ #category : #folding }
GtsV2RdfFold >> read: wire visible: visible [
	graph := GtsGraph new.
	pool := Dictionary new.
	values := GtsV2RdfValues new.
	lines := OrderedDictionary new.
	suppression := GtsV2RdfSuppression new.
	textBytes := 0.
	wire eventsDo: [ :event |
		[ [ self observe: event ] on: GtsV2FramingError do: [ :error |
			GtsV2EnvelopeError signal: 'invalid decoded RDF record' ] ]
			on: GtsV2EnvelopeError
			do: [ :error | GtsArchiveError signal: 'byte ', event offset printString, ': ', error messageText ] ]
		retainDecoded: true.
	self projectLinesVisible: visible.
	self validateReachableTerms.
	^ graph
]

{ #category : #private }
GtsV2RdfFold >> observe: event [
	| type content bytes |
	event kind = #stop ifTrue: [
		GtsV2EnvelopeError signal: event problem ].
	(#(offsets locator) includes: event kind) ifTrue: [ ^ self ].
	event frameType = 16 ifTrue: [
		(event frame isTransparent not and: [ event link = #broken ])
			ifTrue: [ GtsV2EnvelopeError signal: 'broken_link' ].
		(#(linked healed) includes: event link) ifTrue: [
			event effectiveIdentifier ifNotNil: [ :identifier | suppression record: identifier segment: blanks ] ].
		^ self ].
	event problem ifNotNil: [ :problem | GtsV2EnvelopeError signal: problem ].
	event startsSegment ifTrue: [
		(event effectiveIdentifier isNil or: [ event opener registry ~= 1 ])
			ifTrue: [ GtsV2EnvelopeError signal: 'unsupported opener' ].
		dictionary := OrderedCollection new.
		blanks := Dictionary new.
		values beginSegment.
		^ self ].
	event kind = #frame ifFalse: [ GtsV2EnvelopeError signal: 'unreadable item' ].
	(#(linked healed) includes: event link) ifFalse: [ GtsV2EnvelopeError signal: 'broken_link' ].
	event effectiveIdentifier ifNotNil: [ :identifier | suppression record: identifier segment: blanks ].
	event storedMatches = false ifTrue: [ GtsV2EnvelopeError signal: 'stored_check_mismatch' ].
	event payload state = #verified ifFalse: [
		GtsV2EnvelopeError signal: (event payload reason ifNil: [ 'payload_unavailable' ]) ].
	type := event frameType.
	content := event frame content.
	bytes := event payload decoded.
	type = 1 ifTrue: [ ^ GtsV2RdfTerms new read: content bytes: bytes fold: self values: values ].
	(type between: 2 and: 5) ifTrue: [
		^ GtsV2RdfRows readType: type content: content bytes: bytes into: self ].
	type = 7 ifTrue: [
		^ suppression read: content bytes: bytes dictionary: dictionary segment: blanks ].
	(#(8 9 10 11 13) includes: type) ifTrue: [
		^ values readType: type content: content bytes: bytes ].
	(#(6 12 14 15) includes: type) ifTrue: [ ^ self ].
	GtsV2EnvelopeError signal: 'unsupported RDF frame type ', type printString
]

{ #category : #accessing }
GtsV2RdfFold >> dictionarySize [
	^ dictionary size
]

{ #category : #validation }
GtsV2RdfFold >> chargeText: length [
	textBytes := textBytes + length.
	textBytes > 268435456 ifTrue: [ GtsV2EnvelopeError signal: 'RDF text exceeds projection capacity' ]
]

{ #category : #terms }
GtsV2RdfFold >> resolveReference: digest length: length octets: octets values: store [
	octets ifTrue: [
		self chargeText: length * 2.
		^ store octetsFor: digest length: length ].
	self chargeText: length.
	^ store textFor: digest length: length
]

{ #category : #terms }
GtsV2RdfFold >> addBlank: label [
	dictionary add: (blanks at: label ifAbsentPut: [
		| identifier |
		identifier := graph termCount.
		graph addTerm: (GtsTerm blankNode: 'b', identifier printString).
		identifier ])
]

{ #category : #terms }
GtsV2RdfFold >> addIri: text [
	dictionary add: (self internIri: text)
]

{ #category : #terms }
GtsV2RdfFold >> internIri: text [
	^ self intern: { #iri. text } term: [ GtsTerm iri: text ]
]

{ #category : #terms }
GtsV2RdfFold >> addLiteral: text datatype: local [
	self addTypedLiteral: text datatypeId: (self resolve: local kinds: #(iri))
]

{ #category : #terms }
GtsV2RdfFold >> addLiteral: text datatypeIri: iri [
	self addTypedLiteral: text datatypeId: (self internIri: iri)
]

{ #category : #private }
GtsV2RdfFold >> addTypedLiteral: text datatypeId: datatype [
	dictionary add: (self intern: { #literal. text. datatype } term: [
		GtsTerm literal: text language: nil datatype: (
			(graph termAtId: datatype) value = GtsGraph xsdString ifFalse: [ datatype ]) ])
]

{ #category : #terms }
GtsV2RdfFold >> addLiteral: text language: language direction: direction [
	dictionary add: (self intern: { #language. text. language. direction } term: [
		GtsTerm literal: text language: language direction: direction datatype: nil ])
]

{ #category : #terms }
GtsV2RdfFold >> addTripleSubject: subject predicate: predicate object: object [
	dictionary add: (self internTriple:
		{ (self node: subject). (self resolve: predicate kinds: #(iri)). (self resolve: object kinds: nil) })
]

{ #category : #terms }
GtsV2RdfFold >> internTriple: components [
	^ self intern: { #triple. components } term: [ GtsTerm tripleComponents: components ]
]

{ #category : #accessing }
GtsV2RdfFold >> kindOf: identifier [
	^ (graph termAtId: identifier) kind
]

{ #category : #private }
GtsV2RdfFold >> intern: key term: aBlock [
	^ pool at: key ifAbsentPut: [
		| identifier |
		identifier := graph termCount.
		graph addTerm: aBlock value.
		identifier ]
]

{ #category : #accessing }
GtsV2RdfFold >> node: local [
	^ self resolve: local kinds: #(iri blankNode)
]

{ #category : #accessing }
GtsV2RdfFold >> resolve: local kinds: kinds [
	| identifier |
	identifier := dictionary at: local + 1 ifAbsent: [ GtsV2EnvelopeError signal: 'undefined RDF term' ].
	(kinds isNotNil and: [ (kinds includes: (graph termAtId: identifier) kind) not ])
		ifTrue: [ GtsV2EnvelopeError signal: 'position_constraint' ].
	^ identifier
]

{ #category : #rows }
GtsV2RdfFold >> addLine: type terms: terms graph: name [
	lines at: { type. (name ifNil: [ terms ] ifNotNil: [ terms copyWith: (self node: name) ]) } put: nil
]

{ #category : #private }
GtsV2RdfFold >> projectLinesVisible: visible [
	| rows |
	suppression resolveWith: self graph: graph.
	rows := Set new.
	lines keysDo: [ :line |
		(visible not or: [ (suppression hides: line first terms: line second) not ]) ifTrue: [
			| row |
			row := line second.
			line first = 3 ifTrue: [
				row := { row first. (self internIri: 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies') },
					(row allButFirst) ].
			(rows includes: row) ifFalse: [
				rows add: row.
				graph addQuad: row ] ] ]
]

{ #category : #validation }
GtsV2RdfFold >> validateReachableTerms [
	| open visited measurements remaining |
	open := OrderedCollection new.
	graph quads do: [ :row | open addAll: row ].
	visited := Set new.
	[ open isNotEmpty ] whileTrue: [
		| identifier term |
		identifier := open removeLast.
		(visited includes: identifier) ifFalse: [
			visited add: identifier.
			term := graph termAtId: identifier.
			term kind = #iri ifTrue: [ GtsFromNQuads requireAbsoluteIri: term value ].
			term kind = #triple ifTrue: [ open addAll: term triple ].
			term kind = #literal ifTrue: [
				term datatype ifNotNil: [ :datatype | open add: datatype ].
				term language ifNotNil: [ :tag | (GtsNQuadsTokenizer on: '') validateLanguage: tag ] ] ] ].
	measurements := Dictionary new.
	visited asSortedCollection do: [ :identifier |
		measurements at: identifier put: (self measure: (graph termAtId: identifier) using: measurements) ].
	remaining := 268435456.
	graph quads do: [ :row |
		remaining := remaining - row size - 2.
		row do: [ :identifier | remaining := remaining - (measurements at: identifier) first ].
		remaining < 0 ifTrue: [ GtsArchiveError signal: 'N-Quads output exceeds projection capacity' ] ]
]

{ #category : #validation }
GtsV2RdfFold >> measure: term using: measurements [
	| length depth |
	term kind = #triple ifTrue: [
		length := 10.
		depth := 1.
		term triple do: [ :identifier |
			| child |
			child := measurements at: identifier.
			length := length + child first.
			depth := depth max: child second + 1 ].
		(depth > 256 or: [ length > 268435456 ]) ifTrue: [
			GtsArchiveError signal: 'N-Quads triple term exceeds projection capacity' ].
		^ { length. depth } ].
	length := (self textSize: term value escaped: term kind = #literal) + 2.
	term kind = #literal ifTrue: [
		term datatype ifNotNil: [ :id | length := length + 2 + (measurements at: id) first ].
		term language ifNotNil: [ :tag |
			length := length + 1 + tag size.
			term direction ifNotNil: [ :direction | length := length + 2 + direction size ] ] ].
	^ { length. 0 }
]

{ #category : #validation }
GtsV2RdfFold >> textSize: text escaped: escaped [
	| bytes |
	bytes := 0.
	text do: [ :character |
		| code |
		code := character asciiValue.
		bytes := bytes + ((escaped and: [ code < 32 or: [ code = 34 or: [ code = 92 ] ] ])
			ifTrue: [ (#(9 10 13 34 92) includes: code) ifTrue: [ 2 ] ifFalse: [ 6 ] ]
			ifFalse: [ code < 128 ifTrue: [ 1 ] ifFalse: [
				code < 2048 ifTrue: [ 2 ] ifFalse: [ code < 65536 ifTrue: [ 3 ] ifFalse: [ 4 ] ] ] ]) ].
	^ bytes
]

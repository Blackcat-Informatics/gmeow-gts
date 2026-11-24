Class {
	#name : #GtsFromNQuads,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsFromNQuads class >> atom: node in: interner [
	| datatypeId ids key terms term termId |
	ids := interner at: 'ids'.
	terms := interner at: 'terms'.
	key := {
		'atom'.
		node at: 'kind'.
		node at: 'value'.
		node at: 'language' ifAbsent: [ nil ].
		node at: 'direction' ifAbsent: [ nil ].
		node at: 'datatype' ifAbsent: [ nil ] }.
	ids at: key ifPresent: [ :existing | ^ existing ].
	(node at: 'kind') = #literal
		ifTrue: [
			datatypeId := (node includesKey: 'datatype')
				ifTrue: [ self atom: (self iriNode: (node at: 'datatype')) in: interner ]
				ifFalse: [ nil ].
			term := GtsTerm
				literal: (node at: 'value')
				language: (node at: 'language' ifAbsent: [ nil ])
				direction: (node at: 'direction' ifAbsent: [ nil ])
				datatype: datatypeId ]
		ifFalse: [
			(node at: 'kind') = #blankNode
				ifTrue: [ term := GtsTerm blankNode: (node at: 'value') ]
				ifFalse: [ term := GtsTerm iri: (node at: 'value') ] ].
	termId := terms size.
	terms add: term.
	ids at: key put: termId.
	^ termId
]

{ #category : #private }
GtsFromNQuads class >> interner [
	^ Dictionary newFromPairs: {
		'ids'. Dictionary new.
		'terms'. OrderedCollection new }
]

{ #category : #private }
GtsFromNQuads class >> iriNode: iri [
	^ Dictionary newFromPairs: {
		'nodeType'. 'atom'.
		'kind'. #iri.
		'value'. iri }
]

{ #category : #private }
GtsFromNQuads class >> isAtom: node [
	^ (node at: 'nodeType') = 'atom'
]

{ #category : #private }
GtsFromNQuads class >> isAtom: node kind: kind [
	^ (self isAtom: node) and: [ (node at: 'kind') = kind ]
]

{ #category : #private }
GtsFromNQuads class >> isTriple: node [
	^ (node at: 'nodeType') = 'triple'
]

{ #category : #private }
GtsFromNQuads class >> node: node in: interner reifiers: reifiers [
	"A bare quoted triple used as a quad component interns as a SELF-DESCRIBING triple
	term carrying its own (s p o) (§7.3). An unreified triple term is a value, not a
	statement — it mints NO reifier and emits NO reifies row, so it round-trips as
	itself."
	(self isAtom: node) ifTrue: [ ^ self atom: node in: interner ].
	^ self internTriple: {
		(self node: (node at: 's') in: interner reifiers: reifiers).
		(self node: (node at: 'p') in: interner reifiers: reifiers).
		(self node: (node at: 'o') in: interner reifiers: reifiers) } in: interner
]

{ #category : #private }
GtsFromNQuads class >> internTriple: components in: interner [
	| ids key termId terms |
	ids := interner at: 'ids'.
	terms := interner at: 'terms'.
	key := { 'triple' }, components.
	ids at: key ifPresent: [ :existing | ^ existing ].
	termId := terms size.
	terms add: (GtsTerm tripleComponents: components).
	ids at: key put: termId.
	^ termId
]

{ #category : #parsing }
GtsFromNQuads class >> parseStatements: text [
	^ self parseStatements: text versionHints: false
]

{ #category : #parsing }
GtsFromNQuads class >> parseStatements: text versionHints: versionHints [
	| statements |
	statements := OrderedCollection new.
	text lines do: [ :raw |
		| line nodes tokenizer |
		line := raw trimBoth.
		(line isEmpty or: [ line beginsWith: '#' ]) ifFalse: [
			tokenizer := GtsNQuadsTokenizer on: line.
			(versionHints and: [ line beginsWith: 'VERSION' ])
				ifTrue: [ tokenizer consumeVersionHint ]
				ifFalse: [
					nodes := OrderedCollection new.
					[ tokenizer atEnd ] whileFalse: [ nodes add: tokenizer node ].
					tokenizer consumeStatementTerminator.
					(nodes size = 3 or: [ nodes size = 4 ])
						ifFalse: [ self error: 'expected 3 or 4 terms, got ', nodes size asString, ': ', line ].
					self validate: nodes asArray line: line.
					statements add: nodes asArray ] ] ].
	^ statements asArray
]

{ #category : #private }
GtsFromNQuads class >> setReifier: reifierId spo: spo graph: graphId in: reifiers [
	"Record one `r rdf:reifies <<( s p o )>>` statement.

	rdf:reifies is NOT functional (RDF 1.2 / §7.3): two bindings on one reifier are
	legal input and BOTH are kept. Only a byte-identical repeat of a row collapses."
	| entry |
	1 to: reifiers size do: [ :index |
		(((reifiers at: index) at: 'rid') = reifierId
			and: [ ((reifiers at: index) at: 'spo') = spo
			and: [ ((reifiers at: index) at: 'graph' ifAbsent: [ nil ]) = graphId ] ])
			ifTrue: [ ^ self ] ].
	entry := Dictionary newFromPairs: {
		'rid'. reifierId.
		'spo'. spo }.
	graphId ifNotNil: [ entry at: 'graph' put: graphId ].
	reifiers add: entry
]

{ #category : #private }
GtsFromNQuads class >> setReifier: reifierId spo: spo in: reifiers [
	^ self setReifier: reifierId spo: spo graph: nil in: reifiers
]

{ #category : #converting }
GtsFromNQuads class >> fromString: text [
	| interner quads reifiers statements writer |
	statements := self parseStatements: text.
	interner := self interner.
	reifiers := OrderedCollection new.
	quads := OrderedCollection new.
	statements do: [ :nodes |
		| graphName object predicate subject |
		subject := nodes at: 1.
		predicate := nodes at: 2.
		object := nodes at: 3.
		graphName := nodes size = 4 ifTrue: [ nodes at: 4 ] ifFalse: [ nil ].
		((self isAtom: subject)
			and: [
				(self isAtom: predicate kind: #iri)
					and: [
						((predicate at: 'value') = 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies')
							and: [ self isTriple: object ] ] ])
			ifTrue: [
				self
					setReifier: (self atom: subject in: interner)
					spo: {
						self node: (object at: 's') in: interner reifiers: reifiers.
						self node: (object at: 'p') in: interner reifiers: reifiers.
						self node: (object at: 'o') in: interner reifiers: reifiers }
					graph: (graphName ifNotNil: [ self node: graphName in: interner reifiers: reifiers ])
					in: reifiers ]
			ifFalse: [
				| quad |
				quad := {
					self node: subject in: interner reifiers: reifiers.
					self node: predicate in: interner reifiers: reifiers.
					self node: object in: interner reifiers: reifiers }.
				graphName ifNotNil: [
					quad := quad, { self node: graphName in: interner reifiers: reifiers } ].
				quads add: quad ] ].
	writer := GtsWriter forProfile: 'dist'.
	(interner at: 'terms') isEmpty ifFalse: [ writer addTerms: (interner at: 'terms') asArray ].
	quads isEmpty ifFalse: [ writer addQuads: quads asArray ].
	reifiers isEmpty ifFalse: [ writer addReifiers: reifiers asArray ].
	^ writer bytes
]

{ #category : #converting }
GtsFromNQuads class >> v2FromString: text [
	| interner quads seen |
	interner := self interner.
	quads := OrderedCollection new.
	seen := Set new.
	(self parseStatements: text versionHints: true) do: [ :statement |
		| quad |
		self requireV2Subject: statement first.
		quad := statement collect: [ :node | self v2Node: node in: interner ].
		(seen includes: quad) ifFalse: [
			seen add: quad.
			quads add: quad ] ].
	^ GtsV2RdfWriter terms: (interner at: 'terms') quads: quads
]

{ #category : #private }
GtsFromNQuads class >> v2Node: node in: interner [
	| kind datatype |
	(self isTriple: node) ifTrue: [
		self requireV2Subject: (node at: 's').
		(self isAtom: (node at: 'p') kind: #iri) ifFalse: [
			GtsParserError signal: 'a quoted triple predicate must be an IRI' ].
		^ self internTriple: {
			(self v2Node: (node at: 's') in: interner).
			(self v2Node: (node at: 'p') in: interner).
			(self v2Node: (node at: 'o') in: interner) } in: interner ].
	kind := node at: 'kind'.
	kind = #iri ifTrue: [ self requireAbsoluteIri: (node at: 'value') ].
	(kind = #literal and: [ (node includesKey: 'language') not ]) ifTrue: [
		datatype := node at: 'datatype' ifAbsent: [ GtsGraph xsdString ].
		self requireAbsoluteIri: datatype.
		^ self atom: (node copy at: 'datatype' put: datatype; yourself) in: interner ].
	^ self atom: node in: interner
]

{ #category : #private }
GtsFromNQuads class >> requireV2Subject: node [
	((self isAtom: node kind: #iri) or: [ self isAtom: node kind: #blankNode ]) ifFalse: [
		GtsParserError signal: 'a GTS v2 subject must be an IRI or a blank node' ]
]

{ #category : #private }
GtsFromNQuads class >> requireAbsoluteIri: iri [
	| colon scheme |
	colon := iri indexOf: $:.
	colon > 1 ifFalse: [ GtsParserError signal: 'N-Quads requires an absolute IRI' ].
	scheme := iri copyFrom: 1 to: colon - 1.
	((self isAsciiLetter: scheme first) and: [ scheme allSatisfy: [ :character |
		(self isAsciiLetter: character) or: [
			(character between: $0 and: $9) or: [ '+.-' includes: character ] ] ] ])
		ifFalse: [ GtsParserError signal: 'invalid N-Quads IRI scheme' ].
	(iri anySatisfy: [ :character |
		character asciiValue <= 32 or: [ '<>"{}|^\`' includes: character ] ])
		ifTrue: [ GtsParserError signal: 'invalid whitespace or delimiter in N-Quads IRI' ]
]

{ #category : #private }
GtsFromNQuads class >> isAsciiLetter: character [
	^ (character between: $A and: $Z) or: [ character between: $a and: $z ]
]

{ #category : #private }
GtsFromNQuads class >> validate: nodes line: line [
	((self isAtom: (nodes at: 1) kind: #iri)
		or: [ (self isAtom: (nodes at: 1) kind: #blankNode)
		or: [ self isTriple: (nodes at: 1) ] ])
		ifFalse: [ self error: 'invalid subject term: ', line ].
	(self isAtom: (nodes at: 2) kind: #iri)
		ifFalse: [ self error: 'predicate must be IRI: ', line ].
	((self isAtom: (nodes at: 3) kind: #iri)
		or: [ (self isAtom: (nodes at: 3) kind: #blankNode)
		or: [ (self isAtom: (nodes at: 3) kind: #literal)
		or: [ self isTriple: (nodes at: 3) ] ] ])
		ifFalse: [ self error: 'invalid object term: ', line ].
	nodes size = 4 ifTrue: [
		((self isAtom: (nodes at: 4) kind: #iri)
			or: [ self isAtom: (nodes at: 4) kind: #blankNode ])
			ifFalse: [ self error: 'invalid graph name term: ', line ] ]
]

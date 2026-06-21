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
	| ids key object predicate reifierId subject terms |
	(self isAtom: node) ifTrue: [ ^ self atom: node in: interner ].
	subject := self node: (node at: 's') in: interner reifiers: reifiers.
	predicate := self node: (node at: 'p') in: interner reifiers: reifiers.
	object := self node: (node at: 'o') in: interner reifiers: reifiers.
	ids := interner at: 'ids'.
	terms := interner at: 'terms'.
	key := { 'triple'. subject. predicate. object }.
	ids at: key ifPresent: [ :existing | ^ existing ].
	reifierId := terms size.
	terms add: (GtsTerm tripleReifier: reifierId).
	ids at: key put: reifierId.
	self setReifier: reifierId spo: { subject. predicate. object } in: reifiers.
	^ reifierId
]

{ #category : #parsing }
GtsFromNQuads class >> parseStatements: text [
	| statements |
	statements := OrderedCollection new.
	text lines do: [ :raw |
		| line nodes tokenizer |
		line := raw trimBoth.
		(line isEmpty or: [ line beginsWith: '#' ]) ifFalse: [
			tokenizer := GtsNQuadsTokenizer on: line.
			nodes := OrderedCollection new.
			[ tokenizer atEnd ] whileFalse: [ nodes add: tokenizer node ].
			tokenizer consumeStatementTerminator.
			(nodes size = 3 or: [ nodes size = 4 ])
				ifFalse: [ self error: 'expected 3 or 4 terms, got ', nodes size asString, ': ', line ].
			self validate: nodes asArray line: line.
			statements add: nodes asArray ] ].
	^ statements asArray
]

{ #category : #private }
GtsFromNQuads class >> setReifier: reifierId spo: spo in: reifiers [
	1 to: reifiers size do: [ :index |
		((reifiers at: index) at: 'rid') = reifierId ifTrue: [
			reifiers at: index put: (Dictionary newFromPairs: {
				'rid'. reifierId.
				'spo'. spo }).
			^ self ] ].
	reifiers add: (Dictionary newFromPairs: {
		'rid'. reifierId.
		'spo'. spo })
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
		(graphName isNil
			and: [ (self isAtom: subject)
			and: [ (self isAtom: predicate kind: #iri)
			and: [ (predicate at: 'value') = 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies'
			and: [ self isTriple: object ] ] ] ])
			ifTrue: [
				self
					setReifier: (self atom: subject in: interner)
					spo: {
						self node: (object at: 's') in: interner reifiers: reifiers.
						self node: (object at: 'p') in: interner reifiers: reifiers.
						self node: (object at: 'o') in: interner reifiers: reifiers }
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
